mod async_index;

use self::async_index::{AsyncFsIndex, Update};
use crate::Context;
use anyhow::Result;
use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket},
        ConnectInfo, Path, Query, State, WebSocketUpgrade,
    },
    http::{header::CONTENT_TYPE, HeaderValue, Method, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Json, Router,
};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use clap::Args;
use cooklang::{error::PassResult, CooklangParser};
use futures::{sink::SinkExt, stream::StreamExt as _};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::SystemTime};
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Allow external connections
    #[arg(long)]
    host: bool,

    /// Set http server port
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Open browser on start
    #[cfg(feature = "ui")]
    #[arg(long, default_value_t = false)]
    open: bool,
}

#[tokio::main]
pub async fn run(ctx: Context, args: ServeArgs) -> Result<()> {
    let state = build_state(ctx)?;

    let app = Router::new().nest("/api", api(&state)?);

    #[cfg(feature = "ui")]
    let app = app.merge(ui::ui());

    let app = app.with_state(state).layer(
        CorsLayer::new()
            .allow_origin("*".parse::<HeaderValue>().unwrap())
            .allow_methods([Method::GET]),
    );

    let addr = if args.host {
        SocketAddr::from(([0, 0, 0, 0], args.port))
    } else {
        SocketAddr::from(([127, 0, 0, 1], args.port))
    };

    info!("Listening on {addr}");

    #[cfg(feature = "ui")]
    if args.open {
        let port = args.port;
        let url = format!("http://localhost:{port}");
        info!("Serving web UI on {url}");
        tokio::task::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Err(e) = open::that(url) {
                tracing::error!("Could not open the web browser: {e}");
            }
        });
    }

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    info!("Server stopped");

    Ok(())
}

fn build_state(ctx: Context) -> Result<Arc<AppState>> {
    ctx.parser()?;
    let Context {
        parser,
        recipe_index,
        base_path,
        config,
        global_config,
        ..
    } = ctx;
    let parser = parser.into_inner().unwrap();
    let (recipe_index, updates) = AsyncFsIndex::new(recipe_index)?;

    // from mpsc to debounced broadcast
    let (updates_tx, updates_rx) = broadcast::channel(1);
    tokio::spawn(async move {
        let mut debounced_updates = debounced::debounced(
            tokio_stream::wrappers::ReceiverStream::new(updates),
            std::time::Duration::from_millis(500),
        );
        while let Some(u) = debounced_updates.next().await {
            let _ = updates_tx.send(u);
        }
    });

    Ok(Arc::new(AppState {
        parser,
        base_path,
        recipe_index,
        updates_stream: updates_rx,
        config,
        editor_command: global_config.editor_command,
    }))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    };

    info!("Stopping server");
}

#[cfg(feature = "ui")]
mod ui {
    use super::*;
    use rust_embed::RustEmbed;

    pub fn ui() -> Router<Arc<AppState>> {
        Router::new()
            .route("/ui_config", get(config))
            .fallback(static_ui)
    }

    #[derive(RustEmbed)]
    #[folder = "./ui/build/"]
    struct Assets;

    async fn static_ui(uri: Uri) -> impl axum::response::IntoResponse {
        use axum::response::IntoResponse;

        const INDEX_HTML: &str = "index.html";

        fn index_html() -> impl axum::response::IntoResponse {
            Assets::get(INDEX_HTML)
                .map(|content| {
                    let body = axum::body::boxed(axum::body::Full::from(content.data));
                    Response::builder()
                        .header(axum::http::header::CONTENT_TYPE, "text/html")
                        .body(body)
                        .unwrap()
                })
                .ok_or(StatusCode::NOT_FOUND)
        }

        let path = uri.path().trim_start_matches('/');

        if path.is_empty() || path == INDEX_HTML {
            return Ok(index_html().into_response());
        }

        match Assets::get(path) {
            Some(content) => {
                let body = axum::body::boxed(axum::body::Full::from(content.data));
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Ok(Response::builder()
                    .header(axum::http::header::CONTENT_TYPE, mime.as_ref())
                    .body(body)
                    .unwrap())
            }
            None => {
                if path.contains('.') {
                    return Err(StatusCode::NOT_FOUND);
                }
                Ok(index_html().into_response())
            }
        }
    }

    async fn config(State(state): State<Arc<AppState>>) -> impl axum::response::IntoResponse {
        Json(state.config.ui.clone())
    }
}

pub struct AppState {
    parser: CooklangParser,
    base_path: Utf8PathBuf,
    recipe_index: AsyncFsIndex,
    updates_stream: broadcast::Receiver<Update>,
    config: crate::config::Config,
    editor_command: Option<Vec<String>>,
}

fn api(state: &AppState) -> Result<Router<Arc<AppState>>> {
    let router = Router::new()
        .nest_service(
            "/src",
            ServiceBuilder::new()
                .layer(middleware::from_fn(filter_files))
                .layer(middleware::from_fn(cook_mime_type))
                .service(tower_http::services::ServeDir::new(&state.base_path)),
        )
        .route("/updates", get(ws_handler))
        .route("/recipe", get(all_recipes))
        .route("/recipe/metadata", get(all_recipes_metadata))
        .route("/recipe/*path", get(recipe))
        .route("/recipe/metadata/*path", get(recipe_metadata))
        .route("/recipe/open_editor/*path", get(open_editor));

    Ok(router)
}

async fn filter_files<B>(req: Request<B>, next: Next<B>) -> impl axum::response::IntoResponse {
    let path = req.uri().path();
    let (_, ext) = path.rsplit_once('.').ok_or(StatusCode::NOT_FOUND)?;
    if ext == "cook" || cooklang_fs::IMAGE_EXTENSIONS.contains(&ext) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn cook_mime_type<B>(req: Request<B>, next: Next<B>) -> Response {
    let is_dot_cook = req.uri().path().ends_with(".cook");
    let mut res = next.run(req).await;
    if is_dot_cook {
        res.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );
    }
    res
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(who): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
) -> Response {
    tracing::debug!("Preparing web socket connection to {who}");
    ws.on_upgrade(move |socket| handle_ws_socket(socket, who, state.updates_stream.resubscribe()))
}

async fn handle_ws_socket(
    mut socket: WebSocket,
    who: SocketAddr,
    mut updates_stream: broadcast::Receiver<Update>,
) {
    tracing::info!("Established ws connection with {who}");
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        tracing::trace!("Pinged {who}");
    } else {
        tracing::warn!("Could not send ping {who}");
        return;
    }

    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        while let Ok(update) = updates_stream.recv().await {
            if sender
                .send(Message::Text(serde_json::to_string(&update).unwrap()))
                .await
                .is_err()
            {
                return;
            }
        }

        let _ = sender
            .send(Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: "Server closed".into(),
            })))
            .await;
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => {
            tracing::debug!("Send task finish");
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            tracing::debug!("Recv task finish");
            send_task.abort();
        }
    }

    tracing::info!("Closed ws connection with {who}");
}

fn check_path(p: &str) -> Result<(), StatusCode> {
    let path = Utf8Path::new(p);
    if !path
        .components()
        .all(|c| matches!(c, Utf8Component::Normal(_)))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

fn clean_path(p: &Utf8Path, base_path: &Utf8Path) -> Utf8PathBuf {
    let p = p
        .strip_prefix(base_path)
        .expect("dir entry path not relative to base path");
    #[cfg(windows)]
    let p = Utf8PathBuf::from(p.to_string().replace('\\', "/"));
    #[cfg(not(windows))]
    let p = p.to_path_buf();
    p
}

fn images(entry: &cooklang_fs::RecipeEntry, base_path: &Utf8Path) -> Vec<cooklang_fs::Image> {
    let mut images = entry.images();
    images
        .iter_mut()
        .for_each(|i| i.path = clean_path(&i.path, base_path));
    images
}

async fn all_recipes(State(state): State<Arc<AppState>>) -> Result<Json<Vec<String>>, StatusCode> {
    let recipes = cooklang_fs::all_recipes(&state.base_path, state.config.max_depth)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|e| {
            clean_path(e.path(), &state.base_path)
                .with_extension("")
                .into_string()
        })
        .collect();
    Ok(Json(recipes))
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(default)]
struct ColorConfig {
    color: bool,
}

async fn all_recipes_metadata(
    State(state): State<Arc<AppState>>,
    Query(color): Query<ColorConfig>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let mut handles = Vec::new();
    for entry in cooklang_fs::all_recipes(&state.base_path, state.config.max_depth)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let state = Arc::clone(&state);
        handles.push(tokio::spawn(async move {
            let Ok(content) = tokio::fs::read_to_string(entry.path()).await else { return None; };
            let metadata = state.parser.parse_metadata(&content);
            let path = clean_path(entry.path(), &state.base_path);
            let report = Report::from_pass_result(metadata, path.as_str(), &content, color.color);
            let value = serde_json::json!({
                "name": entry.name(),
                "metadata": report,
                "path": path.with_extension(""),
                "src_path": path,
                "images": images(&entry, &state.base_path)
            });
            Some(value)
        }));
    }
    let mut recipes = Vec::new();
    for h in handles {
        if let Some(recipe) = h.await.ok().flatten() {
            recipes.push(recipe)
        }
    }
    Ok(Json(recipes))
}

#[derive(Deserialize)]
struct RecipeQuery {
    scale: Option<u32>,
    units: Option<cooklang::convert::System>,
}

async fn recipe(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<RecipeQuery>,
    Query(color): Query<ColorConfig>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    check_path(&path)?;

    let entry = state
        .recipe_index
        .get(path.to_string())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let content = tokio::fs::read_to_string(&entry.path())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let times = get_times(entry.path()).await?;

    let recipe = state
        .parser
        .parse(&content, entry.name())
        .try_map(|recipe| -> Result<_, StatusCode> {
            let mut scaled = if let Some(servings) = query.scale {
                recipe.scale(servings, state.parser.converter())
            } else {
                recipe.default_scale()
            };
            if let Some(system) = query.units {
                let errors = scaled.convert(system, state.parser.converter());
                if !errors.is_empty() {
                    tracing::warn!("Errors converting units: {errors:?}");
                }
            }
            Ok(scaled)
        })?
        .map(|r| {
            #[derive(Serialize)]
            struct ApiRecipe {
                #[serde(flatten)]
                recipe: cooklang::ScaledRecipe,
                grouped_ingredients: Vec<serde_json::Value>,
                timers_seconds: Vec<Option<cooklang::Value>>,
            }

            let grouped_ingredients = r
                .group_ingredients(state.parser.converter())
                .into_iter()
                .map(|entry| {
                    serde_json::json!({
                        "index": entry.index,
                        "quantity": entry.quantity.total().into_vec(),
                        "outcome": entry.outcome
                    })
                })
                .collect();
            let timers_seconds = r
                .timers
                .iter()
                .map(|t| {
                    t.quantity.clone().and_then(|mut q| {
                        if q.convert("s", state.parser.converter()).is_err() {
                            None
                        } else {
                            Some(q.value)
                        }
                    })
                })
                .collect();

            let api_recipe = ApiRecipe {
                recipe: r,
                grouped_ingredients,
                timers_seconds,
            };

            serde_json::to_value(api_recipe).unwrap()
        });
    let path = clean_path(entry.path(), &state.base_path);
    let report = Report::from_pass_result(recipe, path.as_str(), &content, color.color);
    let value = serde_json::json!({
        "recipe": report,
        "images": images(&entry, &state.base_path),
        "src_path": path,
        "modified": times.modified,
        "created": times.created,
    });

    Ok(Json(value))
}

async fn recipe_metadata(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(color): Query<ColorConfig>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    check_path(&path)?;

    let entry = state
        .recipe_index
        .get(path.to_string())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let content = tokio::fs::read_to_string(&entry.path())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let times = get_times(entry.path()).await?;

    let metadata = state.parser.parse_metadata(&content);
    let path = clean_path(entry.path(), &state.base_path);
    let report = Report::from_pass_result(metadata, path.as_str(), &content, color.color);
    let value = serde_json::json!({
        "name": entry.name(),
        "metadata": report,
        "path": path.with_extension(""),
        "src_path": path,
        "images": images(&entry, &state.base_path),
        "modified": times.modified,
        "created": times.created,
    });

    Ok(Json(value))
}

async fn open_editor(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    ConnectInfo(who): ConnectInfo<SocketAddr>,
) -> Result<(), StatusCode> {
    if !who.ip().is_loopback() {
        tracing::info!("Denied open editor request from '{who}': Not loopback ip");
        return Err(StatusCode::UNAUTHORIZED);
    }

    check_path(&path)?;

    let entry = state
        .recipe_index
        .get(path.to_string())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    tracing::info!("Opening editor for '{}'", entry.path());

    let args = if let Some(editor_command) = &state.editor_command {
        editor_command.iter().map(String::as_str).collect()
    } else {
        // TODO get system editor
        if cfg!(windows) {
            vec!["code.cmd", "-n"]
        } else {
            vec!["code", "-n"]
        }
    };
    let (editor, args) = (&args[0], &args[1..]);

    if tokio::process::Command::new(editor)
        .args(args)
        .arg(entry.path())
        .spawn()
        .is_err()
    {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(())
}

struct Times {
    modified: Option<u64>,
    created: Option<u64>,
}
async fn get_times(path: &Utf8Path) -> Result<Times, StatusCode> {
    fn f(st: std::io::Result<SystemTime>) -> Option<u64> {
        st.ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let modified = f(metadata.modified());
    let created = f(metadata.created());
    Ok(Times { modified, created })
}

#[derive(Serialize)]
struct Report<T> {
    value: Option<T>,
    warnings: Vec<String>,
    errors: Vec<String>,
    fancy_report: Option<String>,
}

impl<T> Report<T> {
    fn from_pass_result<E, W>(
        value: PassResult<T, E, W>,
        file_name: &str,
        source_code: &str,
        color: bool,
    ) -> Self
    where
        E: cooklang::error::RichError,
        W: cooklang::error::RichError,
    {
        let (value, w, e) = value.into_tuple();
        let warnings: Vec<_> = w.iter().map(|w| w.to_string()).collect();
        let errors: Vec<_> = e.iter().map(|e| e.to_string()).collect();
        let fancy_report = if warnings.is_empty() && errors.is_empty() {
            None
        } else {
            let mut buf = Vec::new();
            cooklang::error::Report::new(e, w)
                .write(file_name, source_code, false, color, &mut buf)
                .expect("Write fancy report");
            Some(String::from_utf8_lossy(&buf).into_owned())
        };
        Self {
            value,
            warnings,
            errors,
            fancy_report,
        }
    }
}

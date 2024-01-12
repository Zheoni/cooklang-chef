mod async_index;
mod handlers;
mod locale;

use self::{
    async_index::{AsyncFsIndex, Update},
    locale::{make_locale_store, LocaleStore},
};
use crate::Context;
use anyhow::{Context as _, Result};
use axum::{
    extract::Request,
    http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use camino::Utf8PathBuf;
use clap::Args;
use cooklang::CooklangParser;
use minijinja::{context, Environment, Value};
use rust_embed::RustEmbed;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::broadcast;
use tower::ServiceBuilder;
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
    #[arg(long, conflicts_with = "host", default_value_t = false)]
    open: bool,
}

#[tokio::main]
pub async fn run(ctx: Context, args: ServeArgs) -> Result<()> {
    let state = build_state(ctx).context("failed to build web server")?;
    let app = make_router(state);

    let addr = if args.host {
        SocketAddr::from(([0, 0, 0, 0], args.port))
    } else {
        SocketAddr::from(([127, 0, 0, 1], args.port))
    };

    info!("Listening on {addr}");

    if args.open {
        let url = format!("http://{}:{}", addr.ip(), addr.port());
        info!("Serving web UI on {url}");
        tokio::task::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Err(e) = open::that(url) {
                tracing::error!("Could not open the web browser: {e}");
            }
        });
    }

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    let app_service = app.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app_service).await.unwrap();

    info!("Server stopped");

    Ok(())
}

#[tracing::instrument(level = "debug", skip_all)]
fn make_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(handlers::index))
        .route("/d/*path", get(handlers::index))
        .route("/search", get(handlers::search))
        .route("/about", get(handlers::about))
        .route("/r/*path", get(handlers::recipe))
        .route("/updates", get(handlers::sse_updates))
        .route("/open_editor/*path", get(handlers::open_editor))
        .route("/convert_modal", post(handlers::convert_popover))
        .nest_service(
            "/src",
            ServiceBuilder::new()
                .layer(middleware::from_fn(filter_files))
                .layer(middleware::from_fn(cook_mime_type))
                .service(tower_http::services::ServeDir::new(".")),
        )
        .fallback(handlers::static_file)
        .with_state(state)
}

pub struct AppState {
    templates: Environment<'static>,
    locales: LocaleStore,
    parser: CooklangParser,
    base_path: Utf8PathBuf,
    recipe_index: AsyncFsIndex,
    updates_stream: broadcast::Receiver<Update>,
    config: crate::config::Config,
    editor_command: Option<Vec<String>>,
}

type S = Arc<AppState>;

#[tracing::instrument(level = "debug", skip_all)]
fn build_state(ctx: Context) -> Result<S> {
    ctx.parser()?;
    let Context {
        parser,
        recipe_index,
        base_path,
        config,
        chef_config,
        ..
    } = ctx;
    let parser = parser.into_inner().unwrap();
    let complete_index = recipe_index
        .index_all()
        .context("failed to index the recipes")?;
    let (recipe_index, updates) = AsyncFsIndex::new(complete_index);

    let locales = make_locale_store();
    let templates = make_template_env(&locales);

    Ok(Arc::new(AppState {
        templates,
        locales,
        parser,
        base_path,
        recipe_index,
        updates_stream: updates,
        config,
        editor_command: chef_config.editor().ok(),
    }))
}

fn make_template_env(locales: &LocaleStore) -> Environment<'static> {
    let mut env = Environment::new();

    env.set_loader(|name| match Templates::get(name) {
        Some(template) => {
            let source = String::from_utf8(template.data.into_owned()).expect("template not utf8");
            Ok(Some(source))
        }
        None => Ok(None),
    });

    env.add_global(
        "all_locales",
        Value::from_iter(locales.locales.iter().map(|l| {
            context! {
                code => l.code,
                lang => l.get("_lang")
            }
        })),
    );

    env.add_test("empty", |v: Value| v.len().is_some_and(|l| l == 0));

    env.add_function("youtube_videoid", |v: &str| {
        let re =
            crate::util::regex!(r"^(https?://)?(www\.)?youtube\.\w+/watch\?v=(?<videoid>[^&]*)$");

        match re.captures(v) {
            Some(caps) => Value::from(&caps["videoid"]),
            None => Value::from(()),
        }
    });

    env.add_function("step_ingredients", handlers::recipe::step_ingredients);

    env.add_filter(
        "or_else",
        |v: Value, default: Value| {
            if v.is_true() {
                v
            } else {
                default
            }
        },
    );

    env.add_filter(
        "select_value",
        |v: Value| -> Result<Value, minijinja::Error> {
            if v.kind() == minijinja::value::ValueKind::Map {
                let mut rv = HashMap::with_capacity(v.len().unwrap_or(0));
                for key in v.try_iter()? {
                    let value = v.get_item(&key).unwrap_or(Value::UNDEFINED);
                    if value.is_true() {
                        rv.insert(key, value);
                    }
                }
                Ok(Value::from(rv))
            } else {
                Err(minijinja::Error::new(
                    minijinja::ErrorKind::InvalidOperation,
                    "select_value only supports a mapping",
                ))
            }
        },
    );

    env.add_filter("unicode_fraction", |v: &str| {
        Value::from(match v {
            "1/2" => "½",
            "1/3" => "⅓",
            "2/3" => "⅔",
            "1/4" => "¼",
            "3/4" => "¾",
            "1/5" => "⅕",
            "2/5" => "⅖",
            "3/5" => "⅗",
            "4/5" => "⅘",
            "1/6" => "⅙",
            "5/6" => "⅚",
            "1/7" => "⅐",
            "1/8" => "⅛",
            "3/8" => "⅜",
            "5/8" => "⅝",
            "7/8" => "⅞",
            "1/9" => "⅑",
            "1/10" => "⅒",
            other => other,
        })
    });

    env.add_filter(
        "select_image",
        |v: Value, section: u32, step: u32| -> Result<Value, minijinja::Error> {
            for it in v.try_iter()? {
                let indexes = it.get_attr("indexes")?;
                if indexes.is_none() {
                    continue;
                }
                let i_sect: u32 = indexes.get_attr("section")?.try_into()?;
                let i_step: u32 = indexes.get_attr("step")?.try_into()?;
                if section == i_sect && step == i_step {
                    return Ok(it);
                }
            }
            Ok(Value::from(()))
        },
    );

    env
}

/// filters static files to only expose images and cook files
async fn filter_files(req: Request, next: Next) -> impl axum::response::IntoResponse {
    let path = req.uri().path();
    let (_, ext) = path.rsplit_once('.').ok_or(StatusCode::NOT_FOUND)?;
    if ext == "cook" || cooklang_fs::IMAGE_EXTENSIONS.contains(&ext) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// sets the mime type for .cook files based on extension
async fn cook_mime_type(req: Request, next: Next) -> Response {
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

fn get_cookie<'a>(headers: &'a HeaderMap, cookie: &str) -> Option<&'a str> {
    let key = format!("{cookie}=");
    let cookies = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())?;
    let lang_cookie = cookies.split(';').find(|c| c.trim().starts_with(&key))?;
    let (_, value) = lang_cookie.split_once('=').unwrap();
    Some(value)
}

#[derive(RustEmbed)]
#[folder = "ui/templates/"]
struct Templates;

#[derive(RustEmbed)]
#[folder = "ui/assets/"]
struct Assets;

#[derive(RustEmbed)]
#[folder = "ui/i18n/"]
#[include = "*.json"]
#[exclude = "_template.json"]
struct Locales;

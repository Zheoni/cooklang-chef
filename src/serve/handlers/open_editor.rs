use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use minijinja::context;

use crate::serve::{handlers::mj_ok, locale::UserLocale, S};

use super::check_path;

pub async fn open_editor(
    UserLocale(t): UserLocale,
    Path(path): Path<String>,
    State(state): State<S>,
    ConnectInfo(who): ConnectInfo<SocketAddr>,
) -> Response {
    if !who.ip().is_loopback() {
        tracing::warn!("Denied open editor request from '{who}': Not loopback ip");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let toast_html = |text_key: &str, color: &str| -> Result<Html<String>, minijinja::Error> {
        let tmpl = state.templates.get_template("components/oob_toast.html")?;
        let html = tmpl.render(context! {
            t,
            text_key,
            color
        })?;
        Ok(Html(html))
    };

    let err_html = || toast_html("openInEditor.error", "red");

    if let Err(err) = check_path(&path) {
        return (err, mj_ok!(err_html())).into_response();
    }

    let entry = match state.recipe_index.get(&path).await {
        Ok(entry) => entry,
        Err(_) => return (StatusCode::NOT_FOUND, mj_ok!(err_html())).into_response(),
    };

    tracing::info!("Opening editor for '{}'", entry.path());

    let args = if let Some(args) = &state.editor_command {
        args
    } else {
        return (StatusCode::SERVICE_UNAVAILABLE, mj_ok!(err_html())).into_response();
    };
    let (editor, args) = (&args[0], &args[1..]);

    if tokio::process::Command::new(editor)
        .args(args)
        .arg(entry.path())
        .spawn()
        .is_err()
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
    }

    mj_ok!(toast_html("openInEditor.success", "green")).into_response()
}

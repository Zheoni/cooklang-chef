use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use minijinja::context;

use crate::cmd::serve::{handlers::mj_ok, locale::UserLocale, S};

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

    // to be safe
    let editor_count = state
        .editor_count
        .load(std::sync::atomic::Ordering::Relaxed);
    if editor_count >= 10 {
        tracing::warn!("Too many editors");
        return (StatusCode::SERVICE_UNAVAILABLE, mj_ok!(err_html())).into_response();
    }

    match tokio::process::Command::new(editor)
        .args(args)
        .arg(entry.path())
        .spawn()
    {
        Ok(mut child) => {
            let s = Arc::clone(&state);
            s.editor_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            tokio::task::spawn(async move {
                let _ = child.wait().await;
                s.editor_count
                    .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            });
        }
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
        }
    }

    mj_ok!(toast_html("openInEditor.success", "green")).into_response()
}

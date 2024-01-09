use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
};

use crate::serve::Assets;

pub async fn static_file(uri: Uri, headers: HeaderMap) -> Result<Response, StatusCode> {
    const INDEX_HTML: &str = "index.html";

    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return Ok(Redirect::permanent("/").into_response());
    }

    match Assets::get(path) {
        Some(content) => {
            use axum::http::header;
            let body = Body::from(content.data);
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let etag = content
                .metadata
                .last_modified()
                .unwrap_or_else(|| {
                    tracing::warn!("can't cache '{path}'");
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("system time before unix epoch")
                        .as_secs()
                })
                .to_string();

            if let Some(client_etag) = headers
                .get(header::IF_NONE_MATCH)
                .and_then(|v| v.to_str().ok())
            {
                if client_etag == etag {
                    return Ok(StatusCode::NOT_MODIFIED.into_response());
                }
            }

            let cache_control = {
                #[cfg(not(debug_assertions))]
                {
                    "max-age=3600"
                }
                #[cfg(debug_assertions)]
                if path.starts_with("vendor") {
                    "max-age=3600"
                } else {
                    "no-cache, max-age=3600"
                }
            };

            let response = Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::CACHE_CONTROL, cache_control)
                .header(header::ETAG, etag);
            Ok(response.body(body).unwrap())
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

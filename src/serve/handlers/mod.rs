use camino::{Utf8Path, Utf8PathBuf};
use cooklang_fs::RecipeEntry;
use minijinja::{context, Value};

use crate::config::UiConfig;

use super::AppState;

pub mod about;
pub mod convert_popover;
pub mod index;
pub mod open_editor;
pub mod recipe;
pub mod search;
pub mod sse_updates;
pub mod static_file;

pub use about::about;
pub use convert_popover::convert_popover;
pub use index::index;
pub use open_editor::open_editor;
pub use recipe::recipe;
pub use search::search;
pub use sse_updates::sse_updates;
pub use static_file::static_file;

macro_rules! ok_status {
    ($res:expr) => {
        ok_status!($res, INTERNAL_SERVER_ERROR)
    };
    ($res:expr, $status:ident) => {
        match $res {
            Ok(val) => val,
            Err(err) => {
                tracing::error!("Error in handler: {err}");
                return axum::http::StatusCode::$status.into_response();
            }
        }
    };
}
pub(crate) use ok_status;

macro_rules! mj_ok {
    ($res:expr) => {{
        #[cfg(debug_assertions)]
        match $res {
            Ok(val) => val,
            Err(err) => {
                let mut s = format!("{err:#}");
                let mut err = &err as &dyn std::error::Error;
                while let Some(next_err) = err.source() {
                    s.push_str(&format!("\ncaused by: {next_err:#}"));
                    err = next_err;
                }
                let html = minijinja::render!("<h1>Error rendering template</h1><pre>{{ err|escape }}</pre>", err => s);
                return Html(html).into_response();
            }
        }
        #[cfg(not(debug_assertions))]
        crate::serve::handlers::ok_status!($res, INTERNAL_SERVER_ERROR)
    }};
}
pub(crate) use mj_ok;

pub fn check_path(p: &str) -> Result<(), axum::http::StatusCode> {
    let path = camino::Utf8Path::new(p);
    if !path
        .components()
        .all(|c| matches!(c, camino::Utf8Component::Normal(_)))
    {
        return Err(axum::http::StatusCode::BAD_REQUEST);
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

fn recipe_entry_context(
    r: RecipeEntry,
    state: &AppState,
    srch: Option<&Searcher>,
) -> Option<Value> {
    let mut metadata = Value::UNDEFINED;
    let mut error = false;
    let mut image = None;
    match r.read() {
        Ok(content) => {
            let res = content.metadata(&state.parser).clone();
            if res.is_valid() {
                let m = res.unwrap_output();
                if srch.is_some_and(|s| !s.matches_recipe(r.name(), &m.tags)) {
                    return None;
                }

                let tags = m
                    .tags
                    .iter()
                    .map(|t| tag_context(t.as_str(), &state.config.ui));
                if let Some(external_image) = m.map.get("image") {
                    image = Some(external_image.clone());
                }
                metadata = context! {
                    tags => Value::from_iter(tags),
                    emoji => m.emoji,
                    desc => m.description,
                }
            } else {
                error = true;
            }
        }
        Err(_) => error = true,
    }

    if error && srch.is_some() {
        return None;
    }

    if image.is_none() {
        image = r
            .images()
            .iter()
            .find(|i| i.indexes.is_none())
            .map(|i| format!("/src/{}", clean_path(&i.path, &state.base_path)));
    }

    let path = clean_path(r.path(), &state.base_path);
    let path = path.as_str().trim_end_matches(".cook");

    Some(context! {
        name => r.name(),
        href => format!("/r/{path}"),
        error,
        image,
        ..metadata,
    })
}

fn tag_context(name: &str, ui_config: &UiConfig) -> Value {
    let emoji = ui_config.tags.get(name).and_then(|c| c.emoji.as_deref());
    context! { emoji, name }
}

#[derive(Debug)]
struct Searcher {
    tags: Vec<String>,
    name_parts: Vec<String>,
}

impl Searcher {
    fn matches_recipe(&self, name: &str, tags: &[String]) -> bool {
        let name = name.to_lowercase();
        for part in &self.name_parts {
            if !name.contains(part) {
                return false;
            }
        }

        for tag in &self.tags {
            if !tags.contains(tag) {
                return false;
            }
        }

        true
    }
}

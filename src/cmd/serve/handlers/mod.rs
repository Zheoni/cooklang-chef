use camino::{Utf8Path, Utf8PathBuf};
use cooklang_fs::RecipeEntry;
use minijinja::{context, Value};

use crate::{config::UiConfig, util::meta_name};

use super::async_index::RecipeData;
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
        crate::cmd::serve::handlers::ok_status!($res, INTERNAL_SERVER_ERROR)
    }};
}
pub(crate) use mj_ok;

pub fn check_path(p: &str) -> Result<(), axum::http::StatusCode> {
    let path = camino::Utf8Path::new(p);
    if !path.components().all(|c| match c {
        camino::Utf8Component::Normal(comp) => {
            // https://github.com/tower-rs/tower-http/pull/204
            let valid = Utf8Path::new(comp)
                .components()
                .all(|c| matches!(c, camino::Utf8Component::Normal(_)));
            valid
        }
        _ => false,
    }) {
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
    recipe: Option<&RecipeData>,
) -> Option<Value> {
    let mut metadata = Value::UNDEFINED;
    let mut error = false;
    let mut image = None;

    if let Some(m) = recipe.and_then(|res| res.metadata.valid_output()) {
        let tags = Value::from_iter(
            m.tags()
                .unwrap_or(&[])
                .iter()
                .map(|t| tag_context(t.as_str(), &state.config.ui)),
        );
        if let Some(external_image) = m.map.get("image") {
            image = Some(external_image.clone());
        }

        let name = meta_name(m).unwrap_or(r.name()).to_string();
        metadata = context! {
            tags,
            emoji => m.emoji(),
            desc => m.description(),
            name,
        }
    } else {
        error = true;
    }

    if image.is_none() {
        image = r
            .images()
            .iter()
            .find(|i| i.indexes.is_none())
            .map(|i| image_url(&i.path, &state.base_path));
    }

    let path = clean_path(r.path(), &state.base_path).with_extension("");

    Some(context! {
        fallback_name => r.name(),
        href => format!("/r/{path}"),
        error,
        image,
        ..metadata,
    })
}

fn image_url(path: &Utf8Path, base_path: &Utf8Path) -> String {
    format!("/src/{}", clean_path(path, base_path))
}

fn tag_context(name: &str, ui_config: &UiConfig) -> Value {
    let emoji = ui_config
        .tags
        .get(name)
        .and_then(|c| c.emoji.as_deref())
        .and_then(|s| {
            if s.starts_with(':') && s.ends_with(':') {
                emojis::get_by_shortcode(&s[1..s.len() - 1])
            } else {
                emojis::get(s)
            }
        })
        .map(|e| e.as_str());
    context! { emoji, name }
}

#[derive(Debug)]
enum Searcher {
    All(Vec<Self>),
    NamePart(String),
    Tag(String),
    Ingredient(String),
}

impl Searcher {
    fn matches_recipe(&self, name: &str, tokens: &RecipeData) -> bool {
        match self {
            Self::All(v) => v.iter().all(|s| s.matches_recipe(name, tokens)),
            Self::NamePart(part) => name.to_lowercase().contains(part),
            Self::Tag(tag) => match tokens.metadata.valid_output() {
                Some(meta) => meta.tags().unwrap_or(&[]).contains(tag),
                None => false,
            },
            Self::Ingredient(ingredient) => tokens
                .ingredients
                .iter()
                .any(|str| &str.to_lowercase() == ingredient),
        }
    }
}

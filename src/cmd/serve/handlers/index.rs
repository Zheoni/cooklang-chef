use std::io;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use camino::Utf8PathBuf;
use minijinja::{context, Value};
use serde::Deserialize;

use crate::cmd::serve::{locale::UserLocale, S};

use super::{check_path, clean_path, mj_ok, recipe_entry_context};

#[derive(Deserialize)]
pub struct IndexQuery {
    deleted: Option<String>,
}

pub async fn index(
    UserLocale(t): UserLocale,
    State(state): State<S>,
    requested_path: Option<Path<String>>,
    Query(q): Query<IndexQuery>,
) -> Response {
    let mut path = Utf8PathBuf::from(&state.base_path);
    if let Some(Path(p)) = &requested_path {
        match check_path(p) {
            Ok(_) => {
                path = path.join(p);
            }
            Err(e) => return e.into_response(),
        }
    }

    let entries = match cooklang_fs::walk_dir(&path) {
        Ok(entries) => entries,
        Err(err) => {
            let status = if err.kind() == io::ErrorKind::NotFound {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            return status.into_response();
        }
    };

    let mut folders = Vec::new();
    let mut recipes = Vec::new();
    for e in entries {
        match e {
            cooklang_fs::Entry::Dir(dir) => folders.push(context! {
                name => dir.file_name(),
                path => clean_path(dir.path(), &state.base_path)
            }),
            cooklang_fs::Entry::Recipe(r) => {
                let meta = r.read().ok().map(|c| c.metadata(&state.parser));
                recipes.push(recipe_entry_context(r, &state, meta.as_ref()).unwrap());
            }
        }
    }

    let tmpl = mj_ok!(state.templates.get_template("index.html"));
    let path_parts = path
        .strip_prefix(&state.base_path)
        .unwrap()
        .components()
        .map(|c| c.as_str());

    let res = tmpl.render(context! {
        t,
        recipes,
        folders,
        path => Value::from_iter(path_parts),
        deleted => q.deleted,
    });
    let content = mj_ok!(res);
    Html(content).into_response()
}

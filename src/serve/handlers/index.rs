use std::io;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use minijinja::{context, Value};
use serde::Deserialize;

use crate::serve::{handlers::clean_path, locale::UserLocale, S};

use super::{check_path, mj_ok, recipe_entry_context};

#[derive(Deserialize)]
pub struct IndexQuery {
    deleted: Option<String>,
}

pub async fn index(
    UserLocale(t): UserLocale,
    State(state): State<S>,
    path: Option<Path<String>>,
    Query(q): Query<IndexQuery>,
) -> Response {
    let path = path.as_ref().map(|p| p.0.as_str()).unwrap_or("");
    if let Err(e) = check_path(path) {
        return e.into_response();
    }

    let path = state.base_path.join(path);

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
                recipes.push(recipe_entry_context(r, &state, None).unwrap());
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

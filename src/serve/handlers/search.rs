use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{Html, IntoResponse, Response},
};
use minijinja::{context, Value};
use serde::Deserialize;

use crate::{serve::S, util::meta_name};

use super::{mj_ok, recipe_entry_context, Searcher};

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    q: Option<String>,
}

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => std::str::FromStr::from_str(s)
            .map_err(serde::de::Error::custom)
            .map(Some),
    }
}

pub async fn search(
    headers: HeaderMap,
    State(state): State<S>,
    Query(query): Query<SearchQuery>,
) -> Response {
    let srch = Searcher::from(query);

    let recipes = if srch.is_empty() {
        Vec::new()
    } else {
        state
            .recipe_index
            .search(
                |entry, meta| match meta.and_then(|r| r.valid_output()) {
                    Some(m) => {
                        let name = meta_name(&m).unwrap_or(entry.name());
                        srch.matches_recipe(name, &m.tags)
                    }
                    None => false,
                },
                |entry, meta| recipe_entry_context(entry, &state, meta),
                0,
                12,
            )
            .await
    };

    let is_htmx_search = headers.get("HX-Trigger").is_some_and(|v| v == "search");

    let template = if is_htmx_search {
        "components/recipe_grid.html"
    } else {
        "search.html"
    };

    let tmpl = mj_ok!(state.templates.get_template(template));
    let t = Value::from(state.locales.get_from_headers(&headers));
    let res = tmpl.render(context! {
        t,
        recipes,
        search_query => srch.to_query(),
        is_htmx_search,
    });
    let content = mj_ok!(res);

    Html(content).into_response()
}

impl From<SearchQuery> for Searcher {
    fn from(value: SearchQuery) -> Self {
        let mut tags = Vec::new();
        let mut name_parts = Vec::new();
        if let Some(q) = value.q {
            for part in q.split_whitespace() {
                if let Some(tag) = part.strip_prefix("tag:") {
                    if cooklang::metadata::is_valid_tag(tag) {
                        tags.push(tag.to_string())
                    }
                } else {
                    name_parts.push(part.to_lowercase());
                }
            }
        }
        Self { tags, name_parts }
    }
}

impl Searcher {
    fn to_query(&self) -> String {
        let mut q = String::new();
        for part in &self.name_parts {
            q.push_str(part);
            q.push(' ');
        }
        for t in &self.tags {
            q.push_str(&format!("tag:{t}"));
            q.push(' ');
        }
        q.pop();
        q
    }

    fn is_empty(&self) -> bool {
        self.name_parts.is_empty() && self.tags.is_empty()
    }
}

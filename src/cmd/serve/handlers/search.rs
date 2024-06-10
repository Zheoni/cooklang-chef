use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{Html, IntoResponse, Response},
};
use minijinja::{context, Value};
use serde::Deserialize;

use crate::{
    cmd::serve::S,
    util::{is_valid_tag, meta_name},
};

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
                |entry, tokens| match tokens {
                    Some(t) => {
                        let name = if let Some(meta) = t.metadata.valid_output() {
                            meta_name(meta).unwrap_or(entry.name())
                        } else {
                            entry.name()
                        };
                        srch.matches_recipe(name, t)
                    }
                    None => false,
                },
                |entry, tokens| recipe_entry_context(entry, &state, tokens),
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
        let mut parts = Vec::new();
        if let Some(q) = value.q {
            for part in q.split_whitespace() {
                let mut negated = false;
                let part = {
                    let new_part = part.strip_prefix("!");
                    match new_part {
                        Some(n) => {
                            negated = true;
                            n
                        }
                        None => part,
                    }
                }
                .replace("+", " ");
                let next = if let Some(tag) = part.strip_prefix("tag:") {
                    if is_valid_tag(tag) {
                        Some(Searcher::Tag(tag.to_owned()))
                    } else {
                        None
                    }
                } else if let Some(ingredient) = part.strip_prefix("uses:") {
                    Some(Searcher::Ingredient(ingredient.to_owned()))
                } else if let Some(cookware) = part.strip_prefix("needs:") {
                    Some(Searcher::Cookware(cookware.to_owned()))
                } else {
                    Some(Searcher::NamePart(part.to_owned()))
                };
                if let Some(n) = next {
                    if negated {
                        parts.push(Searcher::Not(Box::new(n)));
                    } else {
                        parts.push(n);
                    }
                }
            }
        }
        if parts.len() == 1 {
            return parts.pop().unwrap();
        } else {
            return Searcher::All(parts);
        }
    }
}

impl Searcher {
    fn to_query(&self) -> String {
        match self {
            Searcher::All(v) => v.iter().map(|s| s.to_query()).collect::<Vec<_>>().join(" "),
            Searcher::Not(s) => {
                let str = s.to_query();
                format!("!{str}")
            }
            Searcher::NamePart(name) => name.replace(" ", "+"),
            Searcher::Tag(tag) => format!("tag:{tag}").replace(" ", "+"),
            Searcher::Ingredient(ingredient) => format!("uses:{ingredient}").replace(" ", "+"),
            Searcher::Cookware(cookware) => format!("needs:{cookware}").replace(" ", "+"),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Searcher::All(v) => v.is_empty(),
            Searcher::Not(s) => s.is_empty(),
            Searcher::NamePart(name) => name.is_empty(),
            Searcher::Tag(tag) => tag.is_empty(),
            Searcher::Ingredient(ingredient) => ingredient.is_empty(),
            Searcher::Cookware(cookware) => cookware.is_empty(),
        }
    }
}

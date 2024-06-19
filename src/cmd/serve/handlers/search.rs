use std::borrow::Borrow;

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{Html, IntoResponse, Response},
};
use minijinja::context;
use serde::Deserialize;

use crate::{
    cmd::serve::{locale::UserLocale, S},
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
    UserLocale(t): UserLocale,
) -> Response {
    let srch = Searcher::from(query);

    let recipes = state
        .recipe_index
        .search(
            |entry, tokens| match tokens {
                Some(t) => {
                    let name = if let Some(meta) = t.metadata.as_ref() {
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
        .await;

    let is_htmx_search = headers.get("HX-Trigger").is_some_and(|v| v == "search");

    let template = if is_htmx_search {
        "components/recipe_grid.html"
    } else {
        "search.html"
    };

    let tmpl = mj_ok!(state.templates.get_template(template));
    let res = tmpl.render(context! {
        t,
        recipes,
        search_query => srch.to_query(),
        is_htmx_search,
    });
    let content = mj_ok!(res);

    Html(content).into_response()
}

/// Balances parenthesis in the query.
fn error_correct_query(query: &str) -> String {
    let mut depth = 0;
    let mut pad_left = 0;
    for ch in query.chars() {
        if ch == '(' {
            depth += 1;
        }
        if ch == ')' {
            if depth == 0 {
                pad_left += 1;
            } else {
                depth -= 1;
            }
        }
    }
    let mut working_string = "(".repeat(pad_left);
    working_string.push_str(query);
    working_string.push_str(&")".repeat(depth));
    working_string
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_correct_query() {
        assert_eq!(error_correct_query("a b c"), "a b c");
        assert_eq!(error_correct_query("a | c"), "a | c");
        assert_eq!(error_correct_query("(b c"), "(b c)");
        assert_eq!(error_correct_query("(a b)"), "(a b)");
        assert_eq!(error_correct_query("a | (b | c)"), "a | (b | c)");
        assert_eq!(error_correct_query("b) c"), "(b) c");
    }
}

fn parse_disjunct_chunks(query: &str) -> Vec<&str> {
    let mut depth = 0;
    let mut from = 0;
    let mut output = Vec::new();
    for (to, ch) in query.char_indices() {
        if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth -= 1;
        } else if depth == 0 && ch == '|' {
            if from != to {
                output.push(query[from..to].trim())
            }
            from = to + 1;
        }
    }
    if from < query.len() {
        output.push(query[from..query.len()].trim());
    }
    output
}

fn parse_conjunct_chunks(query: &str) -> Vec<&str> {
    let mut depth = 0;
    let mut from = 0;
    let mut output = Vec::new();
    for (to, ch) in query.char_indices() {
        if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth -= 1;
        } else if depth == 0 && ch.is_whitespace() {
            if from != to {
                output.push(query[from..to].trim())
            }
            from = to + 1;
        }
    }
    if from < query.len() {
        output.push(query[from..query.len()].trim());
    }
    output
}

impl From<SearchQuery> for Searcher {
    fn from(value: SearchQuery) -> Self {
        if value.q.is_none() {
            return Searcher::All(Vec::new());
        }
        let q = error_correct_query(
            value
                .q
                .unwrap()
                // We can bring back the necessary parenthesis via error correction.
                .trim_matches(|m: char| m.is_whitespace() || m == ')' || m == '('),
        );

        let mut parts = parse_disjunct_chunks(&q);
        let mut output = if parts.len() == 1 {
            parts = parse_conjunct_chunks(&q);
            Searcher::All(Vec::new())
        } else {
            Searcher::Any(Vec::new())
        };
        for part in parts {
            let mut negated = false;
            let part = match part.strip_prefix('!') {
                None => part,
                Some(new_part) => {
                    negated = true;
                    new_part
                }
            };
            if let Some(mut next) = if part.contains(['|', ' ', '(', ')']) {
                Some(Searcher::from(SearchQuery {
                    q: Some(part.to_owned()),
                }))
            } else {
                let part = part.replace('+', " ");
                if let Some(tag) = part.strip_prefix("tag:") {
                    if is_valid_tag(tag) {
                        Some(Searcher::Tag(tag.to_owned()))
                    } else {
                        None
                    }
                } else if let Some(ingredient) = part.strip_prefix("ingredient:") {
                    Some(Searcher::Ingredient(ingredient.to_owned()))
                } else if let Some(cookware) = part.strip_prefix("cookware:") {
                    Some(Searcher::Cookware(cookware.to_owned()))
                } else {
                    Some(Searcher::NamePart(part.to_owned()))
                }
            } {
                if negated {
                    next = Searcher::Not(Box::new(next));
                }
                match &mut output {
                    Searcher::All(v) => v.push(next),
                    Searcher::Any(v) => v.push(next),
                    _ => unreachable!(),
                }
            }
        }
        match &mut output {
            Searcher::All(v) => {
                if v.len() == 1 {
                    v.pop().unwrap()
                } else {
                    output
                }
            }
            Searcher::Any(v) => {
                if v.len() == 1 {
                    v.pop().unwrap()
                } else {
                    output
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Searcher {
    fn to_query(&self) -> String {
        match self {
            Searcher::All(v) => v
                .iter()
                .map(|s| match s {
                    Searcher::Any(_) => format!("({})", s.to_query()),
                    _ => s.to_query(),
                })
                .collect::<Vec<_>>()
                .join(" "),
            Searcher::Any(v) => v
                .iter()
                .map(|s| s.to_query())
                .collect::<Vec<_>>()
                .join(" | "),
            Searcher::Not(s) => {
                let str = s.to_query();
                match s.borrow() {
                    Searcher::Any(_) => format!("!({str})"),
                    Searcher::All(_) => format!("!({str})"),
                    _ => format!("!{str}"),
                }
            }
            Searcher::NamePart(name) => name.replace(' ', "+"),
            Searcher::Tag(tag) => format!("tag:{tag}").replace(' ', "+"),
            Searcher::Ingredient(ingredient) => {
                format!("ingredient:{ingredient}").replace(' ', "+")
            }
            Searcher::Cookware(cookware) => format!("cookware:{cookware}").replace(' ', "+"),
        }
    }
}

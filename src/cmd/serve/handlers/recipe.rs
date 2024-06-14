use std::{collections::HashMap, net::SocketAddr, time::SystemTime};

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
};
use camino::Utf8Path;
use cooklang::{error::SourceReport, Converter, Modifiers, ParseOptions, ScaledRecipe};
use minijinja::{context, Value};
use serde::{Deserialize, Serialize};
use tokio::task::block_in_place;

use crate::{
    cmd::serve::{
        get_cookie,
        handlers::{clean_path, ok_status, tag_context},
        AppState, S,
    },
    config::Config,
    util::{meta_name, metadata_validator},
    RECIPE_REF_ERROR,
};

use super::{check_path, image_url, mj_ok};

#[derive(Deserialize, Serialize)]
pub struct RecipeQuery {
    scale: Option<u32>,
    units: Option<String>,
}

pub async fn recipe(
    headers: HeaderMap,
    State(state): State<S>,
    Path(path): Path<String>,
    Query(query): Query<RecipeQuery>,
    uri: Uri,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    let units: Option<cooklang::convert::System> = match query.units.as_deref() {
        None => None,
        Some("default") => None,
        Some(sys) => match sys.parse() {
            Ok(sys) => Some(sys),
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
    };

    if let Err(e) = check_path(&path) {
        return e.into_response();
    }

    let entry = ok_status!(state.recipe_index.get(&path).await, NOT_FOUND);
    let content = ok_status!(tokio::fs::read_to_string(&entry.path()).await, NOT_FOUND);

    let res = block_in_place(|| {
        state
            .parser
            .parse_with_options(&content, state.parse_options(Some(entry.path())))
            .into_result()
    });

    let t = Value::from(state.locales.get_from_headers(&headers));
    let tmpl = mj_ok!(state.templates.get_template("recipe.html"));

    let src_path = clean_path(entry.path(), &state.base_path);
    let ctx = context! {
        t,
        is_valid => res.is_ok(),
        href => format!("/r/{}", src_path.with_extension("")),
        src_path,
    };

    match res {
        Ok((scalable, warnings)) => {
            let scaled = {
                let mut r = if let Some(servings) = query.scale {
                    scalable.scale(servings, state.parser.converter())
                } else {
                    scalable.default_scale()
                };
                if let Some(target) = units {
                    let _ = r.convert(target, state.parser.converter());
                }
                r
            };

            let report_html = if warnings.is_empty() {
                None
            } else {
                Some(ok_status!(report_to_html(
                    &warnings,
                    entry.file_name(),
                    &content
                )))
            };

            let times = ok_status!(get_times(entry.path()).await, NOT_FOUND);

            let name = meta_name(&scaled.metadata)
                .unwrap_or(entry.name())
                .to_string();

            let recipe_refs: HashMap<String, Value> = block_in_place(|| {
                scaled
                    .ingredients
                    .iter()
                    .filter(|igr| igr.modifiers().contains(Modifiers::RECIPE))
                    .filter_map(|igr| {
                        let res = state.recipe_index.resolve_blocking(
                            &igr.name,
                            Some(entry.path().parent().expect("no parent for recipe entry")),
                        );

                        match res {
                            Ok(entry) => {
                                let path =
                                    clean_path(entry.path(), &state.base_path).with_extension("");
                                let value = Value::from(format!("/r/{path}"));
                                Some((igr.name.clone(), value))
                            }
                            Err(_) => None,
                        }
                    })
                    .collect()
            });

            let images = Value::from_iter(entry.images().iter().map(|img| {
                context! {
                    indexes => img.indexes,
                    href => image_url(&img.path, &state.base_path)
                }
            }));
            let main_image = scaled.metadata.map.get("image").cloned().or_else(|| {
                entry
                    .images()
                    .iter()
                    .find(|img| img.indexes.is_none())
                    .map(|img| image_url(&img.path, &state.base_path))
            });

            let r = make_recipe_context(scaled, state.parser.converter(), &state.config);

            let ctx = context! {
                name,
                r,
                query,
                path => uri.path(),
                recipe_refs,

                times,
                images,
                main_image,

                is_loopback => addr.ip().is_loopback(),
                igr_layout => get_cookie(&headers, "igr_layout").unwrap_or("line"),

                report_html,
                severity => "warning",
                ..ctx
            };
            let content = mj_ok!(tmpl.render(ctx));
            Html(content).into_response()
        }
        Err(report) => {
            let report_html = ok_status!(report_to_html(&report, entry.file_name(), &content));

            let content = mj_ok!(tmpl.render(context! {
                name => entry.name(),
                report_html,
                severity => "error",
                ..ctx
            }));
            Html(content).into_response()
        }
    }
}

fn make_recipe_context(r: ScaledRecipe, converter: &Converter, config: &Config) -> Value {
    let grouped_ingredients = r
        .group_ingredients(converter)
        .into_iter()
        .map(|entry| {
            context! {
                index => entry.index,
                outcome => entry.outcome,
                quantities => entry.quantity.iter().map(|q| context! {
                    value => q.value,
                    unit => q.unit_text()
                }).collect::<Value>(),
            }
        })
        .collect::<Value>();

    let grouped_cookware = r
        .group_cookware()
        .into_iter()
        .map(|entry| {
            context! {
                index => entry.index,
                amounts => entry.amount.iter().map(Value::from_serialize).collect::<Value>()
            }
        })
        .collect::<Value>();

    let timers_seconds = r
        .timers
        .iter()
        .filter_map(|t| {
            if let Some(q) = &t.quantity {
                let mut q = q.clone();
                q.convert("s", converter).ok()?;
                let seconds = match q.value {
                    cooklang::Value::Number(n) => n.value(),
                    cooklang::Value::Range { start, .. } => start.value(),
                    cooklang::Value::Text(_) => return None,
                };
                return Some(Value::from(seconds));
            }
            None
        })
        .collect::<Value>();

    context! {
        meta => context! {
            description => r.metadata.description(),
            tags => Value::from_iter(r.metadata.tags().iter().flat_map(|tags| {
                tags.iter()
                    .map(|t| tag_context(t.as_str(), &config.ui))
            })),
            emoji => r.metadata.emoji(),
            author => r.metadata.author(),
            source => r.metadata.source(),
            time => r.metadata.time(),
            servings => r.metadata.servings(),
            other => Value::from_iter(r.metadata.map_filtered())
        },
        grouped_ingredients,
        grouped_cookware,

        sections => r.sections,

        ingredients => r.ingredients.into_iter().map(TemplateIngredient).map(Value::from_struct_object).collect::<Value>(),
        cookware => r.cookware.into_iter().map(TemplateCookware).map(Value::from_struct_object).collect::<Value>(),
        timers => r.timers,
        timers_seconds,
        inline_quantities => r.inline_quantities,
    }
}

macro_rules! mj_opt {
    ($opt:expr) => {
        match $opt {
            Some(t) => minijinja::Value::from(t),
            None => minijinja::Value::from(()),
        }
    };
}

struct TemplateIngredient(cooklang::Ingredient<cooklang::Value>);

impl minijinja::value::StructObject for TemplateIngredient {
    fn get_field(&self, name: &str) -> Option<Value> {
        let v = match name {
            "name" => self.0.name.as_str().into(),
            "display_name" => self.0.display_name().into(),
            "alias" => mj_opt!(self.0.alias.as_deref()),
            "quantity" => Value::from_serialize(&self.0.quantity),
            "note" => mj_opt!(self.0.note.as_deref()),
            "references_to" => mj_opt!(self.0.relation.references_to().map(|rel| context! {
                index => rel.0,
                target => rel.1
            })),
            "modifiers" => Value::from_serialize(self.0.modifiers()),
            _ => return None,
        };

        Some(v)
    }
}

struct TemplateCookware(cooklang::Cookware<cooklang::Value>);

impl minijinja::value::StructObject for TemplateCookware {
    fn get_field(&self, name: &str) -> Option<Value> {
        let v = match name {
            "name" => self.0.name.as_str().into(),
            "display_name" => self.0.display_name().into(),
            "alias" => mj_opt!(self.0.alias.as_deref()),
            "quantity" => Value::from_serialize(&self.0.quantity),
            "note" => mj_opt!(self.0.note.as_deref()),
            "references_to" => mj_opt!(self.0.relation.references_to().map(Value::from_serialize)),
            "modifiers" => Value::from_serialize(self.0.modifiers()),
            _ => return None,
        };

        Some(v)
    }
}

async fn get_times(path: &Utf8Path) -> anyhow::Result<Value> {
    fn f(st: std::io::Result<SystemTime>) -> Option<u64> {
        st.ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }
    let metadata = tokio::fs::metadata(path).await?;
    let modified = f(metadata.modified()).unwrap_or(0);
    let created = f(metadata.created()).unwrap_or(0);
    Ok(context! { modified, created })
}

impl AppState {
    fn checker(
        &self,
        relative_to: Option<&Utf8Path>,
    ) -> Option<cooklang::analysis::RecipeRefCheck> {
        if self.config.recipe_ref_check {
            let relative_to =
                relative_to.map(|r| r.parent().expect("no parent for recipe entry").to_owned());
            Some(Box::new(move |name: &str| {
                if self
                    .recipe_index
                    .resolve_blocking(name, relative_to.as_deref())
                    .is_ok()
                {
                    cooklang::analysis::CheckResult::Ok
                } else {
                    cooklang::analysis::CheckResult::Warning(vec![RECIPE_REF_ERROR.into()])
                }
            }) as cooklang::analysis::RecipeRefCheck)
        } else {
            None
        }
    }

    fn parse_options(&self, relative_to: Option<&Utf8Path>) -> ParseOptions {
        ParseOptions {
            recipe_ref_check: self.checker(relative_to),
            metadata_validator: Some(Box::new(metadata_validator)),
        }
    }
}

fn report_to_html(report: &SourceReport, file_name: &str, content: &str) -> anyhow::Result<String> {
    let mut buf = Vec::new();
    report.write(file_name, content, true, &mut buf)?;
    let ansi = String::from_utf8(buf)?;
    let html = ansi_to_html::convert(&ansi)?;
    Ok(html)
}

pub fn step_ingredients(
    items: &dyn minijinja::value::SeqObject,
    ingredients: Vec<Value>,
) -> Result<Value, minijinja::Error> {
    let get_igr =
        |index: usize| -> Result<&cooklang::Ingredient<cooklang::Value>, minijinja::Error> {
            ingredients
                .get(index)
                .ok_or(minijinja::Error::new(
                    minijinja::ErrorKind::UndefinedError,
                    "undefined ingredient by index",
                ))?
                .downcast_object_ref::<TemplateIngredient>()
                .ok_or(minijinja::Error::new(
                    minijinja::ErrorKind::InvalidOperation,
                    "ingrediens not TemplateIngredient",
                ))
                .map(|i| &i.0)
        };

    let mut dedup = HashMap::<String, Vec<usize>>::new();
    for item in items.iter() {
        let is_ingredient = item
            .get_attr("type")?
            .as_str()
            .is_some_and(|s| s == "ingredient");
        if !is_ingredient {
            continue;
        }

        let index: usize = item.get_attr("index")?.try_into()?;
        let igr = get_igr(index)?;
        dedup.entry(igr.name.clone()).or_default().push(index);
    }
    for group in dedup.values_mut() {
        let first = group[0];
        group.retain(|&i| {
            // unwrap is ok should have already been done once
            get_igr(i).unwrap().quantity.is_some()
        });
        if group.is_empty() {
            group.push(first);
        }
    }

    let mut step_ingredients = HashMap::<usize, Value>::new();
    for item in items.iter() {
        let is_ingredient = item
            .get_attr("type")?
            .as_str()
            .is_some_and(|s| s == "ingredient");
        if !is_ingredient {
            continue;
        }
        let index: usize = item.get_attr("index")?.try_into()?;
        let igr = get_igr(index).unwrap();
        let group = match dedup.get(&igr.name) {
            Some(g) => g,
            None => continue,
        };

        // The subscript is the position in the group of the current item
        // if it's no the only one.
        let mut subscript = None;
        let group_index = group.iter().position(|&i| i == index);
        if group.len() > 1 {
            subscript = group_index.map(|index| index + 1);
        }
        step_ingredients.insert(
            index,
            context! {
                in_ingredients_line => group_index.is_some(),
                subscript
            },
        );
    }
    Ok(Value::from(step_ingredients))
}

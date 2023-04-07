#![allow(unused)]
use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::Result;
use clap::Args;
use cooklang::quantity::GroupedQuantity;
use cooklang::{aisle::AileConf, quantity::Quantity};
use cooklang::{Recipe, ScaledRecipe};
use yansi::Paint;

use crate::{unwrap_recipe, Context};

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Recipe to add to the list
    ///
    /// Name or path to the file. It will use the default scaling of the recipe.
    /// To use a custom scaling, add '*<servings>' at the end.
    recipes: Vec<String>,
}

pub fn run(ctx: &Context, aisle: AileConf, args: CreateArgs) -> Result<()> {
    // let parser = ctx.parser()?;
    // let converter = parser.converter();
    // let mut all_ingredients: HashMap<Cow<str>, GroupedQuantity> = HashMap::new();
    // let recipes = args
    //     .recipes
    //     .into_iter()
    //     .map(|entry| -> Result<ScaledRecipe> {
    //         let (name, servings) = entry
    //             .trim()
    //             .rsplit_once('*')
    //             .map(|(name, servings)| {
    //                 let target = servings.parse::<u32>();
    //                 if target.is_err() {
    //                     tracing::warn!("Invalid scaling target: {}", servings);
    //                 }
    //                 (name, target.ok())
    //             })
    //             .unwrap_or((&entry, None));

    //         let entry = ctx.recipe_index.get(name)?;
    //         let content = entry.read()?;
    //         let r = content.parse(parser);
    //         let recipe = unwrap_recipe(r, entry.path().file_name().unwrap(), content.text(), ctx)?;
    //         if let Some(servings) = servings {
    //             Ok(recipe.scale(servings, converter))
    //         } else {
    //             Ok(recipe.default_scale())
    //         }
    //     })
    //     .collect::<Result<Vec<_>, _>>()?;

    // for recipe in &recipes {
    //     for (ingredient, quantity, outcome) in recipe.ingredient_list(converter) {
    //         all_ingredients
    //             .entry(ingredient.display_name())
    //             .or_default()
    //             .merge(&quantity, converter)
    //     }
    // }

    // let aisle = aisle.reverse();
    // let (categories, other) = {
    //     let mut m = HashMap::<&str, Vec<_>>::new();
    //     let mut other = Vec::new();
    //     for (igr, q) in all_ingredients {
    //         if let Some(cat) = aisle.get(igr.as_ref()) {
    //             m.entry(cat).or_default().push((igr, q))
    //         } else {
    //             other.push((igr, q));
    //         }
    //     }
    //     (m, other)
    // };

    // let mut table = tabular::Table::new("{:<} {:<}");
    // for (cat, items) in categories {
    //     table.add_heading(format!("[{}]", Paint::green(cat)));
    //     for (igr, q) in items {
    //         let mut row = tabular::Row::new().with_cell(igr);
    //         let q_str = match q.total() {
    //             cooklang::quantity::TotalQuantity::None => {
    //                 row.add_cell("");
    //             }
    //             cooklang::quantity::TotalQuantity::Single(quantity) => {
    //                 row.add_ansi_cell(quantity_fmt(&quantity));
    //             }
    //             cooklang::quantity::TotalQuantity::Many(list) => {
    //                 let list = list
    //                     .into_iter()
    //                     .map(|q| quantity_fmt(&q))
    //                     .reduce(|s, q| format!("{s}, {q}"))
    //                     .unwrap();
    //                 row.add_ansi_cell(list);
    //             }
    //         };
    //     }
    // }

    todo!()
}

fn quantity_fmt(qty: &Quantity) -> String {
    if let Some(unit) = qty.unit() {
        format!("{} {}", qty.value, Paint::new(unit.text()).italic())
    } else {
        format!("{}", qty.value)
    }
}

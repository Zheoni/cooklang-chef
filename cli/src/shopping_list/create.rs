#![allow(unused)]
use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Args;
use cooklang::model::IngredientListEntry;
use cooklang::quantity::GroupedQuantity;
use cooklang::{aisle::AileConf, quantity::Quantity};
use cooklang::{Recipe, ScaledRecipe};
use yansi::Paint;

use crate::{unwrap_recipe, write_to_output, Context};

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Recipe to add to the list
    ///
    /// Name or path to the file. It will use the default scaling of the recipe.
    /// To use a custom scaling, add `*<servings>` at the end.
    recipes: Vec<String>,

    /// Output file, none for stdout.
    #[arg(short, long)]
    output: Option<Utf8PathBuf>,
}

pub fn run(ctx: &Context, aisle: AileConf, args: CreateArgs) -> Result<()> {
    let parser = ctx.parser()?;
    let converter = parser.converter();
    let mut all_ingredients: HashMap<String, GroupedQuantity> = HashMap::new();

    for entry in args.recipes {
        let (name, servings) = entry
            .trim()
            .rsplit_once('*')
            .map(|(name, servings)| {
                let target = servings.parse::<u32>();
                if target.is_err() {
                    tracing::warn!("Invalid scaling target: {}", servings);
                }
                (name, target.ok())
            })
            .unwrap_or((&entry, None));

        let entry = ctx.recipe_index.get(name)?;
        let content = entry.read()?;
        let r = content.parse(parser);
        let recipe = unwrap_recipe(r, entry.path().file_name().unwrap(), content.text(), ctx)?;
        let recipe = if let Some(servings) = servings {
            recipe.scale(servings, converter)
        } else {
            recipe.default_scale()
        };

        for IngredientListEntry {
            index,
            quantity,
            outcome,
        } in recipe.ingredient_list(converter)
        {
            let ingredient = &recipe.ingredients[index];
            all_ingredients
                .entry(ingredient.display_name().into_owned())
                .or_default()
                .merge(&quantity, converter)
        }
    }

    let aisle = aisle.reverse();
    let (categories, other) = {
        let mut m = HashMap::<&str, Vec<_>>::new();
        let mut other = Vec::new();
        for (igr, q) in all_ingredients {
            if let Some(cat) = aisle.get(igr.as_str()) {
                m.entry(cat).or_default().push((igr, q))
            } else {
                other.push((igr, q));
            }
        }
        (m, other)
    };

    let mut table = tabular::Table::new("{:<} {:<}");
    let mut add_items = |table: &mut tabular::Table, items: Vec<(String, GroupedQuantity)>| {
        for (igr, q) in items {
            let mut row = tabular::Row::new().with_cell(igr);
            match q.total() {
                cooklang::quantity::TotalQuantity::None => {
                    row.add_cell("");
                }
                cooklang::quantity::TotalQuantity::Single(quantity) => {
                    row.add_ansi_cell(quantity_fmt(&quantity));
                }
                cooklang::quantity::TotalQuantity::Many(list) => {
                    let list = list
                        .into_iter()
                        .map(|q| quantity_fmt(&q))
                        .reduce(|s, q| format!("{s}, {q}"))
                        .unwrap();
                    row.add_ansi_cell(list);
                }
            };
            table.add_row(row);
        }
    };
    for (cat, items) in categories {
        table.add_heading(format!("[{}]", Paint::green(cat)));
        add_items(&mut table, items);
    }
    table.add_heading(format!("[{}]", Paint::cyan("other")));
    add_items(&mut table, other);

    write_to_output(args.output.as_deref(), |mut w| {
        write!(w, "{table}")?;
        Ok(())
    })
}

fn quantity_fmt(qty: &Quantity) -> String {
    if let Some(unit) = qty.unit() {
        format!("{} {}", qty.value, Paint::new(unit.text()).italic())
    } else {
        format!("{}", qty.value)
    }
}

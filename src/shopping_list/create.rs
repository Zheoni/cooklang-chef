use std::collections::BTreeMap;

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Args, CommandFactory, ValueEnum};
use cooklang::{
    aisle::AileConf,
    model::IngredientListEntry,
    quantity::{GroupedQuantity, Quantity, QuantityValue, TotalQuantity, Value},
};
use cooklang_fs::resolve_recipe;
use serde::Serialize;
use yansi::Paint;

use crate::{write_to_output, Context, Input};

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

    /// Do not display categories
    #[arg(short, long)]
    plain: bool,

    /// Output format
    ///
    /// Tries to infer it from output file extension. Defaults to "human".
    #[arg(short, long, value_enum)]
    format: Option<OutputFormat>,

    /// Pretty output format, if available
    #[arg(long)]
    pretty: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

pub fn run(ctx: &Context, aisle: AileConf, args: CreateArgs) -> Result<()> {
    let format = args.format.unwrap_or_else(|| match &args.output {
        Some(p) => match p.extension() {
            Some("json") => OutputFormat::Json,
            _ => OutputFormat::Human,
        },
        None => OutputFormat::Human,
    });

    // retrieve, scale and merge ingredients
    let mut all_ingredients: BTreeMap<String, GroupedQuantity> = BTreeMap::new();
    for entry in args.recipes {
        extract_ingredients(&entry, &mut all_ingredients, ctx)?;
    }

    write_to_output(args.output.as_deref(), |mut w| {
        match format {
            OutputFormat::Human => {
                let table = build_human_table(all_ingredients, &aisle, args.plain);
                write!(w, "{table}")?;
            }
            OutputFormat::Json => {
                let value = build_json_value(all_ingredients, &aisle, args.plain);
                if args.pretty {
                    serde_json::to_writer_pretty(w, &value)?;
                } else {
                    serde_json::to_writer(w, &value)?;
                }
            }
        }
        Ok(())
    })
}

fn extract_ingredients(
    entry: &str,
    all_ingredients: &mut BTreeMap<String, GroupedQuantity>,
    ctx: &Context,
) -> Result<()> {
    let converter = ctx.parser()?.converter();

    // split into name and servings
    let (name, servings) = entry
        .trim()
        .rsplit_once('*')
        .map(|(name, servings)| {
            let target = servings.parse::<u32>().unwrap_or_else(|err| {
                let mut cmd = crate::CliArgs::command();
                cmd.error(
                    clap::error::ErrorKind::InvalidValue,
                    format!("Invalid scaling target for '{name}': {err}"),
                )
                .exit()
            });
            (name, Some(target))
        })
        .unwrap_or((&entry, None));

    // Resolve and parse the recipe
    let input = {
        let entry = resolve_recipe(name, &ctx.recipe_index, None)?;
        Input::File {
            content: entry.read()?,
            override_name: None,
        }
    };
    let recipe = input.parse(ctx)?;

    // Scale
    let recipe = if let Some(servings) = servings {
        recipe.scale(servings, converter)
    } else {
        recipe.default_scale()
    };

    // Read ingredients
    for entry in recipe.ingredient_list(converter) {
        let IngredientListEntry {
            index,
            quantity,
            outcome,
        } = entry;
        let ingredient = &recipe.ingredients[index];

        if let Some(cooklang::scale::ScaleOutcome::Error(err)) = outcome {
            tracing::error!("Error scaling ingredient: {err}");
        }

        all_ingredients
            .entry(ingredient.display_name().into_owned())
            .or_default()
            .merge(&quantity, converter);
    }

    Ok(())
}

fn total_quantity_fmt(qty: &TotalQuantity, row: &mut tabular::Row) {
    match qty {
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
}

fn quantity_fmt(qty: &Quantity) -> String {
    if let Some(unit) = qty.unit() {
        format!("{} {}", qty.value, Paint::new(unit.text()).italic())
    } else {
        format!("{}", qty.value)
    }
}

fn split_into_categories<'a>(
    all_ingredients: BTreeMap<String, GroupedQuantity>,
    aisle: &'a AileConf<'a>,
) -> Vec<(&'a str, Vec<(String, GroupedQuantity)>)> {
    let aisle = aisle.reverse();
    let mut m = BTreeMap::<&str, Vec<_>>::new();
    let mut other = Vec::new();
    for (igr, q) in all_ingredients {
        if let Some(cat) = aisle.get(igr.as_str()) {
            m.entry(cat).or_default().push((igr, q))
        } else {
            other.push((igr, q));
        }
    }
    m.into_iter()
        .map(|(cat, items)| (cat, items))
        .chain(std::iter::once(("other", other)))
        .collect()
}

fn build_human_table(
    all_ingredients: BTreeMap<String, GroupedQuantity>,
    aisle: &AileConf,
    plain: bool,
) -> tabular::Table {
    let mut table = tabular::Table::new("{:<} {:<}");
    if plain {
        for (igr, q) in all_ingredients {
            let mut row = tabular::Row::new().with_cell(igr);
            total_quantity_fmt(&q.total(), &mut row);
            table.add_row(row);
        }
    } else {
        let categories = split_into_categories(all_ingredients, aisle);
        for (cat, items) in categories {
            table.add_heading(format!("[{}]", Paint::green(cat)));
            for (igr, q) in items {
                let mut row = tabular::Row::new().with_cell(igr);
                total_quantity_fmt(&q.total(), &mut row);
                table.add_row(row);
            }
        }
    }
    table
}

fn build_json_value<'a>(
    all_ingredients: BTreeMap<String, GroupedQuantity>,
    aisle: &'a AileConf<'a>,
    plain: bool,
) -> serde_json::Value {
    #[derive(Serialize)]
    struct Quantity {
        value: Value,
        unit: Option<String>,
    }
    impl From<cooklang::quantity::Quantity> for Quantity {
        fn from(qty: cooklang::quantity::Quantity) -> Self {
            let unit = qty.unit_text().map(|s| s.to_owned());
            let QuantityValue::Fixed(value) = qty.value
            else { panic!("Unexpected unscaled value while serializing") };
            Self { value, unit }
        }
    }
    #[derive(Serialize)]
    #[serde(untagged)]
    enum TotalQuantity {
        None,
        Single(Quantity),
        Many(Vec<Quantity>),
    }
    impl From<cooklang::quantity::TotalQuantity> for TotalQuantity {
        fn from(value: cooklang::quantity::TotalQuantity) -> Self {
            match value {
                cooklang::quantity::TotalQuantity::None => TotalQuantity::None,
                cooklang::quantity::TotalQuantity::Single(q) => TotalQuantity::Single(q.into()),
                cooklang::quantity::TotalQuantity::Many(v) => {
                    TotalQuantity::Many(v.into_iter().map(|q| q.into()).collect())
                }
            }
        }
    }
    #[derive(Serialize)]
    struct Ingredient {
        name: String,
        quantity: TotalQuantity,
    }
    impl From<(String, GroupedQuantity)> for Ingredient {
        fn from((name, qty): (String, GroupedQuantity)) -> Self {
            Ingredient {
                name,
                quantity: qty.total().into(),
            }
        }
    }
    #[derive(Serialize)]
    struct Category<'a> {
        category: &'a str,
        items: Vec<Ingredient>,
    }

    if plain {
        serde_json::to_value(
            all_ingredients
                .into_iter()
                .map(Ingredient::from)
                .collect::<Vec<_>>(),
        )
        .unwrap()
    } else {
        serde_json::to_value(
            split_into_categories(all_ingredients, aisle)
                .into_iter()
                .map(|(category, items)| Category {
                    category,
                    items: items.into_iter().map(Ingredient::from).collect(),
                })
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }
}

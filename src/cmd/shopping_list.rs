use anyhow::{bail, Context as _, Result};
use camino::Utf8PathBuf;
use clap::{Args, CommandFactory, ValueEnum};
use cooklang::{
    aisle::AisleConf,
    ingredient_list::IngredientList,
    quantity::{GroupedQuantity, Quantity, Value},
    ScaledQuantity,
};
use serde::Serialize;

use crate::{util::write_to_output, util::Input, Context};

#[derive(Debug, Args)]
pub struct ShoppingListArgs {
    /// Recipes to add with optional servings number
    #[arg(value_name = "RECIPE[*<SERVINGS>]")]
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

    /// Load aisle conf file
    #[arg(short, long)]
    aisle: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

pub fn run(ctx: &Context, mut args: ShoppingListArgs) -> Result<()> {
    let aisle_conf = args
        .aisle
        .or_else(|| ctx.config.aisle(&ctx.base_path))
        .map(|path| -> Result<_> {
            let content = std::fs::read_to_string(&path).context("Failed to read aisle file")?;
            Ok((content, path))
        })
        .transpose()?;

    let aisle = aisle_conf
        .as_ref()
        .map(|(content, path)| {
            let res = cooklang::aisle::parse(content);
            if let Err(e) = res {
                cooklang::error::write_rich_error(
                    &e,
                    path.as_str(),
                    content,
                    true,
                    anstream::stderr().lock(),
                )?;
                bail!("Error parsing aisle file")
            }
            Ok(res.unwrap()) // anyhow
        })
        .transpose()?;
    let aisle = aisle.unwrap_or_default();
    if aisle.categories.is_empty() {
        args.plain = true;
    }

    let format = args.format.unwrap_or_else(|| match &args.output {
        Some(p) => match p.extension() {
            Some("json") => OutputFormat::Json,
            _ => OutputFormat::Human,
        },
        None => OutputFormat::Human,
    });

    // retrieve, scale and merge ingredients
    let mut list = IngredientList::new();
    for entry in args.recipes {
        extract_ingredients(&entry, &mut list, ctx)?;
    }

    write_to_output(args.output.as_deref(), |mut w| {
        match format {
            OutputFormat::Human => {
                let table = build_human_table(list, &aisle, args.plain);
                write!(w, "{table}")?;
            }
            OutputFormat::Json => {
                let value = build_json_value(list, &aisle, args.plain);
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

fn extract_ingredients(entry: &str, list: &mut IngredientList, ctx: &Context) -> Result<()> {
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
        .unwrap_or((entry, None));

    // Resolve and parse the recipe
    let input = {
        let entry = ctx.recipe_index.resolve(name, None)?;
        Input::File {
            entry,
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

    // Add ingredients to the list
    list.add_recipe(&recipe, converter);

    Ok(())
}

fn grouped_qty_fmt(qty: &GroupedQuantity, row: &mut tabular::Row) {
    let content = qty
        .iter()
        .map(quantity_fmt)
        .reduce(|s, q| format!("{s}, {q}"))
        .unwrap_or_default();
    row.add_ansi_cell(content);
}

fn quantity_fmt(qty: &Quantity) -> String {
    use yansi::Paint;

    if let Some(unit) = qty.unit() {
        format!("{} {}", qty.value(), unit.italic())
    } else {
        format!("{}", qty.value())
    }
}

fn build_human_table(list: IngredientList, aisle: &AisleConf, plain: bool) -> tabular::Table {
    use yansi::Paint;

    let mut table = tabular::Table::new("{:<} {:<}");
    if plain {
        for (igr, q) in list {
            let mut row = tabular::Row::new().with_cell(igr);
            grouped_qty_fmt(&q, &mut row);
            table.add_row(row);
        }
    } else {
        let categories = list.categorize(aisle);
        for (cat, items) in categories {
            table.add_heading(format!("[{}]", cat.green()));
            for (igr, q) in items {
                let mut row = tabular::Row::new().with_cell(igr);
                grouped_qty_fmt(&q, &mut row);
                table.add_row(row);
            }
        }
    }
    table
}

fn build_json_value<'a>(
    list: IngredientList,
    aisle: &'a AisleConf<'a>,
    plain: bool,
) -> serde_json::Value {
    #[derive(Serialize)]
    struct Quantity {
        value: Value,
        unit: Option<String>,
    }
    impl From<cooklang::quantity::Quantity> for Quantity {
        fn from(qty: cooklang::quantity::Quantity) -> Self {
            let unit = qty.unit().map(|s| s.to_owned());
            let value = qty.value().clone();
            Self { value, unit }
        }
    }
    #[derive(Serialize)]
    struct Ingredient {
        name: String,
        quantity: Vec<ScaledQuantity>,
    }
    impl From<(String, GroupedQuantity)> for Ingredient {
        fn from((name, qty): (String, GroupedQuantity)) -> Self {
            Ingredient {
                name,
                quantity: qty.into_vec(),
            }
        }
    }
    #[derive(Serialize)]
    struct Category {
        category: String,
        items: Vec<Ingredient>,
    }

    if plain {
        serde_json::to_value(list.into_iter().map(Ingredient::from).collect::<Vec<_>>()).unwrap()
    } else {
        serde_json::to_value(
            list.categorize(aisle)
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

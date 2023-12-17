use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};

use crate::{util::write_to_output, Context};

#[derive(Debug, Args)]
pub struct ReadArgs {
    #[command(flatten)]
    input: super::RecipeInputArgs,

    /// Output file, none for stdout.
    #[arg(short, long)]
    output: Option<Utf8PathBuf>,

    /// Output format
    ///
    /// Tries to infer it from output file extension. Defaults to "human".
    #[arg(short, long, value_enum)]
    format: Option<OutputFormat>,

    /// Pretty output format, if available
    #[arg(long)]
    pretty: bool,

    /// Scale to a number of servings
    #[arg(short, long, alias = "servings", value_name = "SERVINGS")]
    scale: Option<u32>,

    /// Convert to a unit system
    #[arg(short, long, alias = "system", value_name = "SYSTEM")]
    convert: Option<System>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
    #[value(alias("cook"))]
    Cooklang,
    #[value(alias("md"))]
    Markdown,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum System {
    Metric,
    #[value(alias("freedom"))]
    Imperial,
}

pub fn run(ctx: &Context, args: ReadArgs) -> Result<()> {
    let input = args.input.read(&ctx.recipe_index)?;
    let recipe = input.parse(ctx)?;

    let mut scaled_recipe = if let Some(scale) = args.scale {
        recipe.scale(scale, ctx.parser()?.converter())
    } else {
        recipe.default_scale()
    };

    if let Some(system) = args.convert {
        let to = match system {
            System::Metric => cooklang::convert::System::Metric,
            System::Imperial => cooklang::convert::System::Imperial,
        };
        let _ = scaled_recipe.convert(to, ctx.parser()?.converter());
    }

    let format = args.format.unwrap_or_else(|| match &args.output {
        Some(p) => match p.extension() {
            Some("json") => OutputFormat::Json,
            Some("cook") => OutputFormat::Cooklang,
            Some("md") => OutputFormat::Markdown,
            _ => OutputFormat::Human,
        },
        None => OutputFormat::Human,
    });

    write_to_output(args.output.as_deref(), |writer| {
        match format {
            OutputFormat::Human => cooklang_to_human::print_human(
                &scaled_recipe,
                input.name(),
                ctx.parser()?.converter(),
                writer,
            )?,
            OutputFormat::Json => {
                #[derive(serde::Serialize)]
                struct JsonRecipe<'a> {
                    name: &'a str,
                    #[serde(flatten)]
                    recipe: &'a cooklang::ScaledRecipe,
                }

                let recipe = JsonRecipe {
                    recipe: &scaled_recipe,
                    name: input.name(),
                };

                if args.pretty {
                    serde_json::to_writer_pretty(writer, &recipe)?;
                } else {
                    serde_json::to_writer(writer, &recipe)?;
                }
            }
            OutputFormat::Cooklang => cooklang_to_cooklang::print_cooklang(&scaled_recipe, writer)?,
            OutputFormat::Markdown => cooklang_to_md::print_md(
                &scaled_recipe,
                input.name(),
                ctx.parser()?.converter(),
                writer,
            )?,
        }

        Ok(())
    })?;

    Ok(())
}

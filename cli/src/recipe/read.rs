use anyhow::{bail, Result};
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};
use cooklang::scale::ScaleTarget;

use crate::{write_to_output, Context};

use super::RecipeInputArgs;

#[derive(Debug, Args)]
pub struct ReadArgs {
    #[command(flatten)]
    input: RecipeInputArgs,

    /// Output file, none for stdout.
    #[arg(short, long)]
    output: Option<Utf8PathBuf>,

    /// Output format
    #[arg(short, long, value_enum)]
    format: Option<OutputFormat>,

    /// Pretty output format, if available
    #[arg(long)]
    pretty: bool,

    /// Do not display warnings
    #[arg(long)]
    ignore_warnings: bool,

    #[arg(short, long, alias = "servings")]
    scale: Option<u32>,

    #[arg(short, long, alias = "system")]
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
    let input = args.input.read()?;
    let recipe = input.parse(ctx)?;

    let mut scaled_recipe = if let Some(scale) = args.scale {
        let target = if let Some(servings) = recipe.metadata.servings.as_ref() {
            let Some(base) = servings.first().copied() else { bail!("Empty servings list") };
            ScaleTarget::new(base, scale, servings)
        } else {
            ScaleTarget::new(1, scale, &[])
        };
        recipe.scale(target, ctx.parser()?.converter())
    } else {
        recipe.default_scaling()
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
            OutputFormat::Human => {
                cooklang_to_human::print_human(&scaled_recipe, ctx.parser()?.converter(), writer)?
            }
            OutputFormat::Json => {
                if args.pretty {
                    serde_json::to_writer_pretty(writer, &scaled_recipe)?;
                } else {
                    serde_json::to_writer(writer, &scaled_recipe)?;
                }
            }
            OutputFormat::Cooklang => cooklang_to_cooklang::print_cooklang(&scaled_recipe, writer)?,
            OutputFormat::Markdown => {
                cooklang_to_md::print_md(&scaled_recipe, ctx.parser()?.converter(), writer)?
            }
        }

        Ok(())
    })?;

    Ok(())
}

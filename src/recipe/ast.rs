use anyhow::{bail, Result};
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};

use crate::Context;

use super::RecipeInputArgs;

#[derive(Debug, Args)]
pub struct AstArgs {
    #[command(flatten)]
    input: RecipeInputArgs,

    /// Output file, none for stdout.
    #[arg(short, long)]
    output: Option<Utf8PathBuf>,

    /// Output format
    #[arg(short, long, value_enum)]
    format: Option<OutputFormat>,

    /// Pretty output format
    #[arg(long)]
    pretty: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Debug,
}

pub fn run(ctx: &Context, args: AstArgs) -> Result<()> {
    let input = args.input.read(&ctx.recipe_index)?;

    let (text, file_name) = match &input {
        crate::Input::File { content, .. } => (content.text(), content.file_name()),
        crate::Input::Stdin { text, recipe_name } => (text.as_str(), recipe_name.as_str()),
    };
    let r = cooklang::parser::parse(text, ctx.parser()?.extensions());
    if r.invalid() || ctx.global_args.warnings_as_errors && r.has_warnings() {
        r.into_report()
            .eprint(file_name, text, ctx.global_args.ignore_warnings)?;
        bail!("Error parsing recipe");
    };
    let (ast, warnings) = r.into_result().unwrap();
    if !ctx.global_args.ignore_warnings && warnings.has_warnings() {
        warnings.eprint(file_name, text, false)?;
    }

    let format = args.format.unwrap_or_else(|| match &args.output {
        Some(p) => match p.extension() {
            Some("json") => OutputFormat::Json,
            _ => OutputFormat::Debug,
        },
        None => OutputFormat::Json,
    });

    crate::write_to_output(args.output.as_deref(), |mut w| {
        match format {
            OutputFormat::Json => {
                if args.pretty {
                    serde_json::to_writer_pretty(w, &ast)?;
                } else {
                    serde_json::to_writer(w, &ast)?;
                }
            }
            OutputFormat::Debug => write!(w, "{ast:#?}")?,
        };
        Ok(())
    })?;

    Ok(())
}

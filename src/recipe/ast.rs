use anyhow::{bail, Result};
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};

use crate::{util::Input, Context};

#[derive(Debug, Args)]
pub struct AstArgs {
    #[command(flatten)]
    input: super::RecipeInputArgs,

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
    let text = input.text();
    let file_name = match &input {
        Input::File { content, .. } => content.file_name(),
        Input::Stdin { name, .. } => name,
    };

    let events = cooklang::parser::PullParser::new(text, ctx.parser()?.extensions());
    let r = cooklang::parser::build_ast(events);
    if !r.is_valid() || ctx.global_args.warnings_as_errors && r.has_warnings() {
        r.into_report().eprint(
            file_name,
            text,
            ctx.global_args.ignore_warnings,
            ctx.color.color_stderr,
        )?;
        bail!("Error parsing recipe");
    };
    let (ast, warnings) = r.into_result().unwrap();
    if !ctx.global_args.ignore_warnings && !warnings.is_empty() {
        warnings.eprint(file_name, text, ctx.color.color_stderr)?;
    }

    let format = args.format.unwrap_or(OutputFormat::Json);

    crate::util::write_to_output(args.output.as_deref(), |mut w| {
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

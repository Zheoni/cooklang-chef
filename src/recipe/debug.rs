use anyhow::{bail, Result};
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};

use crate::{util::Input, Context};

#[derive(Debug, Args)]
pub struct DebugArgs {
    #[command(flatten)]
    input: super::RecipeInputArgs,

    /// Output file, none for stdout
    #[arg(short, long)]
    output: Option<Utf8PathBuf>,

    /// Build an AST instead of the event stream
    #[arg(long)]
    ast: bool,

    /// Output format
    #[arg(short, long, value_enum, requires = "ast")]
    format: Option<OutputFormat>,

    /// Pretty output format
    #[arg(long)]
    pretty: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum OutputFormat {
    Json,
    #[default]
    Debug,
}

pub fn run(ctx: &Context, args: DebugArgs) -> Result<()> {
    let input = args.input.read(&ctx.recipe_index)?;
    let text = input.text();
    let file_name = match &input {
        Input::File { content, .. } => content.file_name(),
        Input::Stdin { name, .. } => name,
    };

    let events = cooklang::parser::PullParser::new(text, ctx.parser()?.extensions());

    if args.ast {
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

        let format = args.format.unwrap_or_else(|| {
            args.output
                .as_ref()
                .map(|p| match p.extension() {
                    Some("json") => OutputFormat::Json,
                    _ => Default::default(),
                })
                .unwrap_or_default()
        });

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
    } else {
        crate::util::write_to_output(args.output.as_deref(), |mut w| {
            for ev in events {
                if args.pretty {
                    writeln!(w, "{ev:#?}")?;
                } else {
                    writeln!(w, "{ev:?}")?;
                }
            }
            Ok(())
        })?;
    }

    Ok(())
}

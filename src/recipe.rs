use std::io::Read;

use anyhow::{bail, Context as _, Result};
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};
use cooklang_fs::{check_recipe_images, recipe_images, resolve_recipe, FsIndex};
use owo_colors::OwoColorize;

use crate::{
    util::{meta_name, unwrap_recipe, write_to_output, Input},
    Context,
};

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct ReadArgs {
    /// Input recipe, none for stdin
    ///
    /// This can be a full path, a partial path, or just the name.
    #[arg(value_hint = clap::ValueHint::FilePath)]
    recipe: Option<Utf8PathBuf>,

    /// Give or override a name for the recipe
    ///
    /// If not given will be obtained from input path.
    #[arg(long, required_unless_present = "recipe")]
    name: Option<String>,

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

    #[group(flatten)]
    values: ScalingArgs,

    #[group(flatten)]
    debug: DebugArgs,

    /// Check the recipe for errors, warnings and images
    #[arg(long, conflicts_with_all = ["ScalingArgs", "DebugArgs"])]
    check: bool,
}

#[derive(Debug, Args)]
#[group(multiple = true)]
struct ScalingArgs {
    /// Scale to a number of servings
    #[arg(short, long, alias = "servings", value_name = "SERVINGS")]
    scale: Option<u32>,

    /// Convert to a unit system
    #[arg(short, long, alias = "system", value_name = "SYSTEM")]
    convert: Option<System>,
}

#[derive(Debug, Args)]
#[group(conflicts_with = "ScalingArgs", multiple = false)]
struct DebugArgs {
    /// Debug output as events
    #[arg(long, hide = true)]
    events: bool,
    /// Debug output as AST
    #[arg(long, hide = true)]
    ast: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
enum OutputFormat {
    Human,
    Json,
    #[value(alias("cook"))]
    Cooklang,
    #[value(alias("md"))]
    Markdown,
    #[value(hide = true)]
    Debug,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum System {
    Metric,
    #[value(alias("freedom"))]
    Imperial,
}

pub fn run(ctx: &Context, args: ReadArgs) -> Result<()> {
    if args.debug.events || args.debug.ast {
        return just_events(ctx, args);
    }
    if args.check {
        return just_check(ctx, args);
    }

    let input = args.read(&ctx.recipe_index)?;

    let recipe = input.parse(ctx)?;

    let mut scaled_recipe = if let Some(scale) = args.values.scale {
        recipe.scale(scale, ctx.parser()?.converter())
    } else {
        recipe.default_scale()
    };

    if let Some(system) = args.values.convert {
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

    let name = match meta_name(&scaled_recipe.metadata) {
        Some(n) => n,
        None => input.name()?,
    };

    write_to_output(args.output.as_deref(), |mut writer| {
        match format {
            OutputFormat::Human => cooklang_to_human::print_human(
                &scaled_recipe,
                name,
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
                    name,
                };

                if args.pretty {
                    serde_json::to_writer_pretty(writer, &recipe)?;
                } else {
                    serde_json::to_writer(writer, &recipe)?;
                }
            }
            OutputFormat::Cooklang => cooklang_to_cooklang::print_cooklang(&scaled_recipe, writer)?,
            OutputFormat::Markdown => cooklang_to_md::print_md_with_options(
                &scaled_recipe,
                name,
                &ctx.config.export.markdown,
                ctx.parser()?.converter(),
                writer,
            )?,
            OutputFormat::Debug => write!(writer, "{scaled_recipe:?}")?,
        }

        Ok(())
    })?;

    Ok(())
}

impl ReadArgs {
    fn read(&self, index: &FsIndex) -> Result<Input> {
        let input = if let Some(query) = &self.recipe {
            // RecipeInputArgs::recipe is a pathbuf even if inmediatly converted
            // to a string to enforce validation.
            let entry = resolve_recipe(query.as_str(), index, None)?;

            Input::File {
                entry,
                override_name: self.name.clone(),
            }
        } else {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("Failed to read stdin")?;
            Input::Stdin {
                text: buf,
                name: self.name.clone(),
            }
        };
        Ok(input)
    }
}

fn just_events(ctx: &Context, args: ReadArgs) -> Result<()> {
    let input = args.read(&ctx.recipe_index)?;
    let text = input.text()?;
    let file_name = input.file_name();

    let events = cooklang::parser::PullParser::new(text, ctx.parser()?.extensions());

    if args.debug.ast {
        let r = cooklang::ast::build_ast(events);
        if !r.is_valid() || ctx.global_args.warnings_as_errors && r.report().has_warnings() {
            let mut report = r.into_report();
            if ctx.global_args.ignore_warnings {
                report.remove_warnings();
            }
            report.eprint(file_name, text, ctx.color.color_stderr)?;
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
                    _ => OutputFormat::Debug,
                })
                .unwrap_or(OutputFormat::Debug)
        });

        write_to_output(args.output.as_deref(), |mut w| {
            match format {
                OutputFormat::Json => {
                    if args.pretty {
                        serde_json::to_writer_pretty(w, &ast)?;
                    } else {
                        serde_json::to_writer(w, &ast)?;
                    }
                }
                OutputFormat::Debug => write!(w, "{ast:#?}")?,
                _ => bail!("Format not supported"),
            };
            Ok(())
        })?;
    } else {
        write_to_output(args.output.as_deref(), |mut w| {
            if args.format.is_some_and(|f| f != OutputFormat::Debug) {
                bail!("Format not supported");
            }
            for ev in events {
                use cooklang::parser::Event;
                let color = match ev {
                    Event::Warning(_) => owo_colors::AnsiColors::Yellow,
                    Event::Error(_) => owo_colors::AnsiColors::Red,
                    Event::Start(_) | Event::End(_) => owo_colors::AnsiColors::Cyan,
                    Event::Ingredient(_) => owo_colors::AnsiColors::BrightGreen,
                    Event::Cookware(_) => owo_colors::AnsiColors::BrightYellow,
                    Event::Timer(_) => owo_colors::AnsiColors::BrightBlue,
                    Event::Metadata { .. } => owo_colors::AnsiColors::Magenta,
                    Event::Section { .. } => owo_colors::AnsiColors::BrightCyan,
                    _ => owo_colors::AnsiColors::Default,
                };
                if args.pretty {
                    writeln!(w, "{:#?}", ev.color(color))?;
                } else {
                    writeln!(w, "{:?}", ev.color(color))?;
                }
            }
            Ok(())
        })?;
    }

    Ok(())
}

fn just_check(ctx: &Context, args: ReadArgs) -> Result<()> {
    let input = args.read(&ctx.recipe_index)?;
    let res = input.parse_result(ctx)?;
    let mut n_warns = 0;
    let mut n_errs = 0;
    let mut n_image_errs = 0;
    for err in res.report().iter() {
        match err.severity {
            cooklang::error::Severity::Error => n_errs += 1,
            cooklang::error::Severity::Warning => n_warns += 1,
        }
    }
    let file_name = input.file_name();
    let recipe = unwrap_recipe(res, file_name, input.text()?, ctx).ok();

    if let Some(recipe) = &recipe {
        if let Some(path) = &input.path() {
            let images = recipe_images(path);

            if let Err(errors) = check_recipe_images(&images, recipe) {
                n_image_errs = errors.len();
                for e in errors {
                    eprintln!("{e}");
                }
            } else {
                eprintln!("Found {} image(s)", images.len());
            }
        } else {
            tracing::warn!("Could not check images, no path given");
        }
    }

    if n_errs > 0 {
        println!("{}: {}", "Errors".red().bold(), n_errs);
    }
    if n_image_errs > 0 {
        println!("{}: {}", "Image errors".purple().bold(), n_image_errs);
    }
    if n_warns > 0 {
        println!("{}: {}", "Warnings".yellow().bold(), n_warns);
    }

    let err_flag =
        n_errs > 0 || n_image_errs > 0 || n_warns > 0 && ctx.global_args.warnings_as_errors;
    let warn_flag = n_warns > 0 && !ctx.global_args.warnings_as_errors;
    if err_flag || warn_flag {
        std::process::exit((warn_flag as i32) << 1 | err_flag as i32);
    }

    eprintln!("{}", "Ok".green().bold());

    Ok(())
}

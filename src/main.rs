use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, Parser, Subcommand};
use config::Config;
use cooklang::{
    convert::{builder::ConverterBuilder, units_file::UnitsFile},
    CooklangParser, Extensions,
};
use cooklang_fs::{resolve_recipe, FsIndex, RecipeContent};
use once_cell::sync::OnceCell;
use tracing::{debug, warn};

mod config;
mod convert;
mod generate_completions;
mod list;
mod recipe;
#[cfg(feature = "serve")]
mod serve;
mod shopping_list;
mod units;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[clap(color = concolor_clap::color_choice())]
struct CliArgs {
    #[command(subcommand)]
    command: Command,

    #[command(flatten)]
    global_args: GlobalArgs,
}

#[derive(Debug, Subcommand, strum::Display)]
enum Command {
    /// Manage recipe files
    #[command(alias = "r")]
    Recipe(recipe::RecipeArgs),
    /// List all the recipes
    #[command(alias = "l", visible_alias = "ls")]
    List(list::ListArgs),
    #[cfg(feature = "serve")]
    /// Recipes web server
    Serve(serve::ServeArgs),
    /// Creates a shopping list from a given list of recipes
    #[command(visible_alias = "sl")]
    ShoppingList(shopping_list::ShoppingListArgs),
    /// Manage unit files
    Units(units::UnitsArgs),
    /// Convert values and units
    #[command(visible_alias = "c")]
    Convert(convert::ConvertArgs),
    /// See loaded configuration
    Config,
    /// Generate shell completions
    GenerateCompletions(generate_completions::GenerateCompletionsArgs),
}

#[derive(Debug, Args)]
pub struct GlobalArgs {
    /// A units TOML file
    #[arg(long, action = clap::ArgAction::Append, global = true)]
    units: Vec<Utf8PathBuf>,

    /// Do not use the bundled units
    #[arg(long, hide_short_help = true, global = true)]
    no_default_units: bool,

    /// Disable all extensions
    #[arg(long, alias = "no-default-extensions", group = "ext", global = true)]
    no_extensions: bool,

    /// Enable all extensions
    #[arg(long, group = "ext", global = true)]
    all_extensions: bool,

    /// Enable a set of extensions
    ///
    /// Can be specified multiple times.
    #[arg(
        short,
        long,
        group = "ext",
        value_parser = bitflags::parser::from_str::<Extensions>,
        action = clap::ArgAction::Append,
        global = true
    )]
    extensions: Vec<Extensions>,

    /// Treat warnings as errors
    #[arg(long, hide_short_help = true, global = true)]
    warnings_as_errors: bool,

    /// Do not display warnings generated from parsing recipes
    #[arg(
        long,
        hide_short_help = true,
        conflicts_with = "warnings_as_errors",
        global = true
    )]
    ignore_warnings: bool,

    #[command(flatten)]
    color: concolor_clap::Color,

    /// Change the base path. By default uses the current working directory.
    ///
    /// This path is used to load configuration files, search for images and
    /// recipe references.
    #[arg(long, value_name = "PATH", value_hint = clap::ValueHint::DirPath, global = true)]
    path: Option<Utf8PathBuf>,

    /// Skip checking if referenced recipes exist
    #[arg(long, hide_short_help = true, global = true)]
    no_recipe_ref_check: bool,

    /// Override recipe indexing depth
    ///
    /// This is used to search for referenced recipes.
    #[arg(long, global = true, default_value_t = 10)]
    max_depth: usize,

    #[arg(long, hide_short_help = true, global = true)]
    debug_trace: bool,
}

pub fn main() -> Result<()> {
    let args = CliArgs::parse();

    init_color(args.global_args.color);

    if args.global_args.debug_trace {
        tracing_subscriber::FmtSubscriber::builder()
            .compact()
            .with_max_level(tracing::Level::TRACE)
            .with_span_events(
                tracing_subscriber::fmt::format::FmtSpan::CLOSE
                    | tracing_subscriber::fmt::format::FmtSpan::NEW,
            )
            .with_ansi(concolor::get(concolor::Stream::Stderr).ansi_color())
            .init();
    } else {
        tracing_subscriber::FmtSubscriber::builder()
            .compact()
            .with_target(false)
            .with_ansi(concolor::get(concolor::Stream::Stderr).ansi_color())
            .init();
    }

    let ctx = configure_context(args.global_args)?;

    match args.command {
        Command::Recipe(args) => recipe::run(&ctx, args),
        Command::List(args) => list::run(&ctx, args),
        #[cfg(feature = "serve")]
        Command::Serve(args) => serve::run(ctx, args),
        Command::ShoppingList(args) => shopping_list::run(&ctx, args),
        Command::Units(args) => units::run(ctx.parser()?.converter(), args),
        Command::Convert(args) => convert::run(ctx.parser()?.converter(), args),
        Command::Config => config::run(&ctx),
        Command::GenerateCompletions(args) => generate_completions::run(args),
    }
}

fn init_color(color: concolor_clap::Color) {
    color.apply();
    let stdout_support = concolor::get(concolor::Stream::Stdout);
    if stdout_support.ansi_color() {
        yansi::Paint::enable();
    } else if stdout_support.color() {
        // Legacy Windows version, control the console as needed
        if cfg!(windows) && !yansi::Paint::enable_windows_ascii() {
            yansi::Paint::disable();
        }
    } else {
        // No coloring
        yansi::Paint::disable();
    }
}

pub struct Context {
    parser: OnceCell<CooklangParser>,
    recipe_index: FsIndex,
    global_args: GlobalArgs,
    base_dir: Utf8PathBuf,
    config: config::Config,
    config_path: PathBuf,
}

const COOK_DIR: &str = ".cooklang";
const APP_NAME: &str = "cooklang-chef";

#[tracing::instrument(level = "debug", skip_all)]
fn configure_context(args: GlobalArgs) -> Result<Context> {
    let base_dir = args
        .path
        .as_deref()
        .unwrap_or(Utf8Path::new("."))
        .to_path_buf();
    let (mut config, config_path) = Config::read(&base_dir)?;
    config.override_with_args(&args);
    if !base_dir.is_dir() {
        bail!("Base path '{base_dir}' is not a directory");
    }

    let mut index = FsIndex::new(&base_dir, config.max_depth)?;
    index.set_config_dir(COOK_DIR.to_string());

    Ok(Context {
        parser: OnceCell::new(),
        recipe_index: index,
        config,
        config_path,
        global_args: args,
        base_dir,
    })
}

impl Context {
    fn parser(&self) -> Result<&CooklangParser> {
        self.parser.get_or_try_init(|| {
            configure_parser(&self.config, self.base_dir.as_std_path(), &self.config_path)
        })
    }

    fn checker(&self, relative_to: Option<&Utf8Path>) -> Option<cooklang::RecipeRefChecker> {
        if self.global_args.no_recipe_ref_check {
            None
        } else {
            let relative_to = relative_to.map(|r| r.to_path_buf());
            Some(Box::new(move |name: &str| {
                resolve_recipe(name, &self.recipe_index, relative_to.as_deref()).is_ok()
            }) as cooklang::RecipeRefChecker)
        }
    }

    fn parse_content(
        &self,
        content: &cooklang_fs::RecipeContent,
    ) -> Result<cooklang::RecipeResult> {
        Ok(self.parser()?.parse_with_recipe_ref_checker(
            content.text(),
            content.name(),
            self.checker(Some(content.path())),
        ))
    }
}

#[tracing::instrument(level = "debug", skip_all)]
fn configure_parser(
    config: &Config,
    base_path: &Path,
    config_path: &Path,
) -> Result<CooklangParser> {
    let mut parser = CooklangParser::builder().with_extensions(config.extensions);

    let units = config.units(config_path, base_path);
    if config.default_units || !units.is_empty() {
        let mut builder = ConverterBuilder::new();

        if config.default_units {
            builder
                .add_units_file(UnitsFile::bundled())
                .expect("Failed to add bundled units");
        }
        for file in units {
            debug!("Loading units {}", file.display());
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot find units file: {}", file.display()))?;
            let units = toml::from_str(&text)?;
            builder.add_units_file(units)?;
        }
        parser.set_converter(builder.finish()?);
    }
    Ok(parser.finish())
}

fn write_to_output<F>(output: Option<&Utf8Path>, f: F) -> Result<()>
where
    F: FnOnce(Box<dyn std::io::Write>) -> Result<()>,
{
    if let Some(path) = output {
        let file = std::fs::File::create(path).context("Failed to create output file")?;
        let colors = yansi::Paint::is_enabled();
        yansi::Paint::disable();
        f(Box::new(file))?;
        if colors {
            yansi::Paint::enable();
        }
        Ok(())
    } else {
        f(Box::new(std::io::stdout()))?;
        Ok(())
    }
}

enum Input {
    File {
        content: RecipeContent,
        override_name: Option<String>,
    },
    Stdin {
        text: String,
        recipe_name: String,
    },
}

impl Input {
    fn parse(&self, ctx: &Context) -> Result<cooklang::Recipe> {
        match self {
            Input::File {
                content,
                override_name,
            } => {
                let r = ctx.parse_content(content)?.map(|mut r| {
                    if let Some(name) = override_name {
                        r.name = name.clone();
                    }
                    r
                });
                unwrap_recipe(r, content.file_name(), content.text(), ctx)
            }
            Input::Stdin { text, recipe_name } => {
                let r = ctx.parser()?.parse_with_recipe_ref_checker(
                    text,
                    recipe_name,
                    ctx.checker(None),
                );
                unwrap_recipe(r, recipe_name, text, ctx)
            }
        }
    }

    fn path(&self) -> Option<&Utf8Path> {
        match self {
            Input::File { content, .. } => Some(content.path()),
            Input::Stdin { .. } => None,
        }
    }
}

fn unwrap_recipe(
    r: cooklang::RecipeResult,
    file_name: &str,
    text: &str,
    ctx: &Context,
) -> Result<cooklang::Recipe> {
    if r.invalid() || ctx.global_args.warnings_as_errors && r.has_warnings() {
        r.into_report()
            .eprint(file_name, text, ctx.global_args.ignore_warnings)?;
        bail!("Error parsing recipe");
    } else {
        let (recipe, warnings) = r.into_result().unwrap();
        if !ctx.global_args.ignore_warnings && warnings.has_warnings() {
            warnings.eprint(file_name, text, false)?;
        }
        Ok(recipe)
    }
}

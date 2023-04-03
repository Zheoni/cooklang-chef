use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, Parser, Subcommand};
use config::Config;
use cooklang::{
    convert::{builder::ConverterBuilder, units_file::UnitsFile},
    CooklangParser,
};
use cooklang_fs::FsIndex;
use once_cell::sync::OnceCell;
use tracing::{info, warn};

mod config;
mod convert;
mod recipe;
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
    Recipe(Box<recipe::RecipeArgs>),
    /// Creates a shopping list from a given list of recipes
    #[command(alias = "list")]
    ShoppingList(shopping_list::ShoppingListArgs),
    /// Manage unit files
    #[command(alias = "u")]
    Units(units::UnitsArgs),
    /// Convert values and units
    #[command(alias = "c")]
    Convert(convert::ConvertArgs),
    /// See loaded configuration
    Config,
}

#[derive(Debug, Args)]
pub struct GlobalArgs {
    /// A units TOML file
    #[arg(long, action = clap::ArgAction::Append, global = true)]
    units: Vec<Utf8PathBuf>,

    /// Do not use the bundled units
    #[arg(long, hide_short_help = true, global = true)]
    no_default_units: bool,

    /// Disable all extensions to the cooklang
    /// spec <https://cooklang.org/docs/spec/>
    #[arg(long, global = true)]
    no_extensions: bool,

    /// Treat warnings as errors
    #[arg(long, global = true)]
    warnings_as_errors: bool,

    /// Do not display warnings generated from parsing recipes
    #[arg(long, conflicts_with = "warnings_as_errors", global = true)]
    ignore_warnings: bool,

    #[command(flatten)]
    color: concolor_clap::Color,

    /// Change the base path. By default uses the current working directory.
    ///
    /// This path is used to load configuration files, search for images and
    /// recipe references.
    #[arg(long, value_name = "PATH", global = true)]
    path: Option<Utf8PathBuf>,

    /// Skip checking if referenced recipes exist
    #[arg(long, hide_short_help = true, global = true)]
    no_recipe_ref_check: bool,

    /// Override recipe indexing depth. Defaults to 3.
    ///
    /// This is used to search for referenced recipes.
    #[arg(long, global = true)]
    max_depth: Option<usize>,

    #[arg(long, hide_short_help = true, global = true)]
    debug_trace: bool,
}

pub fn main() -> Result<()> {
    let args = CliArgs::parse();

    init_color(args.global_args.color);

    if args.global_args.debug_trace {
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
            .with_ansi(concolor::get(concolor::Stream::Stderr).ansi_color())
            .init();
    }

    let ctx = configure_context(args.global_args)?;

    let _enter = tracing::info_span!("run", cmd = %args.command).entered();
    match args.command {
        Command::Recipe(args) => recipe::run(&ctx, *args),
        Command::ShoppingList(args) => shopping_list::run(&ctx, args),
        Command::Units(args) => units::run(ctx.parser()?.converter(), args),
        Command::Convert(args) => convert::run(ctx.parser()?.converter(), args),
        Command::Config => config::run(&ctx),
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

#[tracing::instrument(skip_all)]
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

    let index = FsIndex::new(&base_dir, config.max_depth)?;

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
}

#[tracing::instrument(skip_all)]
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
            info!("Loading units {}", file.display());
            let text = std::fs::read_to_string(file)?;
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

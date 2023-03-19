use anyhow::{bail, Context as _, Result};
use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use clap::{Parser, Subcommand};
use cooklang::{
    convert::{builder::ConverterBuilder, units_file::UnitsFile},
    CooklangParser, Extensions,
};
use cooklang_fs::FsIndex;
use tracing::warn;

mod convert;
mod recipe;
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
    Recipe(Box<recipe::RecipeArgs>),
    /// Run a web server that serves of your recipes
    Serve,
    /// Creates a shopping list from a given list of recipes
    #[command(alias = "list")]
    ShoppingList(shopping_list::ShoppingListArgs),
    /// Manage unit files
    #[command(alias = "u")]
    Units(units::UnitsArgs),
    /// Convert values and units
    #[command(alias = "c")]
    Convert(convert::ConvertArgs),
}

#[derive(Debug, Parser)]
struct GlobalArgs {
    /// A units TOML file
    #[arg(long, action = clap::ArgAction::Append, global = true)]
    units: Vec<PathBuf>,

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
    base: Option<PathBuf>,

    /// Skip checking if referenced recipes exist
    #[arg(long, hide_short_help = true, global = true)]
    no_recipe_ref_check: bool,

    /// Recipe indexing depth
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
        Command::Serve => serve::run(&ctx),
        Command::ShoppingList(args) => shopping_list::run(&ctx, args),
        Command::Units(args) => units::run(ctx.parser.converter(), args),
        Command::Convert(args) => convert::run(ctx.parser.converter(), args),
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
    parser: CooklangParser,
    recipe_index: FsIndex,
    global_args: GlobalArgs,
    base_dir: PathBuf,
    max_depth: usize,
    config_dir: Option<PathBuf>,
}

const CONFIG_DIR: &str = ".cooklang";

#[tracing::instrument(skip_all)]
fn configure_context(args: GlobalArgs) -> Result<Context> {
    let parser = configure_parser(&args)?;
    let base_dir = args
        .base
        .as_deref()
        .unwrap_or(Path::new("."))
        .canonicalize_utf8()
        .context("Invalid base path")?;
    if !base_dir.is_dir() {
        bail!("Base path '{base_dir}' is not a directory");
    }
    let config_dir = {
        let dir = base_dir.join(CONFIG_DIR);
        if dir.is_dir() {
            Some(dir)
        } else {
            warn!(
                "Config directory '{}' not found. Some functionality may be limited.",
                CONFIG_DIR
            );
            None
        }
    };

    // If we are not in a known cooklang dir (with a config dir) limit the indexing
    let depth = args.max_depth.unwrap_or_else(|| match config_dir {
        Some(_) => usize::MAX,
        None => 2,
    });
    let index = FsIndex::new(&base_dir, depth)?;

    Ok(Context {
        parser,
        recipe_index: index,
        global_args: args,
        base_dir,
        config_dir,
        max_depth: depth,
    })
}

fn configure_parser(args: &GlobalArgs) -> Result<CooklangParser> {
    let mut parser = CooklangParser::builder().with_extensions(if args.no_extensions {
        Extensions::empty()
    } else {
        Extensions::all()
    });
    if !args.no_default_units || !args.units.is_empty() {
        let mut builder = ConverterBuilder::new();
        if !args.no_default_units {
            builder
                .add_units_file(UnitsFile::bundled())
                .expect("Failed to add bundled units");
        }
        for file in &args.units {
            let text = std::fs::read_to_string(file)?;
            let units = toml::from_str(&text)?;
            builder.add_units_file(units)?;
        }
        parser.set_converter(builder.finish()?);
    }
    Ok(parser.finish())
}

fn write_to_output<F>(output: Option<&Path>, f: F) -> Result<()>
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

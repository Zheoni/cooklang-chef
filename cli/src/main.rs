use std::path::PathBuf;

use clap::{ColorChoice, Parser, Subcommand};
use cooklang::{
    convert::{builder::ConverterBuilder, units_file::UnitsFile},
    CooklangParser, Extensions,
};
use miette::{IntoDiagnostic, Result};

mod recipe;
mod serve;
mod shopping_list;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[command(flatten)]
    global_args: GlobalArgs,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Manage recipe files
    Recipe(Box<recipe::RecipeArgs>),
    /// Run a web server that serves of your recipes
    Serve,
    /// Creates a shopping list from a given list of recipes
    ShoppingList,
}

#[derive(Debug, Parser)]
struct GlobalArgs {
    /// A units TOML file
    #[arg(long, action = clap::ArgAction::Append, global = true)]
    units: Vec<PathBuf>,

    /// Do not use the bundled units
    #[arg(long, global = true)]
    no_default_units: bool,

    /// Disable all extensions to the cooklang
    /// spec <https://cooklang.org/docs/spec/>
    #[arg(long, global = true)]
    no_extensions: bool,

    /// Treat warnings as errors
    #[arg(long, global = true)]
    warnings_as_errors: bool,

    #[arg(long, global = true, default_value = "auto")]
    color: ColorChoice,
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    init_color(&args.global_args.color);

    let parser = configure_parser(args.global_args)?;

    match args.command {
        Command::Recipe(args) => recipe::run(&parser, *args),
        Command::Serve => serve::run(&parser),
        Command::ShoppingList => shopping_list::run(&parser),
    }
}

static STD_OUT_COLOR_ENABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
static STD_ERR_COLOR_ENABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

fn init_color(color: &ColorChoice) {
    fn set_miette_color(color: bool) {
        miette::set_hook(Box::new(move |_| {
            Box::new(miette::MietteHandlerOpts::new().color(color).build())
        }))
        .expect("Error initializing error formatter")
    }

    match color {
        ColorChoice::Auto => {
            STD_OUT_COLOR_ENABLED.store(
                console::colors_enabled(),
                std::sync::atomic::Ordering::Relaxed,
            );
            STD_ERR_COLOR_ENABLED.store(
                console::colors_enabled_stderr(),
                std::sync::atomic::Ordering::Relaxed,
            );
        }
        ColorChoice::Always => {
            console::set_colors_enabled(true);
            set_miette_color(true);
            STD_OUT_COLOR_ENABLED.store(true, std::sync::atomic::Ordering::Relaxed);
            STD_ERR_COLOR_ENABLED.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        ColorChoice::Never => {
            console::set_colors_enabled(false);
            set_miette_color(false);
            STD_OUT_COLOR_ENABLED.store(false, std::sync::atomic::Ordering::Relaxed);
            STD_ERR_COLOR_ENABLED.store(false, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

fn configure_parser(args: GlobalArgs) -> Result<CooklangParser> {
    let mut parser = CooklangParser::builder()
        .warnings_as_errors(args.warnings_as_errors)
        .with_extensions(if args.no_extensions {
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
            let text = std::fs::read_to_string(file).into_diagnostic()?;
            let units = toml::from_str(&text).into_diagnostic()?;
            builder.add_units_file(units)?;
        }
        parser.set_converter(builder.finish()?);
    }
    Ok(parser.finish())
}

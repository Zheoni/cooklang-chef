use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cooklang::{
    convert::{builder::ConverterBuilder, units_file::UnitsFile},
    CooklangParser, Extensions,
};

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
    Recipe(Box<recipe::RecipeArgs>),
    /// Run a web server that serves of your recipes
    Serve,
    /// Creates a shopping list from a given list of recipes
    ShoppingList(shopping_list::ShoppingListArgs),
    /// Manage unit files
    Units(units::UnitsArgs),
    /// Convert values and units
    Convert(convert::ConvertArgs),
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

    #[command(flatten)]
    color: concolor_clap::Color,

    #[arg(long, global = true)]
    debug_trace: bool,
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

    let parser = configure_parser(args.global_args)?;

    let _enter = tracing::info_span!("run", command = %args.command).entered();
    match args.command {
        Command::Recipe(args) => recipe::run(&parser, *args),
        Command::Serve => serve::run(&parser),
        Command::ShoppingList(args) => shopping_list::run(&parser, args),
        Command::Units(args) => units::run(parser.converter(), args),
        Command::Convert(args) => convert::run(parser.converter(), args),
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

#[tracing::instrument(skip_all)]
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
            let text = std::fs::read_to_string(file)?;
            let units = toml::from_str(&text)?;
            builder.add_units_file(units)?;
        }
        parser.set_converter(builder.finish()?);
    }
    Ok(parser.finish())
}

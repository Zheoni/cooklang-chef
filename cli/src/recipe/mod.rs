use std::{io::Read, path::PathBuf};

use clap::{Args, Subcommand};
use cooklang::CooklangParser;
use miette::{Context, IntoDiagnostic, Result};

use self::read::ReadArgs;

mod check;
mod format;
mod image;
mod read;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct RecipeArgs {
    #[command(subcommand)]
    command: Option<RecipeCommand>,

    #[command(flatten)]
    read_args: ReadArgs,
}

#[derive(Debug, Args)]
struct RecipeInput {
    /// Input file path, none for stdin
    file: Option<PathBuf>,

    /// Give or override a name for the recipe
    ///
    /// If not given will be obtained from input path.
    #[arg(short, long, required_unless_present = "file")]
    name: Option<String>,
}

#[derive(Debug, Subcommand)]
enum RecipeCommand {
    /// Reads a recipe file
    Read(ReadArgs),
    /// Checks a recipe file for errors or warnings
    Check,
    /// Formats a recipe file with a consistent format
    Format,
    /// Automatically downloads an image for the recipe based on it's name
    Image,
}

pub fn run(parser: &CooklangParser, args: RecipeArgs) -> Result<()> {
    let command = args.command.unwrap_or(RecipeCommand::Read(args.read_args));

    match command {
        RecipeCommand::Read(args) => read::run(parser, args),
        RecipeCommand::Check => check::run(parser),
        RecipeCommand::Format => format::run(parser),
        RecipeCommand::Image => image::run(parser),
    }
}

impl RecipeInput {
    pub fn read(&self) -> Result<(String, String)> {
        let (text, filename) = if let Some(path) = &self.file {
            let filename = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .map(|name| {
                    name.strip_suffix(".cook")
                        .map(|s| s.to_string())
                        .unwrap_or(name)
                });
            let text = std::fs::read_to_string(path)
                .into_diagnostic()
                .wrap_err("Failed to read input file")?;
            (text, filename)
        } else {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .into_diagnostic()
                .wrap_err("Failed to read stdin")?;
            (buf, None)
        };

        if let Some(name) = self.name.as_ref().or(filename.as_ref()) {
            Ok((text, name.to_owned()))
        } else {
            miette::bail!("No name for the recipe")
        }
    }
}

use std::io::Read;

use anyhow::{bail, Context as _, Result};
use camino::Utf8PathBuf as PathBuf;
use clap::{Args, Subcommand};

use crate::Context;

use self::read::ReadArgs;

mod check;
mod image;
mod list;
mod read;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct RecipeArgs {
    #[command(subcommand)]
    command: Option<RecipeCommand>,

    #[command(flatten)]
    read_args: ReadArgs,
}

#[derive(Debug, Subcommand)]
enum RecipeCommand {
    /// Reads a recipe file
    #[command(alias = "r")]
    Read(ReadArgs),
    /// Checks a recipe file for errors or warnings
    #[command(alias = "c")]
    Check(check::CheckArgs),
    /// List all the recipes
    #[command(alias = "l")]
    List(list::ListArgs),
    /// Automatically downloads an image for the recipe based on it's name
    Image,
}

pub fn run(ctx: &Context, args: RecipeArgs) -> Result<()> {
    let command = args.command.unwrap_or(RecipeCommand::Read(args.read_args));

    match command {
        RecipeCommand::Read(args) => read::run(ctx, args),
        RecipeCommand::Check(args) => check::run(ctx, args),
        RecipeCommand::List(args) => list::run(ctx, args),
        RecipeCommand::Image => image::run(ctx),
    }
}

#[derive(Debug, Args)]
struct RecipeInputArgs {
    /// Input file path, none for stdin
    file: Option<PathBuf>,

    /// Give or override a name for the recipe
    ///
    /// If not given will be obtained from input path.
    #[arg(short, long, required_unless_present = "file")]
    name: Option<String>,
}

impl RecipeInputArgs {
    pub fn read(&self) -> Result<Input> {
        let (text, recipe_name, file_name) = if let Some(path) = &self.file {
            if !path.is_file() {
                bail!("Input is not a file");
            }
            let recipe_name = path.file_stem().expect("file without filename").to_string();
            let file_name = path.file_name().expect("file without filename").to_string();
            let text = std::fs::read_to_string(path).context("Failed to read input file")?;
            (text, Some(recipe_name), Some(file_name))
        } else {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("Failed to read stdin")?;
            (buf, None, None)
        };

        if let Some(name) = self.name.as_ref().or(recipe_name.as_ref()) {
            Ok(Input {
                text,
                recipe_name: name.to_owned(), // ? Unnecesary alloc
                file_name: file_name.unwrap_or_else(|| name.to_owned()),
                path: self.file.clone(),
            })
        } else {
            bail!("No name for the recipe")
        }
    }
}

struct Input {
    text: String,
    recipe_name: String,
    file_name: String,
    path: Option<PathBuf>,
}

impl Input {
    fn parse<'a>(&'a self, ctx: &Context) -> Result<cooklang::Recipe<'a>> {
        let checker = if ctx.global_args.no_recipe_ref_check {
            None
        } else {
            Some(Box::new(|name: &str| ctx.recipe_index.contains(name))
                as cooklang::RecipeRefChecker)
        };
        let r = ctx
            .parser
            .parse_with_recipe_ref_checker(&self.text, &self.recipe_name, checker);

        if r.invalid() || ctx.global_args.warnings_as_errors && r.has_warnings() {
            r.into_report()
                .eprint(&self.file_name, &self.text, ctx.global_args.ignore_warnings)?;
            bail!("Error parsing recipe");
        } else {
            let (recipe, warnings) = r.into_result().unwrap();
            if !ctx.global_args.ignore_warnings && warnings.has_warnings() {
                warnings.eprint(&self.file_name, &self.text, false)?;
            }
            Ok(recipe)
        }
    }
}

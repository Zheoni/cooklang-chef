use anyhow::{bail, Result};
use clap::Args;
use cooklang_fs::{check_recipe_images, recipe_images};

use crate::Context;

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[command(flatten)]
    input: super::RecipeInputArgs,

    /// Check images
    #[arg(long, short)]
    images: bool,
}

pub fn run(ctx: &Context, args: CheckArgs) -> Result<()> {
    let input = args.input.read()?;
    let recipe = input.parse(ctx)?;
    if args.images {
        if let Some(path) = &input.path {
            if let Err(errors) = check_recipe_images(&recipe_images(path), &recipe) {
                for e in errors {
                    eprintln!("{e}");
                }
                bail!("Error in images");
            }
        } else {
            bail!("No path given to check images")
        }
    }
    Ok(())
}

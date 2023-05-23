use anyhow::{bail, Result};
use clap::Args;
use cooklang_fs::{check_recipe_images, recipe_images};

use crate::Context;

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[command(flatten)]
    input: super::RecipeInputArgs,

    /// Skip images check
    #[arg(long, short = 'I')]
    no_images: bool,
}

pub fn run(ctx: &Context, args: CheckArgs) -> Result<()> {
    let input = args.input.read(&ctx.recipe_index)?;
    let recipe = input.parse(ctx)?;
    if !args.no_images {
        if let Some(path) = &input.path() {
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

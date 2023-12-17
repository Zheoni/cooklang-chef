use anstream::{eprintln, println};
use anyhow::{bail, Result};
use clap::Args;
use cooklang_fs::{check_recipe_images, recipe_images};
use owo_colors::OwoColorize;

use crate::{
    util::{unwrap_recipe, Input},
    Context,
};

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[command(flatten)]
    input: super::RecipeInputArgs,

    /// Skip images check
    #[arg(long, short = 'I')]
    no_images: bool,

    /// Show only count
    #[arg(long, short)]
    count: bool,
}

pub fn run(ctx: &Context, args: CheckArgs) -> Result<()> {
    let input = args.input.read(&ctx.recipe_index)?;
    let res = input.parse_result(ctx)?;
    let mut n_warns = 0;
    let mut n_errs = 0;
    for err in res.report().iter() {
        match err.severity {
            cooklang::error::Severity::Error => n_errs += 1,
            cooklang::error::Severity::Warning => n_warns += 1,
        }
    }

    let file_name = match &input {
        Input::File { content, .. } => content.file_name(),
        Input::Stdin { name, .. } => name,
    };

    let recipe = if args.count {
        let valid =
            res.is_valid() && !(res.report().has_warnings() && ctx.global_args.warnings_as_errors);
        res.into_output().filter(|_| valid)
    } else {
        unwrap_recipe(res, file_name, input.text(), ctx).ok()
    };

    let mut n_image_errs = 0;
    if !args.no_images && recipe.is_some() {
        let recipe = recipe.as_ref().unwrap();
        if let Some(path) = &input.path() {
            if let Err(errors) = check_recipe_images(&recipe_images(path), recipe) {
                n_image_errs = errors.len();
                if !args.count {
                    for e in errors {
                        eprintln!("{e}");
                    }
                }
            }
        } else {
            bail!("No path given to check images")
        }
    }

    if n_errs > 0 {
        println!("{}: {}", "Errors".red().bold(), n_errs);
    }
    if n_image_errs > 0 && !args.no_images {
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

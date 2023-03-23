use anyhow::{bail, Context as _, Result};
use camino::Utf8PathBuf;
use clap::{Args, Subcommand};
use tracing::warn;

use crate::Context;

mod conf;
mod create;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct ShoppingListArgs {
    #[command(subcommand)]
    command: Option<ShoppingListCommands>,
    #[command(flatten)]
    create_args: create::CreateArgs,
    #[command(flatten)]
    global_args: ShoppingListGlobalArgs,
}

#[derive(Debug, Subcommand)]
enum ShoppingListCommands {
    /// Create a shopping list
    Create(create::CreateArgs),
    /// Manage shopping list aile configuration
    Conf(conf::ConfArgs),
}

#[derive(Debug, Args)]
struct ShoppingListGlobalArgs {
    #[arg(short, long, global = true)]
    aile: Option<Utf8PathBuf>,
}

pub fn run(ctx: &Context, args: ShoppingListArgs) -> Result<()> {
    let command = args
        .command
        .unwrap_or(ShoppingListCommands::Create(args.create_args));

    let aile_path = args
        .global_args
        .aile
        .map(|a| a.into_std_path_buf())
        .or_else(|| ctx.config.aile(ctx))
        .map(|path| -> Result<(_, _)> {
            let content = std::fs::read_to_string(&path).context("Failed to read aile file")?;
            Ok((path, content))
        })
        .transpose()?;

    let aile = if let Some((path, content)) = &aile_path {
        match cooklang::shopping_list::parse(content) {
            Ok(conf) => conf,
            Err(e) => {
                cooklang::error::write_rich_error(
                    &e,
                    path.to_str().unwrap_or("<aile>"),
                    content,
                    std::io::stderr(),
                )?;
                bail!("Error parsing aile file")
            }
        }
    } else {
        warn!("No aile file found");
        Default::default()
    };

    match command {
        ShoppingListCommands::Create(args) => create::run(ctx, args),
        ShoppingListCommands::Conf(args) => conf::run(aile, args),
    }
}

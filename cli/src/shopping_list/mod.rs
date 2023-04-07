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
    /// Manage shopping list aisle configuration
    Conf(conf::ConfArgs),
}

#[derive(Debug, Args)]
struct ShoppingListGlobalArgs {
    #[arg(short, long, global = true)]
    aisle: Option<Utf8PathBuf>,
}

pub fn run(ctx: &Context, args: ShoppingListArgs) -> Result<()> {
    let command = args
        .command
        .unwrap_or(ShoppingListCommands::Create(args.create_args));

    let aile_path = args
        .global_args
        .aisle
        .map(|a| a.into_std_path_buf())
        .or_else(|| ctx.config.aisle(ctx))
        .map(|path| -> Result<(_, _)> {
            let content = std::fs::read_to_string(&path).context("Failed to read aisle file")?;
            Ok((path, content))
        })
        .transpose()?;

    let aisle = if let Some((path, content)) = &aile_path {
        match cooklang::aisle::parse(content) {
            Ok(conf) => conf,
            Err(e) => {
                cooklang::error::write_rich_error(
                    &e,
                    path.to_str().unwrap_or("<aisle>"),
                    content,
                    std::io::stderr(),
                )?;
                bail!("Error parsing aisle file")
            }
        }
    } else {
        warn!("No aisle file found");
        Default::default()
    };

    match command {
        ShoppingListCommands::Create(args) => create::run(ctx, aisle, args),
        ShoppingListCommands::Conf(args) => conf::run(aisle, args),
    }
}

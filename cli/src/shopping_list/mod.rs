use anyhow::{bail, Context as _, Result};
use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
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
    aile: Option<PathBuf>,
}

const DEFAULT_AILE_FILE: &str = "aile.conf";

pub fn run(ctx: &Context, args: ShoppingListArgs) -> Result<()> {
    let command = args
        .command
        .unwrap_or(ShoppingListCommands::Create(args.create_args));

    let aile_path = args
        .global_args
        .aile
        .or_else(|| {
            let in_config_dir = ctx.config_dir.as_ref().map(|c| c.join(DEFAULT_AILE_FILE))?;
            in_config_dir.is_file().then_some(in_config_dir)
        })
        .or_else(|| {
            let in_current_dir = Path::new("aile.conf");
            in_current_dir
                .is_file()
                .then(|| in_current_dir.to_path_buf())
        })
        .map(|path| -> Result<(_, _)> {
            let content = std::fs::read_to_string(&path).context("Failed to read aile file")?;
            Ok((path, content))
        })
        .transpose()?;

    let aile = if let Some((path, content)) = &aile_path {
        match cooklang::shopping_list::parse(content) {
            Ok(conf) => conf,
            Err(e) => {
                cooklang::error::write_rich_error(&e, path.as_str(), content, std::io::stderr())?;
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

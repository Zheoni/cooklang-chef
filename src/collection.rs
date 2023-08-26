use std::fs;

use anyhow::{bail, Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, Subcommand};

use crate::{
    config::{
        config_file_path, global_file_path, global_store, store_at_path, Config, GlobalConfig,
        DEFAULT_CONFIG_FILE, GLOBAL_CONFIG_FILE,
    },
    Context, COOK_DIR,
};

#[derive(Debug, Args)]
pub struct CollectionArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a new recipe collection
    New {
        #[arg(value_name = "PATH", conflicts_with = "path")]
        new_path: Utf8PathBuf,
        /// Copy the default config into the local `config.toml` file
        #[arg(long)]
        copy_config: bool,
        #[arg(long, alias = "default")]
        set_default: bool,
    },
    /// Set or get the default collection
    Default {
        #[arg(value_name = "PATH", conflicts_with = "path")]
        default_path: Option<Utf8PathBuf>,
        #[arg(long, conflicts_with_all = ["path", "default_path"])]
        remove: bool,
    },
}

pub fn run(ctx: &Context, args: CollectionArgs) -> Result<()> {
    match args.command {
        Command::New {
            new_path: path,
            copy_config,
            set_default,
        } => {
            create_collection(&path)?;
            if copy_config {
                let config = config_file_path(&path);
                let default = global_file_path(DEFAULT_CONFIG_FILE)?;
                if default.is_file() {
                    fs::copy(default, config).context("Failed to copy default config file")?;
                } else {
                    store_at_path(config, &Config::default())?;
                }
            }
            if set_default {
                set_default_collection(&ctx.global_config, Some(path))?;
            }
        }
        Command::Default {
            default_path: path,
            remove,
        } => {
            if remove {
                return set_default_collection(&ctx.global_config, None);
            }

            if let Some(path) = path {
                if path.is_dir() && path.join(COOK_DIR).is_dir() {
                    set_default_collection(&ctx.global_config, Some(path))?;
                } else {
                    bail!("Invalid collection path");
                }
            } else if let Some(default) = &ctx.global_config.default_collection {
                println!("{default}");
            } else {
                eprintln!("No default collection is set");
            }
        }
    }
    Ok(())
}

fn create_collection(path: &Utf8Path) -> Result<()> {
    if path.exists() {
        if !path.is_dir() {
            bail!("Path exists and it's not a dir");
        }
        if path.read_dir()?.any(|_| true) {
            bail!("Path exists and it's not empty");
        }
    } else {
        fs::create_dir_all(&path).context("Failed to create collection dir")?;
    }
    fs::create_dir_all(path.join(COOK_DIR))?;
    Ok(())
}

fn set_default_collection(global: &GlobalConfig, path: Option<Utf8PathBuf>) -> Result<()> {
    let mut global = global.clone();
    global.default_collection = path;
    global_store(GLOBAL_CONFIG_FILE, &global)
}

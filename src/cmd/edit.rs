use anyhow::{Context as _, Result};
use clap::Args;

use crate::Context;

#[derive(Debug, Args)]
pub struct EditArgs {
    /// Recipe name
    name: String,
}

pub fn run(args: EditArgs, ctx: &Context) -> Result<()> {
    let entry = ctx.recipe_index.resolve(&args.name, None)?;
    let path = entry.path();

    let editor = ctx
        .chef_config
        .editor()
        .context("Could not determine editor")?;
    let (cmd, args) = editor.split_first().expect("empty editor cmd");

    let ok = std::process::Command::new(cmd)
        .args(args)
        .arg(path)
        .status()?
        .success();

    if !ok {
        tracing::warn!("Editor didn't exit successfully")
    }

    Ok(())
}

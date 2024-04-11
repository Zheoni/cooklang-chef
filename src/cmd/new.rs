use std::fs;

use anyhow::{bail, Context as _, Result};
use camino::Utf8Path;
use clap::Args;

use crate::Context;

#[derive(Debug, Args)]
pub struct NewArgs {
    /// Recipe name
    ///
    /// Split directories with "/"
    name: String,

    /// Skip opening the editor
    #[arg(long, short = 'E')]
    no_edit: bool,
}

pub fn run(args: NewArgs, ctx: &Context) -> Result<()> {
    let file = Utf8Path::new(&args.name).with_extension("cook");
    let valid = !file.is_absolute()
        && file
            .components()
            .all(|c| matches!(c, camino::Utf8Component::Normal(_)));
    if !valid {
        bail!("Invalid name: {}", args.name);
    }

    let path = ctx.base_path.join(file);

    if path.is_file() {
        bail!("File already exists: {}", path);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::File::create(&path)?;

    if !args.no_edit {
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
    }

    Ok(())
}

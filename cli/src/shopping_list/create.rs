use anyhow::Result;
use clap::Args;

use crate::Context;

#[derive(Debug, Args)]
pub struct CreateArgs {}

pub fn run(_ctx: &Context, _args: CreateArgs) -> Result<()> {
    todo!()
}

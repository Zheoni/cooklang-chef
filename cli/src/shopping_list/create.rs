use anyhow::Result;
use clap::Args;
use cooklang::CooklangParser;

#[derive(Debug, Args)]
pub struct CreateArgs {}

pub fn run(_parser: &CooklangParser, _args: CreateArgs) -> Result<()> {
    todo!()
}

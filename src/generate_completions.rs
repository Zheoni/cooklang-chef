use std::io;

use clap::{Args, CommandFactory};

#[derive(Debug, Args)]
pub struct GenerateCompletionsArgs {
    shell: clap_complete::Shell,
}

pub fn run(args: GenerateCompletionsArgs) -> anyhow::Result<()> {
    clap_complete::generate(
        args.shell,
        &mut crate::CliArgs::command(),
        env!("CARGO_BIN_NAME"),
        &mut io::stdout(),
    );
    Ok(())
}

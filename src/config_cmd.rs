use anstream::{print, println};
use anyhow::Result;

use crate::{
    config::{config_file_path, global_file_path, DEFAULT_CONFIG_FILE, GLOBAL_CONFIG_FILE},
    Context,
};

pub fn run(ctx: &Context) -> Result<()> {
    use owo_colors::OwoColorize;

    println!("Recipes path: {}", ctx.base_path.yellow());

    let mut config_path = config_file_path(&ctx.base_path);
    if !config_path.is_file() {
        print!("{}", "No config at: ".dimmed());
        println!("{}", config_path.dimmed().bright_red());
        config_path = global_file_path(DEFAULT_CONFIG_FILE)?;
    }
    println!("Config: {}", config_path.yellow());
    println!(
        "Global config: {}",
        global_file_path(GLOBAL_CONFIG_FILE)?.yellow()
    );

    let fence = "+++".dimmed();
    println!("{fence}");
    let c = toml::to_string_pretty(&ctx.config)?;
    println!("{}", c.trim());
    println!("{fence}{}", "global".dimmed());
    let c = toml::to_string_pretty(&ctx.global_config)?;
    println!("{}", c.trim());
    println!("{fence}");

    for file in ctx
        .config
        .units(&ctx.base_path)
        .iter()
        .chain(ctx.config.aisle(&ctx.base_path).iter())
    {
        print!("{file} {} ", "--".dimmed());
        if file.is_file() {
            println!("{}", "found".green().bold());
        } else {
            println!("{}", "not found".red().bold());
        }
    }

    Ok(())
}

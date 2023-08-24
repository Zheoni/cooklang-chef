use anstream::{print, println};
use anyhow::Result;

use crate::{config::config_file_path, Context};

pub fn run(ctx: &Context) -> Result<()> {
    use owo_colors::OwoColorize;

    println!("Recipes path: {}", ctx.base_path.yellow());

    let config_path = config_file_path(&ctx.base_path);
    if config_path.is_file() {
        println!("Config loaded from: {}", config_path.green());
    } else {
        println!("Using default config");
        print!("{}", "No config at: ".dimmed().italic());
        println!("{}", config_path.italic().red());
    }

    let fence = "+++".dimmed();
    println!("{fence}");
    let c = toml::to_string_pretty(&ctx.config)?;
    println!("{}", c.trim());
    println!("{fence}{}", "global".dimmed().italic());
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

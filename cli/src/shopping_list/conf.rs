use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};
use cooklang::shopping_list::ShoppingListConf;
use yansi::Paint;

use crate::write_to_output;

#[derive(Debug, Args)]
pub struct ConfArgs {
    /// Output file, none for stdout.
    #[arg(short, long)]
    output: Option<Utf8PathBuf>,

    /// Output format
    #[arg(short, long, value_enum)]
    format: Option<OutputFormat>,

    /// Pretty output format, if available
    #[arg(long)]
    pretty: bool,

    /// Sort the aile file alphabetically
    #[arg(long)]
    sorted: bool,
}

#[derive(Debug, ValueEnum, Clone, Copy)]
enum OutputFormat {
    Human,
    Conf,
    Json,
}

pub fn run(mut aile: ShoppingListConf, args: ConfArgs) -> Result<()> {
    if args.sorted {
        aile.categories.sort_unstable_by_key(|c| c.name);
        for c in &mut aile.categories {
            for i in &mut c.ingredients {
                i.names.sort_unstable();
            }
            c.ingredients
                .sort_unstable_by_key(|i| i.names.first().copied().unwrap_or(""))
        }
    }

    let format = args.format.unwrap_or_else(|| match &args.output {
        Some(p) => match p.extension() {
            Some("json") => OutputFormat::Json,
            _ => OutputFormat::Conf,
        },
        None => OutputFormat::Human,
    });

    write_to_output(args.output.as_deref(), |writer| {
        match format {
            OutputFormat::Human => print_aile_human(&aile, writer)?,
            OutputFormat::Conf => cooklang::shopping_list::write(&aile, writer)?,
            OutputFormat::Json => {
                if args.pretty {
                    serde_json::to_writer_pretty(writer, &aile)?;
                } else {
                    serde_json::to_writer(writer, &aile)?;
                }
            }
        };
        Ok(())
    })?;

    Ok(())
}

fn print_aile_human(
    aile: &ShoppingListConf,
    mut writer: impl std::io::Write,
) -> std::io::Result<()> {
    let w = &mut writer;
    for category in aile.categories.iter() {
        writeln!(
            w,
            "{}{}{}",
            Paint::magenta('['),
            Paint::green(&category.name).bold(),
            Paint::magenta(']')
        )?;
        for igr in &category.ingredients {
            if !igr.names.is_empty() {
                let mut iter = igr.names.iter();
                write!(w, "  {}", iter.next().unwrap())?;
                for name in iter {
                    write!(w, "{} {name}", Paint::magenta(','))?;
                }
            }
            writeln!(w, "  {}", igr.names.join(", "))?;
        }
        writeln!(w)?;
    }

    Ok(())
}

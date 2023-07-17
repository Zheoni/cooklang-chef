use anstream::println;
use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};
use cooklang::aisle::AisleConf;

use crate::util::write_to_output;

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

    /// Sort the aisle file alphabetically
    #[arg(long)]
    sorted: bool,

    /// Only get a count of entries
    #[arg(short = 'n', long, conflicts_with_all = ["output", "format"])]
    count: bool,
}

#[derive(Debug, ValueEnum, Clone, Copy)]
enum OutputFormat {
    Human,
    Conf,
    Json,
}

pub fn run(mut aisle: AisleConf, args: ConfArgs) -> Result<()> {
    if args.count {
        let mut table = tabular::Table::new("{:<}  {:<}")
            .with_heading(format!("total {}", aisle.categories.len()));
        for cat in &aisle.categories {
            table.add_row(tabular::row!(cat.name, cat.ingredients.len()));
        }
        println!("{table}");
        return Ok(());
    }

    if args.sorted {
        aisle.categories.sort_unstable_by_key(|c| c.name);
        for c in &mut aisle.categories {
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
            OutputFormat::Human => print_aile_human(&aisle, writer)?,
            OutputFormat::Conf => cooklang::aisle::write(&aisle, writer)?,
            OutputFormat::Json => {
                if args.pretty {
                    serde_json::to_writer_pretty(writer, &aisle)?;
                } else {
                    serde_json::to_writer(writer, &aisle)?;
                }
            }
        };
        Ok(())
    })?;

    Ok(())
}

fn print_aile_human(aisle: &AisleConf, mut writer: impl std::io::Write) -> std::io::Result<()> {
    use owo_colors::OwoColorize;

    let w = &mut writer;
    for category in aisle.categories.iter() {
        writeln!(
            w,
            "{}{}{}",
            '['.magenta(),
            category.name.green().bold(),
            ']'.magenta()
        )?;
        for igr in &category.ingredients {
            if !igr.names.is_empty() {
                let mut iter = igr.names.iter();
                write!(w, "  {}", iter.next().unwrap())?;
                for name in iter {
                    write!(w, "{} {name}", ','.magenta())?;
                }
                writeln!(w)?;
            }
        }
        writeln!(w)?;
    }

    Ok(())
}

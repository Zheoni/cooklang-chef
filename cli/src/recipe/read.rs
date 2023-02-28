use std::path::PathBuf;

use clap::{Args, ValueEnum};
use cooklang::{convert::Converter, quantity::Quantity, CooklangParser};
use miette::{Context, IntoDiagnostic, Result};
use owo_colors::{style, Stream};

use super::RecipeInput;

#[derive(Debug, Args)]
pub struct ReadArgs {
    #[command(flatten)]
    input: RecipeInput,

    /// Output file, none for stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value_t)]
    format: Output,

    /// Pretty output format if available
    #[arg(long)]
    pretty: bool,

    /// Do not display warnings
    #[arg(long)]
    ignore_warnings: bool,
}

#[derive(Debug, Default, Clone, Copy, ValueEnum)]
enum Output {
    #[default]
    Human,
    Json,
}

pub fn run(parser: &CooklangParser, args: ReadArgs) -> Result<()> {
    let (input, name) = args.input.read()?;

    let (recipe, warnings) = parser.parse(&input, &name)?;
    if !args.ignore_warnings {
        cooklang::error::print_warnings(
            &input,
            &warnings,
            crate::STD_ERR_COLOR_ENABLED.load(std::sync::atomic::Ordering::Relaxed),
        );
    }
    args.to_output(&recipe, parser.converter())?;

    Ok(())
}

impl ReadArgs {
    fn to_output(&self, value: &cooklang::Recipe, converter: &Converter) -> Result<()> {
        if let Some(path) = &self.output {
            let file = std::fs::File::create(path)
                .into_diagnostic()
                .wrap_err("Failed to create output file")?;
            self.write(value, converter, file)?;
        } else {
            self.write(value, converter, std::io::stdout())?;
        };
        Ok(())
    }

    fn write(
        &self,
        value: &cooklang::Recipe,
        converter: &Converter,
        writer: impl std::io::Write,
    ) -> Result<()> {
        match self.format {
            Output::Human => {
                let mut total_quantities = Vec::with_capacity(value.ingredients.len());
                for igr in &value.ingredients {
                    total_quantities.push(igr.total_quantity(converter).ok().flatten())
                }
                print_human(value, total_quantities, writer).into_diagnostic()?;
            }
            Output::Json => {
                if self.pretty {
                    serde_json::to_writer_pretty(writer, value).into_diagnostic()?;
                } else {
                    serde_json::to_writer(writer, value).into_diagnostic()?;
                }
            }
        };
        Ok(())
    }
}

fn print_human(
    value: &cooklang::Recipe,
    total_quantities: Vec<Option<Quantity>>,
    mut writer: impl std::io::Write,
) -> Result<(), std::io::Error> {
    use cooklang::model::{Component, Item};
    use owo_colors::OwoColorize;
    use tabular::{Row, Table};

    let termwidth = textwrap::termwidth().min(80);

    let w = &mut writer;

    let quantity_fmt = |qty: &Quantity| {
        if let Some(unit) = qty.unit() {
            format!(
                "{} {}",
                qty.value,
                unit.text()
                    .if_supports_color(Stream::Stdout, |t| t.italic())
            )
        } else {
            format!("{}", qty.value)
        }
    };

    writeln!(
        w,
        "\n  {}\n",
        format!(" {} ", value.name).if_supports_color(Stream::Stdout, |t| t
            .style(style().bright_white().on_purple()))
    )?;

    writeln!(w, "Ingredients:")?;
    assert_eq!(total_quantities.len(), value.ingredients.len());
    let mut table = Table::new("  {:<}    {:<} {:<}");
    for (igr, total_quantity) in value
        .ingredients
        .iter()
        .zip(total_quantities)
        .filter(|(igr, _)| !igr.is_hidden())
    {
        let mut row = Row::new().with_cell(&igr.name);
        if let Some(quantity) = total_quantity {
            row.add_ansi_cell(quantity_fmt(&quantity));
        } else {
            let list = igr
                .all_quantities()
                .into_iter()
                .map(|q| quantity_fmt(&q))
                .reduce(|s, q| s + ", " + &q);
            if let Some(list) = list {
                row.add_ansi_cell(list);
            } else {
                row.add_cell("");
            }
        }
        if let Some(note) = &igr.note {
            row.add_cell(note);
        } else {
            row.add_cell("");
        }
        table.add_row(row);
    }
    writeln!(w, "{table}")?;

    writeln!(w, "Steps:")?;
    for section in &value.sections {
        if let Some(name) = &section.name {
            writeln!(
                w,
                "{}",
                name.if_supports_color(Stream::Stdout, |t| t.underline())
            )?;
        }
        let mut step_counter = 0;
        for step in &section.steps {
            if step.is_text {
                write!(w, "")?;
                for item in &step.items {
                    if let Item::Text(text) = item {
                        write!(w, "{text}")?;
                    } else {
                        panic!("Not text in text step");
                    }
                }
            } else {
                step_counter += 1;
                write!(w, "{step_counter:>4}. ")?;

                let mut step_text = String::new();
                let mut step_igrs = Vec::new();
                for item in &step.items {
                    match item {
                        Item::Text(text) => step_text += text,
                        Item::Component(c) => match c {
                            Component::Ingredient(igr) => {
                                step_text += &igr
                                    .name
                                    .if_supports_color(Stream::Stdout, |t| t.green())
                                    .to_string();
                                step_igrs.push(igr);
                            }
                            Component::Cookware(cookware) => {
                                step_text += &cookware
                                    .name
                                    .if_supports_color(Stream::Stdout, |t| t.yellow())
                                    .to_string();
                            }
                            Component::Timer(timer) => {
                                step_text += &quantity_fmt(&timer.quantity)
                                    .if_supports_color(Stream::Stdout, |t| t.cyan())
                                    .to_string();
                            }
                        },
                        Item::Temperature(temp) => write!(w, "{}", temp)?,
                    }
                }
                let step_text = textwrap::fill(
                    &step_text,
                    textwrap::Options::new(termwidth).subsequent_indent("      "),
                );
                writeln!(w, "{step_text}")?;
                if step_igrs.is_empty() {
                    write!(w, "      [-]")?;
                } else {
                    let mut igrs_text = String::from("      [");
                    for (i, igr) in step_igrs.iter().enumerate() {
                        if let Some(q) = &igr.quantity {
                            igrs_text += &format!(
                                "{}: {}",
                                igr.name,
                                quantity_fmt(q).if_supports_color(Stream::Stdout, |t| t.dimmed())
                            );
                        } else {
                            igrs_text += &igr.name;
                        }
                        if i != step_igrs.len() - 1 {
                            igrs_text += ", ";
                        }
                    }
                    igrs_text += "]";
                    let igrs_text = textwrap::fill(
                        &igrs_text,
                        textwrap::Options::new(termwidth).subsequent_indent("      "),
                    );
                    write!(w, "{igrs_text}")?;
                }
            }
            writeln!(w)?;
        }
    }

    Ok(())
}

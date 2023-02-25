use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use clap::{Parser, ValueEnum};
use cooklang::{
    convert::{builder::ConverterBuilder, units_file::UnitsFile},
    model::{Component, Item, Recipe},
    quantity::Quantity,
    CooklangParser, Extensions,
};
use miette::{Context, IntoDiagnostic, Result};
use owo_colors::{style, Style};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file path, none for stdin
    input: Option<PathBuf>,

    /// Output file, none for stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Give or override a name for the recipe
    ///
    /// If not given will be obtained from input path.
    #[arg(short, long)]
    name: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value_t)]
    format: Output,

    /// A units TOML file
    #[arg(long, action = clap::ArgAction::Append)]
    units: Vec<PathBuf>,

    /// Do not use the bundled units
    #[arg(long)]
    no_default_units: bool,

    /// Disable all extensions to the cooklang
    /// spec <https://cooklang.org/docs/spec/>
    #[arg(long)]
    no_extensions: bool,

    /// Pretty output format
    #[arg(long)]
    pretty: bool,

    /// Treat warnings as errors
    #[arg(long)]
    warnings_as_errors: bool,
}

#[derive(Debug, Default, Clone, Copy, ValueEnum)]
enum Output {
    #[default]
    Human,
    Json,
}

pub fn main() -> Result<()> {
    let cli = Args::parse();

    let mut parser = CooklangParser::new();
    parser
        .warnings_as_errors(cli.warnings_as_errors)
        .with_extensions(if cli.no_extensions {
            Extensions::empty()
        } else {
            Extensions::all()
        });
    if !cli.no_default_units || !cli.units.is_empty() {
        let mut builder = ConverterBuilder::new();
        if !cli.no_default_units {
            builder
                .add_units_file(UnitsFile::bundled())
                .expect("Failed to add bundled units");
        }
        for file in &cli.units {
            let text = std::fs::read_to_string(file).into_diagnostic()?;
            let units = toml::from_str(&text).into_diagnostic()?;
            builder.add_units_file(units)?;
        }
        parser.with_converter(builder.finish()?);
    }

    let (input_txt, filename) = read_input_txt(cli.input.as_ref())?;

    let name = cli
        .name
        .as_ref()
        .or(filename.as_ref())
        .map(|name| name.strip_suffix(".cook").unwrap_or(&name))
        .ok_or_else(|| miette::miette!("No name for the recipe"))?;

    let (recipe, warnings) = parser.parse(&input_txt, name)?;
    cooklang::error::print_warnings(&input_txt, &warnings);
    cli.to_output(&recipe)?;

    Ok(())
}

fn read_input_txt(input: Option<&PathBuf>) -> Result<(String, Option<String>)> {
    let mut input_txt = String::new();
    let mut filename = None;
    if let Some(path) = input {
        filename = path.file_name().map(|s| s.to_string_lossy().to_string());
        input_txt = std::fs::read_to_string(path)
            .into_diagnostic()
            .wrap_err("Failed to read input file")?;
    } else {
        std::io::stdin()
            .read_to_string(&mut input_txt)
            .into_diagnostic()
            .wrap_err("Failed to read stdin")?;
    };
    Ok((input_txt, filename))
}

impl Args {
    fn to_output(&self, value: &Recipe) -> Result<()> {
        if let Some(path) = &self.output {
            let file = File::create(path)
                .into_diagnostic()
                .wrap_err("Failed to create output file")?;
            self.write(value, file)?;
        } else {
            self.write(value, std::io::stdout())?;
        };
        Ok(())
    }

    fn write(&self, value: &Recipe, writer: impl Write) -> Result<()> {
        match self.format {
            Output::Human => {
                let styles = if self.output.is_none()
                    && supports_color::on_cached(supports_color::Stream::Stdout).is_some()
                {
                    Styles::colorized()
                } else {
                    Styles::disabled()
                };
                print_human(value, styles, writer).into_diagnostic()?;
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

#[derive(Default)]
struct Styles {
    title: Style,
    section: Style,
    quantity_unit: Style,
    ingredient: Style,
    cookware: Style,
    timer: Style,
}

impl Styles {
    fn colorized() -> Self {
        Self {
            title: style().bold(),
            section: style().underline(),
            quantity_unit: style().italic(),
            ingredient: style().green(),
            cookware: style().yellow(),
            timer: style().cyan(),
        }
    }

    fn disabled() -> Self {
        Self::default()
    }
}

fn print_human(
    value: &Recipe,
    styles: Styles,
    mut writer: impl Write,
) -> Result<(), std::io::Error> {
    use owo_colors::OwoColorize;
    use tabular::{Row, Table};

    let w = &mut writer;

    let quantity_fmt = |qty: &Quantity| {
        if let Some(unit) = qty.unit() {
            format!("{} {}", qty.value, unit.text().style(styles.quantity_unit))
        } else {
            format!("{}", qty.value)
        }
    };

    writeln!(w, "{}\n", value.name.style(styles.title))?;

    writeln!(w, "Ingredients:")?;
    let mut table = Table::new("  {:<}    {:<} {:<}");
    for igr in value
        .ingredients
        .iter()
        .filter(|igr| !igr.is_hidden() && !igr.is_reference())
    {
        let mut row = Row::new().with_cell(&igr.name);
        if let Some(quantity) = &igr.quantity {
            row.add_ansi_cell(quantity_fmt(quantity));
        } else {
            row.add_cell("");
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
            writeln!(w, "{}", name.style(styles.section))?;
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

                let mut step_igrs = Vec::new();
                for item in &step.items {
                    match item {
                        Item::Text(text) => write!(w, "{text}")?,
                        Item::Component(c) => match c {
                            Component::Ingredient(igr) => {
                                write!(w, "{}", igr.name.style(styles.ingredient))?;
                                step_igrs.push(igr);
                            }
                            Component::Cookware(cookware) => {
                                write!(w, "{}", cookware.name.style(styles.cookware))?
                            }
                            Component::Timer(timer) => {
                                write!(w, "{}", quantity_fmt(&timer.quantity).style(styles.timer))?
                            }
                        },
                        Item::Temperature(temp) => write!(w, "{}", temp)?,
                    }
                }
                writeln!(w)?;
                if step_igrs.is_empty() {
                    write!(w, "      [-]")?;
                } else {
                    write!(w, "      [")?;
                    for (i, igr) in step_igrs.iter().enumerate() {
                        if let Some(q) = &igr.quantity {
                            write!(w, "{}: {}", igr.name, quantity_fmt(q))?;
                        } else {
                            write!(w, "{}", igr.name)?;
                        }
                        if i != step_igrs.len() - 1 {
                            write!(w, ", ")?;
                        }
                    }
                    write!(w, "]")?;
                }
            }
            writeln!(w)?;
        }
    }

    Ok(())
}

use std::cmp::Ordering;

use anstream::println;
use anyhow::Result;
use clap::{Args, ValueEnum};
use cooklang::convert::{Converter, Unit};
use owo_colors::OwoColorize;

#[derive(Debug, Args)]
pub struct UnitsArgs {
    /// More data
    #[arg(short, long)]
    long: bool,

    /// Show all names/symbols, not just the first
    #[arg(short, long)]
    all: bool,

    /// Show unit count only
    #[arg(short = 'n', long)]
    count: bool,

    /// Filter by unit system
    #[arg(long, value_enum)]
    system: Option<System>,

    /// Filter by physical quantity
    #[arg(long)]
    quantity: Option<PhysicalQuantity>,

    /// Sort results. Can be specified multiple times
    #[arg(short, long, value_enum)]
    sort: Vec<Sort>,

    /// Writes all units in json format, one per line along with conversion data
    #[arg(long, exclusive = true)]
    dump: bool,
}

#[derive(Debug, ValueEnum, Clone, Copy)]
pub enum System {
    Metric,
    Imperial,
    None,
}

#[derive(Debug, ValueEnum, Clone, Copy)]
pub enum PhysicalQuantity {
    Volume,
    Mass,
    Length,
    Temperature,
    Time,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Sort {
    System,
    PhysicalQuantity,
    Ratio,
    Best,
}

impl From<System> for Option<cooklang::convert::System> {
    fn from(val: System) -> Self {
        match val {
            System::Metric => Some(cooklang::convert::System::Metric),
            System::Imperial => Some(cooklang::convert::System::Imperial),
            System::None => None,
        }
    }
}

impl From<PhysicalQuantity> for cooklang::convert::PhysicalQuantity {
    fn from(val: PhysicalQuantity) -> Self {
        match val {
            PhysicalQuantity::Volume => cooklang::convert::PhysicalQuantity::Volume,
            PhysicalQuantity::Mass => cooklang::convert::PhysicalQuantity::Mass,
            PhysicalQuantity::Length => cooklang::convert::PhysicalQuantity::Length,
            PhysicalQuantity::Temperature => cooklang::convert::PhysicalQuantity::Temperature,
            PhysicalQuantity::Time => cooklang::convert::PhysicalQuantity::Time,
        }
    }
}

pub fn run(converter: &Converter, args: UnitsArgs) -> Result<()> {
    if args.dump {
        dump_units(converter);
    } else if args.count {
        if args.long {
            let unit_count = UnitCount::new(converter);
            let table = unit_count_table(&unit_count);
            println!("{table}");
        } else {
            println!("{}", converter.unit_count());
        }
    } else {
        let mut all_units = converter
            .all_units()
            .filter(filter_units(&args))
            .collect::<Vec<_>>();

        if !args.sort.is_empty() {
            all_units.sort_unstable_by(|a, b| {
                for sort in &args.sort {
                    let ord = match sort {
                        Sort::System => a.system.cmp(&b.system),
                        Sort::PhysicalQuantity => a.physical_quantity.cmp(&b.physical_quantity),
                        Sort::Ratio => a.ratio.total_cmp(&b.ratio),
                        Sort::Best => converter.is_best_unit(b).cmp(&converter.is_best_unit(a)),
                    };

                    if ord != Ordering::Equal {
                        return ord;
                    }
                }
                Ordering::Equal
            });
        }

        if args.long {
            let mut table = tabular::Table::new("{:<} {:<} {:<} {:<} {:<} {:<} {:<} {:<}");
            let mut total = 0;

            for unit in all_units {
                total += 1;
                table.add_row(
                    tabular::Row::new()
                        .with_ansi_cell(list(&unit.names, args.all))
                        .with_ansi_cell(list(&unit.symbols, args.all))
                        .with_ansi_cell(list(&unit.aliases, true))
                        .with_ansi_cell(style_quantity(unit.physical_quantity))
                        .with_ansi_cell(
                            unit.system
                                .map(style_system)
                                .unwrap_or_else(|| "-".dimmed().to_string()),
                        )
                        .with_ansi_cell(display_best_unit(converter, unit))
                        .with_cell(unit.ratio)
                        .with_cell(unit.difference),
                );
            }
            println!("total {total}\n{table}");
        } else {
            for unit in converter.all_units().filter(filter_units(&args)) {
                println!("{}", unit.names.first().unwrap());
            }
        }
    }

    Ok(())
}

fn list(l: &[std::sync::Arc<str>], all: bool) -> String {
    if l.is_empty() {
        return "-".dimmed().to_string();
    }
    let mut l = l.iter().map(|l| {
        if l.contains(char::is_whitespace) {
            format!("\"{l}\"")
        } else {
            l.to_string()
        }
    });
    if all {
        l.reduce(|acc, s| format!("{acc},{s}")).unwrap()
    } else {
        l.next().unwrap()
    }
}

fn style_quantity(q: cooklang::convert::PhysicalQuantity) -> String {
    use owo_colors::AnsiColors;

    let color = match q {
        cooklang::convert::PhysicalQuantity::Volume => AnsiColors::Green,
        cooklang::convert::PhysicalQuantity::Mass => AnsiColors::Magenta,
        cooklang::convert::PhysicalQuantity::Length => AnsiColors::Blue,
        cooklang::convert::PhysicalQuantity::Temperature => AnsiColors::Yellow,
        cooklang::convert::PhysicalQuantity::Time => AnsiColors::Cyan,
    };
    q.color(color).to_string()
}

fn style_system(system: cooklang::convert::System) -> String {
    use owo_colors::AnsiColors;

    let color = match system {
        cooklang::convert::System::Metric => AnsiColors::Green,
        cooklang::convert::System::Imperial => AnsiColors::Red,
    };
    system.color(color).to_string()
}

fn display_best_unit(converter: &Converter, unit: &Unit) -> String {
    if converter.is_best_unit(unit) {
        "b".yellow().to_string()
    } else {
        "-".dimmed().to_string()
    }
}

fn filter_units(args: &UnitsArgs) -> impl Fn(&&cooklang::convert::Unit) -> bool + '_ {
    |u| {
        if let Some(wanted_system) = &args.system {
            if u.system != (*wanted_system).into() {
                return false;
            }
        }
        if let Some(wanted_quantity) = &args.quantity {
            if u.physical_quantity != (*wanted_quantity).into() {
                return false;
            }
        }
        true
    }
}

struct UnitCount {
    all: usize,
    by_system: enum_map::EnumMap<cooklang::convert::System, usize>,
    by_quantity: enum_map::EnumMap<cooklang::convert::PhysicalQuantity, usize>,
}

impl UnitCount {
    fn new(converter: &Converter) -> Self {
        let mut all = 0;
        let mut by_system = enum_map::EnumMap::default();
        let mut by_quantity = enum_map::EnumMap::default();
        for unit in converter.all_units() {
            all += 1;
            if let Some(s) = unit.system {
                by_system[s] += 1;
            }
            by_quantity[unit.physical_quantity] += 1;
        }
        Self {
            all,
            by_quantity,
            by_system,
        }
    }
}

fn unit_count_table(unit_count: &UnitCount) -> tabular::Table {
    let mut table = tabular::Table::new("{:>}  {:<}");
    table.add_row(tabular::row!("total", unit_count.all));
    table.add_heading("by system");
    for (s, c) in unit_count.by_system {
        table.add_row(tabular::row!(s, c));
    }
    table.add_row(tabular::row!(
        "none",
        unit_count.all - unit_count.by_system.values().sum::<usize>()
    ));
    table.add_heading("by physical quantity");
    for (q, c) in unit_count.by_quantity {
        table.add_row(tabular::row!(q, c));
    }
    table
}

fn dump_units(converter: &Converter) {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    for unit in converter.all_units() {
        serde_json::to_writer(&mut stdout, &unit).unwrap();
        writeln!(&mut stdout).unwrap();
    }
}

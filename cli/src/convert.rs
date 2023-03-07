use clap::Args;
use console::style;
use cooklang::convert::{ConvertTo, ConvertValue, Converter, System};

#[derive(Debug, Args)]
pub struct ConvertArgs {
    /// Value to convert, can have decimals
    value: f64,
    /// Unit to convert from
    unit: String,
    /// Unit to convert to. Can also be "metric", "imperial", or "fit"
    ///
    /// "metric" and "imperial" will convert to the best possible unit
    /// in one of those systems.
    ///
    /// "fit" will try to convert to the best unit in the same system.
    to: String,

    /// Do not round results
    #[arg(long, short = 'R')]
    no_round: bool,
}

pub fn run(converter: &Converter, args: ConvertArgs) -> miette::Result<()> {
    let to = match args.to.as_str() {
        "fit" => ConvertTo::SameSystem,
        "metric" => ConvertTo::Best(System::Metric),
        "imperial" => ConvertTo::Best(System::Imperial),
        _ => ConvertTo::Unit(cooklang::convert::ConvertUnit::Key(&args.to)),
    };

    let (value, unit) = converter.convert((args.value, args.unit.as_str()), to)?;

    let ConvertValue::Number(mut n) = value else { panic!("unexpected range value") };
    if !args.no_round {
        n = (n * 1000.0).round() / 1000.0;
    }

    println!("{} {}", n, style(unit).italic());

    Ok(())
}

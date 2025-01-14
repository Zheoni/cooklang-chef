use anstream::println;
use clap::Args;
use cooklang::{
    convert::{ConvertTo, Converter, System},
    quantity::Number,
    Quantity, Value,
};

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
}

pub fn run(converter: &Converter, args: ConvertArgs) -> anyhow::Result<()> {
    use yansi::Paint;

    let to = match args.to.as_str() {
        "fit" | "best" => ConvertTo::SameSystem,
        "metric" => ConvertTo::Best(System::Metric),
        "imperial" => ConvertTo::Best(System::Imperial),
        _ => ConvertTo::Unit(cooklang::convert::ConvertUnit::Key(&args.to)),
    };

    let mut quantity = Quantity::new(Value::Number(Number::Regular(args.value)), Some(args.unit));

    quantity.convert(to, converter)?;

    println!(
        "{:#} {}",
        quantity.value(),
        quantity.unit().unwrap().italic()
    );

    Ok(())
}

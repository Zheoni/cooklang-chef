use std::{collections::HashMap, ops::RangeInclusive, sync::Arc};

use enum_map::EnumMap;
use miette::Diagnostic;
use once_cell::sync::OnceCell;

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::quantity::{Quantity, Value};

use self::units_file::SIPrefix;

pub mod builder;
pub mod units_file;

#[derive(Debug, Clone)]
pub struct Converter {
    all_units: Vec<Arc<Unit>>,
    unit_index: UnitIndex,
    quantity_index: EnumMap<PhysicalQuantity, Vec<usize>>,
    best: EnumMap<PhysicalQuantity, BestConversionsStore>,

    temperature_regex: OnceCell<Regex>,
}

#[derive(Debug, Default, Clone)]
struct UnitIndex(HashMap<Arc<str>, usize>);

#[derive(Debug, Clone, Serialize)]
pub struct Unit {
    pub names: Vec<Arc<str>>,
    pub symbols: Vec<Arc<str>>,
    pub aliases: Vec<Arc<str>>,
    pub ratio: f64,
    pub difference: f64,
    pub physical_quantity: PhysicalQuantity,
    #[serde(skip)]
    expand_si: bool,
    #[serde(skip)]
    expanded_units: Option<EnumMap<SIPrefix, usize>>,
}

impl Unit {
    fn all_keys(&self) -> impl Iterator<Item = &Arc<str>> {
        self.names.iter().chain(&self.symbols).chain(&self.aliases)
    }
}

#[derive(Debug, Clone)]
enum BestConversionsStore {
    Unified(BestConversions),
    BySystem {
        metric: BestConversions,
        imperial: BestConversions,
    },
}

impl Default for BestConversionsStore {
    fn default() -> Self {
        Self::Unified(Default::default())
    }
}

#[derive(Debug, Clone, Default)]
struct BestConversions(Vec<(f64, usize)>);

#[derive(
    Clone, Copy, PartialEq, Eq, Debug, strum::Display, Serialize, Deserialize, enum_map::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalQuantity {
    Volume,
    Mass,
    Length,
    Temperature,
    Time,
}

impl BestConversionsStore {
    pub fn is_empty(&self) -> bool {
        match self {
            BestConversionsStore::Unified(v) => v.0.is_empty(),
            BestConversionsStore::BySystem { metric, imperial } => {
                metric.0.is_empty() || imperial.0.is_empty()
            }
        }
    }
}

impl Unit {
    pub fn symbol(&self) -> &str {
        self.symbols
            .first()
            .or_else(|| self.names.first())
            .expect("symbol or name in unit")
    }
}

impl Converter {
    pub fn convert<'t, F: ConvertFrom>(
        &self,
        from: F,
        to: impl Into<ConvertTo<'t>>,
    ) -> Result<F::Output<'_>, ConvertError> {
        let value = from.convert_value()?;
        let unit = from.convert_unit()?;
        let (value, unit_id) = self.convert_impl(value, unit, to.into())?;
        let unit = &self.all_units[unit_id];
        Ok(F::output(value, unit))
    }

    fn convert_impl(
        &self,
        value: ConvertValue,
        unit: &str,
        to: ConvertTo,
    ) -> Result<(ConvertValue, usize), ConvertError> {
        let unit_id = self.get_unit_id(unit)?;

        match to {
            ConvertTo::Unit(target_unit) => self.convert_to_unit(value, unit_id, target_unit),
            ConvertTo::Best(system) => self.convert_to_system(value, unit_id, system),
        }
    }

    fn convert_to_unit(
        &self,
        value: ConvertValue,
        unit_id: usize,
        target_unit: &str,
    ) -> Result<(ConvertValue, usize), ConvertError> {
        let unit = &self.all_units[unit_id];
        let target_unit_id = self.get_unit_id(target_unit)?;
        let target_unit = &self.all_units[target_unit_id];
        if unit.physical_quantity != target_unit.physical_quantity {
            return Err(ConvertError::MixedQuantities {
                from: unit.physical_quantity,
                to: target_unit.physical_quantity,
            });
        }
        Ok((
            self.convert_value(value, unit_id, target_unit_id),
            target_unit_id,
        ))
    }

    fn convert_to_system(
        &self,
        value: ConvertValue,
        unit_id: usize,
        system: System,
    ) -> Result<(ConvertValue, usize), ConvertError> {
        let unit = &self.all_units[unit_id];
        let conversions = match &self.best[unit.physical_quantity] {
            BestConversionsStore::Unified(u) => u,
            BestConversionsStore::BySystem { metric, imperial } => match system {
                System::Metric => metric,
                System::Imperial => imperial,
            },
        };

        let best_unit = conversions.best_unit(self, &value, unit_id);
        let converted = self.convert_value(value, unit_id, best_unit);

        Ok((converted, best_unit))
    }

    fn convert_value(&self, value: ConvertValue, from: usize, to: usize) -> ConvertValue {
        match value {
            ConvertValue::Number(n) => ConvertValue::Number(self.convert_f64(n, from, to)),
            ConvertValue::Range(r) => {
                let s = self.convert_f64(*r.start(), from, to);
                let e = self.convert_f64(*r.end(), from, to);
                ConvertValue::Range(s..=e)
            }
        }
    }

    fn convert_f64(&self, value: f64, from_id: usize, to_id: usize) -> f64 {
        if from_id == to_id {
            return value;
        }

        let from = &self.all_units[from_id];
        let to = &self.all_units[to_id];

        convert_f64(value, from, to)
    }

    fn get_unit_id(&self, unit: &str) -> Result<usize, UnknownUnit> {
        self.unit_index.get_unit_id(unit)
    }
}

pub(crate) fn convert_f64(value: f64, from: &Unit, to: &Unit) -> f64 {
    assert_eq!(from.physical_quantity, to.physical_quantity);

    let norm = (value + from.difference) * from.ratio;
    (norm / to.ratio) - to.difference
}

#[derive(Debug, Error)]
#[error("Unknown unit: {0}")]
pub struct UnknownUnit(String);

impl UnitIndex {
    fn get_unit_id(&self, key: &str) -> Result<usize, UnknownUnit> {
        self.0
            .get(key)
            .copied()
            .ok_or_else(|| UnknownUnit(key.to_string()))
    }
}

impl BestConversions {
    fn base(&self) -> usize {
        self.0.first().map(|c| c.1).expect("empty best conversions")
    }

    fn best_unit(&self, converter: &Converter, value: &ConvertValue, unit_id: usize) -> usize {
        let value = match value {
            ConvertValue::Number(n) => n.abs(),
            ConvertValue::Range(r) => r.start().abs(),
        };
        let base_unit = self.base();
        let norm = converter.convert_f64(value, unit_id, base_unit);

        self.0
            .iter()
            .filter_map(|c| (norm >= c.0).then_some(c))
            .last()
            .or_else(|| self.0.last())
            .map(|c| c.1)
            .expect("empty best units")
    }
}

pub trait ConvertFrom {
    fn convert_value(&self) -> Result<ConvertValue, ConvertError>;
    fn convert_unit(&self) -> Result<&str, ConvertError>;

    type Output<'a>;
    fn output(value: ConvertValue, unit: &Arc<Unit>) -> Self::Output<'_>;
}

#[derive(PartialEq, Clone, Debug)]
pub enum ConvertValue {
    Number(f64),
    Range(RangeInclusive<f64>),
}

pub enum ConvertTo<'a> {
    Best(System),
    Unit(&'a str),
}

pub enum System {
    Metric,
    Imperial,
}

impl<'a> From<&'a str> for ConvertTo<'a> {
    fn from(value: &'a str) -> Self {
        Self::Unit(value)
    }
}

impl From<System> for ConvertTo<'_> {
    fn from(value: System) -> Self {
        Self::Best(value)
    }
}

impl ConvertFrom for Quantity<'_> {
    fn convert_value(&self) -> Result<ConvertValue, ConvertError> {
        match &self.value {
            Value::Number(n) => Ok(ConvertValue::Number(*n)),
            Value::Range(r) => Ok(ConvertValue::Range(r.clone())),
            Value::Text(t) => Err(ConvertError::TextValue(t.to_string())),
        }
    }

    fn convert_unit(&self) -> Result<&str, ConvertError> {
        match self.unit().map(|u| u.text()) {
            Some(u) => Ok(u),
            None => Err(ConvertError::NoUnit(self.value.clone().into_owned())),
        }
    }

    type Output<'a> = Quantity<'a>;
    fn output(value: ConvertValue, unit: &Arc<Unit>) -> Self::Output<'_> {
        Quantity::with_known_unit(value.into(), unit.symbol().into(), Some(Arc::clone(unit)))
    }
}

impl ConvertFrom for (f64, &str) {
    fn convert_value(&self) -> Result<ConvertValue, ConvertError> {
        Ok(ConvertValue::Number(self.0))
    }

    fn convert_unit(&self) -> Result<&str, ConvertError> {
        Ok(self.1)
    }

    type Output<'a> = (f64, &'a str);
    fn output(value: ConvertValue, unit: &Arc<Unit>) -> Self::Output<'_> {
        let value = match value {
            ConvertValue::Number(n) => n,
            ConvertValue::Range(_) => panic!("got range from converting number"),
        };

        (value, unit.symbol())
    }
}

impl From<ConvertValue> for Value<'_> {
    fn from(value: ConvertValue) -> Self {
        match value {
            ConvertValue::Number(n) => Self::Number(n),
            ConvertValue::Range(r) => Self::Range(r),
        }
    }
}

impl From<f64> for ConvertValue {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<RangeInclusive<f64>> for ConvertValue {
    fn from(value: RangeInclusive<f64>) -> Self {
        Self::Range(value)
    }
}

impl PartialOrd<Self> for ConvertValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        fn extract(v: &ConvertValue) -> f64 {
            match v {
                ConvertValue::Number(n) => *n,
                ConvertValue::Range(r) => *r.start(),
            }
        }
        let this = extract(self);
        let other = extract(other);
        this.partial_cmp(&other)
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum ConvertError {
    #[error("Tried to convert a value with no unit: {0}")]
    #[diagnostic(code(cooklang::convert::unitless_quantity))]
    NoUnit(Value<'static>),

    #[error("Tried to convert a text value: {0}")]
    #[diagnostic(code(cooklang::convert::text_value))]
    TextValue(String),

    #[error("Mixed physical quantities: {from} {to}")]
    #[diagnostic(code(cooklang::convert::mixed_quantities))]
    MixedQuantities {
        from: PhysicalQuantity,
        to: PhysicalQuantity,
    },

    #[error(transparent)]
    #[diagnostic(code(cooklang::convert::unknown_unit))]
    UnknownUnit(#[from] UnknownUnit),
}

impl Converter {
    pub(crate) fn quantity_units(
        &self,
        physical_quantity: PhysicalQuantity,
    ) -> impl Iterator<Item = &Unit> {
        self.quantity_index[physical_quantity]
            .iter()
            .map(|&id| self.all_units[id].as_ref())
    }

    pub(crate) fn temperature_regex(&self) -> Result<&Regex, regex::Error> {
        self.temperature_regex.get_or_try_init(|| {
            let symbols = self
                .quantity_units(crate::convert::PhysicalQuantity::Temperature)
                .flat_map(|unit| unit.symbols.iter())
                .map(|symbol| format!("({symbol})"))
                .collect::<Vec<_>>()
                .join("|");
            let float = r"[+-]?\d+([.,]\d+)?";
            RegexBuilder::new(&format!("({float})({symbols})"))
                .size_limit(500_000)
                .build()
        })
    }

    pub fn get_unit(&self, unit: &str) -> Result<&Arc<Unit>, UnknownUnit> {
        let id = self.get_unit_id(unit)?;
        Ok(&self.all_units[id])
    }
}

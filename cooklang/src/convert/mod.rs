use std::{collections::HashMap, ops::RangeInclusive, sync::Arc};

use enum_map::EnumMap;
use once_cell::sync::OnceCell;

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::quantity::{NotScaled, Quantity, QuantityValue, Value};

use self::{
    builder::ConverterBuilder,
    units_file::{SIPrefix, UnitsFile},
};

pub mod builder;
pub mod units_file;

#[derive(Debug, Clone)]
pub struct Converter {
    all_units: Vec<Arc<Unit>>,
    unit_index: UnitIndex,
    quantity_index: UnitQuantityIndex,
    best: EnumMap<PhysicalQuantity, BestConversionsStore>,
    default_system: System,

    temperature_regex: OnceCell<Regex>,
}

impl Converter {
    pub fn builder() -> ConverterBuilder {
        ConverterBuilder::new()
    }

    pub fn default_system(&self) -> System {
        self.default_system
    }

    pub fn unit_count(&self) -> usize {
        self.all_units.len()
    }

    pub fn unit_count_detailed(&self) -> UnitCount {
        UnitCount::new(self)
    }

    pub fn all_units(&self) -> impl Iterator<Item = &Unit> {
        self.all_units.iter().map(|u| u.as_ref())
    }

    pub fn is_best_unit(&self, unit: &Unit) -> bool {
        let unit_id = self
            .unit_index
            .get_unit_id(unit.symbol())
            .expect("unit not found");
        let mut iter = match &self.best[unit.physical_quantity] {
            BestConversionsStore::Unified(u) => u.0.iter(),
            BestConversionsStore::BySystem { metric, imperial } => match unit.system {
                Some(System::Metric) => metric.0.iter(),
                Some(System::Imperial) => imperial.0.iter(),
                None => return false,
            },
        };
        iter.any(|&(_, id)| id == unit_id)
    }
}

#[cfg(not(feature = "bundled_units"))]
impl Default for Converter {
    fn default() -> Self {
        ConverterBuilder::new().finish().unwrap()
    }
}

#[cfg(feature = "bundled_units")]
impl Default for Converter {
    fn default() -> Self {
        ConverterBuilder::new()
            .with_units_file(UnitsFile::bundled())
            .unwrap()
            .finish()
            .unwrap()
    }
}

#[derive(Debug, Default, Clone)]
pub struct UnitIndex(HashMap<Arc<str>, usize>);

pub type UnitQuantityIndex = EnumMap<PhysicalQuantity, Vec<usize>>;

#[derive(Debug, Clone, Serialize)]
pub struct Unit {
    pub names: Vec<Arc<str>>,
    pub symbols: Vec<Arc<str>>,
    pub aliases: Vec<Arc<str>>,
    pub ratio: f64,
    pub difference: f64,
    pub physical_quantity: PhysicalQuantity,
    pub system: Option<System>,
    #[serde(skip)]
    expand_si: bool,
    #[serde(skip)]
    expanded_units: Option<EnumMap<SIPrefix, usize>>,
}

impl Unit {
    fn all_keys(&self) -> impl Iterator<Item = &Arc<str>> {
        self.names.iter().chain(&self.symbols).chain(&self.aliases)
    }

    pub fn symbol(&self) -> &str {
        self.symbols
            .first()
            .or_else(|| self.names.first())
            .or_else(|| self.aliases.first())
            .expect("symbol, name or alias in unit")
    }
}

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.symbol())
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
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    strum::Display,
    strum::EnumString,
    enum_map::Enum,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PhysicalQuantity {
    Volume,
    Mass,
    Length,
    Temperature,
    Time,
}

impl Converter {
    pub fn convert<'f, 't, F: ConvertFrom<'f>>(
        &self,
        from: F,
        to: impl Into<ConvertTo<'t>>,
    ) -> Result<F::Output, ConvertError> {
        let (value, unit) = from.into_convert_value_unit()?;
        let to = to.into();
        let (value, output_unit) = self.convert_impl(value, unit, to)?;
        Ok(F::output(value, output_unit))
    }

    fn convert_impl<'a>(
        &'a self,
        value: ConvertValue,
        unit: ConvertUnit,
        to: ConvertTo,
    ) -> Result<(ConvertValue, &'a Arc<Unit>), ConvertError> {
        let unit = self.get_unit(&unit)?;

        match to {
            ConvertTo::Unit(target_unit) => {
                let to = self.get_unit(&target_unit)?;
                self.convert_to_unit(value, unit, to)
            }
            ConvertTo::Best(system) => self.convert_to_best(value, unit, system),
            ConvertTo::SameSystem => {
                self.convert_to_best(value, unit, unit.system.unwrap_or(self.default_system))
            }
        }
    }

    fn convert_to_unit<'a>(
        &self,
        value: ConvertValue,
        unit: &Arc<Unit>,
        target_unit: &'a Arc<Unit>,
    ) -> Result<(ConvertValue, &'a Arc<Unit>), ConvertError> {
        if unit.physical_quantity != target_unit.physical_quantity {
            return Err(ConvertError::MixedQuantities {
                from: unit.physical_quantity,
                to: target_unit.physical_quantity,
            });
        }
        Ok((self.convert_value(value, unit, target_unit), target_unit))
    }

    fn convert_to_best<'a>(
        &'a self,
        value: ConvertValue,
        unit: &Arc<Unit>,
        system: System,
    ) -> Result<(ConvertValue, &'a Arc<Unit>), ConvertError> {
        let conversions = match &self.best[unit.physical_quantity] {
            BestConversionsStore::Unified(u) => u,
            BestConversionsStore::BySystem { metric, imperial } => match system {
                System::Metric => metric,
                System::Imperial => imperial,
            },
        };

        let best_unit = conversions.best_unit(self, &value, unit);
        let converted = self.convert_value(value, unit, best_unit);

        Ok((converted, best_unit))
    }

    fn convert_value(&self, value: ConvertValue, from: &Arc<Unit>, to: &Arc<Unit>) -> ConvertValue {
        match value {
            ConvertValue::Number(n) => ConvertValue::Number(self.convert_f64(n, from, to)),
            ConvertValue::Range(r) => {
                let s = self.convert_f64(*r.start(), from, to);
                let e = self.convert_f64(*r.end(), from, to);
                ConvertValue::Range(s..=e)
            }
        }
    }

    fn convert_f64(&self, value: f64, from: &Arc<Unit>, to: &Arc<Unit>) -> f64 {
        if Arc::ptr_eq(from, to) {
            return value;
        }
        convert_f64(value, from, to)
    }

    pub fn get_unit<'a>(&'a self, unit: &ConvertUnit) -> Result<&'a Arc<Unit>, UnknownUnit> {
        let id = match unit {
            ConvertUnit::Unit(u) => self.unit_index.get_unit_id(u.symbol())?,
            ConvertUnit::UnitId(id) => *id,
            ConvertUnit::Key(key) => self.unit_index.get_unit_id(key)?,
        };
        Ok(&self.all_units[id])
    }
}

pub(crate) fn convert_f64(value: f64, from: &Unit, to: &Unit) -> f64 {
    assert_eq!(from.physical_quantity, to.physical_quantity);

    let norm = (value + from.difference) * from.ratio;
    (norm / to.ratio) - to.difference
}

#[derive(Debug, Error)]
#[error("Unknown unit: '{0}'")]
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

    fn best_unit<'a>(
        &self,
        converter: &'a Converter,
        value: &ConvertValue,
        unit: &Arc<Unit>,
    ) -> &'a Arc<Unit> {
        let value = match value {
            ConvertValue::Number(n) => n.abs(),
            ConvertValue::Range(r) => r.start().abs(),
        };
        let base_unit_id = self.base();
        let base_unit = &converter.all_units[base_unit_id];
        let norm = converter.convert_f64(value, unit, base_unit);

        let best_id = self
            .0
            .iter()
            .rev()
            .find(|(th, _)| norm >= *th)
            .or_else(|| self.0.first())
            .map(|&(_, id)| id)
            .expect("empty best units");
        &converter.all_units[best_id]
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum ConvertValue {
    Number(f64),
    Range(RangeInclusive<f64>),
}

pub enum ConvertUnit<'a> {
    Unit(&'a Arc<Unit>),
    UnitId(usize),
    Key(&'a str),
}

pub enum ConvertTo<'a> {
    SameSystem,
    Best(System),
    Unit(ConvertUnit<'a>),
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    Default,
    strum::Display,
    strum::EnumString,
    enum_map::Enum,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum System {
    #[default]
    Metric,
    Imperial,
}

impl<'a> From<&'a str> for ConvertUnit<'a> {
    fn from(value: &'a str) -> Self {
        Self::Key(value)
    }
}

impl<'a> From<&'a str> for ConvertTo<'a> {
    fn from(value: &'a str) -> Self {
        Self::Unit(ConvertUnit::Key(value))
    }
}

impl From<System> for ConvertTo<'_> {
    fn from(value: System) -> Self {
        Self::Best(value)
    }
}

impl<'a> From<&'a Arc<Unit>> for ConvertTo<'a> {
    fn from(value: &'a Arc<Unit>) -> Self {
        Self::Unit(ConvertUnit::Unit(value))
    }
}

pub trait ConvertFrom<'a> {
    fn into_convert_value_unit(self) -> Result<(ConvertValue, ConvertUnit<'a>), ConvertError>;

    type Output;
    fn output(value: ConvertValue, unit: &Arc<Unit>) -> Self::Output;
}

impl<'a> ConvertFrom<'a> for &'a Quantity<'a> {
    fn into_convert_value_unit(self) -> Result<(ConvertValue, ConvertUnit<'a>), ConvertError> {
        let value = match &self.value {
            crate::quantity::QuantityValue::Fixed(v) => match v {
                Value::Number(n) => Ok(ConvertValue::Number(*n)),
                Value::Range(r) => Ok(ConvertValue::Range(r.clone())),
                Value::Text(t) => Err(ConvertError::TextValue(t.to_string())),
            },
            crate::quantity::QuantityValue::Scalable(v) => {
                Err(ConvertError::NotScaled(NotScaled(v.clone().into_owned())))
            }
        }?;
        let unit = match self.unit().map(|u| u.text()) {
            Some(u) => Ok(ConvertUnit::Key(u)),
            None => Err(ConvertError::NoUnit(Quantity::clone(self).into_owned())),
        }?;
        Ok((value, unit))
    }

    type Output = Quantity<'static>;
    fn output(value: ConvertValue, unit: &Arc<Unit>) -> Self::Output {
        Quantity::with_known_unit(
            value.into(),
            unit.symbol().to_string().into(), // ? unnecesary alloc
            Some(Arc::clone(unit)),
        )
    }
}

impl<'u, V, U> ConvertFrom<'u> for (V, U)
where
    V: Into<ConvertValue>,
    U: Into<ConvertUnit<'u>>,
{
    fn into_convert_value_unit(self) -> Result<(ConvertValue, ConvertUnit<'u>), ConvertError> {
        Ok((self.0.into(), self.1.into()))
    }

    type Output = (ConvertValue, Unit);

    fn output(value: ConvertValue, unit: &Arc<Unit>) -> Self::Output {
        (value, unit.as_ref().clone())
    }
}

impl From<ConvertValue> for QuantityValue<'_> {
    fn from(value: ConvertValue) -> Self {
        Self::Fixed(value.into())
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

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("Tried to convert a value with no unit: {0}")]
    NoUnit(Quantity<'static>),

    #[error("Tried to convert a text value: {0}")]
    TextValue(String),

    #[error(transparent)]
    NotScaled(#[from] NotScaled),

    #[error("Mixed physical quantities: {from} {to}")]
    MixedQuantities {
        from: PhysicalQuantity,
        to: PhysicalQuantity,
    },

    #[error(transparent)]
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
}

pub struct UnitCount {
    pub all: usize,
    pub by_system: EnumMap<System, usize>,
    pub by_quantity: EnumMap<PhysicalQuantity, usize>,
}

impl UnitCount {
    pub fn new(converter: &Converter) -> Self {
        Self {
            all: converter.all_units.len(),
            by_system: converter
                .all_units
                .iter()
                .fold(EnumMap::default(), |mut m, u| {
                    if let Some(s) = u.system {
                        m[s] += 1
                    }
                    m
                }),
            by_quantity: enum_map::enum_map! {
                q => converter.quantity_index[q].len()
            },
        }
    }
}

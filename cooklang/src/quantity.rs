//! Quantity model

use std::{collections::HashMap, fmt::Display, ops::RangeInclusive, sync::Arc};

use enum_map::EnumMap;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ast,
    convert::{ConvertError, Converter, PhysicalQuantity, Unit},
};

/// A quantity used in components such an [Ingredient](crate::model::Ingredient)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Quantity {
    /// Value
    pub value: QuantityValue,
    pub(crate) unit: Option<QuantityUnit>,
}

/// A value that can or not be changed by scaling it
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum QuantityValue {
    /// Cannot be scaled
    Fixed(Value),
    /// Scaling is linear to the number of servings
    Linear(Value),
    /// Scaling is in defined steps of the number of servings
    ByServings(Vec<Value>),
}

/// Base value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Value {
    /// Numeric
    Number(f64),
    /// Range
    Range(RangeInclusive<f64>),
    /// Text
    ///
    /// It is not possible to operate with this variant.
    Text(String),
}

/// Unit that has the text it has been parsed from and, if recognised,
/// information about what unit it is.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QuantityUnit {
    text: String,
    #[serde(skip)]
    info: OnceCell<UnitInfo>,
}

/// Information about the unit
#[derive(Debug, Clone)]
pub enum UnitInfo {
    /// Unit is known
    Known(Arc<Unit>),
    /// Unknown unit
    Unknown,
}

impl QuantityValue {
    /// Checks if any of the possible values is text
    pub fn contains_text_value(&self) -> bool {
        match self {
            QuantityValue::Fixed(v) | QuantityValue::Linear(v) => v.is_text(),
            QuantityValue::ByServings(v) => v.iter().any(Value::is_text),
        }
    }
}

impl Value {
    pub fn is_text(&self) -> bool {
        matches!(self, Value::Text(_))
    }
}

impl PartialEq for QuantityUnit {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl QuantityUnit {
    /// Original text of the unit
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Cached information about the unit.
    ///
    /// If [None] is returned it means
    /// the unit has not been parsed yet. Try with [Self::unit_or_parse].
    pub fn unit(&self) -> Option<&UnitInfo> {
        self.info.get()
    }

    /// Information about the unit
    pub fn unit_or_parse(&self, converter: &Converter) -> UnitInfo {
        self.info
            .get_or_init(|| UnitInfo::new(&self.text, converter))
            .clone()
    }
}

impl UnitInfo {
    fn new(text: &str, converter: &Converter) -> Self {
        match converter.get_unit(&text.into()) {
            Ok(unit) => Self::Known(Arc::clone(unit)),
            Err(_) => Self::Unknown,
        }
    }
}

impl Quantity {
    /// Creates a new quantity
    pub fn new(value: QuantityValue, unit: Option<String>) -> Self {
        Self {
            value,
            unit: unit.map(|text| QuantityUnit {
                text,
                info: OnceCell::new(),
            }),
        }
    }

    /// Creates a new quantity and parse the unit
    pub fn new_and_parse(
        value: QuantityValue,
        unit: Option<String>,
        converter: &Converter,
    ) -> Self {
        Self {
            value,
            unit: unit.map(|text| QuantityUnit {
                info: OnceCell::from(UnitInfo::new(&text, converter)),
                text,
            }),
        }
    }

    /// Createa a new quantity with a known unit
    pub(crate) fn with_known_unit(
        value: QuantityValue,
        unit_text: String,
        unit: Option<Arc<Unit>>,
    ) -> Self {
        Self {
            value,
            unit: Some(QuantityUnit {
                text: unit_text,
                info: OnceCell::from(match unit {
                    Some(unit) => UnitInfo::Known(unit),
                    None => UnitInfo::Unknown,
                }),
            }),
        }
    }

    /// Get the unit
    pub fn unit(&self) -> Option<&QuantityUnit> {
        self.unit.as_ref()
    }

    /// Get the unit text
    pub fn unit_text(&self) -> Option<&str> {
        self.unit.as_ref().map(|u| u.text.as_ref())
    }

    /// Get the unit info.
    ///
    /// [None] can mean that it has no unit or that the unit has not been parsed
    /// yet. See [QuantityUnit::unit_or_parse].
    pub fn unit_info(&self) -> Option<&UnitInfo> {
        self.unit.as_ref().and_then(|u| u.info.get())
    }
}

impl QuantityValue {
    pub(crate) fn from_ast(value: ast::QuantityValue) -> Self {
        match value {
            ast::QuantityValue::Single {
                value,
                auto_scale: None,
                ..
            } => Self::Fixed(value.take()),
            ast::QuantityValue::Single {
                value,
                auto_scale: Some(_),
                ..
            } => Self::Linear(value.take()),
            ast::QuantityValue::Many(v) => {
                Self::ByServings(v.into_iter().map(crate::located::Located::take).collect())
            }
        }
    }
}

impl Display for Quantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)?;
        if let Some(unit) = &self.unit {
            write!(f, " {}", unit)?;
        }
        Ok(())
    }
}

impl Display for QuantityValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed(v) | Self::Linear(v) => v.fmt(f),
            Self::ByServings(v) => {
                for value in &v[..v.len() - 1] {
                    write!(f, "{}|", value)?;
                }
                write!(f, "{}", v.last().unwrap())
            }
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn float(n: f64) -> f64 {
            (n * 1000.0).round() / 1000.0
        }

        match self {
            Value::Number(n) => write!(f, "{}", float(*n)),
            Value::Range(r) => write!(f, "{}-{}", float(*r.start()), float(*r.end())),
            Value::Text(t) => write!(f, "{}", t),
        }
    }
}

impl Display for QuantityUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<RangeInclusive<f64>> for Value {
    fn from(value: RangeInclusive<f64>) -> Self {
        Self::Range(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

/// Error during adding of quantities
#[derive(Debug, Error)]
pub enum QuantityAddError {
    #[error(transparent)]
    IncompatibleUnits(#[from] IncompatibleUnits),

    #[error(transparent)]
    TextValue(#[from] TextValueError),

    #[error(transparent)]
    Convert(#[from] ConvertError),

    #[error("Quantities must be scaled before adding them")]
    NotScaled(#[from] NotScaled),
}

/// Error that makes quantity units incompatible to be added
#[derive(Debug, Error)]
pub enum IncompatibleUnits {
    #[error("Missing unit: one unit is '{found}' but the other quantity is missing an unit")]
    MissingUnit {
        found: either::Either<QuantityUnit, QuantityUnit>,
    },
    #[error("Different physical quantity: '{a}' '{b}'")]
    DifferentPhysicalQuantities {
        a: PhysicalQuantity,
        b: PhysicalQuantity,
    },
    #[error("Unknown units differ: '{a}' '{b}'")]
    UnknownDifferentUnits { a: String, b: String },
}

impl Quantity {
    /// Checks if two quantities can be added
    pub fn is_compatible(
        &self,
        rhs: &Self,
        converter: &Converter,
    ) -> Result<Option<Arc<Unit>>, IncompatibleUnits> {
        let base = match (&self.unit, &rhs.unit) {
            // No units = ok
            (None, None) => None,
            // Mixed = error
            (None, Some(u)) => {
                return Err(IncompatibleUnits::MissingUnit {
                    found: either::Either::Right(u.to_owned()),
                });
            }
            (Some(u), None) => {
                return Err(IncompatibleUnits::MissingUnit {
                    found: either::Either::Left(u.to_owned()),
                });
            }
            // Units -> check
            (Some(a), Some(b)) => {
                let a_unit = a.unit_or_parse(converter);
                let b_unit = b.unit_or_parse(converter);

                match (a_unit, b_unit) {
                    (UnitInfo::Known(a_unit), UnitInfo::Known(b_unit)) => {
                        if a_unit.physical_quantity != b_unit.physical_quantity {
                            return Err(IncompatibleUnits::DifferentPhysicalQuantities {
                                a: a_unit.physical_quantity,
                                b: b_unit.physical_quantity,
                            });
                        }
                        // common unit is first one
                        Some(a_unit)
                    }
                    _ => {
                        // if units are unknown, their text must be equal
                        if a.text != b.text {
                            return Err(IncompatibleUnits::UnknownDifferentUnits {
                                a: a.text.clone(),
                                b: b.text.clone(),
                            });
                        }
                        None
                    }
                }
            }
        };
        Ok(base)
    }

    /// Try adding two quantities
    pub fn try_add(&self, rhs: &Self, converter: &Converter) -> Result<Quantity, QuantityAddError> {
        // 1. Check if the units are compatible and (maybe) get a common unit
        let convert_to = self.is_compatible(rhs, converter)?;

        // 2. Convert rhs to the unit of the first one if needed
        let rhs = if let Some(to) = convert_to {
            converter.convert(rhs, &to)?
        } else {
            rhs.to_owned()
        };

        // 3. Sum values
        let value = self.value.try_add(&rhs.value)?;

        // 4. New quantity
        let qty = Quantity {
            value,
            unit: self.unit.clone(), // unit is mantained
        };

        Ok(qty)
    }

    /// Converts the unit to the best possible match in the same unit system.
    ///
    /// For example, `1000 ml` would be converted to `1 l`.
    pub fn fit(&mut self, converter: &Converter) -> Result<(), ConvertError> {
        use crate::convert::ConvertTo;

        // if the unit is known, convert to the best match in the same system
        if matches!(
            self.unit().map(|u| u.unit_or_parse(converter)),
            Some(UnitInfo::Known(_))
        ) {
            *self = converter.convert(&*self, ConvertTo::SameSystem)?;
        }
        Ok(())
    }
}

/// Error when try to operate on a non scaled value
#[derive(Debug, Error)]
#[error("Tried to operate on a non scaled value: {0}")]
pub struct NotScaled(pub QuantityValue);

impl QuantityValue {
    pub(crate) fn extract_value(&self) -> Result<&Value, NotScaled> {
        match self {
            Self::Fixed(v) => Ok(v),
            Self::Linear(_) | Self::ByServings(_) => Err(NotScaled(self.to_owned())),
        }
    }

    /// Try adding two [QuantityValue]s.
    pub fn try_add(&self, rhs: &Self) -> Result<Self, QuantityAddError> {
        let value = self.extract_value()?.try_add(rhs.extract_value()?)?;
        Ok(QuantityValue::Fixed(value))
    }
}

/// Error when try to operate on a text value
#[derive(Debug, Error, Clone)]
#[error("Cannot operate on a text value")]
pub struct TextValueError(pub Value);

impl Value {
    /// Try adding two [Value]s
    pub fn try_add(&self, rhs: &Self) -> Result<Value, TextValueError> {
        let val = match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
            (Value::Number(n), Value::Range(r)) | (Value::Range(r), Value::Number(n)) => {
                Value::Range(r.start() + n..=r.end() + n)
            }
            (Value::Range(a), Value::Range(b)) => {
                Value::Range(a.start() + b.start()..=a.end() + b.end())
            }
            (t @ Value::Text(_), _) | (_, t @ Value::Text(_)) => {
                return Err(TextValueError(t.to_owned()));
            }
        };

        Ok(val)
    }
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct GroupedQuantity {
    /// known units
    known: EnumMap<PhysicalQuantity, Option<Quantity>>,
    /// unknown units
    unknown: HashMap<String, Quantity>,
    /// no units
    no_unit: Option<Quantity>,
    /// could not operate/add to others
    other: Vec<Quantity>,
}

impl GroupedQuantity {
    pub fn add(&mut self, q: &Quantity, converter: &Converter) {
        macro_rules! add {
            ($stored:expr, $quantity:ident, $converter:expr, $other:expr) => {
                match $stored.try_add($quantity, $converter) {
                    Ok(q) => *$stored = q,
                    Err(_) => {
                        $other.push($quantity.clone());
                        return;
                    }
                }
            };
        }

        if q.value.contains_text_value() {
            self.other.push(q.clone());
            return;
        }
        if q.unit.is_none() {
            if let Some(stored) = &mut self.no_unit {
                add!(stored, q, converter, self.other);
            } else {
                self.no_unit = Some(q.clone());
            }
            return;
        }

        let unit = q.unit.as_ref().unwrap();
        let info = unit.unit_or_parse(converter);
        match info {
            UnitInfo::Known(unit) => {
                if let Some(stored) = &mut self.known[unit.physical_quantity] {
                    add!(stored, q, converter, self.other);
                } else {
                    self.known[unit.physical_quantity] = Some(q.clone());
                }
            }
            UnitInfo::Unknown => {
                if let Some(stored) = self.unknown.get_mut(unit.text()) {
                    add!(stored, q, converter, self.other);
                } else {
                    self.unknown.insert(unit.text.clone(), q.clone());
                }
            }
        };
    }

    pub fn merge(&mut self, other: &Self, converter: &Converter) {
        for q in other.all_quantities() {
            self.add(&q, converter)
        }
    }

    fn all_quantities(&self) -> impl Iterator<Item = Quantity> + '_ {
        self.known
            .values()
            .filter_map(|q| q.as_ref())
            .chain(self.unknown.values())
            .chain(self.other.iter())
            .chain(self.no_unit.iter())
            .cloned()
    }

    pub fn total(&self) -> TotalQuantity {
        let mut all = self.all_quantities().peekable();

        let Some(first) = all.next()
        else { return TotalQuantity::None; };

        if all.peek().is_none() {
            TotalQuantity::Single(first)
        } else {
            let mut many = Vec::with_capacity(1 + all.size_hint().0);
            many.push(first);
            for q in all {
                many.push(q);
            }
            TotalQuantity::Many(many)
        }
    }
}

pub enum TotalQuantity {
    None,
    Single(Quantity),
    Many(Vec<Quantity>),
}

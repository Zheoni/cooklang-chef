//! Quantity model

use std::{borrow::Cow, fmt::Display, ops::RangeInclusive, sync::Arc};

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ast,
    convert::{ConvertError, Converter, PhysicalQuantity, Unit},
};

/// A quantity used in components such an [Ingredient](crate::model::Ingredient)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Quantity<'a> {
    /// Value
    pub value: QuantityValue<'a>,
    pub(crate) unit: Option<QuantityUnit<'a>>,
}

/// A value that can or not be changed by scaling it
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuantityValue<'a> {
    /// Cannot be scaled
    Fixed(Value<'a>),
    /// Can be scaled
    Scalable(ScalableValue<'a>),
}

/// A value that can be scaled
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScalableValue<'a> {
    /// Scaling is linear to the number of servings
    Linear(Value<'a>),
    /// Scaling is in defined steps of the number of servings
    ByServings(Vec<Value<'a>>),
}

/// Base value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value<'a> {
    /// Numeric
    Number(f64),
    /// Range
    Range(RangeInclusive<f64>),
    /// Text
    ///
    /// It is not possible to operate with this variant.
    Text(Cow<'a, str>),
}

/// Unit that has the text it has been parsed from and, if recognised,
/// information about what unit it is.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QuantityUnit<'a> {
    text: Cow<'a, str>,
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

impl QuantityValue<'_> {
    pub fn into_owned(self) -> QuantityValue<'static> {
        match self {
            QuantityValue::Fixed(v) => QuantityValue::Fixed(v.into_owned()),
            QuantityValue::Scalable(v) => QuantityValue::Scalable(v.into_owned()),
        }
    }
}

impl ScalableValue<'_> {
    pub fn into_owned(self) -> ScalableValue<'static> {
        match self {
            ScalableValue::Linear(v) => ScalableValue::Linear(v.into_owned()),
            ScalableValue::ByServings(v) => {
                ScalableValue::ByServings(v.into_iter().map(Value::into_owned).collect())
            }
        }
    }
}

impl Value<'_> {
    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::Number(n) => Value::Number(n),
            Value::Range(r) => Value::Range(r),
            Value::Text(t) => Value::Text(t.into_owned().into()),
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Value::Text(_))
    }
}

impl PartialEq for QuantityUnit<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl QuantityUnit<'_> {
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
    pub fn unit_or_parse(&self, converter: &Converter) -> &UnitInfo {
        self.info
            .get_or_init(|| UnitInfo::new(&self.text, converter))
    }

    pub fn into_owned(self) -> QuantityUnit<'static> {
        QuantityUnit {
            text: Cow::Owned(self.text.into_owned()),
            ..self
        }
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

impl<'a> Quantity<'a> {
    /// Creates a new quantity
    pub fn new(value: QuantityValue<'a>, unit: Option<Cow<'a, str>>) -> Self {
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
        value: QuantityValue<'a>,
        unit: Option<Cow<'a, str>>,
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
        value: QuantityValue<'a>,
        unit_text: Cow<'a, str>,
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

    pub fn into_owned(self) -> Quantity<'static> {
        Quantity {
            value: self.value.into_owned(),
            unit: self.unit.map(QuantityUnit::into_owned),
        }
    }
}

impl<'a> QuantityValue<'a> {
    pub(crate) fn from_ast(value: ast::QuantityValue<'a>) -> Self {
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
            } => Self::Scalable(ScalableValue::Linear(value.take())),
            ast::QuantityValue::Many(v) => Self::Scalable(ScalableValue::ByServings(
                v.into_iter().map(crate::located::Located::take).collect(),
            )),
        }
    }

    /// Checks if any of the possible values is text
    pub fn contains_text_value(&self) -> bool {
        match self {
            QuantityValue::Fixed(v) => v.is_text(),
            QuantityValue::Scalable(v) => match v {
                ScalableValue::Linear(v) => v.is_text(),
                ScalableValue::ByServings(v) => v.iter().any(Value::is_text),
            },
        }
    }
}

impl Display for Quantity<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)?;
        if let Some(unit) = &self.unit {
            write!(f, " {}", unit)?;
        }
        Ok(())
    }
}

impl Display for QuantityValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuantityValue::Fixed(v) => v.fmt(f),
            QuantityValue::Scalable(v) => v.fmt(f),
        }
    }
}

impl Display for ScalableValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScalableValue::Linear(value) => value.fmt(f),
            ScalableValue::ByServings(v) => {
                for value in &v[..v.len() - 1] {
                    write!(f, "{}|", value)?;
                }
                write!(f, "{}", v.last().unwrap())
            }
        }
    }
}

impl Display for Value<'_> {
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

impl Display for QuantityUnit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl From<f64> for Value<'_> {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<RangeInclusive<f64>> for Value<'_> {
    fn from(value: RangeInclusive<f64>) -> Self {
        Self::Range(value)
    }
}

impl<'a> From<Cow<'a, str>> for Value<'a> {
    fn from(value: Cow<'a, str>) -> Self {
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
        found: either::Either<QuantityUnit<'static>, QuantityUnit<'static>>,
    },
    #[error("Different physical quantity: '{a}' '{b}'")]
    DifferentPhysicalQuantities {
        a: PhysicalQuantity,
        b: PhysicalQuantity,
    },
    #[error("Unknown units differ: '{a}' '{b}'")]
    UnknownDifferentUnits { a: String, b: String },
}

impl Quantity<'_> {
    /// Checks if two quantities can be added
    pub fn is_compatible(
        &self,
        rhs: &Self,
        converter: &Converter,
    ) -> Result<Option<&Arc<Unit>>, IncompatibleUnits> {
        let base = match (&self.unit, &rhs.unit) {
            // No units = ok
            (None, None) => None,
            // Mixed = error
            (None, Some(u)) => {
                return Err(IncompatibleUnits::MissingUnit {
                    found: either::Either::Right(u.clone().into_owned()),
                });
            }
            (Some(u), None) => {
                return Err(IncompatibleUnits::MissingUnit {
                    found: either::Either::Left(u.clone().into_owned()),
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
                                a: a.text.clone().into_owned(),
                                b: b.text.clone().into_owned(),
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
    pub fn try_add(
        &self,
        rhs: &Self,
        converter: &Converter,
    ) -> Result<Quantity<'static>, QuantityAddError> {
        // 1. Check if the units are compatible and (maybe) get a common unit
        let convert_to = self.is_compatible(rhs, converter)?;

        // 2. Convert rhs to the unit of the first one if needed
        let rhs = if let Some(to) = convert_to {
            converter.convert(rhs, to)?
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

        Ok(qty.into_owned())
    }

    /// Converts the unit to the best possible match in the same unit system.
    ///
    /// For example, `1000 ml` would be converted to `1 l`.
    pub fn fit(&mut self, converter: &Converter) {
        use crate::convert::ConvertTo;

        // if the unit is known, convert to the best match in the same system
        if matches!(
            self.unit().map(|u| u.unit_or_parse(converter)),
            Some(UnitInfo::Known(_))
        ) {
            *self = converter
                .convert(&*self, ConvertTo::SameSystem)
                .expect("convert to same system failed");
        }
    }
}

/// Error when try to operate on a non scaled value
#[derive(Debug, Error)]
#[error("Tried to operate on a non scaled value: {0}")]
pub struct NotScaled(pub ScalableValue<'static>);

impl QuantityValue<'_> {
    pub(crate) fn extract_value(&self) -> Result<&Value, NotScaled> {
        match self {
            QuantityValue::Fixed(v) => Ok(v),
            QuantityValue::Scalable(v) => Err(NotScaled(v.clone().into_owned())),
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
pub struct TextValueError(pub Value<'static>);

impl Value<'_> {
    /// Try adding two [Value]s
    pub fn try_add(&self, rhs: &Self) -> Result<Value<'static>, TextValueError> {
        let val = match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
            (Value::Number(n), Value::Range(r)) | (Value::Range(r), Value::Number(n)) => {
                Value::Range(r.start() + n..=r.end() + n)
            }
            (Value::Range(a), Value::Range(b)) => {
                Value::Range(a.start() + b.start()..=a.end() + b.end())
            }
            (t @ Value::Text(_), _) | (_, t @ Value::Text(_)) => {
                return Err(TextValueError(t.clone().into_owned()));
            }
        };

        Ok(val)
    }
}

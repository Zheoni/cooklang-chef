use std::{borrow::Cow, fmt::Display, ops::RangeInclusive, sync::Arc};

use miette::Diagnostic;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    convert::{ConvertError, Converter, PhysicalQuantity, Unit},
    parser::located::OptTake,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Quantity<'a> {
    pub value: Value<'a>,
    unit: Option<QuantityUnit<'a>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QuantityUnit<'a> {
    text: Cow<'a, str>,
    #[serde(skip)]
    unit: OnceCell<MaybeUnit>,
}

impl PartialEq for QuantityUnit<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

#[derive(Debug, Clone)]
pub enum MaybeUnit {
    Known(Arc<Unit>),
    Unknown,
}

impl QuantityUnit<'_> {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn unit(&self) -> Option<&MaybeUnit> {
        self.unit.get()
    }

    pub fn unit_or_parse(&self, converter: &Converter) -> &MaybeUnit {
        self.unit
            .get_or_init(|| MaybeUnit::new(&self.text, converter))
    }

    pub fn into_owned(self) -> QuantityUnit<'static> {
        QuantityUnit {
            text: Cow::Owned(self.text.into_owned()),
            ..self
        }
    }
}

impl MaybeUnit {
    fn new(text: &str, converter: &Converter) -> Self {
        match converter.get_unit(&text.into()) {
            Ok(unit) => Self::Known(Arc::clone(unit)),
            Err(_) => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value<'a> {
    Number(f64),
    Range(RangeInclusive<f64>),
    Text(Cow<'a, str>),
}

impl<'a> Quantity<'a> {
    pub fn new(value: Value<'a>, unit: Option<Cow<'a, str>>) -> Self {
        Self {
            value,
            unit: unit.map(|text| QuantityUnit {
                text,
                unit: OnceCell::new(),
            }),
        }
    }

    pub fn new_and_parse(
        value: Value<'a>,
        unit: Option<Cow<'a, str>>,
        converter: &Converter,
    ) -> Self {
        Self {
            value,
            unit: unit.map(|text| QuantityUnit {
                unit: OnceCell::from(MaybeUnit::new(&text, converter)),
                text,
            }),
        }
    }

    pub fn with_known_unit(
        value: Value<'a>,
        unit_text: Cow<'a, str>,
        unit: Option<Arc<Unit>>,
    ) -> Self {
        Self {
            value,
            unit: Some(QuantityUnit {
                text: unit_text,
                unit: OnceCell::from(match unit {
                    Some(unit) => MaybeUnit::Known(unit),
                    None => MaybeUnit::Unknown,
                }),
            }),
        }
    }

    pub fn unitless(value: Value<'a>) -> Self {
        Self { value, unit: None }
    }

    pub fn unit(&self) -> Option<&QuantityUnit> {
        self.unit.as_ref()
    }

    pub fn unit_text(&self) -> Option<&str> {
        self.unit.as_ref().map(|u| u.text.as_ref())
    }

    pub fn unit_info(&self) -> Option<&MaybeUnit> {
        self.unit.as_ref().and_then(|u| u.unit.get())
    }

    pub fn into_owned(self) -> Quantity<'static> {
        Quantity {
            value: self.value.into_owned(),
            unit: self.unit.map(QuantityUnit::into_owned),
        }
    }

    pub(crate) fn from_ast(quantity: crate::parser::ast::Quantity<'a>) -> Self {
        let crate::parser::ast::Quantity { value, unit } = quantity;
        Quantity::new(value.take(), unit.opt_take())
    }
}

impl<'a> Value<'a> {
    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::Text(t) => Value::Text(Cow::Owned(t.into_owned())),
            Value::Number(n) => Value::Number(n),
            Value::Range(r) => Value::Range(r),
        }
    }
}

impl Display for Quantity<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(unit) = &self.unit {
            write!(f, "{} {}", self.value, unit)
        } else {
            write!(f, "{}", self.value)
        }
    }
}

impl Display for QuantityUnit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
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
            Value::Text(t) => write! {f, "{}", t},
        }
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

#[derive(Debug, Error, Diagnostic)]
pub enum QuantityAddError {
    #[error(transparent)]
    #[diagnostic(code(cooklang::quantity::add::incompatible_units))]
    IncompatibleUnits(#[from] IncompatibleUnits),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Value(#[from] ValueAddError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Convert(#[from] ConvertError),
}

#[derive(Debug, Error, Diagnostic)]
pub enum IncompatibleUnits {
    #[error("Missing unit: one unit is '{found}' but the other quantity is missing a unit")]
    MissingUnit { found: QuantityUnit<'static> },
    #[error("Different physical quantity: '{a}' '{b}'")]
    #[diagnostic(help("The physical quantity must be the same to add the values"))]
    DifferentPhysicalQuantities {
        a: PhysicalQuantity,
        b: PhysicalQuantity,
    },
    #[error("Unknown units differ: '{a}' '{b}'")]
    #[diagnostic(help("Unknown units can only be added when they are exactly the same"))]
    UnknownDifferentUnits { a: String, b: String },
}

impl Quantity<'_> {
    pub fn is_compatible(
        &self,
        rhs: &Self,
        converter: &Converter,
    ) -> Result<Option<&Arc<Unit>>, IncompatibleUnits> {
        let base = match (&self.unit, &rhs.unit) {
            // No units = ok
            (None, None) => None,
            // Mixed = error
            (None, Some(u)) | (Some(u), None) => {
                return Err(IncompatibleUnits::MissingUnit {
                    found: u.clone().into_owned(),
                })
            }
            // Units -> check
            (Some(a), Some(b)) => {
                let a_unit = a.unit_or_parse(converter);
                let b_unit = b.unit_or_parse(converter);

                match (a_unit, b_unit) {
                    (MaybeUnit::Known(a_unit), MaybeUnit::Known(b_unit)) => {
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
        let mut qty = Quantity {
            value,
            unit: self.unit.clone(), // unit is mantained
        };

        // 5. Convert to the best unit in the same system if the unit is known
        if matches!(qty.unit_info(), Some(MaybeUnit::Known(_))) {
            qty = converter.convert(&qty, converter.default_system())?;
        }

        Ok(qty.into_owned())
    }
}

#[derive(Debug, Error, Diagnostic)]
#[diagnostic(code(cooklang::quantity::add::value))]
pub enum ValueAddError {
    #[error("Cannot add text value")]
    TextValue { val: Value<'static> },
}

impl Value<'_> {
    pub fn try_add(&self, rhs: &Self) -> Result<Self, ValueAddError> {
        let val = match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
            (Value::Number(n), Value::Range(r)) | (Value::Range(r), Value::Number(n)) => {
                Value::Range(r.start() + n..=r.end() + n)
            }
            (Value::Range(a), Value::Range(b)) => {
                Value::Range(a.start() + b.start()..=a.end() + b.end())
            }
            (t @ Value::Text(_), _) | (_, t @ Value::Text(_)) => {
                return Err(ValueAddError::TextValue {
                    val: t.clone().into_owned(),
                });
            }
        };

        Ok(val)
    }
}

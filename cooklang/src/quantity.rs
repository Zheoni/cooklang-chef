use std::{borrow::Cow, fmt::Display, ops::RangeInclusive, sync::Arc};

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use crate::{
    convert::{Converter, Unit},
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
}

impl MaybeUnit {
    fn new(text: &str, converter: &Converter) -> Self {
        match converter.get_unit(text) {
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
            unit: self.unit.map(|u| QuantityUnit {
                text: Cow::Owned(u.text.into_owned()),
                ..u
            }),
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
            write!(f, "{} {}", self.value, unit.text)
        } else {
            write!(f, "{}", self.value)
        }
    }
}

impl Display for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Range(r) => write!(f, "{}-{}", r.start(), r.end()),
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

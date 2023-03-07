use enum_map::EnumMap;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{collections::HashMap, fmt::Debug, sync::Arc};

use super::{PhysicalQuantity, System};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct UnitsFile {
    pub default_system: Option<System>,
    pub si: Option<SI>,
    pub extend: Option<Extend>,
    #[serde(default)]
    pub quantity: Vec<QuantityGroup>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct SI {
    pub prefixes: Option<EnumMap<SIPrefix, Vec<String>>>,
    pub symbol_prefixes: Option<EnumMap<SIPrefix, Vec<String>>>,
    #[serde(default)]
    pub precedence: Precedence,
}

#[derive(Debug, Deserialize, Clone, Copy, strum::Display, strum::AsRefStr, enum_map::Enum)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SIPrefix {
    Kilo,
    Hecto,
    Deca,
    Deci,
    Centi,
    Milli,
}

impl SIPrefix {
    pub fn ratio(&self) -> f64 {
        match self {
            SIPrefix::Kilo => 1e3,
            SIPrefix::Hecto => 1e2,
            SIPrefix::Deca => 1e1,
            SIPrefix::Deci => 1e-1,
            SIPrefix::Centi => 1e-2,
            SIPrefix::Milli => 1e-3,
        }
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Extend {
    pub precedence: Precedence,
    pub names: HashMap<String, Vec<Arc<str>>>,
    pub symbols: HashMap<String, Vec<Arc<str>>>,
    pub aliases: HashMap<String, Vec<Arc<str>>>,
}

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum Precedence {
    #[default]
    Before,
    After,
    Override,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct QuantityGroup {
    pub quantity: PhysicalQuantity,
    #[serde(default)]
    pub best: Option<BestUnits>,
    pub units: Units,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged, rename = "snake_case", deny_unknown_fields)]
pub enum BestUnits {
    Unified(Vec<String>),
    BySystem {
        metric: Vec<String>,
        imperial: Vec<String>,
    },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged, rename = "snake_case", deny_unknown_fields)]
pub enum Units {
    Unified(Vec<UnitEntry>),
    BySystem {
        #[serde(default)]
        metric: Vec<UnitEntry>,
        #[serde(default)]
        imperial: Vec<UnitEntry>,
        #[serde(default)]
        unspecified: Vec<UnitEntry>,
    },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct UnitEntry {
    pub names: Vec<Arc<str>>,
    pub symbols: Vec<Arc<str>>,
    #[serde(default)]
    pub aliases: Vec<Arc<str>>,
    pub ratio: f64,
    #[serde(default)]
    pub difference: f64,
    #[serde(default)]
    pub expand_si: bool,
}

impl UnitsFile {
    pub fn bundled() -> Self {
        const TEXT: &str = include_str!("../../units.toml");
        static FILE: Lazy<UnitsFile> = Lazy::new(|| toml::from_str(TEXT).unwrap());
        FILE.clone()
    }
}

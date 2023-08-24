use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::Extensions;
use serde::{Deserialize, Serialize};

use crate::{APP_NAME, COOK_DIR, UTF8_PATH_PANIC};

#[derive(Serialize, Deserialize)]
pub struct GlobalConfig {
    pub base_path: PathBuf,
    pub editor_command: Option<Vec<String>>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        let base_path = dirs::document_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_default()
            .join("Recipes");
        Self {
            base_path,
            editor_command: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub default_units: bool,
    pub warnings_as_errors: bool,
    pub recipe_ref_check: bool,
    pub max_depth: usize,
    #[serde(with = "extensions_serde")]
    pub extensions: Extensions,
    #[serde(skip_serializing_if = "Load::is_empty")]
    pub load: Load,
    #[serde(skip_serializing_if = "UiConfig::is_empty")]
    pub ui: UiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_units: true,
            extensions: Extensions::all() ^ Extensions::MULTILINE_STEPS,
            warnings_as_errors: false,
            recipe_ref_check: true,
            max_depth: 10,
            load: Default::default(),
            ui: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Load {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub units: Vec<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aisle: Option<PathBuf>,
}

impl Load {
    fn is_empty(&self) -> bool {
        self.units.is_empty() && self.aisle.is_none()
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UiConfig {
    tags: HashMap<String, TagProps>,
}

impl UiConfig {
    fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct TagProps {
    emoji: Option<String>,
}

const CONFIG_FILE: &str = "config.toml";
const AUTO_AISLE: &str = "aisle.conf";
const AUTO_UNITS: &str = "units.toml";

impl Config {
    pub fn read(base_path: &Utf8Path) -> Result<Self> {
        let local = config_file_path(base_path);
        if !local.is_file() {
            tracing::debug!("Local config not found, loading global default");
            let global = confy::load(APP_NAME, Some("default-config"))
                .context("Error loading default global config file")?;
            return Ok(global);
        }
        tracing::debug!("Loading local config from {local}");
        let content = std::fs::read_to_string(&local)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn override_with_args(&mut self, args: &crate::GlobalArgs) {
        if args.no_default_units {
            self.default_units = false;
        }
        if args.no_extensions {
            self.extensions = Extensions::empty();
        } else if args.all_extensions {
            self.extensions = Extensions::all();
        } else if args.compat_extensions {
            self.extensions = Extensions::COMPAT;
        } else if !args.extensions.is_empty() {
            use std::ops::BitOr;
            self.extensions = args
                .extensions
                .iter()
                .copied()
                .reduce(Extensions::bitor)
                .unwrap(); // checked not empty
        }
        if args.no_recipe_ref_check {
            self.recipe_ref_check = false;
        }
        if args.warnings_as_errors {
            self.warnings_as_errors = true;
        }
        self.max_depth = args.max_depth;
        if !args.units.is_empty() {
            self.load.units = args
                .units
                .iter()
                .filter_map(|p| p.canonicalize().ok())
                .collect();
        }
    }

    pub fn aisle(&self, base_path: &Utf8Path) -> Option<Utf8PathBuf> {
        self.load
            .aisle
            .as_ref()
            .map(|a| resolve_path(base_path, a))
            .or_else(|| {
                let auto = base_path.join(COOK_DIR).join(AUTO_AISLE);
                tracing::trace!("checking auto aisle file: {auto}");
                auto.is_file().then_some(auto)
            })
            .or_else(|| {
                let global = confy::get_configuration_file_path(APP_NAME, Some(AUTO_AISLE)).ok()?;
                let global = Utf8PathBuf::from_path_buf(global).ok()?.with_extension("");
                tracing::trace!("checking global auto aisle file: {global}");
                global.is_file().then_some(global)
            })
    }

    pub fn units(&self, base_path: &Utf8Path) -> Vec<Utf8PathBuf> {
        (!self.load.is_empty())
            .then(|| {
                self.load
                    .units
                    .iter()
                    .map(|p| resolve_path(base_path, p))
                    .collect()
            })
            .or_else(|| {
                let auto = base_path.join(COOK_DIR).join(AUTO_UNITS);
                tracing::trace!("checking auto units file: {auto}");
                auto.is_file().then_some(vec![auto])
            })
            .or_else(|| {
                let global = confy::get_configuration_file_path(APP_NAME, Some(AUTO_UNITS)).ok()?;
                let global = Utf8PathBuf::from_path_buf(global).ok()?.with_extension("");
                tracing::trace!("checking global auto aisle file: {global}");
                global.is_file().then_some(vec![global])
            })
            .unwrap_or(vec![])
    }
}

pub fn resolve_path(base_path: &Utf8Path, path: &Path) -> Utf8PathBuf {
    let path = Utf8Path::from_path(path).expect(UTF8_PATH_PANIC);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_path.join(COOK_DIR).join(path)
    }
}

pub fn config_file_path(base_path: &Utf8Path) -> Utf8PathBuf {
    base_path.join(COOK_DIR).join(CONFIG_FILE)
}

mod extensions_serde {
    use super::Extensions;

    pub fn serialize<S>(extensions: &Extensions, se: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if extensions.is_all() {
            se.serialize_str("all")
        } else if extensions.is_empty() {
            se.serialize_str("none")
        } else {
            let enabled = extensions.iter_names().map(|(name, _)| (name, true));
            let disabled = extensions
                .complement()
                .iter_names()
                .map(|(name, _)| (name, false));

            se.collect_map(enabled.chain(disabled))
        }
    }

    pub fn deserialize<'de, D>(de: D) -> Result<Extensions, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ExtensionsVisitor;

        impl<'de> serde::de::Visitor<'de> for ExtensionsVisitor {
            type Value = Extensions;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "one of \"all\", \"none\" or map with extension names to booleans, missing keys false",
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "all" => Ok(Extensions::all()),
                    "none" | "empty" => Ok(Extensions::empty()),
                    other => {
                        if let Ok(ext) = bitflags::parser::from_str(other) {
                            Ok(ext)
                        } else {
                            Err(E::custom("invalid extensions string"))
                        }
                    }
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                use serde::de::Error;

                let mut extensions = Extensions::empty();
                while let Ok(Some((name, enabled))) = map.next_entry::<&str, bool>() {
                    let e = Extensions::from_name(&name.replace(' ', "_").to_uppercase())
                        .ok_or_else(|| A::Error::custom("Unknown extension name"))?;
                    if enabled {
                        extensions |= e;
                    }
                }
                Ok(extensions)
            }
        }

        de.deserialize_str(ExtensionsVisitor)
    }
}

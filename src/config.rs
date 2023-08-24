use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::Extensions;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{APP_NAME, COOK_DIR, UTF8_PATH_PANIC};

pub const CONFIG_FILE: &str = "config.toml";
pub const AUTO_AISLE: &str = "aisle.conf";
pub const AUTO_UNITS: &str = "units.toml";
pub const DEFAULT_CONFIG_FILE: &str = "default-config.toml";
pub const GLOBAL_CONFIG_FILE: &str = "global-config.toml";

#[derive(Serialize, Deserialize)]
pub struct GlobalConfig {
    pub base_path: Option<PathBuf>,
    pub editor_command: Option<Vec<String>>,
}

impl GlobalConfig {
    pub fn read() -> Result<Self> {
        global_load(GLOBAL_CONFIG_FILE, true)
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        let base_path = default_base_path();
        Self {
            base_path: Some(base_path),
            editor_command: None,
        }
    }
}

pub fn default_base_path() -> PathBuf {
    let dirs = directories::UserDirs::new();
    let parent = if let Some(d) = &dirs {
        d.document_dir().unwrap_or(d.home_dir())
    } else {
        Path::new(".")
    };
    parent.join("Recipes")
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

impl Config {
    pub fn read(base_path: &Utf8Path) -> Result<Self> {
        let local = config_file_path(base_path);
        if !local.is_file() {
            tracing::debug!("Local config not found, loading global default");
            let global = global_load(DEFAULT_CONFIG_FILE, true)
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
                let global = global_file_path(AUTO_AISLE).ok()?;
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
                let global = global_file_path(AUTO_UNITS).ok()?;
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

pub fn global_file_path(name: &str) -> Result<Utf8PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", APP_NAME)
        .context("Could not determine home directory path")?;
    let config = Utf8Path::from_path(dirs.config_dir()).expect(UTF8_PATH_PANIC);
    let path = config.join(name);
    Ok(path)
}

pub fn global_load<T: DeserializeOwned + Serialize + Default>(
    name: &str,
    create: bool,
) -> Result<T> {
    let path = global_file_path(name)?;
    match File::open(&path) {
        Ok(mut f) => {
            let mut content = String::new();
            f.read_to_string(&mut content)?;
            toml::from_str(&content).context("Bad TOML data")
        }
        Err(e) if create && e.kind() == io::ErrorKind::NotFound => {
            let val = T::default();
            global_store_path(path, &val)?;
            Ok(val)
        }
        Err(e) => Err(e).context("Failed to load config file"),
    }
}

pub fn global_store<T: Serialize>(name: &str, val: T) -> Result<()> {
    let path = global_file_path(name)?;
    global_store_path(path, val)
}

fn global_store_path<T: Serialize>(path: impl AsRef<Path>, val: T) -> Result<()> {
    let parent = path
        .as_ref()
        .parent()
        .expect("Invalid config dir: no parent");
    fs::create_dir_all(parent).context("Failed to create config directory")?;
    let toml_str = toml::to_string_pretty(&val)?;
    fs::write(path, toml_str)?;
    Ok(())
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

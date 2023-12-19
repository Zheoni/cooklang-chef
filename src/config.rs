use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::Extensions;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{APP_NAME, COOK_DIR, UTF8_PATH_PANIC};

pub const CONFIG_FILE: &str = "config.toml";
pub const AUTO_AISLE: &str = "aisle.conf";
pub const AUTO_UNITS: &str = "units.toml";
pub const DEFAULT_CONFIG_FILE: &str = "default-config.toml";
pub const CHEF_CONFIG_FILE: &str = "chef-config.toml";

#[derive(Serialize, Deserialize, Clone)]
pub struct ChefConfig {
    pub default_collection: Option<Utf8PathBuf>,
    pub editor_command: Option<Vec<String>>,
}

impl ChefConfig {
    pub fn editor(&self) -> Result<Vec<String>> {
        let cmd = if let Some(custom) = &self.editor_command {
            if custom.is_empty() {
                bail!("Invalid custom editor command in global config. Fix it please.");
            }
            custom.clone()
        } else {
            const ENV_VARS: &[&str] = &["VISUAL", "EDITOR"];
            // TODO should this be notepad.exe that is installed by default?
            const HARD_CODED: &str = if cfg!(windows) {
                "code.cmd -n -w"
            } else {
                "nano"
            };

            let editor = ENV_VARS
                .iter()
                .filter_map(|v| env::var(v).ok())
                .find(|v| v.is_empty())
                .unwrap_or_else(|| HARD_CODED.to_string());

            shell_words::split(&editor)?
        };
        Ok(cmd)
    }
}

#[allow(clippy::derivable_impls)] // I like to see the exact defaults of the config
impl Default for ChefConfig {
    fn default() -> Self {
        Self {
            default_collection: None,
            editor_command: None,
        }
    }
}

pub fn default_config() -> Result<Config> {
    global_load(DEFAULT_CONFIG_FILE)
}

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Default, Clone)]
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
            return default_config().context("Error loading default global config file");
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
            let new_units = args.units.iter().flat_map(|p| p.canonicalize().ok());
            if args.override_units {
                self.load.units = new_units.collect();
            } else {
                self.load.units.extend(new_units);
            }
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

pub fn global_load<T: DeserializeOwned + Serialize + Default>(name: &str) -> Result<T> {
    let path = global_file_path(name)?;
    match File::open(&path) {
        Ok(mut f) => {
            let mut content = String::new();
            f.read_to_string(&mut content)?;
            toml::from_str(&content).context("Bad TOML data")
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let val = T::default();
            store_at_path(path, &val)?;
            Ok(val)
        }
        Err(e) => Err(e).context("Failed to load config file"),
    }
}

pub fn global_store<T: Serialize>(name: &str, val: T) -> Result<()> {
    let path = global_file_path(name)?;
    store_at_path(path, val)
}

pub fn store_at_path<T: Serialize>(path: impl AsRef<Path>, val: T) -> Result<()> {
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
            se.collect_map(
                Extensions::all()
                    .iter_names()
                    .map(|(name, flag)| (name, extensions.contains(flag))),
            )
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
                // TODO change String for &str when this is solved
                // https://github.com/serde-rs/serde/issues/2467
                while let Some((name, enabled)) = map.next_entry::<String, bool>()? {
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

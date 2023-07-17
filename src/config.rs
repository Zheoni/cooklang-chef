use std::path::{Path, PathBuf};

use anstream::{print, println};
use anyhow::Result;
use camino::Utf8Path;
use cooklang::Extensions;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{Context, COOK_DIR};

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
    pub editor_command: Option<Vec<String>>,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            default_units: true,
            extensions: Extensions::all() ^ Extensions::MULTILINE_STEPS,
            warnings_as_errors: false,
            recipe_ref_check: true,
            max_depth: 10,
            load: Default::default(),
            editor_command: None,
        }
    }
}

const CONFIG_NAME: &str = "config";
const AUTO_AISLE: &str = "aisle.conf";
const AUTO_UNITS: &str = "units.toml";

impl Config {
    pub fn read(base_path: &Utf8Path) -> Result<(Self, PathBuf)> {
        let local_file = base_path
            .join(COOK_DIR)
            .join(CONFIG_NAME)
            .with_extension("toml");

        let path = if local_file.is_file() {
            local_file.into()
        } else {
            confy::get_configuration_file_path(crate::APP_NAME, Some(CONFIG_NAME))?
        };

        debug!("Loading configuration from {}", path.display());

        let config = confy::load_path(&path)?;

        Ok((config, path))
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

    pub fn aisle(&self, ctx: &Context) -> Option<PathBuf> {
        self.load
            .aisle
            .as_ref()
            .map(|a| resolve_path(&ctx.config_path, a))
            .or_else(|| {
                let local = ctx.base_path.as_std_path().join(COOK_DIR).join(AUTO_AISLE);
                local.is_file().then_some(local)
            })
            .or_else(|| {
                let relative = resolve_path(&ctx.config_path, Path::new(AUTO_AISLE));
                relative.is_file().then_some(relative)
            })
    }

    pub fn units(&self, config_path: &Path, base_path: &Path) -> Vec<PathBuf> {
        if !self.load.units.is_empty() {
            return self
                .load
                .units
                .iter()
                .map(|p| resolve_path(config_path, p))
                .collect();
        }

        let local = base_path.join(COOK_DIR).join(AUTO_UNITS);
        let relative = resolve_path(config_path, Path::new(AUTO_UNITS));
        if local.is_file() {
            vec![local]
        } else if relative.is_file() {
            vec![relative]
        } else {
            vec![]
        }
    }
}

pub fn resolve_path(config_path: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.into()
    } else {
        config_path
            .parent()
            .expect("cofig_path does not have a parent")
            .join(path)
    }
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
                    "string with \"all\", \"none\" or map with extension names to booleans, missing keys false",
                )
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
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

pub fn run(ctx: &Context) -> Result<()> {
    use owo_colors::OwoColorize;

    println!(
        "Configuration has been loaded from:\n\t{}",
        ctx.config_path.display().yellow()
    );
    let c = toml::to_string_pretty(&ctx.config)?;
    let fence = "+++".dimmed();
    println!("{fence}");
    println!("{}", c.trim());
    println!("{fence}");

    for file in ctx
        .config
        .units(&ctx.config_path, ctx.base_path.as_std_path())
        .iter()
        .chain(ctx.config.aisle(ctx).iter())
    {
        print!("{} {} ", file.display(), "--".dimmed());
        if file.is_file() {
            println!("{}", "found".green().bold());
        } else {
            println!("{}", "not found".red().bold());
        }
    }

    Ok(())
}

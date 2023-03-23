use std::path::{Path, PathBuf};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::Extensions;
use serde::{Deserialize, Serialize};
use tracing::info;
use yansi::Paint;

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
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Load {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub units: Vec<Utf8PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aile: Option<Utf8PathBuf>,
}

impl Load {
    fn is_empty(&self) -> bool {
        self.units.is_empty() && self.aile.is_none()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_units: true,
            extensions: Extensions::all(),
            warnings_as_errors: false,
            recipe_ref_check: true,
            max_depth: 3,
            load: Default::default(),
        }
    }
}

const CONFIG_NAME: &str = "config";
const AUTO_AILE: &str = "aile.conf";
const AUTO_UNITS: &str = "units.toml";

impl Config {
    pub fn read(base_dir: &Utf8Path) -> Result<(Self, PathBuf)> {
        let local_file = base_dir
            .join(COOK_DIR)
            .join(CONFIG_NAME)
            .with_extension("toml");

        let path = if local_file.is_file() {
            local_file.into()
        } else {
            confy::get_configuration_file_path(crate::APP_NAME, Some(CONFIG_NAME))?
        };

        info!("Loading configuration from {}", path.display());

        let config = confy::load_path(&path)?;

        Ok((config, path))
    }

    pub fn override_with_args(&mut self, args: &crate::GlobalArgs) {
        if args.no_default_units {
            self.default_units = false;
        }
        if args.no_extensions {
            self.extensions = Extensions::empty();
        }
        if args.no_recipe_ref_check {
            self.recipe_ref_check = false;
        }
        if args.warnings_as_errors {
            self.warnings_as_errors = true;
        }
        if let Some(d) = args.max_depth {
            self.max_depth = d;
        }
        if !args.units.is_empty() {
            self.load.units = args.units.clone();
        }
    }

    pub fn aile(&self, ctx: &Context) -> Option<PathBuf> {
        self.load
            .aile
            .as_ref()
            .map(|a| resolve_path(&ctx.config_path, a.as_std_path()))
            .or_else(|| {
                let local = ctx.base_dir.as_std_path().join(COOK_DIR).join(AUTO_AILE);
                local.is_file().then_some(local)
            })
            .or_else(|| {
                let relative = resolve_path(&ctx.config_path, &Path::new(AUTO_AILE));
                relative.is_file().then_some(relative)
            })
    }

    pub fn units(&self, config_path: &Path, base_dir: &Path) -> Vec<PathBuf> {
        if !self.load.units.is_empty() {
            return self
                .load
                .units
                .iter()
                .map(|p| resolve_path(config_path, p.as_std_path()))
                .collect();
        }

        let local = base_dir.join(COOK_DIR).join(AUTO_UNITS);
        let relative = resolve_path(config_path, &Path::new(AUTO_UNITS));
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
            se.collect_map(extensions.iter_names().map(|(name, _)| (name, true)))
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
                    _ => Err(E::custom("invalid extensions string")),
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
    println!(
        "Configuration has been loaded from:\n\t{}",
        Paint::yellow(ctx.config_path.display())
    );
    let c = toml::to_string_pretty(&ctx.config)?;
    let fence = Paint::new("+++").dimmed();
    println!("{fence}");
    println!("{}", c.trim());
    println!("{fence}");

    for file in ctx
        .config
        .units(&ctx.config_path, ctx.base_dir.as_std_path())
        .iter()
        .chain(ctx.config.aile(ctx).iter())
    {
        print!("{} {} ", file.display(), Paint::new("--").dimmed());
        if file.is_file() {
            println!("{}", Paint::green("found"));
        } else {
            println!("{}", Paint::red("not found"));
        }
    }

    Ok(())
}

#[test]
fn default_config() {
    std::fs::write(
        "config.toml",
        toml::to_string_pretty(&Config::default()).unwrap(),
    )
    .unwrap();
}

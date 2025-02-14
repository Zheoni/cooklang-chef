use anstream::ColorChoice;
use anyhow::{bail, Context as _, Result};
use args::{CliArgs, Command, GlobalArgs};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use config::{global_load, ChefConfig, Config, CHEF_CONFIG_FILE};
use cooklang::{convert::ConverterBuilder, Converter, CooklangParser, ParseOptions};
use cooklang_fs::LazyFsIndex;
use once_cell::sync::OnceCell;
use util::metadata_validator;

// commands
mod cmd;

// other modules
mod args;
mod config;
mod util;

const COOK_DIR: &str = ".cooklang";
const APP_NAME: &str = "cooklang-chef";
const UTF8_PATH_PANIC: &str = "chef only supports UTF-8 paths. If this is problem for you, file an issue in the cooklang-chef github repository";

pub fn main() -> Result<()> {
    let args = CliArgs::parse();

    let color_ctx = init_color(args.global_args.color);
    if args.global_args.debug_trace {
        tracing_subscriber::FmtSubscriber::builder()
            .compact()
            .with_max_level(tracing::Level::TRACE)
            .with_span_events(
                tracing_subscriber::fmt::format::FmtSpan::CLOSE
                    | tracing_subscriber::fmt::format::FmtSpan::NEW,
            )
            .with_ansi(color_ctx.color_stderr)
            .init();
    } else {
        tracing_subscriber::FmtSubscriber::builder()
            .compact()
            .with_target(false)
            .with_ansi(color_ctx.color_stderr)
            .init();
    }

    match args.command {
        Command::GenerateCompletions(args) => return cmd::generate_completions::run(args),
        _ => {}
    }

    let ctx = configure_context(args.global_args, color_ctx)?;

    match args.command {
        Command::Recipe(args) => cmd::recipe::run(&ctx, args),
        Command::List(args) => cmd::list::run(&ctx, args),
        #[cfg(feature = "serve")]
        Command::Serve(args) => cmd::serve::run(ctx, args),
        Command::ShoppingList(args) => cmd::shopping_list::run(&ctx, args),
        Command::Units(args) => cmd::units::run(ctx.parser()?.converter(), args),
        Command::Convert(args) => cmd::convert::run(ctx.parser()?.converter(), args),
        Command::Config(args) => cmd::config::run(&ctx, args),
        Command::Collection(args) => cmd::collection::run(&ctx, args),
        Command::New(args) => cmd::new::run(args, &ctx),
        Command::Edit(args) => cmd::edit::run(args, &ctx),
        Command::GenerateCompletions(_) => unreachable!(),
    }
}

struct ColorContext {
    color_stderr: bool,
}

fn init_color(color: colorchoice_clap::Color) -> ColorContext {
    color.write_global();
    let color_stderr = anstream::AutoStream::choice(&std::io::stderr()) != ColorChoice::Never;

    ColorContext { color_stderr }
}

pub struct Context {
    parser: OnceCell<CooklangParser>,
    recipe_index: LazyFsIndex,
    global_args: GlobalArgs,
    base_path: Utf8PathBuf,
    config: config::Config,
    chef_config: config::ChefConfig,
    color: ColorContext,
    is_collection: bool,
}

#[tracing::instrument(level = "debug", skip_all)]
fn configure_context(args: GlobalArgs, color_ctx: ColorContext) -> Result<Context> {
    let chef_config: ChefConfig =
        global_load(CHEF_CONFIG_FILE).context("Error loading global config file")?;

    let base_path = args
        .path
        .as_deref()
        .or_else(|| {
            Utf8Path::new(COOK_DIR)
                .is_dir()
                .then_some(Utf8Path::new("."))
        })
        .or(chef_config.default_collection.as_deref())
        .unwrap_or(Utf8Path::new("."));
    if !base_path.is_dir() {
        bail!("Base path is not a directory: '{base_path}'");
    }

    let mut config = if let Some(file) = &args.config_file {
        Config::read(file)?
    } else {
        Config::read(&config::config_file_path(base_path))?
    };
    config.override_with_args(&args);

    let recipe_index = cooklang_fs::new_index(base_path, config.max_depth)?
        .config_dir(COOK_DIR.to_string())
        .lazy();

    Ok(Context {
        is_collection: base_path.join(COOK_DIR).is_dir(),
        base_path: base_path.to_owned(),
        parser: OnceCell::new(),
        recipe_index,
        config,
        chef_config,
        global_args: args,
        color: color_ctx,
    })
}

const RECIPE_REF_ERROR: &str = "The name must match exactly except lower and upper case.";

impl Context {
    fn parser(&self) -> Result<&CooklangParser> {
        self.parser
            .get_or_try_init(|| configure_parser(&self.config, &self.base_path))
    }

    fn checker(
        &self,
        relative_to: Option<&Utf8Path>,
    ) -> Option<cooklang::analysis::RecipeRefCheck> {
        if self.config.recipe_ref_check {
            let relative_to = relative_to.map(|r| {
                r.to_path_buf()
                    .parent()
                    .expect("no parent for recipe entry")
                    .to_owned()
            });
            Some(Box::new(move |name: &str| {
                if self
                    .recipe_index
                    .resolve(name, relative_to.as_deref())
                    .is_ok()
                {
                    cooklang::analysis::CheckResult::Ok
                } else {
                    cooklang::analysis::CheckResult::Warning(vec![RECIPE_REF_ERROR.into()])
                }
            }) as cooklang::analysis::RecipeRefCheck)
        } else {
            None
        }
    }

    fn parse_options(&self, relative_to: Option<&Utf8Path>) -> ParseOptions {
        ParseOptions {
            recipe_ref_check: self.checker(relative_to),
            metadata_validator: Some(Box::new(metadata_validator)),
        }
    }
}

#[tracing::instrument(level = "debug", skip_all)]
fn configure_parser(config: &Config, base_path: &Utf8Path) -> Result<CooklangParser> {
    let units = config.units(base_path);
    let converter = if config.default_units || !units.is_empty() {
        let mut builder = ConverterBuilder::new();
        if config.default_units {
            builder
                .add_bundled_units()
                .expect("Failed to add bundled units");
        }
        for file in units {
            tracing::debug!("Loading units {}", file);
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot find units file: {}", file))?;
            let units = toml::from_str(&text)?;
            builder.add_units_file(units)?;
        }
        builder.finish().context("Can't build unit configuration")?
    } else {
        Converter::empty()
    };
    Ok(CooklangParser::new(config.extensions, converter))
}

use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};
use cooklang::Extensions;

use crate::{
    collection, config_cmd, convert, edit, generate_completions, list, new, recipe, shopping_list,
    units,
};

#[cfg(feature = "serve")]
use crate::serve;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    after_help = "Docs: https://github.com/Zheoni/cooklang-chef/blob/main/docs/README.md"
)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub global_args: GlobalArgs,
}

#[derive(Debug, Subcommand, strum::Display)]
pub enum Command {
    /// Read a recipe
    #[command(alias = "read", visible_alias = "r")]
    Recipe(recipe::ReadArgs),
    /// List all the recipes
    #[command(visible_alias = "ls")]
    List(list::ListArgs),
    #[cfg(feature = "serve")]
    /// Recipes web server
    Serve(serve::ServeArgs),
    /// Creates a shopping list from a given list of recipes
    #[command(visible_alias = "sl")]
    ShoppingList(shopping_list::ShoppingListArgs),
    /// List loaded units
    Units(units::UnitsArgs),
    /// Convert values to other units
    #[command(visible_alias = "c")]
    Convert(convert::ConvertArgs),
    /// See loaded configuration
    Config(config_cmd::ConfigArgs),
    /// Manage the recipe collection
    Collection(collection::CollectionArgs),
    /// Generate shell completions
    GenerateCompletions(generate_completions::GenerateCompletionsArgs),
    /// Create a new recipe
    New(new::NewArgs),
    /// Edit an existing recipe
    Edit(edit::EditArgs),
}

#[derive(Debug, Args)]
pub struct GlobalArgs {
    /// A units TOML file
    #[arg(long, action = clap::ArgAction::Append, hide_short_help = true, global = true)]
    pub units: Vec<Utf8PathBuf>,

    /// Make the `units` arg remove the other file(s)
    #[arg(long, hide_short_help = true, global = true)]
    pub override_units: bool,

    /// Do not use the bundled units
    #[arg(long, hide_short_help = true, global = true)]
    pub no_default_units: bool,

    /// Disable all extensions
    #[arg(
        long,
        alias = "no-default-extensions",
        group = "ext",
        hide_short_help = true,
        global = true
    )]
    pub no_extensions: bool,

    /// Enable all extensions
    #[arg(long, group = "ext", hide_short_help = true, global = true)]
    pub all_extensions: bool,

    /// Enables a subset of the extensions
    ///
    /// Enable only certain extensions to maximise compatibility with other
    /// cooklang parsers.
    #[arg(
        long,
        alias = "compat",
        group = "ext",
        hide_short_help = true,
        global = true
    )]
    pub compat_extensions: bool,

    /// Enable a set of extensions
    ///
    /// Can be specified multiple times.
    #[arg(
        short,
        long,
        group = "ext",
        value_parser = bitflags::parser::from_str::<Extensions>,
        action = clap::ArgAction::Append,
        global = true
    )]
    pub extensions: Vec<Extensions>,

    /// Treat warnings as errors
    #[arg(long, hide_short_help = true, global = true)]
    pub warnings_as_errors: bool,

    /// Do not display warnings generated from parsing recipes
    #[arg(
        long,
        hide_short_help = true,
        conflicts_with = "warnings_as_errors",
        global = true
    )]
    pub ignore_warnings: bool,

    #[command(flatten)]
    pub color: colorchoice_clap::Color,

    /// Change the base path
    ///
    /// This path is used to load configuration files, search for images and
    /// recipe references.
    #[arg(long, value_name = "PATH", value_hint = clap::ValueHint::DirPath, global = true)]
    pub path: Option<Utf8PathBuf>,

    /// Skip checking if referenced recipes exist
    #[arg(long, hide_short_help = true, global = true)]
    pub no_recipe_ref_check: bool,

    /// Override recipe indexing depth
    ///
    /// This is used to search for referenced recipes.
    #[arg(long, hide_short_help = true, global = true, default_value_t = 10)]
    pub max_depth: usize,

    #[arg(long, hide_short_help = true, global = true)]
    pub debug_trace: bool,
}

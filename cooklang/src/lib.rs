//! # cooklang
//!
//! This crate is a [cooklang](https://cooklang.org/) parser written in rust
//! with some extra opt-in extensions.
//!
//! The extensions form a superset of the original cooklang language and can be
//! turned off. To see a detailed list go to [extensions](_extensions).
//!
//! The parser returns rich errors with annotated code spans. For example.

mod analysis;
mod context;
pub mod convert;
pub mod error;
mod located;
pub mod metadata;
pub mod model;
pub mod parser;
pub mod quantity;
pub mod scale;
pub mod shopping_list;
mod span;

#[cfg(doc)]
pub mod _extensions {
    #![doc = include_str!("../../docs/extensions.md")]
}

use bitflags::bitflags;
use convert::Converter;
use error::{CooklangError, CooklangWarning, PassResult};
use metadata::Metadata;
pub use model::Recipe;
pub use scale::ScaledRecipe;

bitflags! {
    /// Extensions bitflags
    ///
    /// This allows to enable or disable the extensions. See [extensions](_extensions)
    /// for a detailed explanation of all of them.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct Extensions: u32 {
        const MULTINE_STEPS        = 0b000000001;
        const INGREDIENT_MODIFIERS = 0b000000010;
        const INGREDIENT_NOTE      = 0b000000100;
        const INGREDIENT_ALIAS     = 0b000001000;
        const SECTIONS             = 0b000010000;
        const ADVANCED_UNITS       = 0b000100000;
        const MODES                = 0b001000000;
        const TEMPERATURE          = 0b010000000;
        const TEXT_STEPS           = 0b100000000;

        /// Enables [Self::INGREDIENT_MODIFIERS], [Self::INGREDIENT_NOTE] and [Self::INGREDIENT_ALIAS]
        const INGREDIENT_ALL = Self::INGREDIENT_MODIFIERS.bits()
                             | Self::INGREDIENT_ALIAS.bits()
                             | Self::INGREDIENT_NOTE.bits();
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Self::all()
    }
}

#[derive(Debug, Default, Clone)]
pub struct CooklangParser {
    extensions: Extensions,
    converter: Converter,
}

#[derive(Debug, Default, Clone)]
pub struct CooklangParserBuilder {
    extensions: Extensions,
    converter: Option<Converter>,
}

impl CooklangParserBuilder {
    pub fn with_converter(mut self, converter: Converter) -> Self {
        self.set_converter(converter);
        self
    }

    pub fn set_converter(&mut self, converter: Converter) -> &mut Self {
        self.converter = Some(converter);
        self
    }

    pub fn with_extensions(mut self, extensions: Extensions) -> Self {
        self.set_extensions(extensions);
        self
    }

    pub fn set_extensions(&mut self, extensions: Extensions) -> &mut Self {
        self.extensions = extensions;
        self
    }

    pub fn finish(self) -> CooklangParser {
        let converter = self.converter.unwrap_or_default();
        CooklangParser {
            extensions: self.extensions,
            converter,
        }
    }
}

pub type RecipeResult<'a> = PassResult<Recipe<'a>, CooklangError, CooklangWarning>;
pub type MetadataResult<'a> = PassResult<Metadata<'a>, CooklangError, CooklangWarning>;

pub type RecipeRefChecker<'a> = Box<dyn Fn(&str) -> bool + 'a>;

impl CooklangParser {
    pub fn builder() -> CooklangParserBuilder {
        CooklangParserBuilder::default()
    }

    pub fn converter(&self) -> &Converter {
        &self.converter
    }

    pub fn extensions(&self) -> Extensions {
        self.extensions
    }

    pub fn parse<'a>(&self, input: &'a str, recipe_name: &str) -> RecipeResult<'a> {
        self.parse_with_recipe_ref_checker(input, recipe_name, None)
    }

    #[tracing::instrument(name = "parse", skip_all, fields(len = input.len()))]
    pub fn parse_with_recipe_ref_checker<'a>(
        &self,
        input: &'a str,
        recipe_name: &str,
        recipe_ref_checker: Option<RecipeRefChecker>,
    ) -> RecipeResult<'a> {
        let mut r = parser::parse(input, self.extensions).into_context_result();
        if r.invalid() {
            return r.discard_output();
        }
        let ast = r.take_output().unwrap();
        analysis::parse_ast(ast, self.extensions, &self.converter, recipe_ref_checker)
            .into_context_result()
            .merge(r)
            .map(|c| Recipe::from_content(recipe_name.to_string(), c))
    }

    #[tracing::instrument(name = "metadata", skip_all, fields(len = input.len()))]
    pub fn parse_metadata<'a>(&self, input: &'a str) -> MetadataResult<'a> {
        let mut r = parser::parse(input, self.extensions).into_context_result();
        if r.invalid() {
            return r.discard_output();
        }
        let ast = r.take_output().unwrap();
        analysis::parse_ast(ast, self.extensions, &self.converter, None)
            .into_context_result()
            .merge(r)
            .map(|c| c.metadata)
    }
}

pub fn parse<'a>(
    input: &'a str,
    recipe_name: &str,
) -> PassResult<Recipe<'a>, CooklangError, CooklangWarning> {
    CooklangParser::default().parse(input, recipe_name)
}

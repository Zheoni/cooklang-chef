pub mod analysis;
mod context;
pub mod convert;
pub mod error;
pub mod helper;
mod located;
pub mod metadata;
pub mod model;
pub mod parser;
pub mod quantity;
pub mod scale;

use bitflags::bitflags;
use convert::Converter;
use error::{CookResult, CooklangWarning};
pub use model::Recipe;
pub use scale::ScaledRecipe;

bitflags! {
    pub struct Extensions: u32 {
        const MULTINE_STEPS        = 0b00000001;
        const INGREDIENT_MODIFIERS = 0b00000010;
        const INGREDIENT_NOTE      = 0b00000100;
        const INGREDIENT_ALIAS     = 0b00001000;
        const SECTIONS             = 0b00010000;
        const ADVANCED_UNITS       = 0b00100000;
        const MODES                = 0b01000000;
        const TEMPERATURE          = 0b10000000;

        const INGREDIENT_ALL = Self::INGREDIENT_MODIFIERS.bits
                             | Self::INGREDIENT_ALIAS.bits
                             | Self::INGREDIENT_NOTE.bits;
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Self::all()
    }
}

#[derive(Clone, Debug, Default)]
pub struct CooklangParser {
    extensions: Extensions,
    warnings_as_errors: bool,
    converter: Converter,
}

#[derive(Debug, Default, Clone)]
pub struct CooklangParserBuilder {
    extensions: Extensions,
    warnings_as_errors: bool,
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

    pub fn warnings_as_errors(mut self, as_err: bool) -> Self {
        self.set_warnings_as_errors(as_err);
        self
    }

    pub fn set_warnings_as_errors(&mut self, as_err: bool) -> &mut Self {
        self.warnings_as_errors = as_err;
        self
    }

    pub fn finish(self) -> CooklangParser {
        let converter = self.converter.unwrap_or_default();
        CooklangParser {
            extensions: self.extensions,
            warnings_as_errors: self.warnings_as_errors,
            converter,
        }
    }
}

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

    #[tracing::instrument(skip_all, fields(len = input.len()))]
    pub fn parse<'a>(
        &self,
        input: &'a str,
        recipe_name: &str,
    ) -> CookResult<(Recipe<'a>, Vec<CooklangWarning>)> {
        let mut warn = Vec::new();

        let (ast, w) = parser::parse(input, self.extensions, self.warnings_as_errors)?;
        warn.extend(w.into_iter().map(CooklangWarning::from));
        let (content, w) = analysis::analyze_ast(
            input,
            ast,
            self.extensions,
            &self.converter,
            self.warnings_as_errors,
        )?;
        warn.extend(w.into_iter().map(CooklangWarning::from));
        Ok((Recipe::from_content(recipe_name.to_string(), content), warn))
    }
}

pub fn parse<'a>(
    input: &'a str,
    recipe_name: &str,
) -> CookResult<(Recipe<'a>, Vec<CooklangWarning>)> {
    CooklangParser::default().parse(input, recipe_name)
}

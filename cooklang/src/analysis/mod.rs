use std::{borrow::Borrow, ops::Range};

use miette::Diagnostic;
use thiserror::Error;

use crate::{context::Context, convert::Converter, metadata::MetadataError, Extensions};

mod ast_walker;

pub use ast_walker::RecipeContent;

pub fn analyze_ast<'a>(
    input: &str,
    ast: crate::parser::ast::Ast<'a>,
    extensions: Extensions,
    converter: Option<&Converter>,
    warnings_as_errors: bool,
) -> Result<(RecipeContent<'a>, Vec<AnalysisWarning>), AnalysisReport> {
    let (content, context) = ast_walker::parse_ast(ast, extensions, converter);

    let Context { errors, warnings } = context;

    if !errors.is_empty() || warnings_as_errors && !warnings.is_empty() {
        return Err(AnalysisReport {
            input: input.to_string(),
            errors,
            warnings,
        });
    }

    Ok((content, warnings))
}

#[derive(Debug, Error, Diagnostic)]
pub enum AnalysisError {
    #[error("Invalid value for '{key}': {value}")]
    #[diagnostic(
        code(cooklang::analysis::invalid_special_key_value),
        help("Possible values are: {possible_values:?}")
    )]
    InvalidSpecialMetadataValue {
        key: String,
        value: String,
        #[label("this key")]
        key_span: Range<usize>,
        #[label("does not support this value")]
        value_span: Range<usize>,

        possible_values: Vec<&'static str>,
    },
    #[error("Reference not found: {name}")]
    #[diagnostic(
        code(cooklang::analysis::reference_not_found),
        help("A non reference ingredient with the same name defined before cannot be found")
    )]
    ReferenceNotFound {
        name: String,
        #[label]
        reference_span: Range<usize>,
    },
    #[error("Conflicting ingredient reference quantities: {ingredient_name}")]
    #[diagnostic(
        code(cooklang::analysis::conflicting_reference_quantities),
        help("Only the ingredient definition *OR* the reference(s) can have a quantity")
    )]
    ConflictingReferenceQuantities {
        ingredient_name: String,
        #[label("defined here")]
        definition_span: Range<usize>,
        #[label("referenced here")]
        reference_span: Range<usize>,
    },

    #[error("Unknown timer unit: {unit}")]
    #[diagnostic(
        code(cooklang::analysis::unknown_timer_unit),
        help("With the ADVANCED_UNITS extensions, timers are required to have a time unit.")
    )]
    UnknownTimerUnit {
        unit: String,
        #[label]
        timer_span: Range<usize>,
    },

    #[error("Bad timer unit. Expecting time, got: {}", .unit.physical_quantity)]
    #[diagnostic(code(cooklang::analysis::bad_timer_unit))]
    BadTimerUnit {
        unit: crate::convert::Unit,
        #[label]
        timer_span: Range<usize>,
    },
}

#[derive(Debug, Error, Diagnostic)]
#[diagnostic(severity(warning))]
pub enum AnalysisWarning {
    #[error("Ignoring unknown special metadata key: {key}")]
    #[diagnostic(help("Possible values are 'duplicate' and 'reference'"))]
    UnknownSpecialMetadataKey {
        key: String,
        #[label]
        key_span: Range<usize>,
    },

    #[error("Ingoring text in define ingredients mode")]
    TextDefiningIngredients {
        #[label]
        text_span: Range<usize>,
    },

    #[error("Text value in reference prevents calculating total amount")]
    TextValueInReference {
        #[label]
        quantity_span: Range<usize>,
    },

    #[error("Invalid value for key: {key}. Treating it as a regular metadata key.")]
    #[diagnostic(help("Rich information for this metadata will not be available"))]
    InvalidMetadataValue {
        key: String,
        value: String,

        #[label("this key")]
        key_span: Range<usize>,
        #[label("does not understand this value")]
        value_span: Range<usize>,

        #[source]
        #[diagnostic_source]
        source: MetadataError,
    },

    #[error("Component found in text mode")]
    ComponentInTextMode {
        #[label("this will be ignored")]
        component_span: Range<usize>,
    },

    #[error("An error ocurred searching temperature values")]
    #[diagnostic(help("Check the temperature symbols defined in the units.toml file"))]
    TemperatureRegexCompile {
        #[source]
        source: regex::Error,
    },
}

#[derive(Debug, Error)]
#[error("Parse analysis did not finish successfully")]
pub struct AnalysisReport {
    input: String,
    errors: Vec<AnalysisError>,
    warnings: Vec<AnalysisWarning>,
}

impl miette::Diagnostic for AnalysisReport {
    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.input)
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        let related = self
            .warnings
            .iter()
            .map(|x| -> &(dyn miette::Diagnostic) { x.borrow() })
            .chain(
                self.errors
                    .iter()
                    .map(|x| -> &(dyn miette::Diagnostic) { x.borrow() }),
            );

        Some(Box::new(related))
    }

    fn severity(&self) -> Option<miette::Severity> {
        if !self.errors.is_empty() {
            Some(miette::Severity::Error)
        } else if !self.warnings.is_empty() {
            Some(miette::Severity::Warning)
        } else {
            None
        }
    }
}

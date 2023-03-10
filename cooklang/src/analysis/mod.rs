use std::{borrow::Cow, ops::Range};

use thiserror::Error;

use crate::{
    context::Context, convert::Converter, error::RichError, located::Located,
    metadata::MetadataError, Extensions,
};

mod ast_walker;

pub use ast_walker::RecipeContent;

#[tracing::instrument(skip_all, fields(ast_lines = ast.lines.len()))]
pub fn analyze_ast<'a>(
    ast: crate::parser::ast::Ast<'a>,
    extensions: Extensions,
    converter: &Converter,
    warnings_as_errors: bool,
) -> Result<(RecipeContent<'a>, Vec<AnalysisWarning>), AnalysisReport> {
    let (content, context) = ast_walker::parse_ast(ast, extensions, converter);

    let Context { errors, warnings } = context;

    if !errors.is_empty() || warnings_as_errors && !warnings.is_empty() {
        return Err(AnalysisReport::new(errors, warnings));
    }

    Ok((content, warnings))
}

#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("Invalid value for '{key}': {value}")]
    InvalidSpecialMetadataValue {
        key: Located<String>,
        value: Located<String>,
        possible_values: Vec<&'static str>,
    },
    #[error("Reference not found: {name}")]
    ReferenceNotFound {
        name: String,
        reference_span: Range<usize>,
    },
    #[error("Conflicting ingredient reference quantities: {ingredient_name}")]
    ConflictingReferenceQuantities {
        ingredient_name: String,
        definition_span: Range<usize>,
        reference_span: Range<usize>,
    },

    #[error("Unknown timer unit: {unit}")]
    UnknownTimerUnit {
        unit: String,
        timer_span: Range<usize>,
    },

    #[error("Bad timer unit. Expecting time, got: {}", .unit.physical_quantity)]
    BadTimerUnit {
        unit: crate::convert::Unit,
        timer_span: Range<usize>,
    },

    #[error("Quantity scaling error: {reason}")]
    SacalingConflict {
        reason: Cow<'static, str>,
        value_span: Range<usize>,
    },
}

#[derive(Debug, Error)]
pub enum AnalysisWarning {
    #[error("Ignoring unknown special metadata key: {key}")]
    UnknownSpecialMetadataKey { key: String, key_span: Range<usize> },

    #[error("Ingoring text in define ingredients mode")]
    TextDefiningIngredients { text_span: Range<usize> },

    #[error("Text value in reference prevents calculating total amount")]
    TextValueInReference { quantity_span: Range<usize> },

    #[error("Incompatible units in reference prevents calculating total amount")]
    IncompatibleUnits {
        a: Range<usize>,
        b: Range<usize>,

        #[source]
        source: crate::quantity::IncompatibleUnits,
    },

    #[error("Invalid value for key: {key}. Treating it as a regular metadata key.")]
    InvalidMetadataValue {
        key: String,
        value: String,

        key_span: Range<usize>,
        value_span: Range<usize>,

        #[source]
        source: MetadataError,
    },

    #[error("Component found in text mode")]
    ComponentInTextMode { component_span: Range<usize> },

    #[error("An error ocurred searching temperature values")]
    TemperatureRegexCompile {
        #[source]
        source: regex::Error,
    },

    #[error("Redundant auto scale marker")]
    RedundantAutoScaleMarker { quantity_span: Range<usize> },
}

impl RichError for AnalysisError {
    fn labels(&self) -> Vec<(Range<usize>, Option<Cow<'static, str>>)> {
        use crate::error::label;
        match self {
            AnalysisError::InvalidSpecialMetadataValue { key, value, .. } => vec![
                label!(key, "this key"),
                label!(value, "does not support this value"),
            ],
            AnalysisError::ReferenceNotFound { reference_span, .. } => vec![label!(reference_span)],
            AnalysisError::ConflictingReferenceQuantities {
                definition_span,
                reference_span,
                ..
            } => vec![
                label!(definition_span, "defined outside step here"),
                label!(reference_span, "referenced here"),
            ],
            AnalysisError::UnknownTimerUnit { timer_span, .. } => vec![label!(timer_span)],
            AnalysisError::BadTimerUnit { timer_span, .. } => vec![label!(timer_span)],
            AnalysisError::SacalingConflict { value_span, .. } => vec![label!(value_span)],
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        use crate::error::help;
        match self {
            AnalysisError::InvalidSpecialMetadataValue {
                possible_values, ..
            } => help!(format!("Possible values are: {possible_values:?}")),
            AnalysisError::ReferenceNotFound { .. } => help!(
                "A non reference ingredient with the same name defined before cannot be found"
            ),
            AnalysisError::ConflictingReferenceQuantities { .. } => help!(
                "If the ingredient is not defined in a step, its references cannot have a quantity"
            ),
            AnalysisError::UnknownTimerUnit { .. } => {
                help!("With the ADVANCED_UNITS extensions, timers are required to have a time unit")
            }
            AnalysisError::BadTimerUnit { .. } => None,
            AnalysisError::SacalingConflict { .. } => None,
        }
    }

    fn code(&self) -> Option<&'static str> {
        Some("analysis")
    }
}

impl RichError for AnalysisWarning {
    fn labels(&self) -> Vec<(Range<usize>, Option<Cow<'static, str>>)> {
        use crate::error::label;
        match self {
            AnalysisWarning::UnknownSpecialMetadataKey { key_span, .. } => vec![label!(key_span)],
            AnalysisWarning::TextDefiningIngredients { text_span } => vec![label!(text_span)],
            AnalysisWarning::TextValueInReference { quantity_span } => vec![label!(quantity_span)],
            AnalysisWarning::IncompatibleUnits { a, b, .. } => {
                println!("{a:?} -- {b:?}");
                vec![label!(a), label!(b)]
            }
            AnalysisWarning::InvalidMetadataValue {
                key_span,
                value_span,
                ..
            } => vec![
                label!(key_span, "this key"),
                label!(value_span, "does not understand this value"),
            ],
            AnalysisWarning::ComponentInTextMode { component_span } => {
                vec![label!(component_span, "this will be ignored")]
            }
            AnalysisWarning::TemperatureRegexCompile { .. } => vec![],
            AnalysisWarning::RedundantAutoScaleMarker { quantity_span } => {
                vec![label!(quantity_span)]
            }
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        use crate::error::help;
        match self {
            AnalysisWarning::UnknownSpecialMetadataKey { .. } => {
                help!("Possible values are 'duplicate' and 'reference'")
            }
            AnalysisWarning::InvalidMetadataValue { .. } => {
                help!("Rich information for this metadata will not be available")
            }
            AnalysisWarning::TemperatureRegexCompile { .. } => {
                help!("Check the temperature symbols defined in the units.toml file")
            }
            AnalysisWarning::RedundantAutoScaleMarker { .. } => {
                help!("Be careful as every ingredient is already marked to auto scale")
            }
            _ => None,
        }
    }

    fn code(&self) -> Option<&'static str> {
        Some("analysis")
    }

    fn kind(&self) -> ariadne::ReportKind {
        ariadne::ReportKind::Warning
    }
}

pub type AnalysisReport = crate::error::Report<AnalysisError, AnalysisWarning>;

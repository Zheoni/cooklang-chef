use std::borrow::Cow;

use thiserror::Error;

use crate::error::PassResult;
use crate::span::Span;
use crate::{error::RichError, located::Located, metadata::MetadataError};

mod ast_walker;

pub use ast_walker::parse_ast;
pub use ast_walker::RecipeContent;

pub type AnalysisResult<'a> = PassResult<RecipeContent<'a>, AnalysisError, AnalysisWarning>;

#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("Invalid value for '{key}': {value}")]
    InvalidSpecialMetadataValue {
        key: Located<String>,
        value: Located<String>,
        possible_values: Vec<&'static str>,
    },
    #[error("Reference not found: {name}")]
    ReferenceNotFound { name: String, reference_span: Span },
    #[error("Conflicting ingredient reference quantities: {ingredient_name}")]
    ConflictingReferenceQuantities {
        ingredient_name: String,
        definition_span: Span,
        reference_span: Span,
    },

    #[error("Unknown timer unit: {unit}")]
    UnknownTimerUnit { unit: String, timer_span: Span },

    #[error("Bad timer unit. Expecting time, got: {}", .unit.physical_quantity)]
    BadTimerUnit {
        unit: crate::convert::Unit,
        timer_span: Span,
    },

    #[error("Quantity scaling error: {reason}")]
    ScalableValueManyConflict {
        reason: Cow<'static, str>,
        value_span: Span,
        servings_meta_span: Option<Span>,
    },
}

#[derive(Debug, Error)]
pub enum AnalysisWarning {
    #[error("Ignoring unknown special metadata key: {key}")]
    UnknownSpecialMetadataKey { key: Located<String> },

    #[error("Ingoring text in define ingredients mode")]
    TextDefiningIngredients { text_span: Span },

    #[error("Text value in reference prevents calculating total amount")]
    TextValueInReference { quantity_span: Span },

    #[error("Incompatible units in reference prevent calculating total amount")]
    IncompatibleUnits {
        a: Span,
        b: Span,

        #[source]
        source: crate::quantity::IncompatibleUnits,
    },

    #[error("Invalid value for key: {key}. Treating it as a regular metadata key.")]
    InvalidMetadataValue {
        key: Located<String>,
        value: Located<String>,

        #[source]
        source: MetadataError,
    },

    #[error("Component found in text mode")]
    ComponentInTextMode { component_span: Span },

    #[error("An error ocurred searching temperature values")]
    TemperatureRegexCompile {
        #[source]
        source: regex::Error,
    },

    #[error("Redundant auto scale marker")]
    RedundantAutoScaleMarker { quantity_span: Span },

    #[error("Redundant reference (&) modifier")]
    RedundantReferenceModifier {
        modifiers: Located<crate::ast::Modifiers>,
    },

    #[error("Referenced recipe not found: '{name}'")]
    RecipeNotFound { ref_span: Span, name: String },

    #[error("Reference ingredient to a referenced recipe is missing recipe modifier")]
    ReferenceToRecipeMissing {
        ingredient_span: Span,
        referenced_span: Span,
        modifiers: Located<crate::ast::Modifiers>,
    },
}

impl RichError for AnalysisError {
    fn labels(&self) -> Vec<(Span, Option<Cow<'static, str>>)> {
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
            AnalysisError::ScalableValueManyConflict {
                value_span,
                servings_meta_span,
                ..
            } => {
                if let Some(servings) = servings_meta_span {
                    vec![
                        label!(servings, "servings defined here"),
                        label!(value_span, "do not match number of values"),
                    ]
                } else {
                    vec![label!(value_span)]
                }
            }
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
                "If the ingredient is not defined in a step and has a quantity, its references cannot have a quantity"
            ),
            AnalysisError::UnknownTimerUnit { .. } => {
                help!("With the ADVANCED_UNITS extensions, timers are required to have a time unit")
            }
            AnalysisError::BadTimerUnit { .. } => None,
            AnalysisError::ScalableValueManyConflict { .. } => None,
        }
    }

    fn code(&self) -> Option<&'static str> {
        Some("analysis")
    }
}

impl RichError for AnalysisWarning {
    fn labels(&self) -> Vec<(Span, Option<Cow<'static, str>>)> {
        use crate::error::label;
        match self {
            AnalysisWarning::UnknownSpecialMetadataKey { key } => vec![label!(key)],
            AnalysisWarning::TextDefiningIngredients { text_span } => vec![label!(text_span)],
            AnalysisWarning::TextValueInReference { quantity_span } => vec![label!(quantity_span)],
            AnalysisWarning::IncompatibleUnits { a, b, source } => {
                match source {
                    crate::quantity::IncompatibleUnits::MissingUnit { found } => {
                        let m = "this is missing a unit";
                        let f = "matching this one";
                        match found {
                            either::Either::Left(_) => vec![label!(b, m), label!(a, f)],
                            either::Either::Right(_) => vec![label!(a, m), label!(b, f)],
                        }
                    }
                    crate::quantity::IncompatibleUnits::DifferentPhysicalQuantities {
                        a: a_q,
                        b: b_q,
                    } => {
                        vec![label!(b, b_q.to_string()), label!(a, a_q.to_string())]
                    }
                    crate::quantity::IncompatibleUnits::UnknownDifferentUnits { .. } => {
                        vec![label!(a, "this unit"), label!(b, "differs from this")]
                    }
                }
                // vec![label!(a), label!(b)]
            }
            AnalysisWarning::InvalidMetadataValue { key, value, .. } => vec![
                label!(key, "this key"),
                label!(value, "does not understand this value"),
            ],
            AnalysisWarning::ComponentInTextMode { component_span } => {
                vec![label!(component_span, "this will be ignored")]
            }
            AnalysisWarning::TemperatureRegexCompile { .. } => vec![],
            AnalysisWarning::RedundantAutoScaleMarker { quantity_span } => {
                vec![label!(quantity_span)]
            }
            AnalysisWarning::RedundantReferenceModifier { modifiers } => {
                vec![label!(modifiers)]
            }
            AnalysisWarning::RecipeNotFound { ref_span, .. } => vec![label!(ref_span)],
            AnalysisWarning::ReferenceToRecipeMissing {
                ingredient_span,
                referenced_span,
                modifiers,
            } => {
                vec![
                    label!(referenced_span, "this ingredient recipe"),
                    label!(ingredient_span, "referenced here"),
                    label!(modifiers, "add '@' here"),
                ]
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
            AnalysisWarning::RedundantReferenceModifier { .. } => {
                help!("Be careful as every ingredient is already marked to be a reference")
            }
            AnalysisWarning::RecipeNotFound { name, .. } => {
                if name.chars().any(std::path::is_separator) {
                    help!("This is treated as a path relative to the base directory")
                } else {
                    help!("Names must match exactly except for upper and lower case")
                }
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

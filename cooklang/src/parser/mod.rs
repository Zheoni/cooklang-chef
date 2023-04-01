mod metadata;
mod parser;
mod quantity;
mod section;
mod step;
mod token_stream;

use std::borrow::Cow;

pub use parser::parse;
use thiserror::Error;

use crate::{error::RichError, located::Located, span::Span};

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Error parsing input: {message}")]
    Parse { span: Span, message: String },

    #[error("A {container} is missing: {what}")]
    ComponentPartMissing {
        container: &'static str,
        what: &'static str,
        expected_pos: Span,
    },

    #[error("A {container} cannot have: {what}")]
    ComponentPartNotAllowed {
        container: &'static str,
        what: &'static str,
        to_remove: Span,
        help: Option<&'static str>,
    },

    #[error("Invalid {container} {what}: {reason}")]
    ComponentPartInvalid {
        container: &'static str,
        what: &'static str,
        reason: &'static str,
        labels: Vec<(Span, Option<Cow<'static, str>>)>,
        help: Option<&'static str>,
    },

    #[error("Tried to use a disabled extension: {extension_name}")]
    ExtensionNotEnabled {
        span: Span,
        extension_name: &'static str,
    },

    #[error("Invalid ingredient modifiers: {reason}")]
    InvalidModifiers {
        modifiers_span: Span,
        reason: Cow<'static, str>,
        help: Option<&'static str>,
    },

    #[error("Error parsing integer number")]
    ParseInt {
        bad_bit: Span,
        source: std::num::ParseIntError,
    },

    #[error("Error parsing decimal number")]
    ParseFloat {
        bad_bit: Span,
        source: std::num::ParseFloatError,
    },

    #[error("Division by zero")]
    DivisionByZero { bad_bit: Span },

    #[error("Quantity scaling conflict")]
    QuantityScalingConflict { bad_bit: Span },
}

#[derive(Debug, Error)]
pub enum ParserWarning {
    #[error("Empty metadata value for key: {key}")]
    EmptyMetadataValue { key: Located<String> },
    #[error("A {container} cannot have {what}, it will be ignored")]
    ComponentPartIgnored {
        container: &'static str,
        what: &'static str,
        ignored: Span,
        help: Option<&'static str>,
    },
}

impl RichError for ParserError {
    fn labels(&self) -> Vec<(Span, Option<Cow<'static, str>>)> {
        use crate::error::label;
        match self {
            ParserError::Parse { span, .. } => vec![label!(span)],
            ParserError::ComponentPartMissing {
                expected_pos: component_span,
                what,
                ..
            } => {
                vec![label!(component_span, format!("expected {what}"))]
            }
            ParserError::ComponentPartNotAllowed { to_remove, .. } => {
                vec![label!(to_remove, "remove this")]
            }
            ParserError::ComponentPartInvalid { labels, .. } => labels.clone(),
            ParserError::ExtensionNotEnabled { span, .. } => vec![label!(span, "used here")],
            ParserError::InvalidModifiers { modifiers_span, .. } => vec![label!(modifiers_span)],
            ParserError::ParseInt { bad_bit, .. } => vec![label!(bad_bit)],
            ParserError::ParseFloat { bad_bit, .. } => vec![label!(bad_bit)],
            ParserError::DivisionByZero { bad_bit } => vec![label!(bad_bit)],
            ParserError::QuantityScalingConflict { bad_bit } => vec![label!(bad_bit)],
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        use crate::error::help;
        match self {
            ParserError::ComponentPartNotAllowed { help, .. } => help!(opt help),
            ParserError::ExtensionNotEnabled { extension_name, .. } => {
                help!(format!("Remove the usage or enable the {extension_name} extension"))
            }
            ParserError::InvalidModifiers { help, .. } => help!(opt help),
            ParserError::DivisionByZero { .. } => {
                help!("Change this please, we don't want an infinite amount of anything")
            }
            ParserError::QuantityScalingConflict { .. } => help!("A quantity cannot have the auto scaling marker (*) and have fixed values at the same time"),
            _ => None,
        }
    }

    fn code(&self) -> Option<&'static str> {
        Some("parser")
    }
}

impl RichError for ParserWarning {
    fn labels(&self) -> Vec<(Span, Option<Cow<'static, str>>)> {
        use crate::error::label;
        match self {
            ParserWarning::EmptyMetadataValue { key } => {
                vec![label!(key)]
            }
            ParserWarning::ComponentPartIgnored { ignored, .. } => {
                vec![label!(ignored, "this is ignored")]
            }
        }
    }

    fn code(&self) -> Option<&'static str> {
        Some("parser")
    }

    fn kind(&self) -> ariadne::ReportKind {
        ariadne::ReportKind::Warning
    }
}

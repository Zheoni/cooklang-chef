use std::borrow::Cow;
use std::ops::Range;

use pest::Parser;
use pest_derive::Parser;
use thiserror::Error;

use self::pest_ext::Span;
use crate::error::{PassResult, RichError};
use crate::Extensions;

pub(crate) mod ast;
mod pairs_walker;
mod pest_ext;

#[tracing::instrument(skip_all, fields(len = input.len()))]
pub fn parse(input: &str, extensions: Extensions) -> ParserResult {
    let pairs = match CooklangParser::parse(Rule::cooklang, input) {
        Ok(pairs) => pairs,
        Err(e) => {
            return ParserError::Parse {
                span: e.location.span(),
                message: e.variant.message().to_string(),
            }
            .into()
        }
    };

    pairs_walker::build_ast(pairs, extensions)
}

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
struct CooklangParser;

type Pair<'a> = pest::iterators::Pair<'a, Rule>;
type Pairs<'a> = pest::iterators::Pairs<'a, Rule>;

pub type ParserResult<'a> = PassResult<ast::Ast<'a>, ParserError, ParserWarning>;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Error parsing input: {message}")]
    Parse { span: Range<usize>, message: String },

    #[error("A {container} is missing: {what}")]
    ComponentPartMissing {
        container: &'static str,
        what: &'static str,
        component_span: Range<usize>,
    },

    #[error("A {container} cannot have: {what}")]
    ComponentPartNotAllowed {
        container: &'static str,
        what: &'static str,
        to_remove: Range<usize>,
        help: Option<&'static str>,
    },

    #[error("Tried to use a disabled extension: {extension_name}")]
    ExtensionNotEnabled {
        span: Range<usize>,
        extension_name: &'static str,
    },

    #[error("Invalid ingredient modifiers: {reason}")]
    InvalidModifiers {
        modifiers_span: Range<usize>,
        reason: Cow<'static, str>,
        help: Option<&'static str>,
    },

    #[error("Error parsing integer number")]
    ParseInt {
        bad_bit: Range<usize>,
        source: std::num::ParseIntError,
    },

    #[error("Error parsing decimal number")]
    ParseFloat {
        bad_bit: Range<usize>,
        source: std::num::ParseFloatError,
    },

    #[error("Division by zero")]
    DivisionByZero { bad_bit: Range<usize> },

    #[error("Quantity scaling conflict")]
    QuantityScalingConflict { bad_bit: Range<usize> },
}

#[derive(Debug, Error)]
pub enum ParserWarning {
    #[error("Empty metadata value for key: {key}")]
    EmptyMetadataValue { key: String, position: usize },
}

impl RichError for ParserError {
    fn labels(&self) -> Vec<(Range<usize>, Option<Cow<'static, str>>)> {
        use crate::error::label;
        match self {
            ParserError::Parse { span, .. } => vec![label!(span)],
            ParserError::ComponentPartMissing {
                component_span,
                what,
                ..
            } => {
                vec![label!(component_span, format!("expected {what}"))]
            }
            ParserError::ComponentPartNotAllowed { to_remove, .. } => {
                vec![label!(to_remove, "remove this")]
            }
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
    fn labels(&self) -> Vec<(Range<usize>, Option<Cow<'static, str>>)> {
        use crate::error::label;
        match self {
            ParserWarning::EmptyMetadataValue { position, .. } => {
                vec![label!(*position..*position + 1)]
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

pub type ParserReport = crate::error::Report<ParserError, ParserWarning>;

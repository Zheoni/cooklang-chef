use std::borrow::{Borrow, Cow};
use std::ops::Range;

use miette::Diagnostic;
use pest::Parser;
use pest_derive::Parser;
use strum::IntoStaticStr;
use thiserror::Error;

use self::pest_ext::Span;
use crate::Extensions;

pub(crate) mod ast;
pub(crate) mod located;
mod pairs_walker;
mod pest_ext;

#[tracing::instrument(skip_all, fields(len = input.len()))]
pub fn parse(
    input: &str,
    extensions: Extensions,
    warnings_as_errors: bool,
) -> Result<(ast::Ast, Vec<ParserWarning>), ParserReport> {
    let pairs = CooklangParser::parse(Rule::cooklang, input).map_err(|e| {
        ParserReport::from_err(
            ParserError::Parse {
                span: e.location.span(),
                message: e.variant.message().to_string(),
            },
            input,
        )
    })?;

    let (ast, errors, warnings) = pairs_walker::build_ast(pairs, extensions);

    if !errors.is_empty() || warnings_as_errors && !warnings.is_empty() {
        return Err(ParserReport::new(input, errors, warnings));
    }

    Ok((ast, warnings))
}

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
struct CooklangParser;

type Pair<'a> = pest::iterators::Pair<'a, Rule>;
type Pairs<'a> = pest::iterators::Pairs<'a, Rule>;

#[derive(IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
enum ComponentKind {
    Ingredient,
    Cookware,
    Timer,
}

#[derive(Debug, Error, Diagnostic)]
pub enum ParserError {
    #[error("Error parsing input: {message}")]
    Parse {
        #[label]
        span: Range<usize>,
        message: String,
    },

    #[error("A {component_kind} is missing: {what}")]
    #[diagnostic(code(cooklang::parser::componnet_part_missing))]
    ComponentPartMissing {
        component_kind: &'static str,
        what: &'static str,
        #[label]
        component_span: Range<usize>,
    },

    #[error("A {component_kind} cannot have: {what}")]
    #[diagnostic(code(cooklang::parser::component_part_not_allowed))]
    ComponentPartNotAllowed {
        component_kind: &'static str,
        what: &'static str,
        #[label("remove this")]
        to_remove: Range<usize>,
        #[help]
        help: Option<&'static str>,
    },

    #[error("Tried to use a disabled extension: {extension_name}")]
    #[diagnostic(
        code(cooklang::parser::extension_not_enabled),
        help("Remove the usage or enable the {extension_name} extension")
    )]
    ExtensionNotEnabled {
        #[label("used here")]
        span: Range<usize>,
        extension_name: &'static str,
    },

    #[error("Invalid ingredient modifiers: {reason}")]
    #[diagnostic(code(cooklang::parser::invalid_modifiers))]
    InvalidModifiers {
        modifiers_span: Range<usize>,
        reason: Cow<'static, str>,
        #[help]
        help: Option<&'static str>,
    },

    #[error("Error parsing integer number")]
    #[diagnostic(code(cooklang::parser::parse_int))]
    ParseInt {
        #[label]
        bad_bit: Range<usize>,
        source: std::num::ParseIntError,
    },

    #[error("Error parsing decimal number")]
    #[diagnostic(code(cooklang::parser::parse_float))]
    ParseFloat {
        #[label]
        bad_bit: Range<usize>,
        source: std::num::ParseFloatError,
    },

    #[error("Division by zero")]
    #[diagnostic(
        code(cooklang::parser::division_by_zero),
        help("Change this please, we don't want an infinite amount of anything")
    )]
    DivisionByZero {
        #[label("this has to be a positive integer")]
        bad_bit: Range<usize>,
    },

    #[error("Quantity scaling conflict")]
    #[diagnostic(code(cooklang::parser::unit_scaling), help("A quantity cannot have the auto scaling marker (*) and have fixed values at the same time"))]
    QuantityScalingConflict {
        #[label]
        bad_bit: Range<usize>,
    },
}

#[derive(Debug, Error, Diagnostic)]
#[diagnostic(severity(warning))]
pub enum ParserWarning {
    #[error("Empty metadata value for key: {key}")]
    #[diagnostic(code(cooklang::parser::empty_metadata_value))]
    EmptyMetadataValue {
        key: String,
        #[label]
        position: usize,
    },
}

#[derive(Debug, Error)]
#[error("Parser did not finish successfully")]
pub struct ParserReport {
    input: String,
    errors: Vec<ParserError>,
    warnings: Vec<ParserWarning>,
}

impl ParserReport {
    fn new(input: &str, errors: Vec<ParserError>, warnings: Vec<ParserWarning>) -> Self {
        Self {
            input: input.to_string(),
            errors,
            warnings,
        }
    }

    fn from_err(err: ParserError, input: &str) -> Self {
        Self {
            input: input.to_string(),
            errors: vec![err],
            warnings: vec![],
        }
    }
}

impl miette::Diagnostic for ParserReport {
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

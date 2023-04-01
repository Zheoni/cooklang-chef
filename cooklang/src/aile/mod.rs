use std::collections::HashMap;

use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{error::RichError, span::Span};

#[derive(Parser)]
#[grammar = "aile/grammar.pest"]
struct AileConfParser;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AileConf<'a> {
    #[serde(borrow)]
    pub categories: Vec<Category<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category<'a> {
    #[serde(borrow)]
    pub name: &'a str,
    pub ingredients: Vec<Ingredient<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ingredient<'a> {
    #[serde(borrow)]
    pub names: Vec<&'a str>,
}

pub fn parse(input: &str) -> Result<AileConf, AileConfError> {
    let pairs =
        AileConfParser::parse(Rule::shopping_list, input).map_err(|e| AileConfError::Parse {
            span: e.location.into(),
            message: e.variant.message().to_string(),
        })?;

    let mut categories = Vec::new();
    let mut categories_span = HashMap::new();
    let mut names_span = HashMap::new();

    for p in pairs.take_while(|p| p.as_rule() != Rule::EOI) {
        let mut pairs = p.into_inner();
        let name_pair = pairs.next().expect("name");
        let name = name_pair.as_str().trim();
        let current_span = Span::from(name_pair.as_span());

        if let Some(other) = categories_span.insert(name, current_span) {
            return Err(AileConfError::DuplicateCategory {
                name: name.to_string(),
                first_span: other,
                second_span: current_span,
            });
        }

        let mut ingredients = Vec::new();
        for p in pairs {
            assert_eq!(p.as_rule(), Rule::ingredient, "expected ingredient");
            let mut names = Vec::with_capacity(1);
            for p in p.into_inner() {
                assert_eq!(p.as_rule(), Rule::name, "expected name");
                let name = p.as_str().trim();
                let span = Span::from(p.as_span());
                if let Some(other) = names_span.insert(name, span) {
                    return Err(AileConfError::DuplicateIngredient {
                        name: name.to_string(),
                        first_span: other,
                        second_span: span,
                    });
                }
                names.push(name);
            }
            ingredients.push(Ingredient { names });
        }
        let category = Category { name, ingredients };

        categories.push(category);
    }

    Ok(AileConf { categories })
}

pub fn write(conf: &AileConf, mut write: impl std::io::Write) -> std::io::Result<()> {
    let w = &mut write;
    for category in &conf.categories {
        writeln!(w, "[{}]", category.name)?;
        for ingredient in &category.ingredients {
            if !ingredient.names.is_empty() {
                let mut iter = ingredient.names.iter();
                write!(w, "{}", iter.next().unwrap())?;
                for name in iter {
                    write!(w, "|{}", name)?;
                }
                writeln!(w)?
            }
        }
        writeln!(w)?;
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum AileConfError {
    #[error("Error parsing input: {message}")]
    Parse { span: Span, message: String },
    #[error("Duplicate category: '{name}'")]
    DuplicateCategory {
        name: String,
        first_span: Span,
        second_span: Span,
    },
    #[error("Duplicate ingredient: '{name}'")]
    DuplicateIngredient {
        name: String,
        first_span: Span,
        second_span: Span,
    },
}

impl RichError for AileConfError {
    fn labels(&self) -> Vec<(Span<()>, Option<std::borrow::Cow<'static, str>>)> {
        use crate::error::label;
        match self {
            AileConfError::Parse { span, .. } => vec![label!(span)],
            AileConfError::DuplicateCategory {
                first_span,
                second_span,
                ..
            } => vec![
                label!(first_span, "first defined here"),
                label!(second_span, "then here"),
            ],
            AileConfError::DuplicateIngredient {
                first_span,
                second_span,
                ..
            } => vec![
                label!(first_span, "first defined here"),
                label!(second_span, "then here"),
            ],
        }
    }

    fn code(&self) -> Option<&'static str> {
        Some("shopping list")
    }
}

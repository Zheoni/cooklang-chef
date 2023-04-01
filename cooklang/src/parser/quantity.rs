use crate::{
    ast, context::Context, error::label, lexer::T, located::Located, quantity::Value, span::Span,
    Extensions,
};

use super::{token_stream::Token, tokens_span, LineParser, ParserError, ParserWarning};

pub struct ParsedQuantity<'a> {
    pub quantity: Located<ast::Quantity<'a>>,
    pub unit_separator: Option<Span>,
}

/// `tokens` inside '{' '}'. must not be empty
/// whole input
/// enabled extensions
pub fn parse_quantity<'input>(
    tokens: &[Token],
    input: &'input str,
    extensions: Extensions,
    context: &mut Context<ParserError, ParserWarning>,
) -> ParsedQuantity<'input> {
    assert!(!tokens.is_empty(), "empty quantity tokens. this is a bug.");

    let mut line = LineParser::new(
        tokens.first().unwrap().span.start(),
        tokens,
        input,
        extensions,
    );

    if line.extension(Extensions::ADVANCED_UNITS)
        && !tokens
            .iter()
            .any(|t| matches!(t.kind, T![|] | T![*] | T![%]))
    {
        if let Some((value, unit)) = line.with_recover(|line| {
            let start = line.current_offset();
            numeric_value(line).and_then(|value| {
                let end = line.current_offset();
                if line.ws_comments().is_empty() {
                    return None;
                }
                let unit = line.consume_rest();
                if unit.is_empty() {
                    return None;
                }
                let value = Located::new(value, Span::new(start, end));
                let unit = line.text(unit.first().unwrap().span.start(), unit);
                Some((value, unit))
            })
        }) {
            return ParsedQuantity {
                quantity: Located::new(
                    ast::Quantity {
                        value: ast::QuantityValue::Single {
                            value,
                            auto_scale: None,
                        },
                        unit: Some(unit),
                    },
                    tokens_span(tokens),
                ),
                unit_separator: None,
            };
        }
    }

    let mut value = many_values(&mut line);
    let mut unit_separator = None;
    let unit = match line.peek() {
        T![%] => {
            let sep = line.bump_any();
            unit_separator = Some(sep.span);
            let unit = line.consume_rest();
            if unit
                .iter()
                .all(|t| matches!(t.kind, T![ws] | T![block comment]))
            {
                let span = if unit.is_empty() {
                    Span::pos(sep.span.end())
                } else {
                    Span::new(sep.span.start(), unit.last().unwrap().span.end())
                };
                line.error(ParserError::ComponentPartInvalid {
                    container: "quantity",
                    what: "unit",
                    reason: "is empty",
                    labels: vec![
                        label!(sep.span, "remove this"),
                        label!(span, "or add unit here"),
                    ],
                    help: None,
                });
                None
            } else {
                Some(line.text(sep.span.end(), unit))
            }
        }
        T![eof] => None,
        _ => {
            line.consume_rest();
            let text = line.text(line.tokens().first().unwrap().span.start(), line.tokens());
            let text_val = Value::Text(text.text_trimmed());
            value = ast::QuantityValue::Single {
                value: Located::new(text_val, text.span()),
                auto_scale: None,
            };
            None
        }
    };

    context.append(&mut line.finish());

    ParsedQuantity {
        quantity: Located::new(ast::Quantity { value, unit }, tokens_span(tokens)),
        unit_separator,
    }
}

fn many_values<'t, 'input>(line: &mut LineParser<'t, 'input>) -> ast::QuantityValue<'input> {
    let mut values: Vec<Located<Value<'input>>> = vec![];
    let mut auto_scale = None;

    loop {
        values.push(parse_value(line));
        match line.peek() {
            T![|] => {
                line.bump_any();
            }
            T![*] => {
                let tok = line.bump_any();
                auto_scale = Some(tok.span);
                break;
            }
            _ => break,
        }
    }

    match values.len() {
        1 => ast::QuantityValue::Single {
            value: values.pop().unwrap(),
            auto_scale,
        },
        2.. => {
            if let Some(span) = auto_scale {
                line.error(ParserError::ComponentPartInvalid {
                    container: "quantity",
                    what: "value",
                    reason: "auto scale is not compatible with multiple values",
                    labels: vec![label!(span, "remove this")],
                    help: None,
                });
            }
            ast::QuantityValue::Many(values)
        }
        _ => unreachable!(), // first iter is guaranteed
    }
}

fn parse_value<'input>(line: &mut LineParser<'_, 'input>) -> Located<Value<'input>> {
    let start = line.current_offset();
    let val = line.with_recover(numeric_value).unwrap_or_else(|| {
        let offset = line.current_offset();
        let tokens = line.consume_while(|t| !matches!(t, T![|] | T![*] | T![%]));
        let text = line.text(offset, tokens);
        if text.is_text_empty() {
            line.error(ParserError::ComponentPartInvalid {
                container: "quantity",
                what: "value",
                reason: "is empty",
                labels: vec![label!(text.span(), "empty value here")],
                help: None,
            });
        }
        Value::Text(text.text_trimmed())
    });
    line.ws_comments();
    let end = line.current_offset();
    Located::new(val, Span::new(start, end))
}

fn numeric_value(line: &mut LineParser) -> Option<Value<'static>> {
    line.ws_comments();
    let val = match line.peek() {
        T![int] => line
            .with_recover(mixed_num)
            .map(Value::from)
            .or_else(|| line.with_recover(frac).map(Value::from))
            .or_else(|| line.with_recover(range).map(Value::from))
            .unwrap_or_else(|| int(line).map(Value::from).unwrap()),
        T![float] => line
            .with_recover(range)
            .map(Value::from)
            .unwrap_or_else(|| float(line).map(Value::from).unwrap()),
        _ => return None,
    };
    Some(val)
}

fn mixed_num(line: &mut LineParser) -> Option<f64> {
    let a = int(line)?;
    line.ws_comments();
    let f = frac(line)?;
    Some(a + f)
}

fn frac(line: &mut LineParser) -> Option<f64> {
    let a = int(line)?;
    line.ws_comments();
    line.consume(T![/])?;
    line.ws_comments();
    let b = int(line)?;
    Some(a / b)
}

fn range(line: &mut LineParser) -> Option<std::ops::RangeInclusive<f64>> {
    let start = num(line)?;
    line.ws_comments();
    line.consume(T![-])?;
    line.ws_comments();
    let end = num(line)?;
    Some(start..=end)
}

fn num(line: &mut LineParser) -> Option<f64> {
    match line.peek() {
        T![int] => int(line),
        T![float] => float(line),
        _ => None,
    }
}

fn int(line: &mut LineParser) -> Option<f64> {
    let tok = line.consume(T![int])?;
    let val = match line.as_str(tok).parse::<u32>() {
        Ok(n) => n,
        Err(e) => {
            line.error(ParserError::ParseInt {
                bad_bit: tok.span,
                source: e,
            });
            0
        }
    };
    Some(val as f64)
}

fn float(line: &mut LineParser) -> Option<f64> {
    let tok = line.consume(T![float])?;
    let val = match line.as_str(tok).parse::<f64>() {
        Ok(n) => n,
        Err(e) => {
            line.error(ParserError::ParseFloat {
                bad_bit: tok.span,
                source: e,
            });
            0.0
        }
    };
    Some(val)
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{QuantityValue, Text, TextFragment},
        parser::token_stream::TokenStream,
    };

    impl<'a> Text<'a> {
        pub(crate) fn from_str(s: &'a str, offset: usize) -> Self {
            let mut t = Self::empty(offset);
            t.append_fragment(TextFragment::new(s, offset));
            t
        }
    }

    macro_rules! t {
        ($input:literal) => {
            t!($input, $crate::Extensions::all())
        };
        ($input:literal, $extensions:expr) => {{
            let input = $input;
            let tokens = TokenStream::new(input).collect::<Vec<_>>();
            let mut ctx = Context::default();
            let q = parse_quantity(&tokens, input, $extensions, &mut ctx);
            (q.quantity.inner, q.unit_separator, ctx)
        }};
    }

    use super::*;
    #[test]
    fn basic_quantity() {
        let (q, s, _) = t!("100%ml");
        assert_eq!(
            q.value,
            QuantityValue::Single {
                value: Located::new(Value::Number(100.0), 0..3),
                auto_scale: None,
            }
        );
        assert_eq!(s, Some(Span::new(3, 4)));
        assert_eq!(q.unit.unwrap().text(), "ml");
    }

    #[test]
    fn no_separator_ext() {
        let (q, s, ctx) = t!("100 ml");
        assert_eq!(
            q.value,
            QuantityValue::Single {
                value: Located::new(Value::Number(100.0), 0..3),
                auto_scale: None
            }
        );
        assert_eq!(s, None);
        assert_eq!(q.unit.unwrap().text(), "ml");
        assert!(ctx.is_empty());

        let (q, s, ctx) = t!("100 ml", Extensions::all() ^ Extensions::ADVANCED_UNITS);
        assert_eq!(
            q.value,
            QuantityValue::Single {
                value: Located::new(Value::Text("100 ml".into()), 0..6),
                auto_scale: None
            }
        );
        assert_eq!(s, None);
        assert_eq!(q.unit, None);
        assert!(ctx.is_empty());
    }

    #[test]
    fn many_values() {
        let (q, s, ctx) = t!("100|200|300%ml");
        assert_eq!(
            q.value,
            QuantityValue::Many(vec![
                Located::new(Value::Number(100.0), 0..3),
                Located::new(Value::Number(200.0), 4..7),
                Located::new(Value::Number(300.0), 8..11),
            ])
        );
        assert_eq!(s, Some((11..12).into()));
        assert_eq!(q.unit.unwrap(), Text::from_str("ml", 12));
        assert!(ctx.is_empty());

        let (q, s, ctx) = t!("100|2-3|str*%ml");
        assert_eq!(
            q.value,
            QuantityValue::Many(vec![
                Located::new(Value::Number(100.0), 0..3),
                Located::new(Value::Range(2.0..=3.0), 4..7),
                Located::new(Value::Text("str".into()), 8..11),
            ])
        );
        assert_eq!(s, Some((12..13).into()));
        assert_eq!(q.unit.unwrap(), Text::from_str("ml", 13));
        assert_eq!(ctx.errors.len(), 1);
        assert!(ctx.warnings.is_empty());

        let (q, _, ctx) = t!("100|");
        assert_eq!(
            q.value,
            QuantityValue::Many(vec![
                Located::new(Value::Number(100.0), 0..3),
                Located::new(Value::Text("".into()), 4..4)
            ])
        );
        assert_eq!(ctx.errors.len(), 1);
        assert!(ctx.warnings.is_empty());
    }

    #[test]
    fn range_value() {
        let (q, _, _) = t!("2-3");
        assert_eq!(
            q.value,
            QuantityValue::Single {
                value: Located::new(Value::Range(2.0..=3.0), 0..3),
                auto_scale: None
            }
        );
        assert_eq!(q.unit, None);
    }
}

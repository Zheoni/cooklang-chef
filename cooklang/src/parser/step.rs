use crate::{
    ast::{self, Modifiers},
    context::Recover,
    error::label,
    lexer::T,
    located::Located,
    parser::{parser::fatal_err, quantity::parse_quantity, ParserWarning},
    span::Span,
    Extensions,
};

use super::{
    parser::{tokens_span, LineParser, ParseResult},
    token_stream::Token,
    ParserError,
};

pub struct ParsedStep<'input> {
    pub is_text: bool,
    pub items: Vec<ast::Item<'input>>,
}

pub fn step<'t, 'input>(line: &mut LineParser<'t, 'input>, force_text: bool) -> ParsedStep<'input> {
    let is_text = line.consume(T![>]).is_ok();

    let mut items: Vec<ast::Item> = vec![];

    if is_text || force_text {
        let start = line.current_offset();
        let tokens = line.consume_rest();
        items.push(ast::Item::Text(line.text(start, tokens)));
        return ParsedStep { is_text, items };
    }

    while !line.rest().is_empty() {
        let start = line.current_offset();
        let component = match line.peek() {
            T![@] => line
                .with_recover(ingredient)
                .map(ast::Component::Ingredient)
                .ok(),
            T![#] => line
                .with_recover(cookware)
                .map(ast::Component::Cookware)
                .ok(),
            T![~] => line.with_recover(timer).map(ast::Component::Timer).ok(),
            _ => None,
        };
        if let Some(component) = component {
            let end = line.current_offset();
            items.push(ast::Item::Component(Box::new(Located::new(
                component,
                Span::new(start, end),
            ))));
        } else {
            let tokens = line.consume_while(|t| !matches!(t, T![@] | T![#] | T![~]));
            items.push(ast::Item::Text(line.text(start, tokens)));
        }
    }

    ParsedStep {
        is_text: false,
        items,
    }
}

fn comp_body<'t, 'input>(
    line: &mut LineParser<'t, 'input>,
) -> ParseResult<(&'t [Token], Option<Span>, Option<&'t [Token]>)> {
    line.with_recover(|line| {
        let name = line.until(|t| matches!(t, T!['{'] | T![@] | T![#] | T![~]))?;
        let close_span = line.consume(T!['{'])?.span;
        let quantity = line.until(|t| t == T!['}'])?;
        line.bump(T!['}']);
        if quantity
            .iter()
            .any(|t| !matches!(t.kind, T![ws] | T![block comment]))
        {
            Ok((name, Some(close_span), Some(quantity)))
        } else {
            Ok((name, Some(close_span), None))
        }
    })
    .or_else(|_| {
        line.with_recover(|line| {
            line.consume(T![word])?;
            let parsed = line.parsed();
            let name = &parsed[parsed.len() - 1..];
            Ok((name, None, None))
        })
    })
}

fn modifiers<'t, 'input>(line: &mut LineParser<'t, 'input>) -> &'t [Token] {
    line.consume_while(|t| matches!(t, T![@] | T![&] | T![?] | T![+] | T![-]))
}

const INGREDIENT: &str = "ingredient";
const COOKWARE: &str = "cookware";
const TIMER: &str = "timer";

fn ingredient<'t, 'input>(
    line: &mut LineParser<'t, 'input>,
) -> ParseResult<ast::Ingredient<'input>> {
    // Parse
    line.consume(T![@])?;
    let modifiers_pos = line.current_offset();
    let modifiers_tokens = modifiers(line);
    let name_offset = line.current_offset();
    let (name_tokens, _, quantity_tokens) = comp_body(line)?;
    let note = line
        .extension(Extensions::INGREDIENT_NOTE)
        .then(|| {
            line.with_recover(|line| {
                line.consume(T!['('])?;
                let offset = line.current_offset();
                let note = line.until(|t| t == T![')'])?;
                line.bump(T![')']);
                Ok(line.text(offset, note))
            })
            .ok()
        })
        .flatten();

    // Build text(s) and checks
    let (name, alias) = if let Some(alias_sep) = line
        .extension(Extensions::INGREDIENT_ALIAS)
        .then(|| name_tokens.iter().position(|t| t.kind == T![|]))
        .flatten()
    {
        let (name_tokens, alias_tokens) = name_tokens.split_at(alias_sep);
        let (alias_sep, alias_text_tokens) = alias_tokens.split_first().unwrap();
        let alias_text = line.text(alias_sep.span.end(), alias_text_tokens);
        let alias_text = if alias_text_tokens.iter().any(|t| t.kind == T![|]) {
            let bad_bit = Span::new(
                alias_sep.span.start(),
                alias_text_tokens.last().unwrap_or(alias_sep).span.end(),
            );
            line.error(ParserError::ComponentPartInvalid {
                container: INGREDIENT,
                what: "alias",
                reason: "multiple aliases",
                labels: vec![label!(bad_bit, "more than one alias defined here")],
                help: Some("An ingrediedient can only have one alias. Remove the extra '|'."),
            });
            None
        } else if alias_text.is_text_empty() {
            line.error(ParserError::ComponentPartInvalid {
                container: INGREDIENT,
                what: "alias",
                reason: "is empty",
                labels: vec![
                    label!(alias_sep.span, "remove this"),
                    label!(alias_text.span(), "or add something here"),
                ],
                help: None,
            });
            None
        } else {
            Some(alias_text)
        };
        (line.text(name_offset, name_tokens), alias_text)
    } else {
        (line.text(name_offset, name_tokens), None)
    };

    if name.is_text_empty() {
        line.error(ParserError::ComponentPartInvalid {
            container: INGREDIENT,
            what: "name",
            reason: "is empty",
            labels: vec![label!(name.span(), "add a name here")],
            help: None,
        });
    }

    let modifiers = if modifiers_tokens.is_empty() {
        Located::new(Modifiers::empty(), Span::pos(modifiers_pos))
    } else if !line.extension(Extensions::INGREDIENT_MODIFIERS) {
        let modifiers_span = tokens_span(modifiers_tokens);
        line.error(ParserError::ExtensionNotEnabled {
            span: modifiers_span,
            extension_name: "ingredient modifiers",
        });
        Located::new(Modifiers::empty(), modifiers_span)
    } else {
        let modifiers_span = tokens_span(modifiers_tokens);
        let mut m = modifiers_tokens
            .iter()
            .try_fold(Modifiers::empty(), |acc, m| {
                let new_m = match m.kind {
                    T![@] => Modifiers::RECIPE,
                    T![&] => Modifiers::REF,
                    T![?] => Modifiers::OPT,
                    T![+] => Modifiers::NEW,
                    T![-] => Modifiers::HIDDEN,
                    _ => unreachable!(), // checked in [modifiers] function
                };

                if acc.contains(new_m) {
                    line.error(ParserError::InvalidModifiers {
                        modifiers_span,
                        reason: format!("duplicate modifier '{}'", line.as_str(*m)).into(),
                        help: Some(
                            "Modifier order does not matter, but duplicates are not allowed",
                        ),
                    });
                    return Err(());
                } else {
                    Ok(acc | new_m)
                }
            })
            .unwrap_or(Modifiers::empty());

        // REF cannot appear in certain combinations
        if m.contains(Modifiers::REF)
            && m.intersects(Modifiers::NEW | Modifiers::HIDDEN | Modifiers::OPT)
        {
            line.error(ParserError::InvalidModifiers {
                modifiers_span,
                reason: "unsuported combination with reference".into(),
                help: Some("Reference ('&') modifier can only be combined with recipe ('@')"),
            });
            m = Modifiers::empty();
        }

        Located::new(m, modifiers_span)
    };

    let quantity = quantity_tokens.map(|tokens| {
        parse_quantity(tokens, line.input, line.extensions, &mut line.context).quantity
    });

    Ok(ast::Ingredient {
        modifiers,
        name,
        alias,
        quantity,
        note,
    })
}

fn cookware<'t, 'input>(line: &mut LineParser<'t, 'input>) -> ParseResult<ast::Cookware<'input>> {
    // Parse
    line.consume(T![#])?;
    let modifiers_tokens = modifiers(line);
    let name_offset = line.current_offset();
    let (name_tokens, _, quantity_tokens) = comp_body(line)?;

    // Errors
    check_modifiers(line, modifiers_tokens, COOKWARE);
    check_alias(line, name_tokens, COOKWARE);
    check_note(line, COOKWARE);

    let name = line.text(name_offset, name_tokens);
    if name.is_text_empty() {
        line.error(ParserError::ComponentPartInvalid {
            container: COOKWARE,
            what: "name",
            reason: "is empty",
            labels: vec![label!(name, "add a name here")],
            help: None,
        });
    }
    let quantity = quantity_tokens.map(|tokens| {
        let q = parse_quantity(tokens, line.input, line.extensions, &mut line.context);
        if let Some(unit) = &q.quantity.unit {
            let span = if let Some(sep) = q.unit_separator {
                Span::new(sep.start(), unit.span().end())
            } else {
                unit.span()
            };
            line.error(ParserError::ComponentPartNotAllowed {
                container: COOKWARE,
                what: "unit in quantity",
                to_remove: span,
                help: Some("Cookware quantity can't have an unit."),
            });
        }
        if let ast::QuantityValue::Single {
            auto_scale: Some(auto_scale),
            ..
        } = &q.quantity.value
        {
            line.error(ParserError::ComponentPartNotAllowed {
                container: COOKWARE,
                what: "auto scale marker",
                to_remove: *auto_scale,
                help: Some("Cookware quantity can't be auto scaled."),
            });
        }
        q.quantity.map_inner(|q| q.value)
    });

    Ok(ast::Cookware { name, quantity })
}

fn timer<'t, 'input>(line: &mut LineParser<'t, 'input>) -> ParseResult<ast::Timer<'input>> {
    // Parse
    line.consume(T![~])?;
    let modifiers_tokens = modifiers(line);
    let name_offset = line.current_offset();
    let (name_tokens, close_span, quantity_tokens) = comp_body(line)?;

    // Errors
    check_modifiers(line, modifiers_tokens, COOKWARE);
    check_alias(line, name_tokens, COOKWARE);
    check_note(line, COOKWARE);

    let name = line.text(name_offset, name_tokens);

    let quantity = quantity_tokens
        .map(|tokens| {
            let q = parse_quantity(tokens, line.input, line.extensions, &mut line.context);
            if let ast::QuantityValue::Single {
                auto_scale: Some(auto_scale),
                ..
            } = &q.quantity.value
            {
                line.error(ParserError::ComponentPartNotAllowed {
                    container: TIMER,
                    what: "auto scale marker",
                    to_remove: *auto_scale,
                    help: Some("Timer quantity can't be auto scaled."),
                });
            }
            if q.quantity.unit.is_none() {
                line.error(ParserError::ComponentPartMissing {
                    container: TIMER,
                    what: "quantity unit",
                    expected_pos: Span::pos(q.quantity.value.span().end()),
                });
            }
            q.quantity
        })
        .unwrap_or_else(|| {
            let span = if let Some(s) = close_span {
                Span::pos(s.end())
            } else {
                Span::pos(name.span().end())
            };
            line.error(ParserError::ComponentPartMissing {
                container: TIMER,
                what: "quantity",
                expected_pos: span,
            });
            Recover::recover()
        });

    let name = if name.is_text_empty() {
        None
    } else {
        Some(name)
    };
    Ok(ast::Timer { name, quantity })
}

fn check_modifiers(line: &mut LineParser, modifiers_tokens: &[Token], container: &'static str) {
    assert_ne!(container, INGREDIENT);
    if !modifiers_tokens.is_empty() {
        line.error(ParserError::ComponentPartNotAllowed {
            container,
            what: "modifiers",
            to_remove: tokens_span(modifiers_tokens),
            help: Some("Modifiers are only available in ingredients"),
        });
    }
}

fn check_alias(line: &mut LineParser, name_tokens: &[Token], container: &'static str) {
    assert_ne!(container, INGREDIENT);
    if let Some(sep) = name_tokens.iter().position(|t| t.kind == T![|]) {
        let to_remove = Span::new(
            name_tokens[sep].span.start(),
            name_tokens.last().unwrap().span.end(),
        );
        line.error(ParserError::ComponentPartNotAllowed {
            container,
            what: "alias",
            to_remove,
            help: Some("Aliases are only available in ingredients"),
        });
    }
}

fn check_note(line: &mut LineParser, container: &'static str) {
    assert_ne!(container, INGREDIENT);
    if !line.extension(Extensions::INGREDIENT_NOTE) {
        return;
    }

    line.with_recover(|line| {
        let start = line.consume(T!['('])?.span.start();
        let _ = line.until(|t| t == T![')'])?;
        let end = line.bump(T![')']).span.end();
        line.warn(ParserWarning::ComponentPartIgnored {
            container,
            what: "note",
            ignored: Span::new(start, end),
            help: Some("Notes are only available in ingredients"),
        });
        fatal_err::<()>() // always backtrack
    })
    .unwrap_err();
}

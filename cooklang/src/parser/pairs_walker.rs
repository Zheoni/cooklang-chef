use std::ops::RangeInclusive;

use crate::{
    ast::TextFragment,
    context::{Context, Recover},
    error::PassResult,
    located::Located,
    parser::{ast::Separated, pest_ext::PairExt},
    quantity::Value,
    span::Span,
    Extensions,
};

use super::{
    ast::{
        Ast, Component, Cookware, Delimited, Ingredient, Item, Line, Modifiers, Quantity,
        QuantityValue, Text, Timer,
    },
    Pair, Pairs, ParserError, ParserWarning, Rule,
};

const INGREDIENT: &str = "ingredient";
const COOKWARE: &str = "cookware";
const TIMER: &str = "timer";

macro_rules! is_rule {
    ($pair:ident, $rule:expr) => {
        debug_assert_eq!(
            $pair.as_rule(),
            $rule,
            "Expected rule '{:?}', got '{:?}'",
            $rule,
            $pair.as_rule()
        );
    };
}

macro_rules! unexpected_ignore {
    ($pair:expr) => {{
        #[cfg(debug_assertions)]
        {
            eprintln!(
                "[{} {}:{}] Unexpected rule: {:?}",
                file!(),
                line!(),
                column!(),
                $pair.as_rule()
            );
        }
    }};
    ($pair:expr, $val:expr) => {{
        #[cfg(debug_assertions)]
        {
            eprintln!(
                "[{} {}:{}] Unexpected rule: {:?}",
                file!(),
                line!(),
                column!(),
                $pair.as_rule()
            );
        }
        $val
    }};
}

macro_rules! unexpected_panic {
    ($pair:expr, $where:literal) => {{
        unexpected_ignore!($pair);
        panic!(concat!("unexpected rule inside ", $where));
    }};
}

#[tracing::instrument(skip_all)]
pub fn build_ast(
    mut pairs: Pairs,
    extensions: Extensions,
) -> PassResult<Ast, ParserError, ParserWarning> {
    let mut walker = Walker {
        context: Context::<ParserError, ParserWarning>::default(),
        extensions,
    };

    let pair = pairs.next().expect("Empty root pairs");
    debug_assert!(pairs.next().is_none());

    let ast = walker.cooklang(pair);
    walker.context.finish(Some(ast))
}

struct Walker {
    context: Context<ParserError, ParserWarning>,
    extensions: Extensions,
}

crate::context::impl_deref_context!(Walker, ParserError, ParserWarning);

impl Walker {
    fn cooklang<'a>(&mut self, pair: Pair<'a>) -> Ast<'a> {
        is_rule!(pair, Rule::cooklang);

        let mut lines = Vec::new();

        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::metadata => {
                    let (key, value) = self.metadata(pair);
                    lines.push(Line::Metadata { key, value });
                }
                Rule::step => {
                    let (is_text, items) = self.step(pair);
                    lines.push(Line::Step { is_text, items });
                }
                Rule::section => lines.push(Line::Section {
                    name: self.section(pair),
                }),
                Rule::soft_break => lines.push(Line::SoftBreak),
                Rule::EOI => {}
                _ => unexpected_ignore!(pair),
            }
        }

        Ast { lines }
    }

    fn metadata<'a>(&mut self, pair: Pair<'a>) -> (Text<'a>, Text<'a>) {
        is_rule!(pair, Rule::metadata);

        let mut pairs = pair.into_inner();
        let key = pairs.next().expect("No key in metadata").text();
        let value = pairs.next().expect("No value in metadata").text();
        debug_assert!(pairs.next().is_none());

        if key.is_text_empty() {
            self.error(ParserError::ComponentPartInvalid {
                container: "metadata entry",
                what: "key",
                reason: "is empty",
                bad_bit: key.span(),
                span_label: Some("this cannot be empty"),
                help: None,
            });
        }

        if value.is_text_empty() {
            self.warn(ParserWarning::EmptyMetadataValue {
                key: key.located_str().map_inner(|c| c.into_owned()),
            });
        }

        (key, value)
    }

    fn step<'a>(&mut self, pair: Pair<'a>) -> (bool, Vec<Item<'a>>) {
        is_rule!(pair, Rule::step);
        let mut items = Vec::new();
        let mut is_text = false;

        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::only_text_marker => {
                    if self.extensions.contains(Extensions::TEXT_STEPS) {
                        is_text = true;
                    } else {
                        items.push(Item::Text(Text::from_str(
                            pair.as_str(),
                            pair.as_span().start(),
                        )));
                    }
                }
                Rule::component => {
                    let (component, extra_text) = self.component(pair);
                    items.push(Item::Component(Box::new(component)));
                    if let Some(t) = extra_text {
                        items.push(Item::Text(t));
                    }
                }
                Rule::plain_text => items.push(Item::Text(Text::from_str(
                    pair.as_str(),
                    pair.as_span().start(),
                ))),
                Rule::line_comment => items.push(Item::Text(Text::new(
                    pair.as_span().start(),
                    vec![TextFragment::line_comment(
                        pair.as_str(),
                        pair.as_span().start(),
                    )],
                ))),
                Rule::block_comment => items.push(Item::Text(Text::new(
                    pair.as_span().start(),
                    vec![TextFragment::block_comment(
                        pair.as_str(),
                        pair.as_span().start(),
                    )],
                ))),
                _ => unexpected_ignore!(pair),
            }
        }

        (is_text, items)
    }

    fn section<'a>(&mut self, pair: Pair<'a>) -> Option<Text<'a>> {
        is_rule!(pair, Rule::section);

        if !self.extensions.contains(Extensions::SECTIONS) {
            self.error(ParserError::ExtensionNotEnabled {
                span: pair.into(),
                extension_name: "sections",
            });
            return None;
        }

        pair.into_inner().next().map(|p| {
            is_rule!(p, Rule::section_name);
            p.text()
        })
    }

    fn component<'a>(&mut self, pair: Pair<'a>) -> (Located<Component<'a>>, Option<Text<'a>>) {
        is_rule!(pair, Rule::component);

        let mut token = None;
        let mut modifiers = None;
        let mut name = None;
        let mut alias = None;
        let mut quantity = None;
        let mut note = None;

        let span = Span::from(pair.as_span());

        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::component_token => token = Some(pair.first_inner()),
                Rule::modifiers => modifiers = Some(pair.into()),
                Rule::name => {
                    name = Some(pair.text());
                }
                Rule::one_word_component => {
                    name = Some(Text::from_str(pair.as_str(), pair.as_span().start()))
                }
                Rule::alias => {
                    alias = Some(pair.text());
                }
                Rule::close_component => {
                    let mut it = pair.into_inner();
                    let open = it.next().unwrap().as_span().into();
                    let p = it.next().unwrap();
                    match p.as_rule() {
                        Rule::close_quantity => continue,
                        Rule::quantity => {
                            let close = it.next().unwrap().as_span().into();
                            quantity = Some(Delimited::new(open, self.quantity(p), close));
                        }
                        _ => unexpected_panic!(p, "close component"),
                    }
                }
                Rule::note => {
                    let mut it = pair.into_inner();
                    let open = it.next().unwrap().as_span().into();
                    let content = it.next().unwrap().text();
                    let close = it.next().unwrap().as_span().into();
                    debug_assert!(it.next().is_none());
                    note = Some(Delimited::new(open, content, close));
                }
                _ => unexpected_ignore!(pair),
            }
        }

        let token = token.expect("No component_token inside item");

        let mut c = GenericComponent {
            token,
            span,
            name,
            alias,
            modifiers,
            quantity,
            note,
        };

        // If there are modifiers but are disabled, add an error and remove modifiers
        if !self.extensions.contains(Extensions::INGREDIENT_MODIFIERS) {
            if let Some(modifiers) = c.modifiers {
                self.error(ParserError::ExtensionNotEnabled {
                    span: modifiers.into(),
                    extension_name: "ingredient modifiers",
                });
            }
            c.modifiers = None;
        }

        // If there is an alias but aliases are disabled, concat it with the name
        if !self.extensions.contains(Extensions::INGREDIENT_ALIAS) {
            if let Some(alias) = c.alias {
                let name = c.name.as_mut().expect("Alias but no name");
                name.append_str("|");
                name.append(alias);
            }
            c.alias = None;
        }

        // If there is a note but notes are disabled, have an extra text at the end
        let mut extra_text = None;
        if !self.extensions.contains(Extensions::INGREDIENT_NOTE) {
            if let Some(note) = c.note {
                let orig_span = note.span();
                let mut t = Text::from_str("(", note.span().start());
                t.append(note.into_inner());
                t.append_str(")");
                assert_eq!(t.span(), orig_span);
                extra_text = Some(t);
            }
            c.note = None;
        }

        let component = match c.token.as_rule() {
            Rule::ingredient_token => Component::Ingredient(self.ingredient(c)),
            Rule::cookware_token => Component::Cookware(self.cookware(c)),
            Rule::timer_token => Component::Timer(self.timer(c)),
            _ => unexpected_panic!(c.token, "component token"),
        };

        (Located::new(component, span), extra_text)
    }

    fn ingredient<'a>(&mut self, component: GenericComponent<'a>) -> Ingredient<'a> {
        let modifiers = component
            .modifiers
            .map(|p| p.located().map_inner(|p| self.modifiers(p)));

        let name = component.name.unwrap_or_else(Recover::recover);
        if name.is_text_empty() {
            self.error(ParserError::ComponentPartMissing {
                container: INGREDIENT,
                what: "name",
                component_span: component.span,
            });
        }

        let alias = component.alias;
        if let Some(alias) = &alias {
            if alias.is_text_empty() {
                let span = alias.span();
                self.error(ParserError::ComponentPartNotAllowed {
                    container: INGREDIENT,
                    what: "empty alias",
                    to_remove: Span::new(span.start() - 1, span.end()),
                    help: Some("Add an alias or remove the '|'"),
                });
            }
        }

        let quantity = component.quantity;
        let note = component.note;
        let modifiers = modifiers.unwrap_or_else(|| {
            let pos = component.token.as_span().end();
            Located::new(Modifiers::empty(), Span::new(pos, pos))
        });

        Ingredient {
            modifiers,
            name,
            alias,
            quantity,
            note,
        }
    }

    fn cookware<'a>(&mut self, component: GenericComponent<'a>) -> Cookware<'a> {
        if let Some(modifiers) = component.modifiers {
            self.error(ParserError::ComponentPartNotAllowed {
                container: COOKWARE,
                what: "modifiers",
                to_remove: modifiers.as_span().into(),
                help: Some("Modifiers are only available in ingredients"),
            });
        }

        if let Some(alias) = component.alias {
            self.error(ParserError::ComponentPartNotAllowed {
                container: COOKWARE,
                what: "alias",
                to_remove: alias.span(),
                help: Some("Aliases are only available in ingredients"),
            });
        }

        if let Some(note) = component.note {
            self.error(ParserError::ComponentPartNotAllowed {
                container: COOKWARE,
                what: "note",
                to_remove: note.span(),
                help: Some("Notes are only available in ingredients"),
            });
        }

        let name = component.name.unwrap_or_else(Recover::recover);
        if name.is_text_empty() {
            self.error(ParserError::ComponentPartMissing {
                container: COOKWARE,
                what: "name",
                component_span: component.span,
            });
        }

        let quantity = component.quantity.map(|quantity| {
            if let Some(unit_span) = quantity.unit_span() {
                self.error(ParserError::ComponentPartNotAllowed {
                    container: COOKWARE,
                    what: "unit in quantity",
                    to_remove: unit_span,
                    help: Some("Cookware quantity can't have an unit"),
                });
            }
            let open = quantity.open();
            let close = quantity.close();
            Delimited::new(open, quantity.into_inner().value, close)
        });
        if let Some(Delimited {
            inner:
                QuantityValue::Single {
                    scalable: true,
                    auto_scale_marker,
                    ..
                },
            ..
        }) = &quantity
        {
            self.error(ParserError::ComponentPartNotAllowed {
                container: COOKWARE,
                what: "auto scale marker",
                to_remove: auto_scale_marker.expect("auto scale marker span"),
                help: Some("Cookware quantity can't be auto scaled"),
            });
        }

        Cookware { name, quantity }
    }

    fn timer<'a>(&mut self, component: GenericComponent<'a>) -> Timer<'a> {
        if let Some(modifiers) = component.modifiers {
            self.error(ParserError::ComponentPartNotAllowed {
                container: TIMER,
                what: "modifiers",
                to_remove: modifiers.as_span().into(),
                help: Some("Modifiers are only available in ingredients"),
            });
        }

        if let Some(alias) = component.alias {
            self.error(ParserError::ComponentPartNotAllowed {
                container: TIMER,
                what: "alias",
                to_remove: alias.span(),
                help: Some("Aliases are only available in ingredients"),
            });
        }

        if let Some(note) = component.note {
            self.error(ParserError::ComponentPartNotAllowed {
                container: TIMER,
                what: "note",
                to_remove: note.span(),
                help: Some("Notes are only available in ingredients"),
            });
        }

        let name = component.name;
        let quantity = component.quantity.unwrap_or_else(|| {
            self.error(ParserError::ComponentPartMissing {
                container: TIMER,
                what: "quantity",
                component_span: component.span,
            });
            Recover::recover()
        });

        if let QuantityValue::Single {
            scalable: true,
            auto_scale_marker,
            ..
        } = &quantity.value
        {
            self.error(ParserError::ComponentPartNotAllowed {
                container: TIMER,
                what: "auto scale marker",
                to_remove: auto_scale_marker.clone().expect("auto scale marker span"),
                help: Some("Timer quantity can't be auto scaled"),
            });
        }
        if quantity.unit.is_none() {
            self.error(ParserError::ComponentPartMissing {
                container: TIMER,
                what: "quantity unit",
                component_span: component.span,
            });
        }

        Timer { name, quantity }
    }

    fn modifiers(&mut self, pair: Pair) -> Modifiers {
        is_rule!(pair, Rule::modifiers);

        let span = pair.as_span().into();
        let mut modifiers = Modifiers::empty();
        for p in pair.into_inner() {
            let m = match p.as_rule() {
                Rule::mod_recipe => Modifiers::RECIPE,
                Rule::mod_new => Modifiers::NEW,
                Rule::mod_hidden => Modifiers::HIDDEN,
                Rule::mod_ref => Modifiers::REF,
                Rule::mod_opt => Modifiers::OPT,
                _ => unexpected_panic!(p, "modifiers"),
            };
            if modifiers.contains(m) {
                self.context.errors.push(ParserError::InvalidModifiers {
                    modifiers_span: span,
                    reason: format!("duplicate modifier '{}'", p.as_str()).into(),
                    help: Some("Modifier order does not matter, but duplicates are not allowed"),
                });
                return Modifiers::empty();
            }
            modifiers |= m;
        }

        // REF cannot appear in certain combinations
        if modifiers.contains(Modifiers::REF)
            && modifiers.intersects(Modifiers::NEW | Modifiers::HIDDEN | Modifiers::OPT)
        {
            self.context.errors.push(ParserError::InvalidModifiers {
                modifiers_span: span,
                reason: "unsuported combination with reference".into(),
                help: Some("Reference ('&') modifier can only be combined with recipe ('@')"),
            });
            return Modifiers::empty();
        }

        modifiers
    }

    fn quantity<'a>(&mut self, pair: Pair<'a>) -> Quantity<'a> {
        is_rule!(pair, Rule::quantity);
        let mut inner = pair.clone().into_inner();

        let mut values = Vec::new();
        let mut auto_scale = false;
        let mut auto_scale_marker = None;
        let mut unit_separator = None;
        let mut unit = None;
        for pair in inner.by_ref() {
            match pair.as_rule() {
                Rule::numeric_value => {
                    values.push(pair.located().map_inner(|p| self.numeric_value(p)))
                }
                Rule::value_text => values.push(
                    pair.located()
                        .map_inner(|p| Value::Text(p.text().text_trimmed())),
                ),
                Rule::auto_scale => {
                    auto_scale = true;
                    auto_scale_marker = Some(Span::from(pair.as_span()))
                }
                Rule::unit_separator => unit_separator = Some(Span::from(pair.as_span())),
                Rule::unit => {
                    unit = Some(pair.text());
                    break; // should be the last thing
                }
                Rule::value_separator => {}
                _ => unexpected_panic!(pair, "quantity"),
            }
        }
        debug_assert!(inner.next().is_none());

        // if the extension is disabled and there is no unit separator, treat
        // all as a value text, even if a unit has been found. this is the default
        // (original) cooklang behaviour
        if !self.extensions.contains(Extensions::ADVANCED_UNITS)
            && unit_separator.is_none()
            && unit.is_some()
        {
            values = vec![pair
                .clone()
                .located()
                .map_inner(|p| Value::Text(p.as_str().trim().into()))];
            unit = None;
        };

        let value = match values.len() {
            0 => panic!("no value inside quantity"),
            1 => QuantityValue::Single {
                value: values.pop().unwrap(),
                scalable: auto_scale,
                auto_scale_marker,
            },
            _ => {
                if auto_scale {
                    self.error(ParserError::QuantityScalingConflict {
                        bad_bit: pair.into(),
                    });
                }
                QuantityValue::Many(Separated::from_items(values))
            }
        };

        if let Some(u) = &unit {
            if u.is_text_empty() {
                if let Some(separator) = unit_separator {
                    let to_remove = Span::new(separator.start(), u.span().end());
                    self.error(ParserError::ComponentPartNotAllowed {
                        container: "quantity",
                        what: "empty unit",
                        to_remove,
                        help: Some("Or add a unit"),
                    });
                } else {
                    unit = None;
                };
            }
        }

        Quantity {
            value,
            unit,
            unit_separator,
        }
    }

    fn numeric_value<'a>(&mut self, pair: Pair<'a>) -> Value<'a> {
        is_rule!(pair, Rule::numeric_value);
        let pair = pair.first_inner();
        match pair.as_rule() {
            Rule::mixed_number => Value::Number(self.recover(mixed_number(pair))),
            Rule::fraction => Value::Number(self.recover(fraction(pair))),
            Rule::range => {
                if self.extensions.contains(Extensions::RANGE_VALUES) {
                    Value::Range(self.recover_val(range(pair), 1.0..=1.0))
                } else {
                    Value::Text(pair.as_str().into())
                }
            }
            Rule::number => Value::Number(self.recover(number(pair))),
            _ => unexpected_panic!(pair, "numeric_value"),
        }
    }
}

fn range(pair: Pair) -> Result<RangeInclusive<f64>, ParserError> {
    is_rule!(pair, Rule::range);

    let mut inner = pair.into_inner();
    let from = number(inner.next().unwrap())?;
    let to = number(inner.next().unwrap())?;
    debug_assert!(inner.next().is_none());

    Ok(from..=to)
}

fn number(pair: Pair) -> Result<f64, ParserError> {
    is_rule!(pair, Rule::number);
    let pair = pair.first_inner();
    match pair.as_rule() {
        Rule::integer => integer(pair),
        Rule::float => float(pair),
        _ => unexpected_panic!(pair, "number"),
    }
}

fn integer(pair: Pair) -> Result<f64, ParserError> {
    pair.as_str()
        .parse::<i32>()
        .map(|n| n as f64)
        .map_err(|e| ParserError::ParseInt {
            bad_bit: pair.into(),
            source: e,
        })
}

fn float(pair: Pair) -> Result<f64, ParserError> {
    pair.as_str()
        .parse::<f64>()
        .map_err(|e| ParserError::ParseFloat {
            bad_bit: pair.into(),
            source: e,
        })
}

fn fraction(pair: Pair) -> Result<f64, ParserError> {
    is_rule!(pair, Rule::fraction);

    let pair = pair.first_inner();

    let r = match pair.as_rule() {
        Rule::regular_fraction => {
            let mut inner = pair.into_inner();
            let num_pair = inner.next().unwrap();
            let den_pair = inner.next().unwrap();
            debug_assert!(inner.next().is_none());

            let num = integer(num_pair)?;
            let den = integer(den_pair.clone())?;
            if den == 0.0 {
                return Err(ParserError::DivisionByZero {
                    bad_bit: den_pair.into(),
                });
            }

            num / den
        }
        Rule::unicode_fraction => unicode_fraction(pair.as_str()),
        _ => unexpected_panic!(pair, "fraction"),
    };

    Ok(r)
}

fn unicode_fraction(s: &str) -> f64 {
    let (n, d) = match s {
        "½" => (1, 2),
        "⅓" => (1, 3),
        "¼" => (1, 4),
        "⅕" => (1, 5),
        "⅙" => (1, 6),
        "⅐" => (1, 7),
        "⅛" => (1, 8),
        "⅑" => (1, 9),
        "⅒" => (1, 10),
        "⅔" => (2, 3),
        "⅖" => (2, 5),
        "¾" => (3, 4),
        "⅗" => (3, 5),
        "⅜" => (3, 8),
        "⅘" => (4, 5),
        "⅚" => (5, 6),
        "⅝" => (5, 8),
        "⅞" => (7, 8),
        "↉" => (0, 3),
        _ => unreachable!(),
    };

    n as f64 / d as f64
}

fn mixed_number(pair: Pair) -> Result<f64, ParserError> {
    is_rule!(pair, Rule::mixed_number);
    let mut inner = pair.into_inner();
    let integer = integer(inner.next().unwrap())?;
    let frac = fraction(inner.next().unwrap())?;
    debug_assert!(inner.next().is_none());

    Ok(integer + frac)
}

#[derive(Debug)]
struct GenericComponent<'a> {
    token: Pair<'a>,
    span: Span,
    modifiers: Option<Pair<'a>>,
    name: Option<Text<'a>>,
    alias: Option<Text<'a>>,
    quantity: Option<Delimited<Quantity<'a>>>,
    note: Option<Delimited<Text<'a>>>,
}

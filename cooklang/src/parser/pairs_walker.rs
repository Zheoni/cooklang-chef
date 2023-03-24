use std::{borrow::Cow, ops::RangeInclusive};

use crate::{
    context::{Context, Recover},
    error::PassResult,
    located::Located,
    parser::pest_ext::PairExt,
    quantity::Value,
    span::Span,
    Extensions,
};

use super::{
    ast::{
        Ast, Component, Cookware, Ingredient, Item, Line, Modifiers, Quantity, QuantityValue, Timer,
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
    ($pair:ident) => {{
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
    ($pair:ident, $val:expr) => {{
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
    ($pair:ident, $where:literal) => {{
        unexpected_ignore!($pair);
        panic!(concat!("unexpected rule inside ", $where));
    }};
}

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

    fn metadata<'a>(&mut self, pair: Pair<'a>) -> (Located<&'a str>, Located<&'a str>) {
        is_rule!(pair, Rule::metadata);

        let mut pairs = pair.into_inner();
        let key = pairs
            .next()
            .expect("No key in metadata")
            .as_located_str()
            .map_inner(str::trim);
        let value = pairs
            .next()
            .expect("No value in metadata")
            .as_located_str()
            .map_inner(str::trim);

        assert!(!key.is_empty(), "Key is empty");
        if value.trim().is_empty() {
            self.warn(ParserWarning::EmptyMetadataValue {
                key: key.clone().map_inner(str::to_string),
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
                        items.push(Item::Text(Located::new(pair.text(), pair)));
                    }
                }
                Rule::component => {
                    let (component, extra_text) = self.component(pair);
                    items.push(Item::Component(Box::new(component)));
                    if let Some(t) = extra_text {
                        items.push(Item::Text(t));
                    }
                }
                Rule::text => items.push(Item::Text(Located::new(pair.text(), pair))),
                _ => unexpected_ignore!(pair),
            }
        }

        (is_text, items)
    }

    fn section<'a>(&mut self, pair: Pair<'a>) -> Option<Cow<'a, str>> {
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
            p.text_trimmed()
        })
    }

    fn component<'a>(
        &mut self,
        pair: Pair<'a>,
    ) -> (Located<Component<'a>>, Option<Located<Cow<'a, str>>>) {
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
                Rule::component_token => token = Some(pair.as_str()),
                Rule::modifiers => modifiers = Some(pair.into()),
                Rule::name | Rule::one_word_component => {
                    name = Some(pair.located_text_trimmed());
                }
                Rule::alias => {
                    alias = Some(pair.located_text_trimmed());
                }
                Rule::quantity => {
                    quantity = Some(Located::from(pair).map_inner(|p| self.quantity(p)))
                }
                Rule::note => note = Some(pair.located_text()),
                _ => unexpected_ignore!(pair),
            }
        }

        let mut c = GenericComponent {
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
                *name = Located::new(
                    format!("{name}|{alias}").into(),
                    name.range().start..alias.range().end,
                );
            }
            c.alias = None;
        }

        // If there is a note but notes are disabled, have an extra text at the end
        let mut extra_text = None;
        if !self.extensions.contains(Extensions::INGREDIENT_NOTE) {
            if let Some(note) = c.note {
                extra_text = Some(note);
            }
            c.note = None;
        }

        let component = match token.expect("No component_token inside item") {
            "@" => Component::Ingredient(self.ingredient(c)),
            "#" => Component::Cookware(self.cookware(c)),
            "~" => Component::Timer(self.timer(c)),
            _ => unreachable!("Unknown item kind"),
        };

        (Located::new(component, span), extra_text)
    }

    fn ingredient<'a>(&mut self, component: GenericComponent<'a>) -> Ingredient<'a> {
        let modifiers = component
            .modifiers
            .map(|p| p.map_inner(|p| self.modifiers(p)))
            .unwrap_or_else(|| {
                Located::new(
                    Modifiers::empty(),
                    component.span.start() + 1..component.span.start() + 1,
                )
            });

        let name = component.name.unwrap_or_else(Recover::recover);

        if name.is_empty() {
            self.error(ParserError::ComponentPartMissing {
                container: INGREDIENT,
                what: "name",
                component_span: component.span,
            });
        }

        let alias = component.alias;
        if let Some(alias) = &alias {
            if alias.is_empty() {
                self.error(ParserError::ComponentPartNotAllowed {
                    container: INGREDIENT,
                    what: "empty alias",
                    to_remove: Span::new(alias.offset() - 1, alias.offset()),
                    help: Some("Add an alias or remove the '|'"),
                });
            }
        }

        let quantity = component.quantity;
        let note = component.note;

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
                to_remove: modifiers.span(),
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
        if name.is_empty() {
            self.error(ParserError::ComponentPartMissing {
                container: COOKWARE,
                what: "name",
                component_span: component.span,
            });
        }

        let quantity = component.quantity.map(|quantity| {
            if let Some(unit) = &quantity.unit {
                self.error(ParserError::ComponentPartNotAllowed {
                    container: COOKWARE,
                    what: "unit in quantity",
                    to_remove: Span::new(quantity.value.span().end(), unit.span().end()),
                    help: Some("Cookware quantity can't have an unit"),
                });
            }
            quantity.take().value
        });
        if let Some(value @ QuantityValue::Single { scalable: true, .. }) = &quantity {
            self.error(ParserError::ComponentPartNotAllowed {
                container: COOKWARE,
                what: "auto scale marker",
                to_remove: {
                    let span = value.span();
                    Span::new(span.end(), span.end() + 1)
                },
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
                to_remove: modifiers.span(),
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

        if let value @ QuantityValue::Single { scalable: true, .. } = &quantity.value {
            self.error(ParserError::ComponentPartNotAllowed {
                container: COOKWARE,
                what: "auto scale marker",
                to_remove: {
                    let span = value.span();
                    Span::new(span.end(), span.end() + 1)
                },
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

        let mut modifiers = Modifiers::empty();
        for c in pair.as_str().chars() {
            let m = match c {
                '@' => Modifiers::RECIPE,
                '+' => Modifiers::NEW,
                '-' => Modifiers::HIDDEN,
                '&' => Modifiers::REF,
                '?' => Modifiers::OPT,
                _ => panic!("Unknown modifier"),
            };
            if modifiers.contains(m) {
                self.context.errors.push(ParserError::InvalidModifiers {
                    modifiers_span: pair.into(),
                    reason: format!("duplicate modifier '{c}'").into(),
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
                modifiers_span: pair.into(),
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
        let mut separator = None;
        let mut unit = None;
        for pair in inner.by_ref() {
            match pair.as_rule() {
                Rule::numeric_value => {
                    values.push(pair.located().map_inner(|p| self.numeric_value(p)))
                }
                Rule::value_text => {
                    values.push(pair.located().map_inner(|p| Value::Text(p.text_trimmed())))
                }
                Rule::auto_scale => auto_scale = true,
                Rule::unit_separator => separator = Some(pair),
                Rule::unit => {
                    unit = Some(pair.located_text_trimmed());
                    break; // should be the last thing
                }
                _ => unexpected_panic!(pair, "quantity"),
            }
        }
        debug_assert!(inner.next().is_none());

        // if the extension is disabled and there is no unit separator, treat
        // all as a value text, even if a unit has been found. this is the default
        // (original) cooklang behaviour
        if !self.extensions.contains(Extensions::ADVANCED_UNITS)
            && separator.is_none()
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
            },
            _ => {
                if auto_scale {
                    self.error(ParserError::QuantityScalingConflict {
                        bad_bit: pair.into(),
                    });
                }
                QuantityValue::Many(values)
            }
        };

        if let Some(u) = &unit {
            if u.is_empty() {
                if let Some(separator) = separator {
                    let to_remove = Span::new(separator.as_span().start(), u.range().end);
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

        Quantity { value, unit }
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

struct GenericComponent<'a> {
    span: Span,
    modifiers: Option<Located<Pair<'a>>>,
    name: Option<Located<Cow<'a, str>>>,
    alias: Option<Located<Cow<'a, str>>>,
    quantity: Option<Located<Quantity<'a>>>,
    note: Option<Located<Cow<'a, str>>>,
}

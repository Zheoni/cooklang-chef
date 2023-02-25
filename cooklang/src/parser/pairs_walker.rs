use std::{
    borrow::Cow,
    ops::{Range, RangeInclusive},
};

use crate::{
    context::{Context, Recover},
    impl_deref_context,
    parser::pest_ext::{PairExt, Span},
    quantity::Value,
    Extensions,
};

use super::{
    ast::{Ast, Component, Cookware, Ingredient, Item, Line, Modifiers, Quantity, Timer},
    located::Located,
    ComponentKind, Pair, Pairs, ParserError, ParserWarning, Rule,
};

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

pub fn build_ast(
    mut pairs: Pairs,
    extensions: Extensions,
) -> (Ast, Vec<ParserError>, Vec<ParserWarning>) {
    let mut walker = Walker {
        context: Context::<ParserError, ParserWarning>::default(),
        extensions,
    };

    let pair = pairs.next().expect("Empty root pairs");
    debug_assert!(pairs.next().is_none());

    let ast = walker.cooklang(pair);

    (ast, walker.context.errors, walker.context.warnings)
}

struct Walker {
    context: Context<ParserError, ParserWarning>,
    extensions: Extensions,
}

impl_deref_context!(Walker, ParserError, ParserWarning);

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
                Rule::step => lines.push(Line::Step(self.step(pair))),
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
            .transform(str::trim);
        let value = pairs
            .next()
            .expect("No value in metadata")
            .as_located_str()
            .transform(str::trim);

        assert!(!key.is_empty(), "Key is empty");
        if value.is_empty() {
            self.warn(ParserWarning::EmptyMetadataValue {
                key: key.to_string(),
                position: value.offset(),
            });
        }

        (key, value)
    }

    fn step<'a>(&mut self, pair: Pair<'a>) -> Vec<Item<'a>> {
        is_rule!(pair, Rule::step);
        let mut items = Vec::new();

        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::component => {
                    let (component, extra_text) = self.component(pair);
                    items.push(Item::Component(Box::new(component)));
                    if let Some(t) = extra_text {
                        items.push(Item::Text(t));
                    }
                }
                Rule::text => items.push(Item::Text(Located::new(pair.text(), pair.span()))),
                _ => unexpected_ignore!(pair),
            }
        }

        items
    }

    fn section<'a>(&mut self, pair: Pair<'a>) -> Option<Cow<'a, str>> {
        is_rule!(pair, Rule::section);

        if !self.extensions.contains(Extensions::SECTIONS) {
            self.error(ParserError::ExtensionNotEnabled {
                span: pair.span(),
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

        let span = pair.span();

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
                    quantity = Some(Located::from(pair).transform(|p| self.quantity(p)))
                }
                Rule::note => note = Some(pair.located_text()),
                _ => unexpected_ignore!(pair),
            }
        }

        let mut c = GenericComponent {
            span: span.clone(),
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
                    span: modifiers.span(),
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
                    name.span().start..alias.span().end,
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
            .map(|p| p.transform(|p| self.modifiers(p)))
            .unwrap_or_else(Recover::recover);

        let name = component.name.unwrap_or_else(Recover::recover);

        if name.is_empty() {
            self.error(ParserError::ComponentPartMissing {
                component_kind: ComponentKind::Ingredient.into(),
                what: "name",
                component_span: component.span,
            });
        }

        let alias = component.alias;
        if let Some(alias) = &alias {
            if alias.is_empty() {
                self.error(ParserError::ComponentPartNotAllowed {
                    component_kind: ComponentKind::Ingredient.into(),
                    what: "empty alias",
                    to_remove: alias.offset() - 1..alias.offset(),
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
                component_kind: ComponentKind::Cookware.into(),
                what: "modifiers",
                to_remove: modifiers.span(),
                help: Some("Modifiers are only available in ingredients"),
            });
        }

        if let Some(alias) = component.alias {
            self.error(ParserError::ComponentPartNotAllowed {
                component_kind: ComponentKind::Cookware.into(),
                what: "alias",
                to_remove: alias.span(),
                help: Some("Aliases are only available in ingredients"),
            });
        }

        if let Some(note) = component.note {
            self.error(ParserError::ComponentPartNotAllowed {
                component_kind: ComponentKind::Cookware.into(),
                what: "note",
                to_remove: note.span(),
                help: Some("Notes are only available in ingredients"),
            });
        }

        let name = component.name.unwrap_or_else(Recover::recover);
        if name.is_empty() {
            self.error(ParserError::ComponentPartMissing {
                component_kind: ComponentKind::Cookware.into(),
                what: "name",
                component_span: component.span,
            });
        }

        let quantity = component.quantity.map(|quantity| {
            if let Some(unit) = &quantity.unit {
                self.error(ParserError::ComponentPartNotAllowed {
                    component_kind: ComponentKind::Cookware.into(),
                    what: "unit in quantity",
                    to_remove: quantity.value.span().start..unit.span().end,
                    help: Some("Cookware quantity can't have an unit"),
                });
            }
            quantity.take().value
        });

        Cookware { name, quantity }
    }

    fn timer<'a>(&mut self, component: GenericComponent<'a>) -> Timer<'a> {
        if let Some(modifiers) = component.modifiers {
            self.error(ParserError::ComponentPartNotAllowed {
                component_kind: ComponentKind::Timer.into(),
                what: "modifiers",
                to_remove: modifiers.span(),
                help: Some("Modifiers are only available in ingredients"),
            });
        }

        if let Some(alias) = component.alias {
            self.error(ParserError::ComponentPartNotAllowed {
                component_kind: ComponentKind::Timer.into(),
                what: "alias",
                to_remove: alias.span(),
                help: Some("Aliases are only available in ingredients"),
            });
        }

        if let Some(note) = component.note {
            self.error(ParserError::ComponentPartNotAllowed {
                component_kind: ComponentKind::Timer.into(),
                what: "note",
                to_remove: note.span(),
                help: Some("Notes are only available in ingredients"),
            });
        }

        let name = component.name;
        let quantity = component.quantity.unwrap_or_else(|| {
            self.error(ParserError::ComponentPartMissing {
                component_kind: ComponentKind::Timer.into(),
                what: "quantity",
                component_span: component.span.clone(),
            });
            Recover::recover()
        });
        if quantity.unit.is_none() {
            self.error(ParserError::ComponentPartMissing {
                component_kind: ComponentKind::Timer.into(),
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
                self.errors.push(ParserError::InvalidModifiers {
                    modifiers_span: pair.span(),
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
            self.errors.push(ParserError::InvalidModifiers {
                modifiers_span: pair.span(),
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
        let mut value = inner
            .next()
            .expect("No value in quantity")
            .located()
            .transform(|p| self.value(p));
        let has_separator = if let Some(Rule::unit_separator) = inner.peek().map(|p| p.as_rule()) {
            inner.next();
            true
        } else {
            false
        };

        let mut unit = inner.next().map(|p| p.located_text_trimmed());
        debug_assert!(inner.next().is_none());
        if !self.extensions.contains(Extensions::ADVANCED_UNITS) && !has_separator && unit.is_some()
        {
            value = pair.located().transform(|p| Value::Text(p.text_trimmed()));
            unit = None;
        };
        Quantity { value, unit }
    }

    fn value<'a>(&mut self, pair: Pair<'a>) -> Value<'a> {
        is_rule!(pair, Rule::value);
        let pair = pair.first_inner();
        match pair.as_rule() {
            Rule::mixed_number => Value::Number(self.recover(mixed_number(pair))),
            Rule::fraction => Value::Number(self.recover(fraction(pair))),
            Rule::range => Value::Range(self.recover_val(range(pair), 1.0..=1.0)),
            Rule::number => Value::Number(self.recover(number(pair))),
            Rule::value_text => Value::Text(pair.text_trimmed()),
            _ => panic!("unexpected pair inside value"),
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
        _ => panic!("unexpected pair inside number"),
    }
}

fn integer(pair: Pair) -> Result<f64, ParserError> {
    pair.as_str()
        .parse::<i32>()
        .map(|n| n as f64)
        .map_err(|e| ParserError::ParseInt {
            bad_bit: pair.span(),
            source: e,
        })
}

fn float(pair: Pair) -> Result<f64, ParserError> {
    pair.as_str()
        .parse::<f64>()
        .map_err(|e| ParserError::ParseFloat {
            bad_bit: pair.span(),
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
                    bad_bit: den_pair.span(),
                });
            }

            num / den
        }
        Rule::unicode_fraction => unicode_fraction(pair.as_str()),
        _ => panic!("Unexpected rule inside fraction"),
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
    span: Range<usize>,
    modifiers: Option<Located<Pair<'a>>>,
    name: Option<Located<Cow<'a, str>>>,
    alias: Option<Located<Cow<'a, str>>>,
    quantity: Option<Located<Quantity<'a>>>,
    note: Option<Located<Cow<'a, str>>>,
}

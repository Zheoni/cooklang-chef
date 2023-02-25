use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use regex::Regex;

use crate::convert::{Converter, PhysicalQuantity};
use crate::metadata::Metadata;
use crate::parser::ast::{self, Modifiers};
use crate::parser::located::{Located, OptTake};
use crate::quantity::{MaybeUnit, Quantity};
use crate::{context::Context, impl_deref_context};
use crate::{model::*, Extensions};

use super::{AnalysisError, AnalysisWarning};

#[derive(Default, Debug)]
pub struct RecipeContent<'a> {
    pub metadata: Metadata<'a>,
    pub sections: Vec<Section<'a>>,
    pub ingredients: Vec<Rc<Ingredient<'a>>>,
    pub cookware: Vec<Rc<Cookware<'a>>>,
    pub timers: Vec<Rc<Timer<'a>>>,
}

pub fn parse_ast<'a>(
    ast: ast::Ast<'a>,
    extensions: Extensions,
    converter: Option<&Converter>,
) -> (RecipeContent<'a>, Context<AnalysisError, AnalysisWarning>) {
    let mut context = Context::default();
    let temperature_regex = converter.as_ref().and_then(|converter| {
        extensions
            .contains(Extensions::TEMPERATURE)
            .then(|| match converter.temperature_regex() {
                Ok(re) => Some(re),
                Err(source) => {
                    context.warn(AnalysisWarning::TemperatureRegexCompile { source });
                    None
                }
            })
            .flatten()
    });

    let mut walker = Walker {
        extensions,
        temperature_regex,
        converter,

        content: Default::default(),

        define_mode: DefineMode::All,
        duplicate_mode: DuplicateMode::New,
        context,

        ingredient_locations: HashMap::default(),
    };

    walker.ast(ast);

    (walker.content, walker.context)
}

struct Walker<'a, 'c> {
    extensions: Extensions,
    temperature_regex: Option<&'c Regex>,
    converter: Option<&'c Converter>,

    content: RecipeContent<'a>,

    define_mode: DefineMode,
    duplicate_mode: DuplicateMode,
    context: Context<AnalysisError, AnalysisWarning>,

    ingredient_locations: HashMap<*const Ingredient<'a>, Range<usize>>,
}

#[derive(PartialEq)]
enum DefineMode {
    All,
    Components,
    Steps,
    Text,
}

#[derive(PartialEq)]
enum DuplicateMode {
    New,
    Reference,
}

impl_deref_context!(Walker<'_, '_>, AnalysisError, AnalysisWarning);

impl<'a, 'r> Walker<'a, 'r> {
    fn ast(&mut self, ast: ast::Ast<'a>) {
        let mut current_section = Section::default();
        let mut continue_last_step = false;

        for line in ast.lines {
            match line {
                ast::Line::Metadata { key, value } => self.metadata(key, value),
                ast::Line::Step(items) => {
                    let new_step = self.step(items);

                    // If define mode is ingredients, don't add the
                    // step to the section. The components should have been
                    // added to their lists
                    if self.define_mode != DefineMode::Components {
                        if continue_last_step && !current_section.steps.is_empty() {
                            let last_step = current_section.steps.last_mut().unwrap();
                            last_step.items.push(Item::Text(" ".into()));
                            last_step.items.extend(new_step.items);
                        } else {
                            current_section.steps.push(new_step);
                        }
                    }
                }
                ast::Line::Section { name } => {
                    if !current_section.is_empty() {
                        self.content.sections.push(current_section);
                    }
                    current_section = Section::new(name);
                }
                ast::Line::SoftBreak => {
                    if self.extensions.contains(Extensions::MULTINE_STEPS) {
                        continue_last_step = true;
                        continue; // skip set to false
                    }
                }
            }
            continue_last_step = false;
        }
        if !current_section.is_empty() {
            self.content.sections.push(current_section);
        }
    }

    fn metadata(&mut self, key: Located<&'a str>, value: Located<&'a str>) {
        if self.extensions.contains(Extensions::MODES) && key.starts_with('[') && key.ends_with(']')
        {
            let special_key = &key[1..key.len() - 1];
            match special_key {
                "define" | "mode" => match value.as_ref() {
                    "all" | "default" => self.define_mode = DefineMode::All,
                    "components" | "ingredients" => self.define_mode = DefineMode::Components,
                    "steps" => self.define_mode = DefineMode::Steps,
                    "text" => self.define_mode = DefineMode::Text,
                    _ => self.error(AnalysisError::InvalidSpecialMetadataValue {
                        key: special_key.to_string(),
                        value: value.to_string(),
                        key_span: key.span(),
                        value_span: value.span(),
                        possible_values: vec!["all", "components", "steps", "text"],
                    }),
                },
                "duplicate" => match value.as_ref() {
                    "new" | "default" => self.duplicate_mode = DuplicateMode::New,
                    "reference" | "ref" => self.duplicate_mode = DuplicateMode::Reference,
                    _ => self.error(AnalysisError::InvalidSpecialMetadataValue {
                        key: special_key.to_string(),
                        value: value.to_string(),
                        key_span: key.span(),
                        value_span: value.span(),
                        possible_values: vec!["new", "reference"],
                    }),
                },
                _ => self.warn(AnalysisWarning::UnknownSpecialMetadataKey {
                    key: key.to_string(),
                    key_span: key.span(),
                }),
            }
        } else if let Err(warn) = self.content.metadata.insert(key.get(), value.get()) {
            self.warn(AnalysisWarning::InvalidMetadataValue {
                key: key.to_string(),
                value: value.to_string(),
                key_span: key.span(),
                value_span: value.span(),
                source: warn,
            });
        }
    }

    fn step(&mut self, items: Vec<ast::Item<'a>>) -> Step<'a> {
        let mut new_items = Vec::new();

        for item in items {
            match item {
                ast::Item::Text(text) => {
                    if self.define_mode == DefineMode::Components {
                        // only issue warnings for alphanumeric characters
                        // so that the user can format the text with spaces,
                        // hypens or whatever.
                        if text.contains(|c: char| c.is_alphanumeric()) {
                            self.warn(AnalysisWarning::TextDefiningIngredients {
                                text_span: text.span(),
                            });
                        }

                        continue; // ignore text
                    }

                    let text = text.take();

                    if let Some(re) = &self.temperature_regex {
                        if let Some((before, temperature, after)) =
                            find_temperature(text.clone(), re)
                        {
                            if !before.is_empty() {
                                new_items.push(Item::Text(before));
                            }
                            new_items.push(Item::Temperature(temperature));
                            if !after.is_empty() {
                                new_items.push(Item::Text(after));
                            }
                            continue;
                        }
                    }

                    new_items.push(Item::Text(text.clone()));
                }
                ast::Item::Component(c) => {
                    if self.define_mode == DefineMode::Text {
                        self.warn(AnalysisWarning::ComponentInTextMode {
                            component_span: c.span(),
                        });
                        continue; // ignore component
                    }
                    let new_component = self.component(c);
                    new_items.push(Item::Component(new_component))
                }
            };
        }

        Step {
            items: new_items,
            is_text: self.define_mode == DefineMode::Text,
        }
    }

    fn component(&mut self, component: Box<Located<ast::Component<'a>>>) -> Component<'a> {
        let (inner, span) = component.take_pair();

        match inner {
            ast::Component::Ingredient(i) => {
                Component::Ingredient(self.ingredient(Located::new(i, span)))
            }
            ast::Component::Cookware(c) => {
                Component::Cookware(self.cookware(Located::new(c, span)))
            }
            ast::Component::Timer(t) => Component::Timer(self.timer(Located::new(t, span))),
        }
    }

    fn ingredient(&mut self, ingredient: Located<ast::Ingredient<'a>>) -> Rc<Ingredient<'a>> {
        let (ingredient, location) = ingredient.take_pair();

        let same_name = self
            .content
            .ingredients
            .iter()
            // find the LAST ingredient with the same name
            .rfind(|igr| {
                !igr.modifiers.contains(Modifiers::REF) && igr.name == ingredient.name.as_ref()
            })
            .cloned(); // Rc clone

        let mut new_igr = Ingredient {
            name: ingredient.name.take(),
            alias: ingredient.alias.opt_take(),
            quantity: ingredient
                .quantity
                .clone()
                .opt_take()
                .map(Quantity::from_ast),
            note: ingredient.note.opt_take(),
            modifiers: ingredient.modifiers.take(),
            referenced_from: Default::default(),
        };

        let treat_as_reference = (new_igr.modifiers.contains(Modifiers::REF)
            || self.define_mode == DefineMode::Steps
            || same_name.is_some() && self.duplicate_mode == DuplicateMode::Reference)
            && !new_igr.modifiers.contains(Modifiers::NEW);

        let mut references_to = None;
        if treat_as_reference {
            new_igr.modifiers |= Modifiers::REF; // mark as ref if not marked before

            if let Some(referenced) = same_name {
                // only the parent or the reference(s), not both because it can cause
                // confusion when calcualting the total amount
                if referenced.quantity.is_some() && new_igr.quantity.is_some() {
                    let definition_span =
                        self.ingredient_locations[&Rc::as_ptr(&referenced)].clone();
                    self.error(AnalysisError::ConflictingReferenceQuantities {
                        ingredient_name: new_igr.name.to_string(),
                        definition_span,
                        reference_span: location.clone(),
                    });
                }
                references_to = Some(referenced);
            } else {
                self.error(AnalysisError::ReferenceNotFound {
                    name: new_igr.name.to_string(),
                    reference_span: location.clone(),
                });
            }

            // a text value can't be processed when calculating the total sum of
            // all ingredient references. valid, but not optimal
            if matches!(
                new_igr.quantity,
                Some(Quantity {
                    value: crate::quantity::Value::Text(_),
                    ..
                })
            ) {
                self.warn(AnalysisWarning::TextValueInReference {
                    quantity_span: ingredient.quantity.unwrap().span(),
                });
            }
        }

        let new_igr = Rc::new(new_igr);
        self.ingredient_locations
            .insert(Rc::as_ptr(&new_igr), location);
        if let Some(referenced) = references_to {
            referenced
                .referenced_from
                .borrow_mut()
                .push(Rc::clone(&new_igr));
        } else {
            self.content.ingredients.push(Rc::clone(&new_igr));
        }

        new_igr
    }

    fn cookware(&mut self, cookware: Located<ast::Cookware<'a>>) -> Rc<Cookware<'a>> {
        let (cookware, _span) = cookware.take_pair();

        let new_cookware = Cookware {
            name: cookware.name.take(),
            quantity: cookware.quantity.opt_take(),
        };

        let new_cookware = Rc::new(new_cookware);
        self.content.cookware.push(Rc::clone(&new_cookware));
        new_cookware
    }

    fn timer(&mut self, timer: Located<ast::Timer<'a>>) -> Rc<Timer<'a>> {
        let (timer, span) = timer.take_pair();

        let quantity = Quantity::from_ast(timer.quantity.take());
        if self.extensions.contains(Extensions::ADVANCED_UNITS) && self.converter.is_some() {
            if let Some(unit) = quantity.unit() {
                match unit.unit_or_parse(self.converter.unwrap()) {
                    MaybeUnit::Known(unit) => {
                        if unit.physical_quantity != PhysicalQuantity::Time {
                            self.error(AnalysisError::BadTimerUnit {
                                unit: unit.as_ref().clone(),
                                timer_span: span,
                            })
                        }
                    }
                    MaybeUnit::Unknown => self.error(AnalysisError::UnknownTimerUnit {
                        unit: unit.text().to_string(),
                        timer_span: span,
                    }),
                }
            }
        }

        let new_timer = Timer {
            name: timer.name.opt_take(),
            quantity,
        };

        let new_timer = Rc::new(new_timer);
        self.content.timers.push(Rc::clone(&new_timer));
        new_timer
    }
}

fn find_temperature<'a>(
    text: Cow<'a, str>,
    re: &Regex,
) -> Option<(Cow<'a, str>, Quantity<'a>, Cow<'a, str>)> {
    let Some(caps) = re.captures(&text) else { return None; };

    let value = caps[1].replace(',', ".").parse::<f64>().ok()?;
    let unit = caps.get(3).unwrap().range();
    let unit_text = match &text {
        Cow::Borrowed(s) => s[unit].into(),
        Cow::Owned(s) => s[unit].to_owned().into(),
    };
    let temperature = Quantity::new(value.into(), Some(unit_text));

    let range = caps.get(0).unwrap().range();
    let (before, after) = match &text {
        Cow::Borrowed(s) => (
            Cow::Borrowed(&s[..range.start]),
            Cow::Borrowed(&s[range.end..]),
        ),
        Cow::Owned(s) => (
            Cow::Owned(s[..range.start].to_owned()),
            Cow::Owned(s[range.end..].to_owned()),
        ),
    };

    Some((before, temperature, after))
}

use std::borrow::Cow;
use std::collections::HashMap;

use regex::Regex;

use crate::ast::{self, Modifiers, Text};
use crate::context::Context;
use crate::convert::{Converter, PhysicalQuantity};
use crate::located::Located;
use crate::metadata::Metadata;
use crate::quantity::{Quantity, QuantityValue, UnitInfo, Value};
use crate::span::Span;
use crate::{model::*, Extensions, RecipeRefChecker};

use super::{AnalysisError, AnalysisResult, AnalysisWarning};

#[derive(Default, Debug)]
pub struct RecipeContent {
    pub metadata: Metadata,
    pub sections: Vec<Section>,
    pub ingredients: Vec<Ingredient>,
    pub cookware: Vec<Cookware>,
    pub timers: Vec<Timer>,
    pub inline_quantities: Vec<Quantity>,
}

#[tracing::instrument(level = "debug", skip_all, target = "cooklang::analysis", fields(ast_lines = ast.lines.len()))]
pub fn parse_ast<'a>(
    ast: ast::Ast<'a>,
    extensions: Extensions,
    converter: &Converter,
    recipe_ref_checker: Option<RecipeRefChecker>,
) -> AnalysisResult {
    let mut context = Context::default();
    let temperature_regex = extensions
        .contains(Extensions::TEMPERATURE)
        .then(|| match converter.temperature_regex() {
            Ok(re) => Some(re),
            Err(source) => {
                context.warn(AnalysisWarning::TemperatureRegexCompile { source });
                None
            }
        })
        .flatten();

    let mut walker = Walker {
        extensions,
        temperature_regex,
        converter,
        recipe_ref_checker,

        content: Default::default(),

        define_mode: DefineMode::All,
        duplicate_mode: DuplicateMode::New,
        auto_scale_ingredients: false,
        context,

        ingredient_locations: Default::default(),
        metadata_locations: Default::default(),
    };

    walker.ast(ast);

    walker.context.finish(Some(walker.content))
}

struct Walker<'a, 'c> {
    extensions: Extensions,
    temperature_regex: Option<&'c Regex>,
    converter: &'c Converter,
    recipe_ref_checker: Option<RecipeRefChecker<'c>>,

    content: RecipeContent,

    define_mode: DefineMode,
    duplicate_mode: DuplicateMode,
    auto_scale_ingredients: bool,
    context: Context<AnalysisError, AnalysisWarning>,

    ingredient_locations: Vec<Located<ast::Ingredient<'a>>>,
    metadata_locations: HashMap<Cow<'a, str>, (Text<'a>, Text<'a>)>,
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

crate::context::impl_deref_context!(Walker<'_, '_>, AnalysisError, AnalysisWarning);

impl<'a, 'r> Walker<'a, 'r> {
    fn ast(&mut self, ast: ast::Ast<'a>) {
        let mut current_section = Section::default();

        for line in ast.lines {
            match line {
                ast::Line::Metadata { key, value } => self.metadata(key, value),
                ast::Line::Step { is_text, items } => {
                    let new_step = self.step(is_text, items);

                    // If define mode is ingredients, don't add the
                    // step to the section. The components should have been
                    // added to their lists
                    if self.define_mode != DefineMode::Components {
                        current_section.steps.push(new_step);
                    }
                }
                ast::Line::Section { name } => {
                    if !current_section.is_empty() {
                        self.content.sections.push(current_section);
                    }
                    current_section = Section::new(name.map(|t| t.text_trimmed().into_owned()));
                }
            }
        }
        if !current_section.is_empty() {
            self.content.sections.push(current_section);
        }
    }

    fn metadata(&mut self, key: Text<'a>, value: Text<'a>) {
        self.metadata_locations
            .insert(key.text_trimmed(), (key.clone(), value.clone()));

        let invalid_value = |possible_values| AnalysisError::InvalidSpecialMetadataValue {
            key: key.located_string(),
            value: value.located_string(),
            possible_values,
        };

        let key_t = key.text_trimmed();
        let value_t = value.text_trimmed();
        if self.extensions.contains(Extensions::MODES)
            && key_t.starts_with('[')
            && key_t.ends_with(']')
        {
            let special_key = &key_t[1..key_t.len() - 1];
            match special_key {
                "define" | "mode" => match value_t.as_ref() {
                    "all" | "default" => self.define_mode = DefineMode::All,
                    "components" | "ingredients" => self.define_mode = DefineMode::Components,
                    "steps" => self.define_mode = DefineMode::Steps,
                    "text" => self.define_mode = DefineMode::Text,
                    _ => self.error(invalid_value(vec!["all", "components", "steps", "text"])),
                },
                "duplicate" => match value_t.as_ref() {
                    "new" | "default" => self.duplicate_mode = DuplicateMode::New,
                    "reference" | "ref" => self.duplicate_mode = DuplicateMode::Reference,
                    _ => self.error(invalid_value(vec!["new", "reference"])),
                },
                "auto scale" | "auto_scale" => match value_t.as_ref() {
                    "true" => self.auto_scale_ingredients = true,
                    "false" | "default" => self.auto_scale_ingredients = false,
                    _ => self.error(invalid_value(vec!["true", "false"])),
                },
                _ => self.warn(AnalysisWarning::UnknownSpecialMetadataKey {
                    key: key.located_string(),
                }),
            }
        } else if let Err(warn) = self
            .content
            .metadata
            .insert(key_t.into_owned(), value_t.into_owned())
        {
            self.warn(AnalysisWarning::InvalidMetadataValue {
                key: key.located_string(),
                value: value.located_string(),
                source: warn,
            });
        }
    }

    fn step(&mut self, is_text: bool, items: Vec<ast::Item<'a>>) -> Step {
        let mut new_items = Vec::new();

        let is_text = is_text || self.define_mode == DefineMode::Text;

        for item in items {
            match item {
                ast::Item::Text(text) => {
                    let t = text.text();
                    if self.define_mode == DefineMode::Components {
                        // only issue warnings for alphanumeric characters
                        // so that the user can format the text with spaces,
                        // hypens or whatever.
                        if t.contains(|c: char| c.is_alphanumeric()) {
                            self.warn(AnalysisWarning::TextDefiningIngredients {
                                text_span: text.span(),
                            });
                        }
                        continue; // ignore text
                    }

                    if let Some(re) = &self.temperature_regex {
                        if let Some((before, temperature, after)) = find_temperature(&t, re) {
                            if !before.is_empty() {
                                new_items.push(Item::Text(before.to_string()));
                            }
                            new_items
                                .push(Item::InlineQuantity(self.content.inline_quantities.len()));
                            self.content.inline_quantities.push(temperature);
                            if !after.is_empty() {
                                new_items.push(Item::Text(after.to_string()));
                            }
                            continue;
                        }
                    }

                    new_items.push(Item::Text(t.into_owned()));
                }
                ast::Item::Component(c) => {
                    if is_text {
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
            is_text,
        }
    }

    fn component(&mut self, component: Box<Located<ast::Component<'a>>>) -> Component {
        let (inner, span) = component.take_pair();

        match inner {
            ast::Component::Ingredient(i) => Component {
                kind: ComponentKind::Ingredient,
                index: self.ingredient(Located::new(i, span)),
            },
            ast::Component::Cookware(c) => Component {
                kind: ComponentKind::Cookware,
                index: self.cookware(Located::new(c, span)),
            },
            ast::Component::Timer(t) => Component {
                kind: ComponentKind::Timer,
                index: self.timer(Located::new(t, span)),
            },
        }
    }

    fn ingredient(&mut self, ingredient: Located<ast::Ingredient<'a>>) -> usize {
        let located_ingredient = ingredient.clone();
        let (ingredient, location) = ingredient.take_pair();

        let name = ingredient.name.text_trimmed();

        let same_name = self
            .content
            .ingredients
            .iter()
            // find the LAST ingredient with the same name
            .rposition(|igr| {
                !igr.modifiers.contains(Modifiers::REF)
                    && igr.name.to_lowercase() == name.to_lowercase()
            });

        let mut new_igr = Ingredient {
            name: name.into_owned(),
            alias: ingredient.alias.map(|t| t.text_trimmed().into_owned()),
            quantity: ingredient.quantity.clone().map(|q| self.quantity(q, true)),
            note: ingredient.note.map(|n| n.text_trimmed().into_owned()),
            modifiers: ingredient.modifiers.clone().take(),
            references_to: None,
            referenced_from: Default::default(),
            defined_in_step: self.define_mode != DefineMode::Components,
        };

        if new_igr.modifiers.contains(Modifiers::RECIPE) {
            if let Some(checker) = &self.recipe_ref_checker {
                if !(*checker)(&new_igr.name) {
                    self.warn(AnalysisWarning::RecipeNotFound {
                        ref_span: location,
                        name: new_igr.name.clone(),
                    });
                }
            }
        }

        if (self.duplicate_mode == DuplicateMode::Reference
            || self.define_mode == DefineMode::Steps)
            && new_igr.modifiers.contains(Modifiers::REF)
        {
            self.warn(AnalysisWarning::RedundantReferenceModifier {
                modifiers: ingredient.modifiers.clone(),
            });
        }

        let treat_as_reference = !new_igr.modifiers.contains(Modifiers::NEW)
            && (new_igr.modifiers.contains(Modifiers::REF)
                || self.define_mode == DefineMode::Steps
                || same_name.is_some() && self.duplicate_mode == DuplicateMode::Reference);

        if treat_as_reference {
            new_igr.modifiers |= Modifiers::REF; // mark as ref if not marked before

            if let Some(referenced_index) = same_name {
                let referenced = &self.content.ingredients[referenced_index];

                // inherit hidden and optional from definition
                new_igr.modifiers |= referenced.modifiers & (Modifiers::HIDDEN | Modifiers::OPT);

                // When the ingredient is not defined in a step, only the definition
                // or the references can have quantities.
                // This is to avoid confusion when calculating the total amount.
                //  - If the user defines the ingredient in a ingredient list with
                //    a quantity and later references it with a quantity, what does
                //    the definition quantity mean? total? partial and the reference
                //    a portion used? Too messy. This situation is prohibited
                //  - If the user defines the ingredient directly in a step, it's
                //    quantity is used there, and the total is the sum of itself and
                //    all of its references. All clear.
                if referenced.quantity.is_some()
                    && new_igr.quantity.is_some()
                    && !referenced.defined_in_step
                {
                    let definition_span = self.ingredient_locations[referenced_index].span();
                    self.context
                        .error(AnalysisError::ConflictingReferenceQuantities {
                            ingredient_name: new_igr.name.to_string(),
                            definition_span,
                            reference_span: location,
                        });
                }

                if self.extensions.contains(Extensions::ADVANCED_UNITS) {
                    if let Some(new_quantity) = &new_igr.quantity {
                        let all_quantities = std::iter::once(referenced_index)
                            .chain(referenced.referenced_from.iter().copied())
                            .filter_map(|index| {
                                self.content.ingredients[index]
                                    .quantity
                                    .as_ref()
                                    .map(|q| (index, q))
                            });
                        for (index, q) in all_quantities {
                            if let Err(e) = q.is_compatible(new_quantity, self.converter) {
                                let old_q_loc =
                                    self.ingredient_locations[index].quantity.as_ref().unwrap();
                                let a = old_q_loc
                                    .unit
                                    .as_ref()
                                    .map(|l| l.span())
                                    .unwrap_or(old_q_loc.span());
                                let new_q_loc = located_ingredient.quantity.as_ref().unwrap();
                                let b = new_q_loc
                                    .unit
                                    .as_ref()
                                    .map(|l| l.span())
                                    .unwrap_or(new_q_loc.span());
                                self.context.warn(AnalysisWarning::IncompatibleUnits {
                                    a,
                                    b,
                                    source: e,
                                });
                            }
                        }
                    }
                }

                if referenced.modifiers.contains(Modifiers::RECIPE)
                    && !new_igr.modifiers.contains(Modifiers::RECIPE)
                {
                    self.context
                        .warn(AnalysisWarning::ReferenceToRecipeMissing {
                            modifiers: ingredient.modifiers,
                            ingredient_span: location,
                            referenced_span: self.ingredient_locations[referenced_index].span(),
                        })
                }

                new_igr.references_to = Some(referenced_index);
            } else {
                self.error(AnalysisError::ReferenceNotFound {
                    name: new_igr.name.to_string(),
                    reference_span: location,
                });
            }

            if let Some(quantity) = &new_igr.quantity {
                // a text value can't be processed when calculating the total sum of
                // all ingredient references. valid, but not optimal
                if quantity.value.contains_text_value() {
                    self.warn(AnalysisWarning::TextValueInReference {
                        quantity_span: ingredient.quantity.unwrap().span(),
                    });
                }
            }
        }

        // REF cannot appear in certain combinations
        if new_igr.modifiers.contains(Modifiers::REF)
            && new_igr
                .modifiers
                .intersects(Modifiers::NEW | Modifiers::HIDDEN | Modifiers::OPT)
        {
            self.error(AnalysisError::ConflictingModifiers {
                modifiers: located_ingredient.modifiers.clone(),
            });
            new_igr.modifiers = Modifiers::empty();
        }

        self.ingredient_locations.push(located_ingredient);
        let new_index = self.content.ingredients.len();
        if let Some(referenced_index) = new_igr.references_to {
            self.content.ingredients[referenced_index]
                .referenced_from
                .push(new_index)
        };
        self.content.ingredients.push(new_igr);

        new_index
    }

    fn cookware(&mut self, cookware: Located<ast::Cookware<'a>>) -> usize {
        let (cookware, _span) = cookware.take_pair();

        let new_cookware = Cookware {
            name: cookware.name.text_trimmed().into_owned(),
            quantity: cookware.quantity.map(|q| self.value(q.inner, false)),
        };

        self.content.cookware.push(new_cookware);
        self.content.cookware.len() - 1
    }

    fn timer(&mut self, timer: Located<ast::Timer<'a>>) -> usize {
        let located_timer = timer.clone();
        let (timer, span) = timer.take_pair();

        let quantity = self.quantity(timer.quantity, false);
        if self.extensions.contains(Extensions::ADVANCED_UNITS) {
            if let Some(unit) = quantity.unit() {
                match unit.unit_or_parse(self.converter) {
                    UnitInfo::Known(unit) => {
                        if unit.physical_quantity != PhysicalQuantity::Time {
                            self.error(AnalysisError::BadTimerUnit {
                                unit: unit.as_ref().clone(),
                                timer_span: located_timer.quantity.unit.as_ref().unwrap().span(),
                            })
                        }
                    }
                    UnitInfo::Unknown => self.error(AnalysisError::UnknownTimerUnit {
                        unit: unit.text().to_string(),
                        timer_span: span,
                    }),
                }
            }
        }

        let new_timer = Timer {
            name: timer.name.map(|t| t.text_trimmed().into_owned()),
            quantity,
        };

        self.content.timers.push(new_timer);
        self.content.timers.len() - 1
    }

    fn quantity(&mut self, quantity: Located<ast::Quantity<'a>>, is_ingredient: bool) -> Quantity {
        let ast::Quantity { value, unit, .. } = quantity.take();
        Quantity::new(
            self.value(value, is_ingredient),
            unit.map(|t| t.text_trimmed().into_owned()),
        )
    }

    fn value(&mut self, value: ast::QuantityValue, is_ingredient: bool) -> QuantityValue {
        if let ast::QuantityValue::Many(v) = &value {
            if let Some(s) = &self.content.metadata.servings {
                let servings_meta_span = self
                    .metadata_locations
                    .get("servings")
                    .map(|(_, value)| value.span());
                if s.len() != v.len() {
                    self.context
                        .error(AnalysisError::ScalableValueManyConflict {
                            reason: format!(
                                "{} servings defined but {} values in the quantity",
                                s.len(),
                                v.len()
                            )
                            .into(),
                            value_span: value.span(),
                            servings_meta_span,
                        });
                }
            } else {
                self.error(AnalysisError::ScalableValueManyConflict {
                    reason: format!("no servings defined but {} values in quantity", v.len())
                        .into(),
                    value_span: value.span(),
                    servings_meta_span: None,
                })
            }
        }
        let value_span = value.span();
        let mut value = QuantityValue::from_ast(value);

        if is_ingredient && self.auto_scale_ingredients {
            match value {
                QuantityValue::Fixed(v) => value = QuantityValue::Linear(v),
                QuantityValue::Linear(_) => {
                    self.warn(AnalysisWarning::RedundantAutoScaleMarker {
                        quantity_span: Span::new(value_span.end(), value_span.end() + 1),
                    });
                }
                _ => {}
            };
        }

        value
    }
}

fn find_temperature<'a>(text: &'a str, re: &Regex) -> Option<(&'a str, Quantity, &'a str)> {
    let Some(caps) = re.captures(&text) else { return None; };

    let value = caps[1].replace(',', ".").parse::<f64>().ok()?;
    let unit = caps.get(3).unwrap().range();
    let unit_text = text[unit].to_string();
    let temperature = Quantity::new(QuantityValue::Fixed(Value::Number(value)), Some(unit_text));

    let range = caps.get(0).unwrap().range();
    let (before, after) = (&text[..range.start], &text[range.end..]);

    Some((before, temperature, after))
}

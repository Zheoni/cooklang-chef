use std::borrow::Cow;

use regex::Regex;

use crate::context::Context;
use crate::convert::{Converter, PhysicalQuantity};
use crate::located::{Located, OptTake};
use crate::metadata::Metadata;
use crate::parser::ast::{self, Modifiers};
use crate::quantity::{Quantity, QuantityValue, ScalableValue, UnitInfo, Value};
use crate::span::Span;
use crate::{model::*, Extensions, RecipeRefChecker};

use super::{AnalysisError, AnalysisResult, AnalysisWarning};

#[derive(Default, Debug)]
pub struct RecipeContent<'a> {
    pub metadata: Metadata<'a>,
    pub sections: Vec<Section<'a>>,
    pub ingredients: Vec<Ingredient<'a>>,
    pub cookware: Vec<Cookware<'a>>,
    pub timers: Vec<Timer<'a>>,
}

#[tracing::instrument(skip_all, target = "cooklang::analysis", fields(ast_lines = ast.lines.len()))]
pub fn parse_ast<'a>(
    ast: ast::Ast<'a>,
    extensions: Extensions,
    converter: &Converter,
    recipe_ref_checker: Option<RecipeRefChecker>,
) -> AnalysisResult<'a> {
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
    };

    walker.ast(ast);

    walker.context.finish(Some(walker.content))
}

struct Walker<'a, 'c> {
    extensions: Extensions,
    temperature_regex: Option<&'c Regex>,
    converter: &'c Converter,
    recipe_ref_checker: Option<RecipeRefChecker<'c>>,

    content: RecipeContent<'a>,

    define_mode: DefineMode,
    duplicate_mode: DuplicateMode,
    auto_scale_ingredients: bool,
    context: Context<AnalysisError, AnalysisWarning>,

    ingredient_locations: Vec<Span>,
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
        let invalid_value = |possible_values| AnalysisError::InvalidSpecialMetadataValue {
            key: key.clone().map_inner(str::to_string),
            value: value.clone().map_inner(str::to_string),
            possible_values,
        };

        if self.extensions.contains(Extensions::MODES) && key.starts_with('[') && key.ends_with(']')
        {
            let special_key = &key[1..key.len() - 1];
            match special_key {
                "define" | "mode" => match value.as_ref() {
                    "all" | "default" => self.define_mode = DefineMode::All,
                    "components" | "ingredients" => self.define_mode = DefineMode::Components,
                    "steps" => self.define_mode = DefineMode::Steps,
                    "text" => self.define_mode = DefineMode::Text,
                    _ => self.error(invalid_value(vec!["all", "components", "steps", "text"])),
                },
                "duplicate" => match value.as_ref() {
                    "new" | "default" => self.duplicate_mode = DuplicateMode::New,
                    "reference" | "ref" => self.duplicate_mode = DuplicateMode::Reference,
                    _ => self.error(invalid_value(vec!["new", "reference"])),
                },
                "auto scale" | "auto_scale" => match value.as_ref() {
                    "true" => self.auto_scale_ingredients = true,
                    "false" | "default" => self.auto_scale_ingredients = false,
                    _ => self.error(invalid_value(vec!["true", "false"])),
                },
                _ => self.warn(AnalysisWarning::UnknownSpecialMetadataKey {
                    key: key.map_inner(str::to_string),
                }),
            }
        } else if let Err(warn) = self.content.metadata.insert(key.get(), value.get()) {
            self.warn(AnalysisWarning::InvalidMetadataValue {
                key: key.map_inner(str::to_string),
                value: value.map_inner(str::to_string),
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
                            new_items.push(Item::InlineQuantity(temperature));
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
        let (ingredient, location) = ingredient.take_pair();
        let location = Span::from(location);

        let same_name = self
            .content
            .ingredients
            .iter()
            // find the LAST ingredient with the same name
            .rposition(|igr| {
                !igr.modifiers.contains(Modifiers::REF)
                    && igr.name.to_lowercase() == ingredient.name.as_ref().to_lowercase()
            });

        let mut new_igr = Ingredient {
            name: ingredient.name.take(),
            alias: ingredient.alias.opt_take(),
            quantity: ingredient.quantity.clone().map(|q| self.quantity(q, true)),
            note: ingredient.note.opt_take(),
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
                        name: new_igr.name.to_string(),
                    })
                }
            }
        }

        if self.duplicate_mode == DuplicateMode::Reference
            && new_igr.modifiers.contains(Modifiers::REF)
        {
            self.warn(AnalysisWarning::RedundantReferenceModifier {
                modifiers: ingredient.modifiers.clone(),
            });
        }

        let treat_as_reference = (new_igr.modifiers.contains(Modifiers::REF)
            || self.define_mode == DefineMode::Steps
            || same_name.is_some() && self.duplicate_mode == DuplicateMode::Reference)
            && !new_igr.modifiers.contains(Modifiers::NEW);

        if treat_as_reference {
            new_igr.modifiers |= Modifiers::REF; // mark as ref if not marked before

            if let Some(referenced_index) = same_name {
                let referenced = &self.content.ingredients[referenced_index];
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
                    let definition_span = self.ingredient_locations[referenced_index].clone();
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
                                let a = self.ingredient_locations[index].clone().into();
                                let b = location.clone().into();
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
                            ingredient_span: location.into(),
                            referenced_span: self.ingredient_locations[referenced_index]
                                .clone()
                                .into(),
                        })
                }

                new_igr.references_to = Some(referenced_index);
            } else {
                self.error(AnalysisError::ReferenceNotFound {
                    name: new_igr.name.to_string(),
                    reference_span: location.clone(),
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

        self.ingredient_locations.push(location);
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
            name: cookware.name.take(),
            quantity: cookware.quantity.map(|q| self.value(q, false)),
        };

        self.content.cookware.push(new_cookware);
        self.content.cookware.len() - 1
    }

    fn timer(&mut self, timer: Located<ast::Timer<'a>>) -> usize {
        let (timer, span) = timer.take_pair();

        let quantity = self.quantity(timer.quantity, false);
        if self.extensions.contains(Extensions::ADVANCED_UNITS) {
            if let Some(unit) = quantity.unit() {
                match unit.unit_or_parse(self.converter) {
                    UnitInfo::Known(unit) => {
                        if unit.physical_quantity != PhysicalQuantity::Time {
                            self.error(AnalysisError::BadTimerUnit {
                                unit: unit.as_ref().clone(),
                                timer_span: span,
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
            name: timer.name.opt_take(),
            quantity,
        };

        self.content.timers.push(new_timer);
        self.content.timers.len() - 1
    }

    fn quantity(
        &mut self,
        quantity: Located<ast::Quantity<'a>>,
        is_ingredient: bool,
    ) -> Quantity<'a> {
        let ast::Quantity { value, unit } = quantity.take();
        Quantity::new(self.value(value, is_ingredient), unit.opt_take())
    }

    fn value(&mut self, value: ast::QuantityValue<'a>, is_ingredient: bool) -> QuantityValue<'a> {
        if let ast::QuantityValue::Many(v) = &value {
            if let Some(s) = &self.content.metadata.servings {
                if s.len() != v.len() {
                    self.context.error(AnalysisError::SacalingConflict {
                        reason: format!(
                            "{} servings defined but {} values in the quantity",
                            s.len(),
                            v.len()
                        )
                        .into(),
                        value_span: value.span(),
                    });
                }
            } else {
                self.error(AnalysisError::SacalingConflict {
                    reason: format!("no servings defined but {} values in quantity", v.len())
                        .into(),
                    value_span: value.span(),
                })
            }
        }
        let value_span = value.span();
        let mut value = QuantityValue::from_ast(value);

        if is_ingredient && self.auto_scale_ingredients {
            value = match value {
                QuantityValue::Fixed(v) => QuantityValue::Scalable(ScalableValue::Linear(v)),
                v @ QuantityValue::Scalable(ScalableValue::Linear(_)) => {
                    self.warn(AnalysisWarning::RedundantAutoScaleMarker {
                        quantity_span: Span::new(value_span.end(), value_span.end() + 1),
                    });
                    v
                }
                v @ QuantityValue::Scalable(ScalableValue::ByServings(_)) => v,
            };
        }

        value
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
    let temperature = Quantity::new(QuantityValue::Fixed(Value::Number(value)), Some(unit_text));

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

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{
    convert::Converter,
    metadata::Metadata,
    parser::ast::Modifiers,
    quantity::{Quantity, QuantityAddError, QuantityValue},
};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Recipe<'a, D = ()> {
    pub name: String,
    #[serde(borrow)]
    pub metadata: Metadata<'a>,
    pub sections: Vec<Section<'a>>,
    pub ingredients: Vec<Ingredient<'a>>,
    pub cookware: Vec<Cookware<'a>>,
    pub timers: Vec<Timer<'a>>,
    #[serde(skip)]
    pub(crate) data: D,
}

impl<'a> Recipe<'a> {
    pub(crate) fn from_content(name: String, content: crate::analysis::RecipeContent<'a>) -> Self {
        Recipe {
            name,
            metadata: content.metadata,
            sections: content.sections,
            ingredients: content.ingredients,
            cookware: content.cookware,
            timers: content.timers,
            data: (),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Section<'a> {
    pub name: Option<Cow<'a, str>>,
    pub steps: Vec<Step<'a>>,
}

impl<'a> Section<'a> {
    pub fn new(name: Option<Cow<'a, str>>) -> Section<'a> {
        Self {
            name,
            steps: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.steps.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Step<'a> {
    pub items: Vec<Item<'a>>,
    pub is_text: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Item<'a> {
    Text(Cow<'a, str>),
    Component(Component),
    InlineQuantity(Quantity<'a>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Ingredient<'a> {
    pub name: Cow<'a, str>,
    pub alias: Option<Cow<'a, str>>,
    pub quantity: Option<Quantity<'a>>,
    pub note: Option<Cow<'a, str>>,

    pub(crate) modifiers: Modifiers,
    pub(crate) references_to: Option<usize>,
    pub(crate) referenced_from: Vec<usize>,
    pub(crate) defined_in_step: bool,
}

impl Ingredient<'_> {
    pub fn display_name(&self) -> Cow<str> {
        let mut name = self.name.clone();
        if self.modifiers.contains(Modifiers::RECIPE) {
            if let Some(idx) = self.name.rfind(std::path::is_separator) {
                name = self.name.split_at(idx + 1).1.into();
            }
        }
        self.alias.as_ref().cloned().unwrap_or(name)
    }

    pub fn is_hidden(&self) -> bool {
        self.modifiers.contains(Modifiers::HIDDEN)
    }

    pub fn is_optional(&self) -> bool {
        self.modifiers.contains(Modifiers::OPT)
    }

    pub fn is_recipe(&self) -> bool {
        self.modifiers.contains(Modifiers::RECIPE)
    }

    pub fn is_reference(&self) -> bool {
        self.modifiers.contains(Modifiers::REF)
    }

    pub fn referenced_from(&self) -> &[usize] {
        &self.referenced_from
    }

    pub fn total_quantity<'a>(
        &'a self,
        all_ingredients: &'a [Self],
        converter: &Converter,
    ) -> Result<Option<Quantity>, QuantityAddError> {
        let mut quantities = self.all_quantities(all_ingredients);

        let Some(total) = quantities.next() else { return Ok(None); };
        let mut total = total.clone().into_owned();
        for q in quantities {
            total = total.try_add(q, converter)?;
        }

        Ok(Some(total))
    }

    pub fn all_quantities<'a>(
        &'a self,
        all_ingredients: &'a [Self],
    ) -> impl Iterator<Item = &Quantity> {
        std::iter::once(self.quantity.as_ref())
            .chain(
                self.referenced_from
                    .iter()
                    .copied()
                    .map(|i| all_ingredients[i].quantity.as_ref()),
            )
            .flatten()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Cookware<'a> {
    pub name: Cow<'a, str>,
    pub quantity: Option<QuantityValue<'a>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Timer<'a> {
    pub name: Option<Cow<'a, str>>,
    pub quantity: Quantity<'a>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Component {
    pub kind: ComponentKind,
    pub index: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum ComponentKind {
    Ingredient,
    Cookware,
    Timer,
}

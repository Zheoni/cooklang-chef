//! Recipe representation

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{
    ast::Modifiers,
    convert::Converter,
    metadata::Metadata,
    quantity::{Quantity, QuantityAddError, QuantityValue},
};

/// A complete recipe
///
/// A recipe can be [Self::scale] (only once) and only after that [Self::convert]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Recipe<'a, D = ()> {
    /// Recipe name
    pub name: String,
    /// Metadata
    #[serde(borrow)]
    pub metadata: Metadata<'a>,
    /// Each of the sections
    ///
    /// If no sections declared, a section without name
    /// is the default.
    pub sections: Vec<Section<'a>>,
    /// All the ingredients
    pub ingredients: Vec<Ingredient<'a>>,
    /// All the cookware
    pub cookware: Vec<Cookware<'a>>,
    /// All the timers
    pub timers: Vec<Timer<'a>>,
    /// All the inline quantities
    pub inline_quantities: Vec<Quantity<'a>>,
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
            inline_quantities: content.inline_quantities,

            data: (),
        }
    }
}

/// A section holding steps
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Section<'a> {
    /// Name of the section
    pub name: Option<Cow<'a, str>>,
    /// Steps inside
    pub steps: Vec<Step<'a>>,
}

impl<'a> Section<'a> {
    pub(crate) fn new(name: Option<Cow<'a, str>>) -> Section<'a> {
        Self {
            name,
            steps: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.steps.is_empty()
    }
}

/// A step holding step [Item]s
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Step<'a> {
    /// [Item]s inside
    pub items: Vec<Item<'a>>,
    /// Flag that indicates the step is a text step.
    ///
    /// A text step should not increase the step counter, and there are only
    /// text items inside.
    pub is_text: bool,
}

/// A step item
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Item<'a> {
    /// Just plain text
    Text(Cow<'a, str>),
    /// A [Component]
    Component(Component),
    /// An inline quantity.
    ///
    /// The number inside is an index into [Recipe::inline_quantities].
    InlineQuantity(usize),
}

/// A recipe ingredient
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Ingredient<'a> {
    /// Name
    ///
    /// This can have the form of a path if the ingredient references a recipe.
    pub name: Cow<'a, str>,
    /// Alias
    pub alias: Option<Cow<'a, str>>,
    /// Quantity
    pub quantity: Option<Quantity<'a>>,
    /// Note
    pub note: Option<Cow<'a, str>>,

    pub(crate) modifiers: Modifiers,
    pub(crate) references_to: Option<usize>,
    pub(crate) referenced_from: Vec<usize>,
    pub(crate) defined_in_step: bool, // TODO maybe move this into analysis?, is not needed in the model
}

impl Ingredient<'_> {
    /// Gets the name the ingredient should be displayed with
    pub fn display_name(&self) -> Cow<str> {
        let mut name = self.name.clone();
        if self.modifiers.contains(Modifiers::RECIPE) {
            if let Some(idx) = self.name.rfind(std::path::is_separator) {
                name = self.name.split_at(idx + 1).1.into();
            }
        }
        self.alias.as_ref().cloned().unwrap_or(name)
    }

    /// Access the ingredient modifiers
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
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

    /// Gets a list of the ingredients referencing this one.
    ///
    /// Returns a list of indices to [Recipe::ingredients].
    pub fn referenced_from(&self) -> &[usize] {
        &self.referenced_from
    }

    /// Calculates the total quantity adding all the quantities from the
    /// references.
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
        total.fit(converter);

        Ok(Some(total))
    }

    /// Gets an iterator over all quantities of this ingredient and its references.
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

/// A recipe cookware item
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Cookware<'a> {
    /// Name
    pub name: Cow<'a, str>,
    /// Amount needed
    ///
    /// Note that this is a value, not a quantity, so it doesn't have units.
    pub quantity: Option<QuantityValue<'a>>,
}

/// A recipe timer
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Timer<'a> {
    /// Name
    pub name: Option<Cow<'a, str>>,
    /// Time quantity
    ///
    /// If created from parsing and the advanced units extension is enabled,
    /// this is guaranteed to have a time unit.
    pub quantity: Quantity<'a>,
}

/// A component reference
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Component {
    /// What kind of component is
    pub kind: ComponentKind,
    /// The index in the corresponding [Vec] in the [Recipe] struct.
    pub index: usize,
}

/// Component kind used in [Component]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum ComponentKind {
    Ingredient,
    Cookware,
    Timer,
}

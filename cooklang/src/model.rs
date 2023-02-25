use std::{
    borrow::Cow,
    cell::{Ref, RefCell},
    rc::Rc,
};

use serde::{Deserialize, Serialize};

use crate::{
    metadata::Metadata,
    parser::ast::Modifiers,
    quantity::{Quantity, Value},
};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Recipe<'a> {
    pub name: String,
    #[serde(borrow)]
    pub metadata: Metadata<'a>,
    pub sections: Vec<Section<'a>>,
    pub ingredients: Vec<Rc<Ingredient<'a>>>,
    pub cookware: Vec<Rc<Cookware<'a>>>,
    pub timers: Vec<Rc<Timer<'a>>>,
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
    Component(Component<'a>),
    Temperature(Quantity<'a>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Ingredient<'a> {
    pub name: Cow<'a, str>,
    pub alias: Option<Cow<'a, str>>,
    pub quantity: Option<Quantity<'a>>,
    pub note: Option<Cow<'a, str>>,

    pub(crate) modifiers: Modifiers,
    pub(crate) referenced_from: RefCell<Vec<Rc<Self>>>,
}

impl Ingredient<'_> {
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

    pub fn referenced_from(&self) -> Ref<Vec<Rc<Self>>> {
        self.referenced_from.borrow()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Cookware<'a> {
    pub name: Cow<'a, str>,
    pub quantity: Option<Value<'a>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Timer<'a> {
    pub name: Option<Cow<'a, str>>,
    pub quantity: Quantity<'a>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Component<'a> {
    Ingredient(Rc<Ingredient<'a>>),
    Cookware(Rc<Cookware<'a>>),
    Timer(Rc<Timer<'a>>),
}

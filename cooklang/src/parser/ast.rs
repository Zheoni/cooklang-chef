use std::borrow::Cow;

use crate::{context::Recover, quantity::Value};

use super::located::Located;

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Ast<'a> {
    pub lines: Vec<Line<'a>>,
}

#[derive(Debug)]
pub enum Line<'a> {
    Metadata {
        key: Located<&'a str>,
        value: Located<&'a str>,
    },
    Step(Vec<Item<'a>>),
    Section {
        name: Option<Cow<'a, str>>,
    },
    SoftBreak,
}

#[derive(Debug)]
pub enum Item<'a> {
    Text(Located<Cow<'a, str>>),
    Component(Box<Located<Component<'a>>>),
}

#[derive(Debug)]
pub enum Component<'a> {
    Ingredient(Ingredient<'a>),
    Cookware(Cookware<'a>),
    Timer(Timer<'a>),
}

#[derive(Debug)]
pub struct Ingredient<'a> {
    pub modifiers: Located<Modifiers>,
    pub name: Located<Cow<'a, str>>,
    pub alias: Option<Located<Cow<'a, str>>>,
    pub quantity: Option<Located<Quantity<'a>>>,
    pub note: Option<Located<Cow<'a, str>>>,
}

#[derive(Debug)]
pub struct Cookware<'a> {
    pub name: Located<Cow<'a, str>>,
    pub quantity: Option<Located<Value<'a>>>,
}
#[derive(Debug)]
pub struct Timer<'a> {
    pub name: Option<Located<Cow<'a, str>>>,
    pub quantity: Located<Quantity<'a>>,
}

#[derive(Debug, Clone)]
pub struct Quantity<'a> {
    pub value: Located<Value<'a>>,
    pub unit: Option<Located<Cow<'a, str>>>,
}

impl Recover for Quantity<'_> {
    fn recover() -> Self {
        Self {
            value: Recover::recover(),
            unit: Recover::recover(),
        }
    }
}

impl Recover for Value<'_> {
    fn recover() -> Self {
        Self::Number(1.0)
    }
}

bitflags! {
    #[derive(Default, Serialize, Deserialize)]
    pub struct Modifiers: u32 {
        /// refers to a recipe with the same name
        const RECIPE = 0b00001;
        /// not shown in the ingredient list, only inline
        const HIDDEN = 0b00010;
        /// mark as optional
        const OPT    = 0b00100;
        /// references another igr with the same name, if amount given will sum
        const REF    = 0b01000;
        /// forces to create a new ingredient
        const NEW    = 0b10000;
    }
}

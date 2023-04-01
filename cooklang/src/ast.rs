use std::{borrow::Cow, fmt::Display};

use crate::{context::Recover, located::Located, quantity::Value, span::Span};

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Ast<'a> {
    pub lines: Vec<Line<'a>>,
}

#[derive(Debug, Serialize)]
pub enum Line<'a> {
    Metadata { key: Text<'a>, value: Text<'a> },
    Step { is_text: bool, items: Vec<Item<'a>> },
    Section { name: Option<Text<'a>> },
}

#[derive(Debug, Serialize)]
pub enum Item<'a> {
    Text(Text<'a>),
    Component(Box<Located<Component<'a>>>),
}

#[derive(Debug, Serialize)]
pub enum Component<'a> {
    Ingredient(Ingredient<'a>),
    Cookware(Cookware<'a>),
    Timer(Timer<'a>),
}

#[derive(Debug, Clone, Serialize)]
pub struct Ingredient<'a> {
    pub modifiers: Located<Modifiers>,
    pub name: Text<'a>,
    pub alias: Option<Text<'a>>,
    pub quantity: Option<Located<Quantity<'a>>>,
    pub note: Option<Text<'a>>,
}

#[derive(Debug, Serialize)]
pub struct Cookware<'a> {
    pub name: Text<'a>,
    pub quantity: Option<Located<QuantityValue<'a>>>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Timer<'a> {
    pub name: Option<Text<'a>>,
    pub quantity: Located<Quantity<'a>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Quantity<'a> {
    pub value: QuantityValue<'a>,
    pub unit: Option<Text<'a>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum QuantityValue<'a> {
    Single {
        value: Located<Value<'a>>,
        auto_scale: Option<Span>,
    },
    Many(Vec<Located<Value<'a>>>),
}

/* UTILITIES */
#[derive(Debug, Clone, Serialize)]
pub struct Text<'a> {
    offset: usize,
    //TODO Maybe a small vec in the stack? test it
    fragments: Vec<TextFragment<'a>>,
}

impl<'a> Text<'a> {
    pub(crate) fn empty(offset: usize) -> Self {
        Self {
            fragments: vec![],
            offset,
        }
    }

    pub(crate) fn append_fragment(&mut self, fragment: TextFragment<'a>) {
        assert_eq!(self.span().end(), fragment.offset);
        if !fragment.text.is_empty() {
            self.fragments.push(fragment);
        }
    }

    pub(crate) fn append_str(&mut self, s: &'a str) {
        self.append_fragment(TextFragment::new(s, self.span().end()))
    }

    pub fn span(&self) -> Span {
        if self.fragments.is_empty() {
            return Span::pos(self.offset);
        }
        let start = self.offset;
        let end = self.fragments.last().unwrap().end();
        Span::new(start, end)
    }

    pub fn text(&self) -> Cow<'a, str> {
        // TODO can be further optimized to avoid copies.
        // Contiguous text fragments may be joined together without a copy.

        let mut s = Cow::default();
        for f in &self.fragments {
            s += f.text;
        }
        s
    }

    pub fn text_trimmed(&self) -> Cow<'a, str> {
        match self.text() {
            Cow::Borrowed(s) => Cow::Borrowed(s.trim()),
            Cow::Owned(s) => Cow::Owned(s.trim().to_owned()),
        }
    }

    pub fn is_text_empty(&self) -> bool {
        self.fragments.iter().all(|f| f.text.trim().is_empty())
    }

    pub fn fragments(&self) -> &[TextFragment<'a>] {
        &self.fragments
    }

    pub fn located_str(&self) -> Located<Cow<str>> {
        Located::new(self.text_trimmed(), self.span())
    }

    pub fn located_string(&self) -> Located<String> {
        self.located_str().map_inner(Cow::into_owned)
    }
}

impl Display for Text<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text_trimmed())
    }
}

impl PartialEq for Text<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.fragments == other.fragments
    }
}

impl From<Text<'_>> for Span {
    fn from(value: Text<'_>) -> Self {
        value.span()
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct TextFragment<'a> {
    pub text: &'a str,
    offset: usize,
}

impl<'a> TextFragment<'a> {
    pub fn new(text: &'a str, offset: usize) -> Self {
        Self { text, offset }
    }

    pub fn span(&self) -> Span {
        Span::new(self.start(), self.end())
    }
    pub fn start(&self) -> usize {
        self.offset
    }
    pub fn end(&self) -> usize {
        self.offset + self.text.len()
    }
}

impl PartialEq for TextFragment<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Quantity<'_> {
    pub fn unit_span(&self) -> Option<Span> {
        Some(self.unit.as_ref()?.span())
    }
}

impl QuantityValue<'_> {
    pub fn span(&self) -> Span {
        match self {
            QuantityValue::Single {
                value, auto_scale, ..
            } => {
                let s = value.span();
                if let Some(marker) = auto_scale {
                    assert_eq!(s.end(), marker.start());
                    Span::new(s.start(), marker.end())
                } else {
                    s
                }
            }
            QuantityValue::Many(v) => {
                assert!(!v.is_empty(), "QuantityValue::Many with no values");
                let start = v.first().unwrap().span().start();
                let end = v.last().unwrap().span().end();
                Span::new(start, end)
            }
        }
    }
}

impl Recover for Text<'_> {
    fn recover() -> Self {
        Self::empty(0)
    }
}

impl Recover for Quantity<'_> {
    fn recover() -> Self {
        Self {
            value: Recover::recover(),
            unit: Recover::recover(),
        }
    }
}

impl Recover for QuantityValue<'_> {
    fn recover() -> Self {
        Self::Single {
            value: Recover::recover(),
            auto_scale: None,
        }
    }
}

impl Recover for Value<'_> {
    fn recover() -> Self {
        Self::Number(1.0)
    }
}

bitflags! {
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Modifiers: u32 {
        /// refers to a recipe with the same name
        const RECIPE = 0b00001;
        /// references another igr with the same name, if amount given will sum
        const REF    = 0b00010;
        /// not shown in the ingredient list, only inline
        const HIDDEN = 0b00100;
        /// mark as optional
        const OPT    = 0b01000;
        /// forces to create a new ingredient
        const NEW    = 0b10000;
    }
}

impl Modifiers {
    pub fn as_char(self) -> char {
        assert_eq!(self.bits().count_ones(), 1);
        match self {
            Self::RECIPE => '@',
            Self::HIDDEN => '-',
            Self::OPT => '?',
            Self::REF => '&',
            Self::NEW => '+',
            _ => panic!("Unknown modifier: {:?}", self),
        }
    }
}

use std::{borrow::Cow, fmt::Display, ops::Deref};

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
    SoftBreak,
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
    pub quantity: Option<Delimited<Quantity<'a>>>,
    pub note: Option<Delimited<Text<'a>>>,
}

#[derive(Debug, Serialize)]
pub struct Cookware<'a> {
    pub name: Text<'a>,
    pub quantity: Option<Delimited<QuantityValue<'a>>>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Timer<'a> {
    pub name: Option<Text<'a>>,
    pub quantity: Delimited<Quantity<'a>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Quantity<'a> {
    pub value: QuantityValue<'a>,
    pub unit_separator: Option<Span>,
    pub unit: Option<Text<'a>>,
}

#[derive(Debug, Clone, Serialize)]
pub enum QuantityValue<'a> {
    Single {
        value: Located<Value<'a>>,
        scalable: bool,
        auto_scale_marker: Option<Span>,
    },
    Many(Separated<Value<'a>>),
}

/* UTILITIES */
#[derive(Debug, Clone, Serialize)]
pub struct Text<'a> {
    offset: usize,
    //TODO Maybe a small vec in the stack? test it
    fragments: Vec<TextFragment<'a>>,
}

impl<'a> Text<'a> {
    pub fn empty(offset: usize) -> Self {
        Self {
            fragments: vec![],
            offset,
        }
    }

    pub(crate) fn new(offset: usize, fragments: Vec<TextFragment<'a>>) -> Self {
        Self { offset, fragments }
    }

    pub fn from_str(s: &'a str, offset: usize) -> Self {
        Self {
            offset,
            fragments: vec![TextFragment::text(s, offset)],
        }
    }

    pub fn push(&mut self, fragment: TextFragment<'a>) {
        self.fragments.push(fragment);
    }

    pub fn append(&mut self, mut other: Self) {
        assert_eq!(self.span().end(), other.span().start());
        self.fragments.append(&mut other.fragments)
    }

    pub fn append_fragment(&mut self, fragment: TextFragment<'a>) {
        assert_eq!(self.span().end(), fragment.offset);
        if !fragment.text.is_empty() {
            self.fragments.push(fragment);
        }
    }

    pub fn append_str(&mut self, s: &'a str) {
        self.append_fragment(TextFragment::text(s, self.span().end()))
    }
    pub fn append_line_comment(&mut self, s: &'a str) {
        self.append_fragment(TextFragment::line_comment(s, self.span().end()))
    }
    pub fn append_block_comment(&mut self, s: &'a str) {
        self.append_fragment(TextFragment::block_comment(s, self.span().end()))
    }

    pub fn span(&self) -> Span {
        if self.fragments.is_empty() {
            return Span::new(self.offset, self.offset);
        }
        let start = self.offset;
        let end = self.fragments.last().unwrap().end();
        Span::new(start, end)
    }

    pub fn text(&self) -> Cow<'a, str> {
        // TODO can be further optimized to avoid copies.
        // Contiguous text fragments may be joined together without a copy.

        let mut s = Cow::default();
        for f in self.text_fragments() {
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
        self.text_fragments().all(|f| f.text.trim().is_empty())
    }

    pub fn fragments(&self) -> &[TextFragment<'a>] {
        &self.fragments
    }

    pub fn text_fragments<'s>(&'s self) -> impl Iterator<Item = TextFragment<'a>> + 's {
        self.fragments
            .iter()
            .filter(|f| matches!(f.kind, TextFragmentKind::Text))
            .copied()
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
    offset: usize, // TODO can be calculated from text offset
    pub kind: TextFragmentKind,
}

impl<'a> TextFragment<'a> {
    pub fn new(text: &'a str, offset: usize, kind: TextFragmentKind) -> Self {
        Self { text, offset, kind }
    }

    pub fn text(text: &'a str, offset: usize) -> Self {
        Self::new(text, offset, TextFragmentKind::Text)
    }
    pub fn line_comment(text: &'a str, offset: usize) -> Self {
        Self::new(text, offset, TextFragmentKind::LineComment)
    }
    pub fn block_comment(text: &'a str, offset: usize) -> Self {
        Self::new(text, offset, TextFragmentKind::BlockComment)
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
        self.kind == other.kind && self.text == other.text
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum TextFragmentKind {
    Text,
    LineComment,
    BlockComment,
}

#[derive(Debug, Clone, Serialize)]
pub struct Delimited<T> {
    open: Span,
    pub inner: T,
    close: Span,
}

impl<T> Delimited<T> {
    pub fn new(open: Span, inner: T, close: Span) -> Self {
        debug_assert!(open.end() < close.start(), "delimited open after close");
        Self { open, inner, close }
    }

    pub fn span(&self) -> Span {
        Span::new(self.open.start(), self.close.end())
    }

    pub fn inner_span(&self) -> Span {
        Span::new(self.open.end(), self.close.start())
    }

    pub fn into_located_inner(self) -> Located<T> {
        let span = self.inner_span();
        Located::new(self.inner, span)
    }

    pub fn open(&self) -> Span {
        self.open
    }

    pub fn close(&self) -> Span {
        self.close
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Deref for Delimited<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Separated<T> {
    items: Vec<Located<T>>,
}

impl<T> Separated<T> {
    pub fn new(item: Located<T>) -> Self {
        Self { items: vec![item] }
    }

    pub fn push(&mut self, item: Located<T>) {
        self.items.push(item);
    }

    pub fn from_items(items: Vec<Located<T>>) -> Self {
        assert!(!items.is_empty());
        debug_assert!(items
            .windows(2)
            .all(|w| w[0].span().end() <= w[1].span().start()));
        Self { items }
    }

    pub fn span(&self) -> Span {
        let start = self.items.first().unwrap().span().start();
        let end = self.items.last().unwrap().span().end();
        Span::new(start, end)
    }

    pub fn items(&self) -> &[Located<T>] {
        &self.items
    }

    pub fn into_items(self) -> Vec<Located<T>> {
        self.items
    }
}

impl Quantity<'_> {
    pub fn unit_span(&self) -> Option<Span> {
        let u = self.unit.as_ref()?.span();
        if let Some(sep) = &self.unit_separator {
            assert_eq!(sep.end(), u.start());
            Some(Span::new(sep.start(), u.end()))
        } else {
            Some(u)
        }
    }
}

impl QuantityValue<'_> {
    pub fn span(&self) -> Span {
        match self {
            QuantityValue::Single {
                value,
                auto_scale_marker,
                ..
            } => {
                let s = value.span();
                if let Some(marker) = auto_scale_marker {
                    assert_eq!(s.end(), marker.start());
                    Span::new(s.start(), marker.end())
                } else {
                    s
                }
            }
            QuantityValue::Many(v) => v.span(),
        }
    }
}

impl Recover for Text<'_> {
    fn recover() -> Self {
        Self::empty(0)
    }
}

impl<T: Recover> Recover for Delimited<T> {
    fn recover() -> Self {
        Self::new(Recover::recover(), Recover::recover(), Recover::recover())
    }
}

impl Recover for Quantity<'_> {
    fn recover() -> Self {
        Self {
            value: Recover::recover(),
            unit_separator: None,
            unit: Recover::recover(),
        }
    }
}

impl Recover for QuantityValue<'_> {
    fn recover() -> Self {
        Self::Single {
            value: Recover::recover(),
            scalable: false,
            auto_scale_marker: None,
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

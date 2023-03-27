use std::iter::Peekable;

use super::{
    ast::{Text, TextFragment},
    Pair, Pairs, Rule,
};
use crate::located::Located;

pub trait PairExt<'a> {
    fn located(self) -> Located<Self>
    where
        Self: Sized;

    fn first_inner(self) -> Self;

    fn text(&self) -> Text<'a>;
}

impl<'a> PairExt<'a> for Pair<'a> {
    fn located(self) -> Located<Self> {
        self.into()
    }

    fn first_inner(self) -> Self {
        let mut inner = self.into_inner();
        let pair = inner.next().expect("No inner pair found");
        debug_assert!(inner.next().is_none());
        pair
    }

    fn text(&self) -> Text<'a> {
        let mut it = self.clone().into_inner().peekable();
        let mut text = Text::empty(self.as_span().start());
        while let Some(fragment) = next_fragment(&mut it, self.as_str(), self.as_span().start()) {
            text.push(fragment);
        }
        text
    }
}

fn next_fragment<'a>(
    pairs: &mut Peekable<Pairs<'a>>,
    source: &'a str,
    offset: usize,
) -> Option<TextFragment<'a>> {
    let pair = pairs.next()?;
    match pair.as_rule() {
        Rule::any => {
            let mut range = pair.as_span().start()..pair.as_span().end();
            loop {
                match pairs.peek() {
                    Some(p) if p.as_rule() == Rule::any => {
                        range.end = p.as_span().end();
                    }
                    _ => {
                        range.start -= offset;
                        range.end -= offset;
                        return Some(TextFragment::text(
                            &source[range.clone()],
                            pair.as_span().start(),
                        ));
                    }
                }
                pairs.next();
            }
        }
        Rule::line_comment => {
            return Some(TextFragment::line_comment(
                pair.as_str(),
                pair.as_span().start(),
            ))
        }
        Rule::block_comment => {
            return Some(TextFragment::block_comment(
                pair.as_str(),
                pair.as_span().start(),
            ))
        }
        _ => panic!("unexpected text item rule: {:?}", pair.as_rule()),
    }
}

impl<'a> From<Pair<'a>> for Located<Pair<'a>> {
    fn from(value: Pair<'a>) -> Self {
        let span = value.as_span();
        Self::new(value, span)
    }
}

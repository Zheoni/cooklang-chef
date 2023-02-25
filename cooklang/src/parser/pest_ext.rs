use std::{borrow::Cow, ops::Range};

use super::{located::Located, Pair};

pub trait PairExt<'a> {
    fn located(self) -> Located<Self>
    where
        Self: Sized;

    fn first_inner(self) -> Self;

    fn as_located_str(&self) -> Located<&'a str>;
    fn text(&self) -> Cow<'a, str>;
    fn text_trimmed(&self) -> Cow<'a, str>;
    fn located_text(&self) -> Located<Cow<'a, str>>;
    fn located_text_trimmed(&self) -> Located<Cow<'a, str>>;
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

    fn as_located_str(&self) -> Located<&'a str> {
        Located::new(self.as_str(), self.span())
    }

    fn text(&self) -> Cow<'a, str> {
        let mut it = self.clone().into_inner();

        let Some((mut current_start, mut current_end)) = it.next().map(|r| {
            let s = r.as_span();
            (s.start(), s.end())
        }) else {
            return self.as_str().into();
        };

        // Group pairs into segments that are contiguous
        let mut segments = Vec::with_capacity(4);
        for pair in it {
            let span = pair.as_span();

            if current_end == span.start() {
                // extend current segment if is in the same byte position
                current_end = span.end();
            } else {
                // or create a new one
                segments.push((current_start, current_end));
                current_start = span.start();
                current_end = span.end();
            }
        }
        segments.push((current_start, current_end));

        let as_str = self.as_str();
        let offset = self.as_span().start();

        let mut s = Cow::Borrowed("");

        // Collect the segments. If it's one, it will still be a Cow::Borrowed
        // because the += impl in std checks if its empty before allocating
        for (start, end) in segments {
            s += &as_str[start - offset..end - offset];
        }

        s
    }

    fn text_trimmed(&self) -> Cow<'a, str> {
        let s = self.text();

        match s {
            Cow::Borrowed(s) => Cow::Borrowed(s.trim()),
            Cow::Owned(s) => Cow::Owned(s.trim().to_string()),
        }
    }

    fn located_text(&self) -> Located<Cow<'a, str>> {
        Located::new(self.text(), self.span())
    }

    fn located_text_trimmed(&self) -> Located<Cow<'a, str>> {
        Located::new(self.text_trimmed(), self.span())
    }
}

pub trait Span {
    fn span(&self) -> Range<usize>;
}

impl Span for Pair<'_> {
    fn span(&self) -> Range<usize> {
        let span = self.as_span();
        span.start()..span.end()
    }
}

impl Span for pest::error::InputLocation {
    fn span(&self) -> Range<usize> {
        match self.clone() {
            pest::error::InputLocation::Pos(p) => p..p,
            pest::error::InputLocation::Span((start, end)) => start..end,
        }
    }
}

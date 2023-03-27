use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Span<Id = ()> {
    source_id: Id,

    start: usize,
    end: usize,
}

impl<Id: Default> Span<Id> {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            source_id: Id::default(),
            start,
            end,
        }
    }

    pub fn pos(pos: usize) -> Self {
        Self {
            source_id: Id::default(),
            start: pos,
            end: pos,
        }
    }
}

impl<Id> Span<Id> {
    pub fn new_with_id(source_id: Id, start: usize, end: usize) -> Self {
        Self {
            source_id,
            start,
            end,
        }
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

impl<Id: Clone> Span<Id> {
    pub fn to_chars_span(&self, all_source: &str) -> CharsSpan<Id> {
        let start = all_source[..self.start].chars().count();
        let len = all_source[self.range()].chars().count();
        CharsSpan(Span::<Id>::new_with_id(
            self.source_id.clone(),
            start,
            start + len,
        ))
    }
}

pub struct CharsSpan<Id>(Span<Id>);

impl<Id> CharsSpan<Id>
where
    Id: Clone + PartialEq,
{
    pub fn start(&self) -> usize {
        self.0.start
    }

    pub fn end(&self) -> usize {
        self.0.end
    }

    pub fn range(&self) -> Range<usize> {
        self.0.range()
    }
}

impl From<Range<usize>> for Span<()> {
    fn from(value: Range<usize>) -> Self {
        Self::new(value.start, value.end)
    }
}

impl<T> From<Span<T>> for Range<usize> {
    fn from(value: Span<T>) -> Self {
        value.start..value.end
    }
}

impl<T, Id: Clone> From<crate::located::Located<T, Id>> for Span<Id> {
    fn from(value: crate::located::Located<T, Id>) -> Self {
        value.span()
    }
}

impl From<pest::Span<'_>> for Span<()> {
    fn from(value: pest::Span) -> Self {
        Self::new(value.start(), value.end())
    }
}

impl From<pest::error::InputLocation> for Span<()> {
    fn from(value: pest::error::InputLocation) -> Self {
        match value {
            pest::error::InputLocation::Pos(p) => (p..p).into(),
            pest::error::InputLocation::Span((start, end)) => (start..end).into(),
        }
    }
}

impl<R: pest::RuleType> From<pest::iterators::Pair<'_, R>> for Span<()> {
    fn from(value: pest::iterators::Pair<R>) -> Self {
        value.as_span().into()
    }
}

impl crate::context::Recover for Span<()> {
    fn recover() -> Self {
        Self::new(0, 0)
    }
}

impl<Id> ariadne::Span for CharsSpan<Id>
where
    Id: ToOwned + PartialEq,
{
    type SourceId = Id;

    fn source(&self) -> &Self::SourceId {
        &self.0.source_id
    }

    fn start(&self) -> usize {
        self.0.start
    }

    fn end(&self) -> usize {
        self.0.end
    }
}

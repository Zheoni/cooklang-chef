use std::{
    fmt::Display,
    ops::{Deref, DerefMut, Range},
};

use crate::context::Recover;

#[derive(Debug)]
pub struct Located<T> {
    pub(crate) inner: T,
    span: Range<usize>,
}

impl<T> Located<T> {
    pub fn new(inner: T, span: Range<usize>) -> Self {
        Self { inner, span }
    }

    pub fn new_span(inner: T, span: pest::Span) -> Self {
        Self {
            inner,
            span: span.start()..span.end(),
        }
    }

    pub fn map_inner<F, O>(self, f: F) -> Located<O>
    where
        F: FnOnce(T) -> O,
    {
        Located {
            inner: f(self.inner),
            span: self.span,
        }
    }

    pub fn map<F, O>(self, f: F) -> Located<O>
    where
        F: FnOnce(Located<T>) -> Located<O>,
    {
        f(self)
    }

    pub fn offset(&self) -> usize {
        self.span.start
    }

    pub fn take(self) -> T {
        self.inner
    }

    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    pub fn take_pair(self) -> (T, Range<usize>) {
        (self.inner, self.span)
    }

    pub fn replace<O>(self, new_inner: O) -> Located<O> {
        Located {
            inner: new_inner,
            span: self.span,
        }
    }
}

impl<T: Copy> Located<T> {
    pub fn get(&self) -> T {
        self.inner
    }
}

impl<T> Clone for Located<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            span: self.span.clone(),
        }
    }
}

impl<T> Display for Located<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T> Deref for Located<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Located<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> From<Located<T>> for miette::SourceSpan {
    fn from(value: Located<T>) -> Self {
        value.span.into()
    }
}

impl<T> Recover for Located<T>
where
    T: Recover,
{
    fn recover() -> Self {
        Self {
            inner: T::recover(),
            span: 0..0,
        }
    }
}

pub trait OptTake<T> {
    fn opt_take(self) -> Option<T>;
}

impl<T> OptTake<T> for Option<Located<T>> {
    fn opt_take(self) -> Option<T> {
        self.map(Located::take)
    }
}

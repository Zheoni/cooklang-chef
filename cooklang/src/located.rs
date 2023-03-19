use std::{
    fmt::Display,
    ops::{Deref, DerefMut, Range},
};

use crate::{context::Recover, span::Span};

#[derive(Debug)]
pub struct Located<T, Id = ()> {
    pub(crate) inner: T,
    span: Span<Id>,
}

impl<T, Id: Clone> Located<T, Id> {
    pub fn new(inner: T, span: impl Into<Span<Id>>) -> Self {
        Self {
            inner,
            span: span.into(),
        }
    }

    pub fn map_inner<F, O>(self, f: F) -> Located<O, Id>
    where
        F: FnOnce(T) -> O,
    {
        Located {
            inner: f(self.inner),
            span: self.span,
        }
    }

    pub fn map<F, O>(self, f: F) -> Located<O, Id>
    where
        F: FnOnce(Self) -> Located<O, Id>,
    {
        f(self)
    }

    pub fn offset(&self) -> usize {
        self.span.start()
    }

    pub fn take(self) -> T {
        self.inner
    }

    pub fn range(&self) -> Range<usize> {
        self.span.range()
    }

    pub fn span(&self) -> Span<Id> {
        self.span.clone()
    }

    pub fn take_pair(self) -> (T, Span<Id>) {
        (self.inner, self.span)
    }

    pub fn replace<O>(self, new_inner: O) -> Located<O, Id> {
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
            span: self.span,
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

impl<T, Id> From<Located<T, Id>> for Range<usize> {
    fn from(value: Located<T, Id>) -> Self {
        value.span.range()
    }
}

impl<T> Recover for Located<T>
where
    T: Recover,
{
    fn recover() -> Self {
        Self {
            inner: T::recover(),
            span: Recover::recover(),
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

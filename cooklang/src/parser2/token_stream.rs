//! [Cursor](crate::lexer::Cursor) iterator adapter for it's use in
//! Parser(super::parser::Parser).

use std::collections::VecDeque;

pub use crate::lexer::TokenKind;
use crate::{lexer::Cursor, span::Span};

pub struct TokenStream<'input> {
    cursor: Cursor<'input>,
    consumed: usize,
}

impl<'input> TokenStream<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            cursor: Cursor::new(input),
            consumed: 0,
        }
    }
}

impl<'input> Iterator for TokenStream<'input> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let t = self.cursor.advance_token();
        let start = self.consumed;
        self.consumed += t.len as usize;
        if t.kind == TokenKind::Eof && self.cursor.is_eof() {
            None
        } else {
            Some(Token {
                kind: t.kind,
                span: Span::new(start, self.consumed),
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn dummy() -> Self {
        Self::new(TokenKind::Question, Span::new(0, 0))
    }

    pub fn len(&self) -> usize {
        self.span.len()
    }
}

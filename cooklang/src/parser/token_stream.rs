//! [Cursor](crate::lexer::Cursor) iterator adapter for it's use in
//! Parser(super::parser::Parser).

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
    pub fn len(&self) -> usize {
        self.span.len()
    }
}

#[cfg(test)]
macro_rules! tokens {
    ($($kind:tt . $len:expr),*) => {{
        let mut v = Vec::new();
        let mut _len = 0;
        $(
            v.push($crate::parser::token_stream::Token { kind: $crate::lexer::T![$kind], span: $crate::span::Span::new(_len, _len + $len) });
            _len += $len;
        )*
        v
    }};
}
#[cfg(test)]
pub(crate) use tokens;

#[cfg(test)]
mod tests {
    use crate::lexer::T;

    use super::*;

    #[test]
    fn tokens_macro() {
        let t = tokens![word.3, ws.1];
        assert_eq!(
            t,
            vec![
                Token {
                    kind: T![word],
                    span: Span::new(0, 3)
                },
                Token {
                    kind: T![ws],
                    span: Span::new(3, 4)
                },
            ]
        );
    }
}

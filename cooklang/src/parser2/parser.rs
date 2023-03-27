//! [Parser] struct itself and utilities.

use std::{collections::VecDeque, iter::Peekable};

use super::token_stream::{Token, TokenKind, TokenStream};

use crate::{
    ast,
    context::{impl_deref_context, Context},
    parser::{ParserError, ParserWarning},
    T,
};

/// Cooklang parser
///
/// Tokens are [TokenKind].
///
/// Grammar:
/// ```txt
/// recipe     = Newline* (line line_end)* line? Eof
/// line       = metadata | section | step
/// line_end   = soft_break | Newline+
/// soft_break = Newline !Newline
///
/// metadata   = MetadataStart meta_key Colon meta_val
/// meta_key   = (!(Colon | Newline) ANY)*
/// meta_value = (!Newline ANY)*
///
/// section    = Eq+ (section_name Eq*)
/// sect_name  = (!Eq ANY)*
///
/// step       = TextStep? (component | ANY)*
///
/// component  = c_kind modifiers? c_body note?
/// c_kind     = At | Hash | Tilde
/// c_body     = c_close | c_long | Word
/// c_long     = c_l_name c_alias? c_close
/// c_l_name   = (!(Newline | OpenBrace | Or) ANY)*
/// c_alias    = Or c_l_name
/// c_close    = OpenBrace Whitespace? Quantity? Whitespace? CloseBrace
///
/// modifiers  = modifier+
/// modifier   = At | And | Plus | Minus | Question
///
/// note       = OpenParen (!CloseParen ANY)* CloseParen
///
/// quantity   = num_val Whitespace !(unit_sep | auto_scale | val_sep) unit
///            | val (val_sep val)* auto_scale? (unit_sep unit)?
///
/// unit       = (!CloseBrace ANY)*
///
/// val_sep    = Whitespace Or Whitespace
/// auto_scale = Whitespace Star Whitespace
/// unit_sep   = Whitespace Percent Whitespace
///
/// val        = numeric_val | text_val
/// text_val   = (Word | Whitespace)*
/// num_val    = mixed_num | frac | range | num
/// mixed_num  = Int Whitespace frac
/// frac       = int Whitespace Int
/// range      = num Whitespace Minus Whitespace Num
/// num        = Float | Int
///
///
/// ANY        = { Any token }
/// ```
#[derive(Debug)]
pub struct Parser<'input, T>
where
    T: Iterator<Item = Token>,
{
    input: &'input str,
    tokens: T,
    lookahead: VecDeque<Token>,

    /// Current token
    pub(crate) prev: Token,
    /// AST being built
    pub(crate) ast: ast::Ast<'input>,
    /// Error and warning context
    pub(crate) context: Context<ParserError, ParserWarning>,
}

impl<'input> Parser<'input, TokenStream<'input>> {
    pub fn new(input: &'input str) -> Self {
        Self::new_from_token_iter(input, TokenStream::new(input))
    }
}

impl<'input, T> Parser<'input, T>
where
    T: Iterator<Item = Token>,
{
    pub fn new_from_token_iter(input: &'input str, tokens: T) -> Self {
        Self {
            input,
            tokens,
            lookahead: VecDeque::new(),
            prev: Token::dummy(),
            ast: ast::Ast { lines: vec![] },
            context: Context::default(),
        }
    }
}

impl<'input, T> Parser<'input, T>
where
    T: Iterator<Item = Token>,
{
    /// Gets a token's matching slice from the input
    pub(crate) fn text(&self, token: Token) -> &'input str {
        &self.input[token.span.range()]
    }

    /// Peeks the next token without consuming it.
    pub(crate) fn peek(&mut self) -> TokenKind {
        if self.lookahead.is_empty() {
            if let Some(token) = self.tokens.next() {
                self.lookahead.push_front(token);
            }
        }

        self.lookahead
            .front()
            .map(|token| token.kind)
            .unwrap_or(TokenKind::Eof)
    }

    /// Consumes tokens until newline of eof and stores in in [Self::lookahead],
    /// newline/eof included.
    fn lookahead_line(&mut self) {
        if !self.lookahead.is_empty() {
            return;
        }

        while let Some(token) = self.tokens.next() {
            self.lookahead.push_back(token);
            if token.kind == TokenKind::Newline || token.kind == TokenKind::Eof {
                break;
            }
        }
    }

    /// Peeks the not parsed tokens in the current line.
    pub(crate) fn peek_line(&mut self) -> impl Iterator<Item = TokenKind> + '_ {
        self.lookahead_line();
        self.lookahead.iter().map(|t| t.kind)
    }

    /// Peeks the not parsed tokens in the current line.
    /// This may be slower than [Self::peek_line].
    pub(crate) fn peek_line_slice(&mut self) -> &[Token] {
        self.lookahead_line();
        &*self.lookahead.make_contiguous()
    }

    /// Checks the next token without consuming it.
    pub(crate) fn peek_is(&mut self, kind: TokenKind) -> bool {
        self.peek() == kind
    }

    /// Advance to the next token. Saves the current token in [Self::token]
    pub(crate) fn next(&mut self) -> Option<Token> {
        let token = self.lookahead.pop_front().or_else(|| self.tokens.next())?;
        self.prev = token;
        Some(token)
    }

    /// Same as [Self::next] but panics if there are no more tokens
    pub(crate) fn bump(&mut self) -> Token {
        self.next().expect("Expected token, but there was none")
    }

    /// Returns the current (not parsed) position
    pub(crate) fn current_pos(&self) -> usize {
        self.prev.span.end()
    }

    /// Call [Self::next] but panics if the next token is not `expected`.
    pub(crate) fn consume(&mut self, expected: TokenKind) -> Token {
        let token = self.bump();
        assert_eq!(
            token.kind, expected,
            "Expected '{expected:?}', but got '{:?}'",
            token.kind
        );
        token
    }

    /// Call [Self::next] if the next token is `expected`.
    pub(crate) fn try_consume(&mut self, expected: TokenKind) -> Option<Token> {
        if self.peek_is(expected) {
            self.next()
        } else {
            None
        }
    }

    /// Checks if the given token is in the current line
    pub(crate) fn within_line(&mut self, needle: TokenKind, stop: &[TokenKind]) -> bool {
        #[inline]
        fn stop_condition(kind: TokenKind, stop: &[TokenKind]) -> bool {
            kind == T![newline] || kind == T![eof] || stop.contains(&kind)
        }

        // first try to use the lookahead
        for token in &self.lookahead {
            // check stop
            if stop_condition(token.kind, stop) {
                return false;
            }
            if token.kind == needle {
                return true;
            }
        }
        // then get new tokens
        loop {
            if let Some(token) = self.tokens.next() {
                // store the new token
                self.lookahead.push_back(token);
                // check stop
                if stop_condition(token.kind, stop) {
                    return false;
                }
                if token.kind == needle {
                    return true;
                }
            } else {
                // if no more tokens, false
                return false;
            }
        }
    }

    /// Takes any token until the given one or a newline, returns the slice
    /// consumed as [ast::Text].
    pub(crate) fn text_until(&mut self, needle: TokenKind) -> ast::Text<'input> {
        let mut range = SlidingRange::new(self.prev.span.end());
        let mut text = ast::Text::empty(range.start);

        loop {
            let peek = self.peek();
            if peek == needle || peek == T![newline] || peek == T![eof] {
                break;
            }
            let token = self.bump();
            if token.kind == T![line comment] || token.kind == T![block comment] {
                text.append_str(range.fragment(&self.input));
                match token.kind {
                    T![line comment] => text.append_line_comment(self.text(token)),
                    T![block comment] => text.append_block_comment(self.text(token)),
                    _ => unreachable!(),
                }
                range.skip(token.len());
            } else {
                range.grow(token.len());
            }
        }
        text.append_str(range.fragment(&self.input));
        text
    }

    fn error(&mut self, error: ParserError) {
        self.context.error(error);
    }
    fn warn(&mut self, warn: ParserWarning) {
        self.context.warn(warn)
    }
}

impl<'input> ast::Ast<'input> {
    /// Appends a [ast::Text] to the last line of the AST if the last line
    /// is a step. If it's not a step, insert a new one.
    pub(crate) fn append_text(&mut self, text: ast::Text<'input>) {
        let item = ast::Item::Text(text);
        if let Some(ast::Line::Step { items, .. }) = self.lines.last_mut() {
            items.push(item);
        } else {
            self.lines.push(ast::Line::Step {
                is_text: false,
                items: vec![item],
            });
        }
    }
}

struct SlidingRange {
    start: usize,
    end: usize,
}

impl SlidingRange {
    fn new(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    /// Gets the matching str slice. Panics if indexing of the range panics.
    fn fragment<'input>(&mut self, s: &'input str) -> &'input str {
        let s = &s[self.start..self.end];
        self.start = self.end;
        s
    }

    /// Advances the start point n positions forward.
    /// End point will be modified if needed and the range len may become 0.
    fn skip(&mut self, n: usize) {
        self.start += n;
        self.end = self.start.max(self.end);
    }

    /// Advances the end point n positions forward.
    fn grow(&mut self, n: usize) {
        self.end += n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sliding_range_skip() {
        let mut range = SlidingRange { start: 3, end: 10 };
        range.skip(5);
        assert_eq!(range.start, 8);
        assert_eq!(range.end, 10);
        range.skip(4);
        assert_eq!(range.start, 12);
        assert_eq!(range.end, 12);
    }

    #[test]
    fn sliding_range_grow() {
        let mut range = SlidingRange::new(4);
        range.grow(10);
        assert_eq!(range.start, 4);
        assert_eq!(range.end, 14);
    }

    #[test]
    fn sliding_range_fragment() {
        let mut range = SlidingRange { start: 5, end: 8 };
        let s = "hii! uwu :)";
        assert_eq!(range.fragment(s), "uwu");
        assert_eq!(range.start, 8);
        assert_eq!(range.end, 8);
    }

    struct MockLexer {
        calls: usize,
        tokens: std::vec::IntoIter<crate::lexer::Token>,
        consumed: usize,
    }

    macro_rules! t {
        ($kind:tt, $len:expr) => {
            crate::lexer::Token {
                kind: T![$kind],
                len: $len,
            }
        };
    }

    impl MockLexer {
        fn new(tokens: Vec<crate::lexer::Token>) -> Self {
            Self {
                calls: 0,
                tokens: tokens.into_iter(),
                consumed: 0,
            }
        }

        fn basic() -> (&'static str, Self) {
            (
                "a cooklang recipe",
                Self::new(vec![
                    t!(word, 1),
                    t!(ws, 1),
                    t!(word, 8),
                    t!(ws, 1),
                    t!(word, 6),
                    t!(eof, 0),
                ]),
            )
        }
    }

    impl Iterator for MockLexer {
        type Item = Token;

        fn next(&mut self) -> Option<Self::Item> {
            self.calls += 1;
            let t = self.tokens.next()?;
            let start = self.consumed;
            self.consumed += t.len as usize;
            if t.kind == TokenKind::Eof {
                None
            } else {
                Some(Token {
                    kind: t.kind,
                    span: crate::span::Span::new(start, self.consumed),
                })
            }
        }
    }

    #[test]
    fn peek() {
        let (input, tokens) = MockLexer::basic();
        let mut parser = Parser::new_from_token_iter(input, tokens);
        assert_eq!(parser.peek(), T![word]);
        assert_eq!(parser.tokens.calls, 1);
        parser.next();
        assert_eq!(parser.tokens.calls, 1);
        assert_eq!(parser.peek(), T![ws]);
        assert_eq!(parser.prev.kind, T![word]);
        assert_eq!(parser.tokens.calls, 2);
    }

    #[test]
    fn peek_line() {
        let input = "a line\nanother line";
        let tokens = MockLexer::new(vec![
            t!(word, 1),
            t!(ws, 1),
            t!(word, 4),
            t!(newline, 1),
            t!(word, 8),
            t!(ws, 1),
            t!(word, 4),
        ]);
        let mut parser = Parser::new_from_token_iter(input, tokens);
        assert_eq!(parser.peek_line().count(), 4);
        assert_eq!(parser.tokens.calls, 4);
        assert_eq!(parser.peek(), T![word]);
        assert_eq!(parser.tokens.calls, 4);
        parser.next();
        assert_eq!(parser.tokens.calls, 4);
        assert_eq!(parser.prev.kind, T![word]);
        assert_eq!(parser.peek(), T![ws]);
        assert_eq!(parser.tokens.calls, 4);
    }
}

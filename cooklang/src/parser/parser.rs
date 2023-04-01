//! [Parser] struct itself and utilities.

use super::{
    token_stream::{Token, TokenKind, TokenStream},
    ParserError, ParserWarning,
};

use crate::{
    ast,
    context::Context,
    error::PassResult,
    lexer::T,
    parser::{metadata::metadata_entry, section::section, step::step},
    span::Span,
    Extensions,
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
/// val        = num_val | text_val
/// text_val   = (Word | Whitespace)*
/// num_val    = mixed_num | frac | range | num
/// mixed_num  = Int Whitespace frac
/// frac       = Int Whitespace Slash Whitespace Int
/// range      = num Whitespace Minus Whitespace Num
/// num        = Float | Int
///
///
/// ANY        = { Any token }
/// ```
/// This is more of a guideline, there may be edge cases that this grammar does
/// not cover.
#[derive(Debug)]
pub struct Parser<'input, T>
where
    T: Iterator<Item = Token>,
{
    input: &'input str,
    tokens: T,
    line: Vec<Token>,
    offset: usize,

    /// Error and warning context
    pub(crate) context: Context<ParserError, ParserWarning>,
    /// Extensions to cooklang language
    pub(crate) extensions: Extensions,
}

impl<'input> Parser<'input, TokenStream<'input>> {
    pub fn new(input: &'input str, extensions: Extensions) -> Self {
        Self::new_from_token_iter(input, extensions, TokenStream::new(input))
    }
}

impl<'input, I> Parser<'input, I>
where
    I: Iterator<Item = Token>,
{
    pub fn new_from_token_iter(input: &'input str, extensions: Extensions, tokens: I) -> Self {
        Self {
            input,
            tokens,
            line: Vec::new(),
            context: Context::default(),
            extensions,
            offset: 0,
        }
    }
}

impl<'input, I> Parser<'input, I>
where
    I: Iterator<Item = Token>,
{
    /// Advances a line. Store the tokens, newline/eof excluded.
    pub(crate) fn next_line(&mut self) -> Option<LineParser<'_, 'input>> {
        self.line.clear();
        let parsed = self.offset;
        let mut has_terminator = false;
        while let Some(token) = self.tokens.next() {
            self.offset += token.len();
            if matches!(token.kind, T![newline] | T![eof]) {
                has_terminator = true;
                break;
            }
            self.line.push(token);
        }
        if self.line.is_empty() && !has_terminator {
            None
        } else {
            Some(LineParser::new(
                parsed,
                &self.line,
                self.input,
                self.extensions,
            ))
        }
    }
}

#[tracing::instrument(skip_all, fields(len = input.len()))]
pub fn parse<'input>(
    input: &'input str,
    extensions: Extensions,
) -> PassResult<ast::Ast<'input>, ParserError, ParserWarning> {
    let mut parser = Parser::new(input, extensions);

    let mut last_line_is_empty = true;

    let mut lines = Vec::new();
    while let Some(mut line) = parser.next_line() {
        if line
            .tokens()
            .iter()
            .all(|t| matches!(t.kind, T![ws] | T![line comment] | T![block comment]))
        {
            last_line_is_empty = true;
            continue;
        }

        let meta_or_section = match line.peek() {
            T![meta] => line
                .with_recover(metadata_entry)
                .map(|entry| ast::Line::Metadata {
                    key: entry.key,
                    value: entry.value,
                })
                .ok(),
            T![=] => line
                .with_recover(section)
                .map(|name| ast::Line::Section { name })
                .ok(),
            _ => None,
        };

        let ast_line = if let Some(l) = meta_or_section {
            l
        } else {
            if !last_line_is_empty && extensions.contains(Extensions::MULTINE_STEPS) {
                if let Some(ast::Line::Step { items, is_text }) = lines.last_mut() {
                    let parsed_step = step(&mut line, *is_text);
                    items.extend(parsed_step.items);
                    let mut ctx = line.finish();
                    parser.context.append(&mut ctx);
                    continue;
                }
            }

            let parsed_step = step(&mut line, false);
            ast::Line::Step {
                is_text: parsed_step.is_text,
                items: parsed_step.items,
            }
        };

        let mut ctx = line.finish();
        parser.context.append(&mut ctx);

        last_line_is_empty = false;
        lines.push(ast_line);
    }
    let ast = ast::Ast { lines };
    parser.context.finish(Some(ast))
}

pub struct LineParser<'t, 'input> {
    base_offset: usize,
    tokens: &'t [Token],
    current: usize,
    pub(crate) input: &'input str,
    pub(crate) context: Context<ParserError, ParserWarning>,
    pub(crate) extensions: Extensions,
}

pub type ParseResult<T> = Result<T, ()>;
pub const fn fatal_err<T>() -> ParseResult<T> {
    ParseResult::Err(())
}
pub const fn fatal() -> () {
    ()
}

impl<'t, 'input> LineParser<'t, 'input> {
    /// Create it from separate parts.
    /// - tokens must be adjacent (checked in debug)
    /// - slices's tokens's span must refer to the input (checked in debug)
    /// - input is the whole input str given to the lexer
    pub(crate) fn new(
        base_offset: usize,
        line: &'t [Token],
        input: &'input str,
        extensions: Extensions,
    ) -> Self {
        debug_assert!(
            line.is_empty()
                || (line.first().unwrap().span.start() < input.len()
                    && line.last().unwrap().span.end() <= input.len()),
            "tokens out of input bounds"
        );
        debug_assert!(
            line.windows(2)
                .all(|w| w[0].span.end() == w[1].span.start()),
            "tokens are not adjacent"
        );
        Self {
            base_offset,
            tokens: line,
            current: 0,
            input,
            context: Context::default(),
            extensions,
        }
    }

    /// Finish parsing the line, this will return the error/warning
    /// context used in the line.
    ///
    /// Panics if is inside a [Self::with_recover] or if any token is left.
    pub fn finish(self) -> Context<ParserError, ParserWarning> {
        assert_eq!(
            self.current,
            self.tokens.len(),
            "Line tokens not parsed. this is a bug"
        );
        self.context
    }

    pub fn extension(&self, ext: Extensions) -> bool {
        self.extensions.contains(ext)
    }

    /// Runs a function that can fail to parse the input.
    ///
    /// If the function succeeds, is just as it was called withtout recover.
    /// If the function fails, any token eaten by it will be restored.
    ///
    /// Note that any other state modification such as adding errors to the
    /// context will not be rolled back.
    pub fn with_recover<F, O>(&mut self, f: F) -> ParseResult<O>
    where
        F: FnOnce(&mut Self) -> ParseResult<O>,
    {
        let old_current = self.current;
        let r = f(self);
        if r.is_err() {
            self.current = old_current;
        }
        r
    }

    /// Gets a token's matching str from the input
    pub(crate) fn as_str(&self, token: Token) -> &'input str {
        &self.input[token.span.range()]
    }

    pub(crate) fn text(&self, offset: usize, tokens: &[Token]) -> ast::Text<'input> {
        debug_assert!(
            tokens
                .windows(2)
                .all(|w| w[0].span.end() == w[1].span.start()),
            "tokens are not adjacent"
        );

        let mut t = ast::Text::empty(offset);
        if tokens.len() == 0 {
            return t;
        }
        let mut start = tokens[0].span.start();
        let mut end = start;
        assert_eq!(offset, start);

        for token in tokens {
            if token.kind == T![line comment] || token.kind == T![block comment] {
                t.append_str(&self.input[start..end]);
                start = token.span.end();
                end = start;
            } else {
                end = token.span.end();
            }

            match token.kind {
                T![line comment] | T![block comment] => {
                    t.append_str(&self.input[start..end]);
                    start = token.span.end();
                    end = start;
                }
                T![escaped] => {
                    t.append_str(&self.input[start..end]);
                    debug_assert_eq!(token.len(), 2, "unexpected escaped token length");
                    start = token.span.start() + 1; // skip "\"
                }
                _ => end = token.span.end(),
            }
        }
        t.append_str(&self.input[start..end]);
        t
    }

    /// Returns the current offset from the start of input
    pub(crate) fn current_offset(&self) -> usize {
        self.parsed()
            .last()
            .map(|t| t.span.end())
            .unwrap_or(self.base_offset)
    }

    pub(crate) fn tokens(&self) -> &'t [Token] {
        self.tokens
    }

    pub(crate) fn parsed(&self) -> &'t [Token] {
        self.tokens.split_at(self.current).0
    }

    /// Returns the not parsed tokens
    pub(crate) fn rest(&self) -> &'t [Token] {
        self.tokens.split_at(self.current).1
    }

    pub(crate) fn consume_rest(&mut self) -> &'t [Token] {
        let r = self.rest();
        self.current += r.len();
        r
    }

    /// Peeks the next token without consuming it.
    pub fn peek(&self) -> TokenKind {
        self.tokens
            .get(self.current)
            .map(|token| token.kind)
            .unwrap_or(TokenKind::Eof)
    }

    /// Checks the next token without consuming it.
    pub fn at(&self, kind: TokenKind) -> bool {
        self.peek() == kind
    }

    /// Advance to the next token.
    #[must_use]
    pub fn next(&mut self) -> Option<Token> {
        if let Some(token) = self.tokens.get(self.current) {
            self.current += 1;
            Some(*token)
        } else {
            None
        }
    }

    /// Same as [Self::next] but panics if there are no more tokens.
    pub fn bump_any(&mut self) -> Token {
        self.next().expect("Expected token, but there was none")
    }

    /// Call [Self::next] but panics if the next token is not `expected`.
    pub fn bump(&mut self, expected: TokenKind) -> Token {
        let token = self.bump_any();
        assert_eq!(
            token.kind, expected,
            "Expected '{expected:?}', but got '{:?}'",
            token.kind
        );
        token
    }

    pub fn until(&mut self, f: impl Fn(TokenKind) -> bool) -> ParseResult<&'t [Token]> {
        let rest = self.rest();
        let pos = rest.iter().position(|t| f(t.kind)).ok_or(fatal())?;
        let s = &rest[..pos];
        self.current += pos;
        Ok(s)
    }

    pub fn consume_while(&mut self, f: impl Fn(TokenKind) -> bool) -> &'t [Token] {
        let rest = self.rest();
        let pos = rest.iter().position(|t| !f(t.kind)).unwrap_or(rest.len());
        let s = &rest[..pos];
        self.current += pos;
        s
    }

    pub fn ws_comments(&mut self) -> &'t [Token] {
        self.consume_while(|t| matches!(t, T![ws] | T![line comment] | T![block comment]))
    }

    /// Call [Self::next] if the next token is `expected`.
    #[must_use]
    pub fn consume(&mut self, expected: TokenKind) -> ParseResult<Token> {
        if self.at(expected) {
            Ok(self.bump_any())
        } else {
            Err(fatal())
        }
    }

    pub fn error(&mut self, error: ParserError) {
        self.context.error(error);
    }
    pub fn warn(&mut self, warn: ParserWarning) {
        self.context.warn(warn)
    }
}

/// get the span for a slice of tokens. panics if the slice is empty
pub(crate) fn tokens_span(tokens: &[Token]) -> Span {
    debug_assert!(!tokens.is_empty(), "tokens_span tokens empty");
    let start = tokens.first().unwrap().span.start();
    let end = tokens.last().unwrap().span.end();
    Span::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLexer {
        calls: usize,
        tokens: std::vec::IntoIter<crate::lexer::Token>,
        consumed: usize,
    }

    impl MockLexer {
        fn new(tokens: Vec<crate::lexer::Token>) -> Self {
            Self {
                calls: 0,
                tokens: tokens.into_iter(),
                consumed: 0,
            }
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
    fn the_test() {
        let (ast, warn, err) =
            parse("a test @step @salt{1%mg} more text", Extensions::all()).into_tuple();
        println!("{:#?}", ast);
        println!("{:#?}", warn);
        println!("{:#?}", err);
    }
}

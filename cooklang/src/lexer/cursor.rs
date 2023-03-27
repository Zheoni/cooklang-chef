use std::str::Chars;

/// Peekable iterator from a &str
///
/// This was adapted from <https://github.com/rust-lang/rust/blob/2d429f3064cb67710fe64dee293329089871d92b/compiler/rustc_lexer/src/cursor.rs>
pub struct Cursor<'a> {
    len_remaining: usize,
    chars: Chars<'a>,
    prev: char,
}

pub(crate) const EOF_CHAR: char = '\0';

impl<'a> Cursor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            len_remaining: input.len(),
            chars: input.chars(),
            prev: EOF_CHAR,
        }
    }

    /// Returns the last eaten symbol.
    pub(crate) fn prev(&self) -> char {
        self.prev
    }

    /// Peeks the next char. If none, [EOF_CHAR] is returned. But EOF should
    /// be checked with [Self::is_eof].
    pub(crate) fn first(&self) -> char {
        // cloning chars is cheap as it's only a view into memory
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    /// Checks if there is more input to consume.
    pub(crate) fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// Returns amount of already consumed symbols.
    pub(crate) fn pos_within_token(&self) -> u32 {
        (self.len_remaining - self.chars.as_str().len()) as u32
    }

    /// Resets the number of bytes consumed to 0.
    pub(crate) fn reset_pos_within_token(&mut self) {
        self.len_remaining = self.chars.as_str().len();
    }

    /// Moves to the next character.
    pub(crate) fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.prev = c;
        Some(c)
    }

    /// Eats symbols while predicate returns true or until the end of file is reached.
    pub(crate) fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while predicate(self.first()) && !self.is_eof() {
            self.bump();
        }
    }
}

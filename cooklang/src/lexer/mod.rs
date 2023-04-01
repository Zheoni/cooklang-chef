mod cursor;

pub use cursor::Cursor;
use cursor::EOF_CHAR;

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub len: u32,
}

impl Token {
    fn new(kind: TokenKind, len: u32) -> Token {
        Token { kind, len }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// ">>"
    MetadataStart,
    /// ">"
    TextStep,
    /// ":"
    Colon,
    /// "@"
    At,
    /// "#"
    Hash,
    /// "~"
    Tilde,
    /// "?"
    Question,
    /// "+"
    Plus,
    /// "-"
    Minus,
    /// "/"
    Slash,
    /// "*"
    Star,
    /// "&"
    And,
    /// "|"
    Or,
    /// "="
    Eq,
    /// "%"
    Percent,
    /// "{"
    OpenBrace,
    /// "}"
    CloseBrace,
    /// "("
    OpenParen,
    /// ")"
    CloseParen,
    /// "["
    OpenSquare,
    /// "]"
    CloseSquare,

    /// "14", "0", but not "014"
    Int,
    /// "3.14", ".14", but not "14."
    Float,
    /// Everything else, a "\" escapes the next char
    Word,
    /// "\" followed by any char
    Escaped,

    /// " " and \t
    Whitespace,
    /// \r\n and \n
    Newline,
    /// "-- any until newline"
    LineComment,
    /// "[- any until EOF or close -]"
    BlockComment,

    /// End of input
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralKind {}

pub fn tokenize(input: &str) -> impl Iterator<Item = Token> + '_ {
    let mut cursor = Cursor::new(input);
    std::iter::from_fn(move || {
        let token = cursor.advance_token();
        if token.kind != TokenKind::Eof {
            Some(token)
        } else {
            None
        }
    })
}

fn is_newline(c: char, first: char) -> bool {
    c == '\n' || (c == '\r' && first == '\n')
}

fn is_whitespace(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn is_special(c: char) -> bool {
    // faster than str::contains and equally as fast as match
    const SPECIAL_CHARS_LIST: &[char] = &[
        '>', ':', '@', '#', '~', '?', '+', '-', '/', '*', '&', '|', '=', '%', '{', '}', '(', ')',
        '[', ']',
    ];
    SPECIAL_CHARS_LIST.contains(&c)
}

fn is_word_char(c: char) -> bool {
    use finl_unicode::categories::CharacterCategories;
    !is_whitespace(c) && c != '\n' && c != '\r' && !is_special(c) && !c.is_punctuation()
}

impl Cursor<'_> {
    pub fn advance_token(&mut self) -> Token {
        let prev = self.prev();

        let first_char = match self.bump() {
            Some(c) => c,
            None => return Token::new(TokenKind::Eof, 0),
        };

        let token_kind = match first_char {
            // escape next symbol
            '\\' => {
                self.bump();
                TokenKind::Escaped
            }

            // multi char tokens
            '>' if self.first() == '>' => {
                self.bump();
                TokenKind::MetadataStart
            }
            '-' if self.first() == '-' => self.line_comment(),
            '[' if self.first() == '-' => self.block_comment(),
            c if is_whitespace(c) => self.whitespace(),
            c if is_newline(c, self.first()) => self.newline(c),
            c if c.is_ascii_digit() => self.number(c),
            '.' if self.first().is_ascii_digit() && (!is_word_char(prev) || prev == EOF_CHAR) => {
                self.number('.')
            }

            // single char tokens
            '>' => TokenKind::TextStep,
            ':' => TokenKind::Colon,
            '@' => TokenKind::At,
            '#' => TokenKind::Hash,
            '~' => TokenKind::Tilde,
            '?' => TokenKind::Question,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '/' => TokenKind::Slash,
            '*' => TokenKind::Star,
            '&' => TokenKind::And,
            '|' => TokenKind::Or,
            '%' => TokenKind::Percent,
            '=' => TokenKind::Eq,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '[' => TokenKind::OpenSquare,
            ']' => TokenKind::CloseSquare,

            c if is_word_char(c) => self.word(),

            // anything else, a one char word
            _ => TokenKind::Word,
        };
        let token = Token::new(token_kind, self.pos_within_token());
        self.reset_pos_within_token();
        token
    }

    fn line_comment(&mut self) -> TokenKind {
        debug_assert!(self.prev() == '-' && self.first() == '-');
        self.eat_while(|c| c != '\n');
        TokenKind::LineComment
    }

    fn block_comment(&mut self) -> TokenKind {
        debug_assert!(self.prev() == '[' && self.first() == '-');
        self.bump(); // '-'
        while let Some(c) = self.bump() {
            match c {
                '-' if self.first() == ']' => {
                    self.bump();
                    break;
                }
                _ => {}
            }
        }
        TokenKind::BlockComment
    }

    fn word(&mut self) -> TokenKind {
        self.eat_while(|c| is_word_char(c));
        TokenKind::Word
    }

    fn whitespace(&mut self) -> TokenKind {
        debug_assert!(is_whitespace(self.prev()));
        self.eat_while(is_whitespace);
        TokenKind::Whitespace
    }

    fn newline(&mut self, c: char) -> TokenKind {
        debug_assert!(is_newline(self.prev(), self.first()));
        if c == '\r' {
            self.bump();
        }
        TokenKind::Newline
    }

    /// Tokenize number-like
    ///
    /// 0 => int (0)
    /// 01 => word (no leading 0)
    /// 0. => word
    /// 0.[0-9]+ => float
    /// [1-9]+ => int
    ///
    fn number(&mut self, c: char) -> TokenKind {
        debug_assert!(
            self.prev().is_ascii_digit() || (self.prev() == '.' && self.first().is_ascii_digit())
        );

        // no int special case
        if c == '.' {
            if self.eat_digits() {
                return TokenKind::Float;
            } else {
                return TokenKind::Word;
            }
        }

        // Regular number
        let has_int = self.eat_digits() || c.is_ascii_digit();
        let int_leading_zero = c == '0' && self.pos_within_token() > 1;
        let has_divider = self.first() == '.';
        let has_frac = if has_divider {
            self.bump();
            self.eat_digits()
        } else {
            false
        };

        if int_leading_zero {
            return TokenKind::Word;
        }

        match (has_int, has_divider, has_frac) {
            (true, false, false) => TokenKind::Int,
            (_, true, true) => TokenKind::Float,
            _ => TokenKind::Word,
        }
    }

    fn eat_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            if self.first().is_ascii_digit() {
                has_digits = true;
                self.bump();
            } else {
                break;
            }
        }
        has_digits
    }
}

/// Shorthand macro for [TokenKind]
macro_rules! T {
    [+] => {
        $crate::lexer::TokenKind::Plus
    };
    [@] => {
        $crate::lexer::TokenKind::At
    };
    [#] => {
        $crate::lexer::TokenKind::Hash
    };
    [~] => {
        $crate::lexer::TokenKind::Tilde
    };
    [?] => {
        $crate::lexer::TokenKind::Question
    };
    [-] => {
        $crate::lexer::TokenKind::Minus
    };
    [*] => {
        $crate::lexer::TokenKind::Star
    };
    [%] => {
        $crate::lexer::TokenKind::Percent
    };
    [/] => {
        $crate::lexer::TokenKind::Slash
    };
    [=] => {
        $crate::lexer::TokenKind::Eq
    };
    [&] => {
        $crate::lexer::TokenKind::And
    };
    [|] => {
        $crate::lexer::TokenKind::Or
    };
    [:] => {
        $crate::lexer::TokenKind::Colon
    };
    [>] => {
        $crate::lexer::TokenKind::TextStep
    };
    ['['] => {
        $crate::lexer::TokenKind::OpenSquare
    };
    [']'] => {
        $crate::lexer::TokenKind::CloseSquare
    };
    ['{'] => {
        $crate::lexer::TokenKind::OpenBrace
    };
    ['}'] => {
        $crate::lexer::TokenKind::CloseBrace
    };
    ['('] => {
        $crate::lexer::TokenKind::OpenParen
    };
    [')'] => {
        $crate::lexer::TokenKind::CloseParen
    };
    [word] => {
        $crate::lexer::TokenKind::Word
    };
    [escaped] => {
        $crate::lexer::TokenKind::Escaped
    };
    [line comment] => {
        $crate::lexer::TokenKind::LineComment
    };
    [block comment] => {
        $crate::lexer::TokenKind::BlockComment
    };
    [int] => {
        $crate::lexer::TokenKind::Int
    };
    [float] => {
        $crate::lexer::TokenKind::Float
    };
    [meta] => {
        $crate::lexer::TokenKind::MetadataStart
    };
    [>>] => {
        $crate::lexer::TokenKind::MetadataStart
    };
    [ws] => {
        $crate::lexer::TokenKind::Whitespace
    };
    [newline] => {
        $crate::lexer::TokenKind::Newline
    };
    [eof] => {
        $crate::lexer::TokenKind::Eof
    };
}
pub(crate) use T;

/// Utility macro to build a vec of tokens, example:
/// ```
/// # use crate::lexer::Token;
/// # use crate::lexer::tokens;
/// let tokens = tokens![word: 3, ws: 1];
/// assert_eq!(tokens, vec![
///     Token { kind: T![word], len: 3},
///     Token { kind: T![ws], len: 1
/// ]);
/// ```
/// Also see [T].
macro_rules! tokens {
    ($($kind:tt : $len:expr),*) => {{
        let mut v = Vec::new();
        $(
            v.push($crate::lexer::Token { kind: $crate::lexer::T![$kind], len: $len });
        )*
        v
    }};
    (parse; $($kind:tt . $len:expr),*) => {{
        let mut v = Vec::new();
        let mut len = 0;
        $(
            v.push($crate::parser::token_stream::Token { kind: $crate::lexer::T![$kind], span: $crate::span::Span::new(len, len + $len) });
            len += $len;
        )*
        v
    }};
}
pub(crate) use tokens;

#[cfg(test)]
mod tests {
    use super::*;
    use TokenKind::*;

    macro_rules! t {
        ($input:expr, $token_kinds:expr) => {
            let got: Vec<TokenKind> = tokenize($input).map(|t| t.kind).collect();
            assert_eq!(got, $token_kinds, "Input was: '{}'", $input)
        };
    }

    #[test]
    fn word() {
        t!("basic", vec![Word]);
        t!("👀", vec![Word]);
        t!("👀more", vec![Word]);
        t!("thing👀more", vec![Word]);

        t!("two words", vec![Word, Whitespace, Word]);
        t!("word\nanother", vec![Word, Newline, Word]);

        // composed emojis more than one char
        t!("👩🏿‍🔬", vec![Word]);
        t!("\u{1F1EA}\u{1F1F8}", vec![Word]);
        t!("thing👩🏿‍🔬more", vec![Word]);
        t!("thing👩🏿‍🔬more", vec![Word]);
    }

    #[test]
    fn number() {
        t!("1", vec![Int]);
        t!("0", vec![Int]);
        t!("01", vec![Word]);
        t!("01.3", vec![Word]);
        t!("1.3", vec![Float]);
        t!(".3", vec![Float]);
        t!("0.3", vec![Float]);
        t!("0.03", vec![Float]);
        t!("{.3}", vec![OpenBrace, Float, CloseBrace]);
        t!("phraseends.3", vec![Word, Word, Int]);
    }

    #[test]
    fn comment() {
        t!("-- a line comment", vec![LineComment]);
        t!("[- a block comment -]", vec![BlockComment]);
        t!(
            "a word [- then comment -] the more",
            vec![
                Word,
                Whitespace,
                Word,
                Whitespace,
                BlockComment,
                Whitespace,
                Word,
                Whitespace,
                Word
            ]
        );
        t!(
            "word -- and line comment",
            vec![Word, Whitespace, LineComment]
        );
        t!(
            "word -- and line comment\nmore",
            vec![Word, Whitespace, LineComment, Newline, Word]
        );
        t!(
            "word [- non closed block\ncomment",
            vec![Word, Whitespace, BlockComment]
        );
    }

    #[test]
    fn test_component() {
        t!("@basic", vec![At, Word]);
        t!("#basic", vec![Hash, Word]);
        t!("~basic", vec![Tilde, Word]);
        t!("@single word", vec![At, Word, Whitespace, Word]);
        t!(
            "@multi word{}",
            vec![At, Word, Whitespace, Word, OpenBrace, CloseBrace]
        );
        t!("@qty{3}", vec![At, Word, OpenBrace, Int, CloseBrace]);
        t!(
            "@qty{3}(note)",
            vec![At, Word, OpenBrace, Int, CloseBrace, OpenParen, Word, CloseParen]
        );
    }

    #[test]
    fn recipe() {
        const S: TokenKind = TokenKind::Whitespace;
        const L: TokenKind = TokenKind::Newline;
        let input = r#"
>> key: value
Just let him cook.

Use @sauce{100%ml} and @love.
"#;
        #[rustfmt::skip]
        t!(input, vec![
            L,
            MetadataStart, S, Word, Colon, S, Word, L,
            Word, S, Word, S, Word, S, Word, Word, L,
            L,
            Word, S, At, Word, OpenBrace, Int, Percent, Word, CloseBrace, S, Word, S, At, Word, Word, L
        ]);
    }
}

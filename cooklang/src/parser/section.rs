use crate::{ast, lexer::T};

use super::parser::{fatal, LineParser, ParseResult};

pub fn section<'t, 'input>(
    line: &mut LineParser<'t, 'input>,
) -> ParseResult<Option<ast::Text<'input>>> {
    line.consume(T![=])?;
    line.consume_while(|t| t == T![=]);
    let name_pos = line.current_offset();
    let name_tokens = line.consume_while(|t| t != T![=]);
    let name = line.text(name_pos, name_tokens);
    line.consume_while(|t| t == T![=]);

    if !line.rest().is_empty() {
        return Err(fatal());
    }

    if name.is_text_empty() {
        Ok(None)
    } else {
        Ok(Some(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::tokens, parser::parser::LineParser, span::Span, Extensions};

    #[test]
    fn basic_section() {
        let input = "= section";
        let tokens = tokens![parse; =.1, ws.1, word.7];
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        let name = section(&mut line).unwrap().unwrap();
        let context = line.finish();
        assert_eq!(name.text(), " section");
        assert_eq!(name.span(), Span::new(1, 9));
        assert!(context.errors.is_empty());
        assert!(context.warnings.is_empty());

        let input = "== section ==";
        let tokens = tokens![parse; =.1, =.1, ws.1, word.7, ws.1, =.1, =.1];
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        let name = section(&mut line).unwrap().unwrap();
        let context = line.finish();
        assert_eq!(name.text(), " section ");
        assert_eq!(name.span(), Span::new(2, 11));
        assert!(context.errors.is_empty());
        assert!(context.warnings.is_empty());
    }

    #[test]
    fn no_name_section() {
        let input = "====";
        let tokens = tokens![parse; =.1, =.1, =.1, =.1];
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        let name = section(&mut line).unwrap();
        let context = line.finish();
        assert!(name.is_none());
        assert!(context.errors.is_empty());
        assert!(context.warnings.is_empty());

        let input = "==   ==";
        let tokens = tokens![parse; =.1, =.1, ws.3, =.1, =.1];
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        let name = section(&mut line).unwrap();
        let context = line.finish();
        assert!(name.is_none());
        assert!(context.errors.is_empty());
        assert!(context.warnings.is_empty());

        let input = "= =  ==";
        let tokens = tokens![parse; =.1, ws.1, =.1, ws.2, =.1, =.1];
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        assert!(section(&mut line).is_err());
    }
}

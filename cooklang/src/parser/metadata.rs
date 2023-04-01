use crate::{ast, error::label, lexer::T};

use super::{
    parser::{LineParser, ParseResult},
    ParserError, ParserWarning,
};

pub struct MetadataEntry<'input> {
    pub key: ast::Text<'input>,
    pub value: ast::Text<'input>,
}

pub fn metadata_entry<'t, 'input>(
    line: &mut LineParser<'t, 'input>,
) -> ParseResult<MetadataEntry<'input>> {
    // Parse
    line.consume(T![meta])?;
    let key_pos = line.current_offset();
    let key_tokens = line.until(|t| t == T![:])?;
    let key = line.text(key_pos, key_tokens);
    line.bump(T![:]);
    let value_pos = line.current_offset();
    let value_tokens = line.consume_rest();
    let value = line.text(value_pos, value_tokens);

    // Checks
    if key.is_text_empty() {
        line.error(ParserError::ComponentPartInvalid {
            container: "metadata entry",
            what: "key",
            reason: "is empty",
            labels: vec![label!(key.span(), "this cannot be empty")],
            help: None,
        });
    } else if value.is_text_empty() {
        line.warn(ParserWarning::EmptyMetadataValue {
            key: key.located_string(),
        });
    }

    let entry = MetadataEntry { key, value };

    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::tokens, parser::parser::LineParser, span::Span, Extensions};

    #[test]
    fn basic_metadata_entry() {
        let input = ">> key: value";
        let tokens = tokens![parse; meta.2, ws.1, word.3, :.1, ws.1, word.5];
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        let entry = metadata_entry(&mut line).unwrap();
        let context = line.finish();
        assert_eq!(entry.key.text(), " key");
        assert_eq!(entry.key.span(), Span::new(2, 6));
        assert_eq!(entry.value.text(), " value");
        assert_eq!(entry.value.span(), Span::new(7, 13));
        assert!(context.errors.is_empty());
        assert!(context.warnings.is_empty());
    }

    #[test]
    fn no_key_metadata_entry() {
        let input = ">>: value";
        let tokens = tokens![parse; meta.2, :.1, ws.1, word.5];
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        let entry = metadata_entry(&mut line).unwrap();
        let context = line.finish();
        assert_eq!(entry.key.text(), "");
        assert_eq!(entry.key.span(), Span::pos(2));
        assert_eq!(entry.value.text_trimmed(), "value");
        assert_eq!(context.errors.len(), 1);
        assert!(context.warnings.is_empty());
    }

    #[test]
    fn no_val_metadata_entry() {
        let input = ">> key:";
        let tokens = tokens![parse; meta.2, ws.1, word.3, :.1];
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        let entry = metadata_entry(&mut line).unwrap();
        let context = line.finish();
        assert_eq!(entry.key.text_trimmed(), "key");
        assert_eq!(entry.value.text(), "");
        assert_eq!(entry.value.span(), Span::pos(7));
        assert!(context.errors.is_empty());
        assert_eq!(context.warnings.len(), 1);

        let input = ">> key:  ";
        let tokens = tokens![parse; meta.2, ws.1, word.3, :.1, ws.2];
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        let entry = metadata_entry(&mut line).unwrap();
        let context = line.finish();
        assert_eq!(entry.key.text_trimmed(), "key");
        assert_eq!(entry.value.text(), "  ");
        assert_eq!(entry.value.span(), Span::new(7, 9));
        assert!(context.errors.is_empty());
        assert_eq!(context.warnings.len(), 1);
    }

    #[test]
    fn empty_metadata_entry() {
        let input = ">>:";
        let tokens = tokens![parse; meta.2, :.1];
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        let entry = metadata_entry(&mut line).unwrap();
        let context = line.finish();
        assert!(entry.key.text().is_empty());
        assert_eq!(entry.key.span(), Span::pos(2));
        assert!(entry.value.text().is_empty());
        assert_eq!(entry.value.span(), Span::pos(3));
        assert_eq!(context.errors.len(), 1);
        assert!(context.warnings.is_empty()); // no warning if error generated

        let input = ">> ";
        let tokens = tokens![parse; meta.2, ws.1];
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        assert!(metadata_entry(&mut line).is_err())
    }
}

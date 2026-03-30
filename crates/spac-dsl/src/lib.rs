use spac_core::{
    validate_protocol_semantics, Diagnostic, FieldSpec, PayloadKind, PayloadSpec, ProtocolSpec,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    text: String,
    line: usize,
    column: usize,
}

pub fn parse_protocol_text(text: &str) -> Result<ProtocolSpec, Vec<Diagnostic>> {
    let tokens = tokenize(text);
    let mut parser = Parser { tokens, index: 0 };
    let protocol = parser.parse_protocol()?;

    if parser.peek().is_some() {
        let token = parser.peek().expect("peeked token");
        return Err(vec![parse_error(
            token,
            format!("unexpected token '{}'", token.text),
        )]);
    }

    let diagnostics = validate_protocol_semantics(&protocol);
    if diagnostics.is_empty() {
        Ok(protocol)
    } else {
        Err(diagnostics)
    }
}

fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();

    for (line_index, raw_line) in text.lines().enumerate() {
        let line = raw_line.split("//").next().unwrap_or_default();
        let mut current = String::new();
        let mut current_column = 1;

        for (column_index, ch) in line.chars().enumerate() {
            let column = column_index + 1;
            if ch.is_whitespace() {
                push_current(&mut tokens, &mut current, line_index + 1, current_column);
                continue;
            }

            if matches!(ch, '{' | '}' | ':' | ';') {
                push_current(&mut tokens, &mut current, line_index + 1, current_column);
                tokens.push(Token {
                    text: ch.to_string(),
                    line: line_index + 1,
                    column,
                });
                continue;
            }

            if current.is_empty() {
                current_column = column;
            }
            current.push(ch);
        }

        push_current(&mut tokens, &mut current, line_index + 1, current_column);
    }

    tokens
}

fn push_current(tokens: &mut Vec<Token>, current: &mut String, line: usize, column: usize) {
    if current.is_empty() {
        return;
    }

    tokens.push(Token {
        text: std::mem::take(current),
        line,
        column,
    });
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn parse_protocol(&mut self) -> Result<ProtocolSpec, Vec<Diagnostic>> {
        self.expect_text("protocol")?;
        let name = self.expect_identifier("protocol name")?;
        self.expect_text("{")?;

        let mut fields = Vec::new();
        let mut payload = None;

        while !self.next_is("}") {
            match self.peek().map(|token| token.text.as_str()) {
                Some("field") => fields.push(self.parse_field()?),
                Some("payload") => {
                    if payload.is_some() {
                        let token = self.peek().expect("payload token");
                        return Err(vec![parse_error(token, "duplicate payload declaration")]);
                    }
                    payload = Some(self.parse_payload()?);
                }
                Some(_) => {
                    let token = self.peek().expect("unexpected token");
                    return Err(vec![parse_error(
                        token,
                        format!("expected field, payload, or '}}'; found '{}'", token.text),
                    )]);
                }
                None => {
                    return Err(vec![Diagnostic::error(
                        "SPAC_DSL_EOF",
                        "$",
                        "unexpected end of file while parsing protocol",
                    )]);
                }
            }
        }

        self.expect_text("}")?;
        Ok(ProtocolSpec {
            name,
            fields,
            payload,
        })
    }

    fn parse_field(&mut self) -> Result<FieldSpec, Vec<Diagnostic>> {
        self.expect_text("field")?;
        let name = self.expect_identifier("field name")?;
        self.expect_text(":")?;
        let bit_width = self.expect_unsigned_type_width()?;

        let semantic = if self.next_is("semantic") {
            self.expect_text("semantic")?;
            Some(self.expect_identifier("semantic name")?)
        } else {
            None
        };

        self.expect_text(";")?;
        Ok(FieldSpec {
            name,
            bit_width,
            semantic,
        })
    }

    fn parse_payload(&mut self) -> Result<PayloadSpec, Vec<Diagnostic>> {
        self.expect_text("payload")?;
        self.expect_text("bytes")?;
        self.expect_text(";")?;
        Ok(PayloadSpec {
            kind: PayloadKind::Bytes,
        })
    }

    fn expect_text(&mut self, expected: &str) -> Result<(), Vec<Diagnostic>> {
        let Some(token) = self.advance() else {
            return Err(vec![Diagnostic::error(
                "SPAC_DSL_EOF",
                "$",
                format!("expected '{expected}' but reached end of file"),
            )]);
        };

        if token.text == expected {
            Ok(())
        } else {
            Err(vec![parse_error(
                &token,
                format!("expected '{expected}' but found '{}'", token.text),
            )])
        }
    }

    fn expect_identifier(&mut self, label: &str) -> Result<String, Vec<Diagnostic>> {
        let Some(token) = self.advance() else {
            return Err(vec![Diagnostic::error(
                "SPAC_DSL_EOF",
                "$",
                format!("expected {label} but reached end of file"),
            )]);
        };

        if is_identifier(&token.text) {
            Ok(token.text)
        } else {
            Err(vec![parse_error(
                &token,
                format!("expected {label}, found '{}'", token.text),
            )])
        }
    }

    fn expect_unsigned_type_width(&mut self) -> Result<u16, Vec<Diagnostic>> {
        let Some(token) = self.advance() else {
            return Err(vec![Diagnostic::error(
                "SPAC_DSL_EOF",
                "$",
                "expected unsigned integer type such as u8 but reached end of file",
            )]);
        };

        let Some(width_text) = token.text.strip_prefix('u') else {
            return Err(vec![parse_error(
                &token,
                format!(
                    "expected unsigned integer type such as u8, found '{}'",
                    token.text
                ),
            )]);
        };

        let Ok(width) = width_text.parse::<u16>() else {
            return Err(vec![parse_error(
                &token,
                format!("invalid unsigned integer width '{}'", token.text),
            )]);
        };

        if width == 0 {
            return Err(vec![parse_error(
                &token,
                "field width must be greater than zero",
            )]);
        }

        Ok(width)
    }

    fn next_is(&self, expected: &str) -> bool {
        self.peek()
            .map(|token| token.text.as_str() == expected)
            .unwrap_or(false)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.index).cloned();
        if token.is_some() {
            self.index += 1;
        }
        token
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn parse_error(token: &Token, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        "SPAC_DSL_PARSE",
        format!("line {}, column {}", token.line, token.column),
        message.into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_protocol() {
        let protocol = parse_protocol_text(
            r#"
            protocol basic {
              field dst: u8 semantic routing_key;
              field src: u8 semantic source_key;
              field qos: u3 semantic qos;
              payload bytes;
            }
            "#,
        )
        .expect("valid protocol");

        assert_eq!(protocol.name, "basic");
        assert_eq!(protocol.fields.len(), 3);
        assert_eq!(protocol.fields[0].semantic.as_deref(), Some("routing_key"));
    }

    #[test]
    fn rejects_missing_routing_key() {
        let diagnostics = parse_protocol_text(
            r#"
            protocol bad {
              field dst: u8;
              payload bytes;
            }
            "#,
        )
        .expect_err("invalid protocol");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_ROUTING_KEY_MISSING"));
    }

    #[test]
    fn rejects_malformed_field() {
        let diagnostics = parse_protocol_text(
            r#"
            protocol bad {
              field dst u8 semantic routing_key;
            }
            "#,
        )
        .expect_err("invalid protocol");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_DSL_PARSE"));
    }
}

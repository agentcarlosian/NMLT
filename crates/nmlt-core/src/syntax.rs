use std::collections::BTreeSet;

use crate::{Diagnostic, Span};

#[derive(Clone, Debug, Eq, PartialEq)]
enum TokenKind {
    Identifier(String),
    LeftBrace,
    RightBrace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Token {
    kind: TokenKind,
    span: Span,
}

/// A structurally recognized top-level system declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemDecl {
    pub name: String,
    pub span: Span,
}

/// The result of the current structural parser.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedFile {
    pub systems: Vec<SystemDecl>,
}

/// Parse the structural shell of an NMLT source file.
///
/// Only top-level `system Name { ... }` declarations and delimiter integrity
/// are checked. Successful parsing is not type checking or verification.
pub fn parse_source(source: &str) -> Result<ParsedFile, Vec<Diagnostic>> {
    if source.trim().is_empty() {
        return Err(vec![Diagnostic::error(
            "NMLT0001",
            "source file is empty",
            None,
        )]);
    }

    let tokens = lex(source)?;
    let mut diagnostics = validate_braces(&tokens);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut systems = Vec::new();
    let mut names = BTreeSet::new();
    let mut index = 0;

    while index < tokens.len() {
        let TokenKind::Identifier(keyword) = &tokens[index].kind else {
            index += 1;
            continue;
        };
        if keyword != "system" {
            index += 1;
            continue;
        }

        let Some(name_token) = tokens.get(index + 1) else {
            diagnostics.push(Diagnostic::error(
                "NMLT0003",
                "expected a system name after `system`",
                Some(tokens[index].span),
            ));
            break;
        };
        let TokenKind::Identifier(name) = &name_token.kind else {
            diagnostics.push(Diagnostic::error(
                "NMLT0003",
                "expected a system name after `system`",
                Some(name_token.span),
            ));
            index += 1;
            continue;
        };

        let Some(open_index) = tokens[index + 2..]
            .iter()
            .position(|token| matches!(token.kind, TokenKind::LeftBrace))
            .map(|offset| index + 2 + offset)
        else {
            diagnostics.push(Diagnostic::error(
                "NMLT0004",
                format!("system `{name}` has no body"),
                Some(name_token.span),
            ));
            break;
        };

        let close_index = matching_right_brace(&tokens, open_index)
            .expect("brace validation guarantees a matching delimiter");
        let declaration_span = Span::new(tokens[index].span.start, tokens[close_index].span.end);

        if !names.insert(name.clone()) {
            diagnostics.push(Diagnostic::error(
                "NMLT0006",
                format!("duplicate system declaration `{name}`"),
                Some(name_token.span),
            ));
        } else {
            systems.push(SystemDecl {
                name: name.clone(),
                span: declaration_span,
            });
        }
        index = close_index + 1;
    }

    if systems.is_empty() && diagnostics.is_empty() {
        diagnostics.push(Diagnostic::error(
            "NMLT0005",
            "no `system` declaration found",
            None,
        ));
    }

    if diagnostics.is_empty() {
        Ok(ParsedFile { systems })
    } else {
        Err(diagnostics)
    }
}

fn lex(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                let start = index;
                index += 2;
                let mut closed = false;
                while index + 1 < bytes.len() {
                    if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                        index += 2;
                        closed = true;
                        break;
                    }
                    index += 1;
                }
                if !closed {
                    return Err(vec![Diagnostic::error(
                        "NMLT0007",
                        "unterminated block comment",
                        Some(Span::new(start, bytes.len())),
                    )]);
                }
            }
            b'"' => {
                let start = index;
                index += 1;
                let mut closed = false;
                while index < bytes.len() {
                    match bytes[index] {
                        b'\\' => index = (index + 2).min(bytes.len()),
                        b'"' => {
                            index += 1;
                            closed = true;
                            break;
                        }
                        _ => index += 1,
                    }
                }
                if !closed {
                    return Err(vec![Diagnostic::error(
                        "NMLT0008",
                        "unterminated string literal",
                        Some(Span::new(start, bytes.len())),
                    )]);
                }
            }
            b'{' => {
                tokens.push(Token {
                    kind: TokenKind::LeftBrace,
                    span: Span::new(index, index + 1),
                });
                index += 1;
            }
            b'}' => {
                tokens.push(Token {
                    kind: TokenKind::RightBrace,
                    span: Span::new(index, index + 1),
                });
                index += 1;
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                let start = index;
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Identifier(source[start..index].to_owned()),
                    span: Span::new(start, index),
                });
            }
            _ => index += 1,
        }
    }

    Ok(tokens)
}

fn validate_braces(tokens: &[Token]) -> Vec<Diagnostic> {
    let mut stack = Vec::new();
    let mut diagnostics = Vec::new();

    for token in tokens {
        match token.kind {
            TokenKind::LeftBrace => stack.push(token.span),
            TokenKind::RightBrace => {
                if stack.pop().is_none() {
                    diagnostics.push(Diagnostic::error(
                        "NMLT0002",
                        "unmatched closing brace",
                        Some(token.span),
                    ));
                }
            }
            TokenKind::Identifier(_) => {}
        }
    }

    for span in stack {
        diagnostics.push(Diagnostic::error(
            "NMLT0002",
            "unclosed opening brace",
            Some(span),
        ));
    }
    diagnostics
}

fn matching_right_brace(tokens: &[Token], open_index: usize) -> Option<usize> {
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(open_index) {
        match token.kind {
            TokenKind::LeftBrace => depth += 1,
            TokenKind::RightBrace => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            TokenKind::Identifier(_) => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse_source;

    #[test]
    fn parses_a_system_and_nested_blocks() {
        let parsed = parse_source("system Clock { action Tick { set bit = true } }").unwrap();
        assert_eq!(parsed.systems.len(), 1);
        assert_eq!(parsed.systems[0].name, "Clock");
    }

    #[test]
    fn ignores_system_text_in_comments_and_strings() {
        let parsed =
            parse_source("// system Fake {}\nvalue = \"system AlsoFake {}\"\nsystem Real {}")
                .unwrap();
        assert_eq!(parsed.systems.len(), 1);
        assert_eq!(parsed.systems[0].name, "Real");
    }

    #[test]
    fn rejects_duplicate_systems() {
        let diagnostics = parse_source("system Same {} system Same {}").unwrap_err();
        assert!(diagnostics.iter().any(|item| item.code == "NMLT0006"));
    }

    #[test]
    fn rejects_unbalanced_braces() {
        let diagnostics = parse_source("system Broken {").unwrap_err();
        assert!(diagnostics.iter().any(|item| item.code == "NMLT0002"));
    }

    #[test]
    fn rejects_files_without_a_system() {
        let diagnostics = parse_source("data Bool = True | False").unwrap_err();
        assert!(diagnostics.iter().any(|item| item.code == "NMLT0005"));
    }
}

use std::collections::BTreeSet;

use crate::lexer::{Token, TokenKind, lex_source};
use crate::{Diagnostic, Span};

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

/// Parse the structural shell of an NMLT source file over the lossless lexer.
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

    let lexed = lex_source(source);
    if !lexed.diagnostics.is_empty() {
        return Err(lexed.diagnostics);
    }

    let tokens = lexed
        .tokens
        .iter()
        .filter(|token| !token.kind.is_trivia())
        .collect::<Vec<_>>();
    let mut diagnostics = validate_delimiters(&tokens);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut systems = Vec::new();
    let mut names = BTreeSet::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index].kind != TokenKind::Identifier || tokens[index].text(source) != "system" {
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
        if name_token.kind != TokenKind::Identifier {
            diagnostics.push(Diagnostic::error(
                "NMLT0003",
                "expected a system name after `system`",
                Some(name_token.span),
            ));
            index += 1;
            continue;
        }
        let name = name_token.text(source);

        let Some(open_index) = tokens[index + 2..]
            .iter()
            .position(|token| token.kind == TokenKind::LeftBrace)
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
            .expect("delimiter validation guarantees a matching delimiter");
        let declaration_span = Span::new(tokens[index].span.start, tokens[close_index].span.end);

        if !names.insert(name.to_owned()) {
            diagnostics.push(Diagnostic::error(
                "NMLT0006",
                format!("duplicate system declaration `{name}`"),
                Some(name_token.span),
            ));
        } else {
            systems.push(SystemDecl {
                name: name.to_owned(),
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

fn validate_delimiters(tokens: &[&Token]) -> Vec<Diagnostic> {
    let mut stack = Vec::new();
    let mut diagnostics = Vec::new();

    for token in tokens {
        if is_open(token.kind) {
            stack.push((token.kind, token.span));
            continue;
        }
        if !is_close(token.kind) {
            continue;
        }
        let Some((open, _)) = stack.pop() else {
            diagnostics.push(Diagnostic::error(
                "NMLT0002",
                "unmatched closing delimiter",
                Some(token.span),
            ));
            continue;
        };
        if matching_close(open) != token.kind {
            diagnostics.push(Diagnostic::error(
                "NMLT0002",
                "mismatched closing delimiter",
                Some(token.span),
            ));
        }
    }

    for (_, span) in stack {
        diagnostics.push(Diagnostic::error(
            "NMLT0002",
            "unclosed opening delimiter",
            Some(span),
        ));
    }
    diagnostics
}

fn is_open(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LeftBrace | TokenKind::LeftParen | TokenKind::LeftBracket
    )
}

fn is_close(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::RightBrace | TokenKind::RightParen | TokenKind::RightBracket
    )
}

fn matching_close(kind: TokenKind) -> TokenKind {
    match kind {
        TokenKind::LeftBrace => TokenKind::RightBrace,
        TokenKind::LeftParen => TokenKind::RightParen,
        TokenKind::LeftBracket => TokenKind::RightBracket,
        _ => unreachable!("only opening delimiters are stacked"),
    }
}

fn matching_right_brace(tokens: &[&Token], open_index: usize) -> Option<usize> {
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
            _ => {}
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
    fn rejects_unbalanced_or_mismatched_delimiters() {
        for source in ["system Broken {", "system Broken { action x(] }"] {
            let diagnostics = parse_source(source).unwrap_err();
            assert!(diagnostics.iter().any(|item| item.code == "NMLT0002"));
        }
    }

    #[test]
    fn rejects_files_without_a_system() {
        let diagnostics = parse_source("data Bool = True | False").unwrap_err();
        assert!(diagnostics.iter().any(|item| item.code == "NMLT0005"));
    }
}

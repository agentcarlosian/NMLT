use std::fmt;

use crate::{Diagnostic, Span};

/// A lossless lexical category. Token text is always recovered from its span.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    LineComment,
    BlockComment,
    Identifier,
    Integer,
    String,
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Punctuation,
    Unknown,
    Error,
}

impl TokenKind {
    #[must_use]
    pub const fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::Whitespace | Self::LineComment | Self::BlockComment
        )
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// One half-open byte range in the original UTF-8 source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    #[must_use]
    pub fn text(self, source: &str) -> &str {
        &source[self.span.start..self.span.end]
    }
}

/// Lossless lexical output. Diagnostics do not remove malformed bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexedFile {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

impl LexedFile {
    #[must_use]
    pub fn reconstruct(&self, source: &str) -> String {
        self.tokens.iter().map(|token| token.text(source)).collect()
    }

    #[must_use]
    pub fn covers(&self, source: &str) -> bool {
        let mut cursor = 0;
        for token in &self.tokens {
            if token.span.start != cursor
                || token.span.end < token.span.start
                || token.span.end > source.len()
                || !source.is_char_boundary(token.span.start)
                || !source.is_char_boundary(token.span.end)
            {
                return false;
            }
            cursor = token.span.end;
        }
        cursor == source.len()
    }
}

/// Lex UTF-8 source according to RFC 0003 without discarding any bytes.
#[must_use]
pub fn lex_source(source: &str) -> LexedFile {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let start = index;
        let kind = match bytes[index] {
            b' ' | b'\t' | b'\r' | b'\n' => {
                index += 1;
                while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\r' | b'\n') {
                    index += 1;
                }
                TokenKind::Whitespace
            }
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                    index += 1;
                }
                TokenKind::LineComment
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
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
                if closed {
                    TokenKind::BlockComment
                } else {
                    index = bytes.len();
                    diagnostics.push(Diagnostic::error(
                        "NMLT1001",
                        "unterminated block comment",
                        Some(Span::new(start, index)),
                    ));
                    TokenKind::Error
                }
            }
            b'"' => {
                index += 1;
                let mut closed = false;
                while index < bytes.len() {
                    match bytes[index] {
                        b'"' => {
                            index += 1;
                            closed = true;
                            break;
                        }
                        b'\r' | b'\n' => break,
                        b'\\' => {
                            index += 1;
                            if index < bytes.len() {
                                let escaped = source[index..]
                                    .chars()
                                    .next()
                                    .expect("index remains inside source");
                                index += escaped.len_utf8();
                            }
                        }
                        _ => {
                            let character = source[index..]
                                .chars()
                                .next()
                                .expect("index remains inside source");
                            index += character.len_utf8();
                        }
                    }
                }
                if closed {
                    TokenKind::String
                } else {
                    diagnostics.push(Diagnostic::error(
                        "NMLT1002",
                        "unterminated string literal",
                        Some(Span::new(start, index)),
                    ));
                    TokenKind::Error
                }
            }
            b'{' => single(&mut index, TokenKind::LeftBrace),
            b'}' => single(&mut index, TokenKind::RightBrace),
            b'(' => single(&mut index, TokenKind::LeftParen),
            b')' => single(&mut index, TokenKind::RightParen),
            b'[' => single(&mut index, TokenKind::LeftBracket),
            b']' => single(&mut index, TokenKind::RightBracket),
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                TokenKind::Identifier
            }
            byte if byte.is_ascii_digit() => {
                index += 1;
                while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'_')
                {
                    index += 1;
                }
                TokenKind::Integer
            }
            byte if is_operator_punctuation(byte) => {
                index += 1;
                while index < bytes.len()
                    && is_operator_punctuation(bytes[index])
                    && !(bytes[index] == b'/'
                        && matches!(bytes.get(index + 1), Some(b'/') | Some(b'*')))
                {
                    index += 1;
                }
                TokenKind::Punctuation
            }
            _ => {
                let character = source[index..]
                    .chars()
                    .next()
                    .expect("index remains inside source");
                index += character.len_utf8();
                TokenKind::Unknown
            }
        };
        tokens.push(Token {
            kind,
            span: Span::new(start, index),
        });
    }

    let lexed = LexedFile {
        tokens,
        diagnostics,
    };
    debug_assert!(lexed.covers(source));
    lexed
}

fn single(index: &mut usize, kind: TokenKind) -> TokenKind {
    *index += 1;
    kind
}

fn is_operator_punctuation(byte: u8) -> bool {
    byte.is_ascii_punctuation() && !matches!(byte, b'"' | b'{' | b'}' | b'(' | b')' | b'[' | b']')
}

#[cfg(test)]
mod tests {
    use super::{TokenKind, lex_source};

    #[test]
    fn round_trips_every_byte() {
        let source = "// λ\r\nsystem Clock { bit := \"tick\\\"\" /* note */ }\n";
        let lexed = lex_source(source);
        assert!(lexed.diagnostics.is_empty());
        assert!(lexed.covers(source));
        assert_eq!(lexed.reconstruct(source), source);
    }

    #[test]
    fn preserves_unicode_outside_strings_as_unknown() {
        let source = "λ";
        let lexed = lex_source(source);
        assert_eq!(lexed.tokens[0].kind, TokenKind::Unknown);
        assert_eq!(lexed.tokens[0].text(source), "λ");
        assert!(lexed.covers(source));
    }

    #[test]
    fn retains_unterminated_constructs() {
        for (source, code) in [("/* open", "NMLT1001"), ("\"open", "NMLT1002")] {
            let lexed = lex_source(source);
            assert_eq!(lexed.reconstruct(source), source);
            assert_eq!(lexed.tokens[0].kind, TokenKind::Error);
            assert_eq!(lexed.diagnostics[0].code, code);
        }
    }

    #[test]
    fn excludes_line_endings_from_line_comments() {
        let source = "// note\r\nsystem S {}";
        let lexed = lex_source(source);
        assert_eq!(lexed.tokens[0].kind, TokenKind::LineComment);
        assert_eq!(lexed.tokens[0].text(source), "// note");
        assert_eq!(lexed.tokens[1].kind, TokenKind::Whitespace);
    }

    #[test]
    fn retains_maximal_punctuation_runs_around_semicolons() {
        let source = "+; ;; ;+ ;";
        let lexed = lex_source(source);
        let punctuation = lexed
            .tokens
            .iter()
            .filter(|token| token.kind == TokenKind::Punctuation)
            .map(|token| token.text(source))
            .collect::<Vec<_>>();
        assert_eq!(punctuation, ["+;", ";;", ";+", ";"]);
        assert_eq!(lexed.reconstruct(source), source);
    }
}

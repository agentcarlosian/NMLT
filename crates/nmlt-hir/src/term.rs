//! Parser for the frozen M9 raw type and expression fragment.

use nmlt_core::{RawTerm, Span, TokenKind, lex_source};

use crate::{DefPath, SourceSpan};

pub(crate) const MAX_TERM_DEPTH: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalBinderInput {
    pub owner: DefPath,
    pub index: u32,
    pub name: String,
    pub name_span: SourceSpan,
    pub declared_type: RawTermInput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawTermInput {
    pub owner: DefPath,
    pub root: TermRootInput,
    pub source: String,
    pub span: SourceSpan,
    pub kind: RawTermInputKind,
}

impl RawTermInput {
    pub fn new(
        owner: DefPath,
        root: TermRootInput,
        term: &RawTerm,
        kind: RawTermInputKind,
    ) -> Self {
        Self {
            owner,
            root,
            source: term.source.text.clone(),
            span: SourceSpan::new(term.source.span.start, term.source.span.end),
            kind,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TermRootInput {
    DeclaredType,
    Initializer,
    ActionParameterType(u32),
    Guard(u32),
    UpdateTarget(u32),
    UpdateValue(u32),
    Output(u32),
    Consume(u32),
    PropertyBody,
    ObservationItems,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RawTermInputKind {
    Type,
    Expression,
    ExpressionList,
    UpdateTarget,
    Consume,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NameUse {
    pub qualifier: Option<String>,
    pub spelling: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedType {
    pub span: SourceSpan,
    pub depth: usize,
    pub kind: ParsedTypeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ParsedTypeKind {
    Bool,
    Nat,
    Int,
    Named(NameUse),
    Once {
        protocol: String,
        protocol_span: SourceSpan,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedExpr {
    pub span: SourceSpan,
    pub depth: usize,
    pub kind: ParsedExprKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ParsedExprKind {
    Bool(bool),
    Natural(Vec<u8>),
    Name(NameUse),
    Unary {
        operator: ParsedUnaryOp,
        operand: Box<ParsedExpr>,
    },
    Binary {
        operator: ParsedBinaryOp,
        left: Box<ParsedExpr>,
        right: Box<ParsedExpr>,
    },
    Builtin {
        builtin: ParsedBuiltin,
        arguments: Vec<ParsedExpr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParsedUnaryOp {
    Not,
    Negate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParsedBinaryOp {
    Or,
    And,
    Implies,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
}

impl ParsedBinaryOp {
    pub const fn is_comparison(self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::NotEqual
                | Self::Less
                | Self::LessEqual
                | Self::Greater
                | Self::GreaterEqual
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParsedBuiltin {
    ToInt,
    Always,
    Eventually,
    Next,
    Until,
    Enabled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TermParseError {
    pub span: SourceSpan,
    pub message: String,
}

pub(crate) fn parse_type(input: &RawTermInput) -> Result<ParsedType, TermParseError> {
    let mut parser = Parser::new(&input.source, input.span.start)?;
    let parsed = parser.parse_type()?;
    parser.expect_end("type")?;
    Ok(parsed)
}

pub(crate) fn parse_expression(input: &RawTermInput) -> Result<ParsedExpr, TermParseError> {
    let mut parser = Parser::new(&input.source, input.span.start)?;
    let parsed = parser.parse_expression(0)?;
    parser.expect_end("expression")?;
    Ok(parsed)
}

pub(crate) fn parse_expression_list(
    input: &RawTermInput,
) -> Result<Vec<ParsedExpr>, TermParseError> {
    let mut parser = Parser::new(&input.source, input.span.start)?;
    let mut expressions = Vec::new();
    if parser.at_end() {
        return Err(parser.error_here("observation list cannot be empty"));
    }
    loop {
        expressions.push(parser.parse_expression(0)?);
        if !parser.eat(",") {
            break;
        }
        if parser.at_end() {
            return Err(parser.error_here("expected an expression after `,`"));
        }
    }
    parser.expect_end("observation list")?;
    Ok(expressions)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LexicalKind {
    Identifier,
    Integer,
    LeftParen,
    RightParen,
    Punctuation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LexicalToken {
    kind: LexicalKind,
    text: String,
    span: SourceSpan,
}

struct Parser {
    tokens: Vec<LexicalToken>,
    index: usize,
}

impl Parser {
    fn new(source: &str, base: usize) -> Result<Self, TermParseError> {
        let lexed = lex_source(source);
        if let Some(diagnostic) = lexed.diagnostics.first() {
            let relative = diagnostic.span.unwrap_or(Span::new(0, source.len()));
            return Err(TermParseError {
                span: SourceSpan::new(base + relative.start, base + relative.end),
                message: format!("{}: {}", diagnostic.code, diagnostic.message),
            });
        }

        let mut tokens = Vec::new();
        for token in lexed.tokens {
            if token.kind.is_trivia() {
                continue;
            }
            let span = SourceSpan::new(base + token.span.start, base + token.span.end);
            match token.kind {
                TokenKind::Identifier => tokens.push(LexicalToken {
                    kind: LexicalKind::Identifier,
                    text: token.text(source).to_owned(),
                    span,
                }),
                TokenKind::Integer => tokens.push(LexicalToken {
                    kind: LexicalKind::Integer,
                    text: token.text(source).to_owned(),
                    span,
                }),
                TokenKind::LeftParen => tokens.push(LexicalToken {
                    kind: LexicalKind::LeftParen,
                    text: "(".to_owned(),
                    span,
                }),
                TokenKind::RightParen => tokens.push(LexicalToken {
                    kind: LexicalKind::RightParen,
                    text: ")".to_owned(),
                    span,
                }),
                TokenKind::Punctuation => {
                    split_punctuation(token.text(source), span, &mut tokens)?;
                }
                TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment => {}
                TokenKind::String
                | TokenKind::LeftBrace
                | TokenKind::RightBrace
                | TokenKind::LeftBracket
                | TokenKind::RightBracket
                | TokenKind::Unknown
                | TokenKind::Error => {
                    return Err(TermParseError {
                        span,
                        message: format!(
                            "token `{}` is outside the first M9 term fragment",
                            token.text(source)
                        ),
                    });
                }
            }
        }
        Ok(Self { tokens, index: 0 })
    }

    fn parse_type(&mut self) -> Result<ParsedType, TermParseError> {
        let token = self
            .take()
            .ok_or_else(|| self.error_here("expected a type"))?;
        if token.kind != LexicalKind::Identifier {
            return Err(TermParseError {
                span: token.span,
                message: "expected a type name".to_owned(),
            });
        }
        match token.text.as_str() {
            "Bool" => Ok(ParsedType {
                span: token.span,
                depth: 1,
                kind: ParsedTypeKind::Bool,
            }),
            "Nat" => Ok(ParsedType {
                span: token.span,
                depth: 1,
                kind: ParsedTypeKind::Nat,
            }),
            "Int" => Ok(ParsedType {
                span: token.span,
                depth: 1,
                kind: ParsedTypeKind::Int,
            }),
            "Once" => {
                self.expect("<", "expected `<` after `Once`")?;
                let protocol = self
                    .take()
                    .ok_or_else(|| self.error_here("expected a protocol tag inside `Once<...>`"))?;
                if protocol.kind != LexicalKind::Identifier {
                    return Err(TermParseError {
                        span: protocol.span,
                        message: "a `Once` protocol tag must be an identifier".to_owned(),
                    });
                }
                let close = self.expect(">", "expected `>` after `Once` protocol tag")?;
                Ok(ParsedType {
                    span: SourceSpan::new(token.span.start, close.span.end),
                    depth: 2,
                    kind: ParsedTypeKind::Once {
                        protocol: protocol.text,
                        protocol_span: protocol.span,
                    },
                })
            }
            _ => {
                let name = self.finish_name(token)?;
                Ok(ParsedType {
                    span: name.span,
                    depth: 1,
                    kind: ParsedTypeKind::Named(name),
                })
            }
        }
    }

    fn parse_expression(
        &mut self,
        minimum_binding_power: u8,
    ) -> Result<ParsedExpr, TermParseError> {
        let mut left = self.parse_prefix()?;
        loop {
            let Some((operator, left_power, right_power)) = self.current_binary_operator() else {
                break;
            };
            if left_power < minimum_binding_power {
                break;
            }
            if operator.is_comparison()
                && matches!(
                    left.kind,
                    ParsedExprKind::Binary {
                        operator: previous,
                        ..
                    } if previous.is_comparison()
                )
            {
                return Err(self.error_here("comparison operators do not chain in M9"));
            }
            self.index += 1;
            let right = self.parse_expression(right_power)?;
            let depth = 1 + left.depth.max(right.depth);
            self.enforce_depth(depth, SourceSpan::new(left.span.start, right.span.end))?;
            left = ParsedExpr {
                span: SourceSpan::new(left.span.start, right.span.end),
                depth,
                kind: ParsedExprKind::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<ParsedExpr, TermParseError> {
        let token = self
            .take()
            .ok_or_else(|| self.error_here("expected an expression"))?;
        match token.kind {
            LexicalKind::Integer => Ok(ParsedExpr {
                span: token.span,
                depth: 1,
                kind: ParsedExprKind::Natural(decimal_magnitude(&token.text, token.span)?),
            }),
            LexicalKind::Identifier if token.text == "true" || token.text == "false" => {
                Ok(ParsedExpr {
                    span: token.span,
                    depth: 1,
                    kind: ParsedExprKind::Bool(token.text == "true"),
                })
            }
            LexicalKind::Identifier if token.text == "not" => {
                self.parse_unary(token.span.start, ParsedUnaryOp::Not)
            }
            LexicalKind::Punctuation if token.text == "-" => {
                let expression = self.parse_unary(token.span.start, ParsedUnaryOp::Negate)?;
                if matches!(
                    expression.kind,
                    ParsedExprKind::Unary {
                        ref operand,
                        operator: ParsedUnaryOp::Negate,
                    } if matches!(operand.kind, ParsedExprKind::Natural(ref magnitude) if magnitude.is_empty())
                ) {
                    return Err(TermParseError {
                        span: expression.span,
                        message: "negative zero is not canonical".to_owned(),
                    });
                }
                Ok(expression)
            }
            LexicalKind::Identifier => {
                let name = self.finish_name(token)?;
                if self.eat("(") {
                    if name.qualifier.is_some() {
                        return Err(TermParseError {
                            span: name.span,
                            message: "qualified calls are outside the first M9 fragment".to_owned(),
                        });
                    }
                    self.parse_builtin(name)
                } else {
                    Ok(ParsedExpr {
                        span: name.span,
                        depth: 1,
                        kind: ParsedExprKind::Name(name),
                    })
                }
            }
            LexicalKind::LeftParen => {
                let inner = self.parse_expression(0)?;
                let close = self.expect(")", "expected `)` to close expression")?;
                let depth = inner.depth + 1;
                let span = SourceSpan::new(token.span.start, close.span.end);
                self.enforce_depth(depth, span)?;
                Ok(ParsedExpr {
                    span,
                    depth,
                    kind: inner.kind,
                })
            }
            _ => Err(TermParseError {
                span: token.span,
                message: format!("unexpected token `{}` in expression", token.text),
            }),
        }
    }

    fn parse_unary(
        &mut self,
        start: usize,
        operator: ParsedUnaryOp,
    ) -> Result<ParsedExpr, TermParseError> {
        let operand = self.parse_expression(13)?;
        let depth = operand.depth + 1;
        let span = SourceSpan::new(start, operand.span.end);
        self.enforce_depth(depth, span)?;
        Ok(ParsedExpr {
            span,
            depth,
            kind: ParsedExprKind::Unary {
                operator,
                operand: Box::new(operand),
            },
        })
    }

    fn parse_builtin(&mut self, name: NameUse) -> Result<ParsedExpr, TermParseError> {
        let builtin = match name.spelling.as_str() {
            "to_int" => ParsedBuiltin::ToInt,
            "always" => ParsedBuiltin::Always,
            "eventually" => ParsedBuiltin::Eventually,
            "next" => ParsedBuiltin::Next,
            "until" => ParsedBuiltin::Until,
            "enabled" => ParsedBuiltin::Enabled,
            _ => {
                return Err(TermParseError {
                    span: name.span,
                    message: format!(
                        "ordinary function call `{}` is outside the first M9 fragment",
                        name.spelling
                    ),
                });
            }
        };
        let mut arguments = Vec::new();
        if !self.at(")") {
            loop {
                arguments.push(self.parse_expression(0)?);
                if !self.eat(",") {
                    break;
                }
            }
        }
        let close = self.expect(")", "expected `)` after builtin arguments")?;
        let expected = match builtin {
            ParsedBuiltin::Until => 2,
            ParsedBuiltin::ToInt
            | ParsedBuiltin::Always
            | ParsedBuiltin::Eventually
            | ParsedBuiltin::Next
            | ParsedBuiltin::Enabled => 1,
        };
        if arguments.len() != expected {
            return Err(TermParseError {
                span: SourceSpan::new(name.span.start, close.span.end),
                message: format!(
                    "builtin `{}` expects {expected} argument(s), found {}",
                    name.spelling,
                    arguments.len()
                ),
            });
        }
        let depth = 1 + arguments.iter().map(|item| item.depth).max().unwrap_or(0);
        let span = SourceSpan::new(name.span.start, close.span.end);
        self.enforce_depth(depth, span)?;
        Ok(ParsedExpr {
            span,
            depth,
            kind: ParsedExprKind::Builtin { builtin, arguments },
        })
    }

    fn finish_name(&mut self, first: LexicalToken) -> Result<NameUse, TermParseError> {
        if !self.eat(".") {
            return Ok(NameUse {
                qualifier: None,
                spelling: first.text,
                span: first.span,
            });
        }
        let second = self
            .take()
            .ok_or_else(|| self.error_here("expected a name after module qualifier"))?;
        if second.kind != LexicalKind::Identifier {
            return Err(TermParseError {
                span: second.span,
                message: "expected an identifier after module qualifier".to_owned(),
            });
        }
        if self.at(".") {
            return Err(
                self.error_here("the first M9 fragment permits at most one module qualifier")
            );
        }
        Ok(NameUse {
            qualifier: Some(first.text),
            spelling: second.text,
            span: SourceSpan::new(first.span.start, second.span.end),
        })
    }

    fn current_binary_operator(&self) -> Option<(ParsedBinaryOp, u8, u8)> {
        let text = self.current()?.text.as_str();
        Some(match text {
            "implies" => (ParsedBinaryOp::Implies, 1, 1),
            "or" => (ParsedBinaryOp::Or, 2, 3),
            "and" => (ParsedBinaryOp::And, 4, 5),
            "==" => (ParsedBinaryOp::Equal, 6, 7),
            "!=" => (ParsedBinaryOp::NotEqual, 6, 7),
            "<" => (ParsedBinaryOp::Less, 6, 7),
            "<=" => (ParsedBinaryOp::LessEqual, 6, 7),
            ">" => (ParsedBinaryOp::Greater, 6, 7),
            ">=" => (ParsedBinaryOp::GreaterEqual, 6, 7),
            "+" => (ParsedBinaryOp::Add, 8, 9),
            "-" => (ParsedBinaryOp::Subtract, 8, 9),
            "*" => (ParsedBinaryOp::Multiply, 10, 11),
            _ => return None,
        })
    }

    fn enforce_depth(&self, depth: usize, span: SourceSpan) -> Result<(), TermParseError> {
        if depth > MAX_TERM_DEPTH {
            Err(TermParseError {
                span,
                message: format!("term nesting exceeds the M9 limit of {MAX_TERM_DEPTH}"),
            })
        } else {
            Ok(())
        }
    }

    fn expect(&mut self, text: &str, message: &str) -> Result<LexicalToken, TermParseError> {
        if self.at(text) {
            Ok(self.take().expect("current token exists"))
        } else {
            Err(self.error_here(message))
        }
    }

    fn expect_end(&self, context: &str) -> Result<(), TermParseError> {
        if self.at_end() {
            Ok(())
        } else {
            Err(self.error_here(&format!("unexpected trailing token in {context}")))
        }
    }

    fn current(&self) -> Option<&LexicalToken> {
        self.tokens.get(self.index)
    }

    fn take(&mut self) -> Option<LexicalToken> {
        let token = self.current()?.clone();
        self.index += 1;
        Some(token)
    }

    fn at(&self, text: &str) -> bool {
        self.current().is_some_and(|token| token.text == text)
    }

    fn eat(&mut self, text: &str) -> bool {
        if self.at(text) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn at_end(&self) -> bool {
        self.index == self.tokens.len()
    }

    fn error_here(&self, message: &str) -> TermParseError {
        let span = self
            .current()
            .map_or(SourceSpan::new(0, 0), |token| token.span);
        TermParseError {
            span,
            message: message.to_owned(),
        }
    }
}

fn split_punctuation(
    text: &str,
    span: SourceSpan,
    output: &mut Vec<LexicalToken>,
) -> Result<(), TermParseError> {
    let mut offset = 0;
    while offset < text.len() {
        let remaining = &text[offset..];
        let width = if ["==", "!=", "<=", ">="]
            .iter()
            .any(|operator| remaining.starts_with(operator))
        {
            2
        } else if matches!(
            remaining.as_bytes()[0],
            b'<' | b'>' | b'+' | b'-' | b'*' | b',' | b'.'
        ) {
            1
        } else {
            return Err(TermParseError {
                span: SourceSpan::new(span.start + offset, span.end),
                message: format!("punctuation `{remaining}` is outside the first M9 term fragment"),
            });
        };
        output.push(LexicalToken {
            kind: LexicalKind::Punctuation,
            text: remaining[..width].to_owned(),
            span: SourceSpan::new(span.start + offset, span.start + offset + width),
        });
        offset += width;
    }
    Ok(())
}

fn decimal_magnitude(text: &str, span: SourceSpan) -> Result<Vec<u8>, TermParseError> {
    let digits = text
        .bytes()
        .filter(|byte| *byte != b'_')
        .collect::<Vec<_>>();
    if digits.is_empty() || digits.iter().any(|byte| !byte.is_ascii_digit()) {
        return Err(TermParseError {
            span,
            message: "invalid natural-number literal".to_owned(),
        });
    }
    if digits.len() > 1 && digits[0] == b'0' {
        return Err(TermParseError {
            span,
            message: "integer literals must not contain leading zeroes".to_owned(),
        });
    }
    let mut magnitude = Vec::<u8>::new();
    for digit in digits {
        let mut carry = u16::from(digit - b'0');
        for byte in magnitude.iter_mut().rev() {
            let value = u16::from(*byte) * 10 + carry;
            *byte = (value & 0xff) as u8;
            carry = value >> 8;
        }
        if carry != 0 {
            magnitude.insert(0, carry as u8);
        }
        if magnitude.len() > 4_096 {
            return Err(TermParseError {
                span,
                message: "integer magnitude exceeds the M9 limit of 4096 bytes".to_owned(),
            });
        }
    }
    while magnitude.first() == Some(&0) {
        magnitude.remove(0);
    }
    Ok(magnitude)
}

#[cfg(test)]
mod tests {
    use super::{
        ParsedBinaryOp, ParsedBuiltin, ParsedExprKind, ParsedTypeKind, RawTermInput,
        RawTermInputKind, TermRootInput, parse_expression, parse_expression_list, parse_type,
    };
    use crate::{DefPath, Namespace, SourceSpan};

    fn input(source: &str, kind: RawTermInputKind) -> RawTermInput {
        RawTermInput {
            owner: DefPath::top_level(Namespace::Value, "owner"),
            root: TermRootInput::Initializer,
            source: source.to_owned(),
            span: SourceSpan::new(10, 10 + source.len()),
            kind,
        }
    }

    #[test]
    fn parses_the_frozen_type_fragment() {
        assert!(matches!(
            parse_type(&input("Bool", RawTermInputKind::Type))
                .unwrap()
                .kind,
            ParsedTypeKind::Bool
        ));
        assert!(matches!(
            parse_type(&input("Once<Effect>", RawTermInputKind::Type))
                .unwrap()
                .kind,
            ParsedTypeKind::Once { protocol, .. } if protocol == "Effect"
        ));
        assert!(matches!(
            parse_type(&input("Types.Phase", RawTermInputKind::Type))
                .unwrap()
                .kind,
            ParsedTypeKind::Named(name)
                if name.qualifier.as_deref() == Some("Types") && name.spelling == "Phase"
        ));
    }

    #[test]
    fn precedence_and_temporal_forms_are_structural() {
        let parsed = parse_expression(&input(
            "always(x > 0 implies not enabled(step))",
            RawTermInputKind::Expression,
        ))
        .unwrap();
        let ParsedExprKind::Builtin {
            builtin: ParsedBuiltin::Always,
            arguments,
        } = parsed.kind
        else {
            panic!("expected always")
        };
        assert!(matches!(
            arguments[0].kind,
            ParsedExprKind::Binary {
                operator: ParsedBinaryOp::Implies,
                ..
            }
        ));
    }

    #[test]
    fn observation_lists_and_canonical_integers_fail_closed() {
        assert_eq!(
            parse_expression_list(&input(
                "phase, count, ready",
                RawTermInputKind::ExpressionList
            ))
            .unwrap()
            .len(),
            3
        );
        assert!(parse_expression(&input("00", RawTermInputKind::Expression)).is_err());
        assert!(parse_expression(&input("-0", RawTermInputKind::Expression)).is_err());
        assert!(parse_expression(&input("f(x)", RawTermInputKind::Expression)).is_err());
    }
}

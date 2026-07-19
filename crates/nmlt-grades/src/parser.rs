use crate::{Grade, IterationBound, Plan, Program};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseDiagnostic {
    pub code: &'static str,
    pub offset: usize,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TokenKind {
    LeftParen,
    RightParen,
    Word(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Token {
    kind: TokenKind,
    offset: usize,
    line: usize,
    column: usize,
}

/// Parse the extension's deliberately small S-expression surface language.
///
/// ```text
/// program NAME
/// budget COST PRIVACY ENERGY UNCERTAINTY
/// plan (seq (atom NAME COST PRIVACY ENERGY UNCERTAINTY) ...)
/// ```
pub fn parse_program(source: &str) -> Result<Program, Vec<ParseDiagnostic>> {
    let tokens = tokenize(source);
    let mut parser = Parser {
        tokens,
        cursor: 0,
        eof_offset: source.len(),
    };
    match parser.program() {
        Ok(program) if parser.cursor == parser.tokens.len() => Ok(program),
        Ok(_) => Err(vec![parser.diagnostic_here(
            "NMLT-GRADE-PARSE-TRAILING",
            "unexpected tokens after the plan",
        )]),
        Err(diagnostic) => Err(vec![diagnostic]),
    }
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    eof_offset: usize,
}

impl Parser {
    fn program(&mut self) -> Result<Program, ParseDiagnostic> {
        self.expect_keyword("program")?;
        let name = self.expect_word("a program name")?.0;
        self.expect_keyword("budget")?;
        let budget = self.grade()?;
        self.expect_keyword("plan")?;
        let plan = self.plan()?;
        Ok(Program { name, budget, plan })
    }

    fn plan(&mut self) -> Result<Plan, ParseDiagnostic> {
        self.expect_left_paren()?;
        let (operator, operator_token) = self.expect_word("a plan operator")?;
        let plan = match operator.as_str() {
            "atom" => {
                let name = self.expect_word("an atom name")?.0;
                let grade = self.grade()?;
                Plan::Atom { name, grade }
            }
            "seq" => Plan::Sequence(self.plan_list()?),
            "choice" => Plan::Choice(self.plan_list()?),
            "par" => Plan::Parallel(self.plan_list()?),
            "repeat" => {
                let (count, count_token) = self.expect_word("an iteration count or ?")?;
                let count = if count == "?" {
                    IterationBound::Unknown
                } else {
                    IterationBound::Exact(count.parse::<u64>().map_err(|_| ParseDiagnostic {
                        code: "NMLT-GRADE-PARSE-NATURAL",
                        offset: count_token.offset,
                        line: count_token.line,
                        column: count_token.column,
                        message: format!("iteration count {count:?} is not a u64 natural number"),
                    })?)
                };
                let body = Box::new(self.plan()?);
                Plan::Repeat { count, body }
            }
            _ => {
                return Err(ParseDiagnostic {
                    code: "NMLT-GRADE-PARSE-OPERATOR",
                    offset: operator_token.offset,
                    line: operator_token.line,
                    column: operator_token.column,
                    message: format!("unknown plan operator {operator:?}"),
                });
            }
        };
        self.expect_right_paren()?;
        Ok(plan)
    }

    fn plan_list(&mut self) -> Result<Vec<Plan>, ParseDiagnostic> {
        let mut plans = Vec::new();
        loop {
            match self.peek().map(|token| &token.kind) {
                Some(TokenKind::RightParen) => return Ok(plans),
                Some(_) => plans.push(self.plan()?),
                None => {
                    return Err(self
                        .diagnostic_here("NMLT-GRADE-PARSE-EOF", "unterminated plan; expected )"));
                }
            }
        }
    }

    fn grade(&mut self) -> Result<Grade, ParseDiagnostic> {
        let cost = self.natural("cost_ticks")?;
        let privacy = self.natural("privacy_micro_epsilon")?;
        let energy = self.natural("energy_microjoules")?;
        let (uncertainty_text, uncertainty_token) = self.expect_word("uncertainty_ppm")?;
        let uncertainty = uncertainty_text
            .parse::<u32>()
            .map_err(|_| ParseDiagnostic {
                code: "NMLT-GRADE-PARSE-NATURAL",
                offset: uncertainty_token.offset,
                line: uncertainty_token.line,
                column: uncertainty_token.column,
                message: format!(
                    "uncertainty_ppm {uncertainty_text:?} is not a u32 natural number"
                ),
            })?;
        Grade::checked(cost, privacy, energy, uncertainty).map_err(|error| ParseDiagnostic {
            code: "NMLT-GRADE-PARSE-GRADE",
            offset: uncertainty_token.offset,
            line: uncertainty_token.line,
            column: uncertainty_token.column,
            message: error.to_string(),
        })
    }

    fn natural(&mut self, label: &str) -> Result<u64, ParseDiagnostic> {
        let (text, token) = self.expect_word(label)?;
        text.parse::<u64>().map_err(|_| ParseDiagnostic {
            code: "NMLT-GRADE-PARSE-NATURAL",
            offset: token.offset,
            line: token.line,
            column: token.column,
            message: format!("{label} {text:?} is not a u64 natural number"),
        })
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<(), ParseDiagnostic> {
        let (actual, token) = self.expect_word(keyword)?;
        if actual == keyword {
            Ok(())
        } else {
            Err(ParseDiagnostic {
                code: "NMLT-GRADE-PARSE-KEYWORD",
                offset: token.offset,
                line: token.line,
                column: token.column,
                message: format!("expected keyword {keyword:?}, found {actual:?}"),
            })
        }
    }

    fn expect_word(&mut self, expected: &str) -> Result<(String, Token), ParseDiagnostic> {
        let token = self.next().ok_or_else(|| {
            self.diagnostic_here(
                "NMLT-GRADE-PARSE-EOF",
                &format!("expected {expected}, found end of file"),
            )
        })?;
        match &token.kind {
            TokenKind::Word(word) => Ok((word.clone(), token)),
            _ => Err(ParseDiagnostic {
                code: "NMLT-GRADE-PARSE-TOKEN",
                offset: token.offset,
                line: token.line,
                column: token.column,
                message: format!("expected {expected}"),
            }),
        }
    }

    fn expect_left_paren(&mut self) -> Result<(), ParseDiagnostic> {
        self.expect_punctuation(TokenKind::LeftParen, "(")
    }

    fn expect_right_paren(&mut self) -> Result<(), ParseDiagnostic> {
        self.expect_punctuation(TokenKind::RightParen, ")")
    }

    fn expect_punctuation(
        &mut self,
        expected_kind: TokenKind,
        spelling: &str,
    ) -> Result<(), ParseDiagnostic> {
        let token = self.next().ok_or_else(|| {
            self.diagnostic_here(
                "NMLT-GRADE-PARSE-EOF",
                &format!("expected {spelling}, found end of file"),
            )
        })?;
        if token.kind == expected_kind {
            Ok(())
        } else {
            Err(ParseDiagnostic {
                code: "NMLT-GRADE-PARSE-TOKEN",
                offset: token.offset,
                line: token.line,
                column: token.column,
                message: format!("expected {spelling}"),
            })
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.cursor).cloned();
        if token.is_some() {
            self.cursor += 1;
        }
        token
    }

    fn diagnostic_here(&self, code: &'static str, message: &str) -> ParseDiagnostic {
        if let Some(token) = self.peek() {
            ParseDiagnostic {
                code,
                offset: token.offset,
                line: token.line,
                column: token.column,
                message: message.to_owned(),
            }
        } else {
            ParseDiagnostic {
                code,
                offset: self.eof_offset,
                line: 0,
                column: 0,
                message: message.to_owned(),
            }
        }
    }
}

fn tokenize(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut offset = 0;
    let mut line = 1;
    let mut column = 1;
    while offset < bytes.len() {
        match bytes[offset] {
            b'\n' => {
                offset += 1;
                line += 1;
                column = 1;
            }
            byte if byte.is_ascii_whitespace() => {
                offset += 1;
                column += 1;
            }
            b'#' => {
                while offset < bytes.len() && bytes[offset] != b'\n' {
                    offset += 1;
                    column += 1;
                }
            }
            b'(' | b')' => {
                let kind = if bytes[offset] == b'(' {
                    TokenKind::LeftParen
                } else {
                    TokenKind::RightParen
                };
                tokens.push(Token {
                    kind,
                    offset,
                    line,
                    column,
                });
                offset += 1;
                column += 1;
            }
            _ => {
                let start = offset;
                let start_column = column;
                while offset < bytes.len()
                    && !bytes[offset].is_ascii_whitespace()
                    && !matches!(bytes[offset], b'(' | b')' | b'#')
                {
                    offset += 1;
                    column += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Word(source[start..offset].to_owned()),
                    offset: start,
                    line,
                    column: start_column,
                });
            }
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Analysis, analyze};

    #[test]
    fn parses_comments_and_compositions() {
        let source = r#"
            # Units are declared by the extension RFC.
            program sample
            budget 100 600000 180 60000
            plan (seq
              (atom start 12 100000 30 10000)
              (choice
                (atom cache 4 0 8 2000)
                (atom fetch 34 300000 85 30000))
              (par
                (atom audit 8 50000 20 3000)
                (atom metrics 6 0 12 2000))
              (repeat 2 (atom retry 3 25000 4 1000)))
        "#;
        let program = parse_program(source).expect("valid program");
        assert_eq!(program.name, "sample");
        assert_eq!(
            analyze(&program.plan),
            Analysis::Exact(Grade::checked(66, 500_000, 155, 47_000).unwrap())
        );
    }

    #[test]
    fn parses_unknown_iteration_without_approving_it() {
        let program =
            parse_program("program u budget 10 10 10 10 plan (repeat ? (atom work 1 1 1 1))")
                .expect("unknown is syntactically explicit");
        assert!(matches!(analyze(&program.plan), Analysis::Unknown(_)));
    }

    #[test]
    fn rejects_out_of_range_uncertainty() {
        let diagnostics =
            parse_program("program bad budget 10 10 10 10 plan (atom bad 1 1 1 1000001)")
                .expect_err("invalid grade must not parse");
        assert_eq!(diagnostics[0].code, "NMLT-GRADE-PARSE-GRADE");
    }

    #[test]
    fn rejects_trailing_tokens() {
        let diagnostics =
            parse_program("program bad budget 1 1 1 1 plan (atom ok 0 0 0 0) surprise")
                .expect_err("trailing text must be rejected");
        assert_eq!(diagnostics[0].code, "NMLT-GRADE-PARSE-TRAILING");
    }
}

use nmlt_core::{Span, Token, TokenKind, lex_source};

use crate::ast::{
    Action, BinaryOp, Expr, Model, Property, PropertyKind, StateVar, Type, UnaryOp, Update, Value,
};

#[derive(Clone, Debug)]
struct SigToken {
    token: Token,
    text: String,
    newline_before: bool,
}

impl SigToken {
    fn span(&self) -> Span {
        self.token.span
    }
}

/// Parse the executable Phase 2 fragment. Unknown declarations are rejected.
pub fn parse_model(source: &str) -> Result<Model, Vec<String>> {
    let lexed = lex_source(source);
    if !lexed.diagnostics.is_empty() {
        return Err(lexed
            .diagnostics
            .into_iter()
            .map(|item| format!("{}: {}", item.code, item.message))
            .collect());
    }
    let mut newline = false;
    let mut tokens = Vec::new();
    for token in lexed.tokens {
        if token.kind.is_trivia() {
            if token.text(source).contains(['\n', '\r']) {
                newline = true;
            }
            continue;
        }
        tokens.push(SigToken {
            token,
            text: token.text(source).to_owned(),
            newline_before: std::mem::take(&mut newline),
        });
    }
    Parser::new(tokens).parse()
}

struct Parser {
    tokens: Vec<SigToken>,
    index: usize,
    errors: Vec<String>,
}

impl Parser {
    fn new(tokens: Vec<SigToken>) -> Self {
        Self {
            tokens,
            index: 0,
            errors: Vec::new(),
        }
    }

    fn parse(mut self) -> Result<Model, Vec<String>> {
        while !self.at_end() && !self.at("system") {
            self.skip_top_level_declaration();
        }
        let start = self.current_span().unwrap_or(Span::new(0, 0));
        self.expect("system");
        let system_name = self.take_identifier("system name");
        if self.at("(") {
            self.skip_balanced("(", ")");
        }
        self.expect("{");

        let mut states = Vec::new();
        let mut capabilities = Vec::new();
        let mut actions = Vec::new();
        let mut properties = Vec::new();
        let mut observations = Vec::new();

        while !self.at_end() && !self.at("}") {
            match self.current_text() {
                Some("state") => states.push(self.parse_state()),
                Some("capability") => capabilities.push(self.parse_capability()),
                Some("action") => actions.push(self.parse_action()),
                Some("safety") => properties.push(self.parse_property(PropertyKind::Safety)),
                Some("temporal") => properties.push(self.parse_property(PropertyKind::Temporal)),
                Some("observe") => observations.extend(self.parse_observations()),
                Some("port" | "input" | "const" | "resource" | "hide" | "refine") => {
                    self.skip_declaration()
                }
                Some(other) => {
                    self.errors.push(format!(
                        "NMLT2001 at {}: unsupported system declaration `{other}` in executable fragment",
                        self.current_span().map_or(0, |span| span.start)
                    ));
                    self.skip_declaration();
                }
                None => break,
            }
        }
        let end = self.current_span().map_or(start.end, |span| span.end);
        self.expect("}");
        if states.is_empty() {
            self.errors
                .push("NMLT2002: executable system must declare state".to_owned());
        }
        if self.errors.is_empty() {
            Ok(Model {
                system_name,
                states,
                capabilities,
                actions,
                properties,
                observations,
                span: Span::new(start.start, end),
            })
        } else {
            Err(self.errors)
        }
    }

    fn parse_state(&mut self) -> StateVar {
        let start = self.bump().expect("state token").span();
        let name = self.take_identifier("state name");
        self.expect(":");
        let ty_name = self.take_identifier("state type");
        let ty = match ty_name.as_str() {
            "Bool" => Type::Bool,
            "Nat" | "PosNat" => Type::Nat,
            "Int" => Type::Int,
            _ => Type::Named(ty_name),
        };
        if self.at("<") {
            self.skip_type_arguments();
        }
        self.expect("=");
        let expression_tokens = self.take_until_declaration();
        let initial = self.parse_expression_tokens(&expression_tokens, "state initializer");
        let end = expression_tokens
            .last()
            .map_or(start.end, |token| token.span().end);
        StateVar {
            name,
            ty,
            initial,
            span: Span::new(start.start, end),
        }
    }

    fn parse_capability(&mut self) -> String {
        self.bump();
        let name = self.take_identifier("capability name");
        self.skip_declaration();
        name
    }

    fn parse_action(&mut self) -> Action {
        let start = self.bump().expect("action token").span();
        let name = self.take_identifier("action name");
        if self.at("(") {
            self.skip_balanced("(", ")");
        }
        if self.at("grade") {
            self.bump();
            if self.at("{") {
                self.skip_balanced("{", "}");
            }
        }
        self.expect("{");
        let mut guards = Vec::new();
        let mut updates = Vec::new();
        let mut consumes = Vec::new();
        while !self.at_end() && !self.at("}") {
            match self.current_text() {
                Some("require") => {
                    self.bump();
                    let expression_tokens = self.take_until_statement();
                    guards.push(self.parse_expression_tokens(&expression_tokens, "action guard"));
                }
                Some("set") => {
                    let update_start = self.bump().expect("set token").span();
                    let target = self.take_identifier("update target");
                    if self.at("[") {
                        self.errors.push(format!(
                            "NMLT2003 at {}: indexed updates are outside the executable provider fragment",
                            update_start.start
                        ));
                        self.skip_balanced("[", "]");
                    }
                    self.expect("=");
                    let expression_tokens = self.take_until_statement();
                    let value = self.parse_expression_tokens(&expression_tokens, "state update");
                    let end = expression_tokens
                        .last()
                        .map_or(update_start.end, |token| token.span().end);
                    updates.push(Update {
                        target,
                        value,
                        span: Span::new(update_start.start, end),
                    });
                }
                Some("consume") => {
                    self.bump();
                    consumes.push(self.take_identifier("consumed capability"));
                    self.consume_optional(";");
                }
                Some("emit") => {
                    self.skip_statement();
                }
                Some(other) => {
                    self.errors.push(format!(
                        "NMLT2004 at {}: unsupported action statement `{other}`",
                        self.current_span().map_or(0, |span| span.start)
                    ));
                    self.skip_statement();
                }
                None => break,
            }
        }
        let end = self.current_span().map_or(start.end, |span| span.end);
        self.expect("}");
        Action {
            name,
            guards,
            updates,
            consumes,
            span: Span::new(start.start, end),
        }
    }

    fn parse_property(&mut self, kind: PropertyKind) -> Property {
        let start = self.bump().expect("property token").span();
        let name = self.take_identifier("property name");
        self.expect("=");
        let expression_tokens = self.take_until_declaration();
        let expression = self.parse_expression_tokens(&expression_tokens, "property");
        let end = expression_tokens
            .last()
            .map_or(start.end, |token| token.span().end);
        Property {
            name,
            kind,
            expression,
            span: Span::new(start.start, end),
        }
    }

    fn parse_observations(&mut self) -> Vec<String> {
        self.bump();
        let tokens = self.take_until_declaration();
        tokens
            .iter()
            .filter(|token| token.token.kind == TokenKind::Identifier)
            .map(|token| token.text.clone())
            .collect()
    }

    fn parse_expression_tokens(&mut self, tokens: &[SigToken], context: &str) -> Expr {
        let mut parser = ExpressionParser::new(tokens);
        match parser.parse() {
            Ok(expression) => expression,
            Err(error) => {
                self.errors
                    .push(format!("NMLT2005: invalid {context}: {error}"));
                Expr::Value(Value::Bool(false))
            }
        }
    }

    fn take_until_declaration(&mut self) -> Vec<SigToken> {
        self.take_until(|token, depth| {
            depth == 0
                && (token.text == "}"
                    || (token.newline_before && is_declaration_keyword(&token.text)))
        })
    }

    fn take_until_statement(&mut self) -> Vec<SigToken> {
        let tokens = self.take_until(|token, depth| {
            depth == 0
                && (token.text == "}"
                    || token.text == ";"
                    || (token.newline_before && is_statement_keyword(&token.text)))
        });
        self.consume_optional(";");
        tokens
    }

    fn take_until<F>(&mut self, stop: F) -> Vec<SigToken>
    where
        F: Fn(&SigToken, usize) -> bool,
    {
        let start = self.index;
        let mut depth = 0usize;
        while let Some(token) = self.tokens.get(self.index) {
            if stop(token, depth) {
                break;
            }
            match token.text.as_str() {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" if depth > 0 => depth -= 1,
                _ => {}
            }
            self.index += 1;
        }
        self.tokens[start..self.index].to_vec()
    }

    fn skip_top_level_declaration(&mut self) {
        if self.at("enum") || self.at("data") || self.at("module") {
            self.bump();
            while !self.at_end() && !self.at("{") && !self.peek_new_declaration() {
                self.bump();
            }
            if self.at("{") {
                self.skip_balanced("{", "}");
            }
        } else {
            self.bump();
        }
    }

    fn skip_declaration(&mut self) {
        self.bump();
        while !self.at_end() && !self.at("}") && !self.peek_new_declaration() {
            if self.at("{") {
                self.skip_balanced("{", "}");
                break;
            }
            self.bump();
        }
    }

    fn skip_statement(&mut self) {
        self.bump();
        while !self.at_end() && !self.at("}") && !self.peek_new_statement() {
            self.bump();
        }
        self.consume_optional(";");
    }

    fn skip_type_arguments(&mut self) {
        let mut depth = 0usize;
        while let Some(text) = self.current_text() {
            match text {
                "<" => depth += 1,
                ">" => {
                    depth -= 1;
                    self.bump();
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                _ => {}
            }
            self.bump();
        }
    }

    fn skip_balanced(&mut self, open: &str, close: &str) {
        if !self.at(open) {
            return;
        }
        let mut depth = 0usize;
        while let Some(text) = self.current_text() {
            if text == open {
                depth += 1;
            } else if text == close {
                depth -= 1;
            }
            self.bump();
            if depth == 0 {
                break;
            }
        }
    }

    fn peek_new_declaration(&self) -> bool {
        self.tokens
            .get(self.index)
            .is_some_and(|token| token.newline_before && is_declaration_keyword(&token.text))
    }

    fn peek_new_statement(&self) -> bool {
        self.tokens
            .get(self.index)
            .is_some_and(|token| token.newline_before && is_statement_keyword(&token.text))
    }

    fn take_identifier(&mut self, expected: &str) -> String {
        match self.tokens.get(self.index) {
            Some(token) if token.token.kind == TokenKind::Identifier => {
                let text = token.text.clone();
                self.index += 1;
                text
            }
            Some(token) => {
                self.errors.push(format!(
                    "NMLT2006 at {}: expected {expected}, found `{}`",
                    token.span().start,
                    token.text
                ));
                self.index += 1;
                "<error>".to_owned()
            }
            None => {
                self.errors
                    .push(format!("NMLT2006: expected {expected} at end of file"));
                "<error>".to_owned()
            }
        }
    }

    fn expect(&mut self, expected: &str) {
        if self.at(expected) {
            self.bump();
        } else {
            self.errors.push(format!(
                "NMLT2007 at {}: expected `{expected}`, found `{}`",
                self.current_span().map_or(0, |span| span.start),
                self.current_text().unwrap_or("end of file")
            ));
        }
    }

    fn consume_optional(&mut self, text: &str) {
        if self.at(text) {
            self.bump();
        }
    }

    fn at(&self, text: &str) -> bool {
        self.current_text() == Some(text)
    }

    fn current_text(&self) -> Option<&str> {
        self.tokens.get(self.index).map(|token| token.text.as_str())
    }

    fn current_span(&self) -> Option<Span> {
        self.tokens.get(self.index).map(SigToken::span)
    }

    fn bump(&mut self) -> Option<SigToken> {
        let token = self.tokens.get(self.index).cloned();
        self.index += usize::from(token.is_some());
        token
    }

    fn at_end(&self) -> bool {
        self.index >= self.tokens.len()
    }
}

fn is_declaration_keyword(text: &str) -> bool {
    matches!(
        text,
        "state"
            | "capability"
            | "action"
            | "safety"
            | "temporal"
            | "observe"
            | "port"
            | "input"
            | "const"
            | "resource"
            | "hide"
            | "refine"
    )
}

fn is_statement_keyword(text: &str) -> bool {
    matches!(text, "require" | "set" | "consume" | "emit")
}

struct ExpressionParser<'a> {
    tokens: &'a [SigToken],
    index: usize,
}

impl<'a> ExpressionParser<'a> {
    fn new(tokens: &'a [SigToken]) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse(&mut self) -> Result<Expr, String> {
        if self.tokens.is_empty() {
            return Err("empty expression".to_owned());
        }
        let expression = self.parse_precedence(0)?;
        if let Some(token) = self.tokens.get(self.index) {
            Err(format!("unexpected token `{}`", token.text))
        } else {
            Ok(expression)
        }
    }

    fn parse_precedence(&mut self, minimum: u8) -> Result<Expr, String> {
        let mut left = self.parse_prefix()?;
        while let Some((op, precedence, right_associative)) = self.current_binary() {
            if precedence < minimum {
                break;
            }
            self.index += 1;
            let next_minimum = if right_associative {
                precedence
            } else {
                precedence + 1
            };
            let right = self.parse_precedence(next_minimum)?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, String> {
        let Some(token) = self.tokens.get(self.index) else {
            return Err("expected expression".to_owned());
        };
        if token.text == "not" || token.text == "-" {
            let op = if token.text == "not" {
                UnaryOp::Not
            } else {
                UnaryOp::Negate
            };
            self.index += 1;
            return Ok(Expr::Unary {
                op,
                operand: Box::new(self.parse_precedence(7)?),
            });
        }
        if token.text == "(" {
            self.index += 1;
            let expression = self.parse_precedence(0)?;
            self.expect(")")?;
            return Ok(expression);
        }
        self.index += 1;
        let mut expression = match token.text.as_str() {
            "true" => Expr::Value(Value::Bool(true)),
            "false" => Expr::Value(Value::Bool(false)),
            text if token.token.kind == TokenKind::Integer => {
                let number = text.replace('_', "").parse::<i64>().map_err(|error| {
                    format!("integer `{text}` is outside the executable range: {error}")
                })?;
                Expr::Value(Value::Int(number))
            }
            name if token.token.kind == TokenKind::Identifier => Expr::Name(name.to_owned()),
            _ => return Err(format!("unexpected token `{}`", token.text)),
        };
        if self.current_text() == Some("(") {
            let Expr::Name(name) = expression else {
                return Err("only named calls are supported".to_owned());
            };
            self.index += 1;
            let mut arguments = Vec::new();
            while self.current_text() != Some(")") {
                arguments.push(self.parse_precedence(0)?);
                if self.current_text() == Some(",") {
                    self.index += 1;
                } else {
                    break;
                }
            }
            self.expect(")")?;
            expression = Expr::Call { name, arguments };
        }
        Ok(expression)
    }

    fn current_binary(&self) -> Option<(BinaryOp, u8, bool)> {
        match self.current_text()? {
            "implies" => Some((BinaryOp::Implies, 1, true)),
            "or" | "||" => Some((BinaryOp::Or, 2, false)),
            "and" | "&&" => Some((BinaryOp::And, 3, false)),
            "==" => Some((BinaryOp::Equal, 4, false)),
            "!=" => Some((BinaryOp::NotEqual, 4, false)),
            ">" => Some((BinaryOp::Greater, 4, false)),
            ">=" => Some((BinaryOp::GreaterEqual, 4, false)),
            "<" => Some((BinaryOp::Less, 4, false)),
            "<=" => Some((BinaryOp::LessEqual, 4, false)),
            "+" => Some((BinaryOp::Add, 5, false)),
            "-" => Some((BinaryOp::Subtract, 5, false)),
            _ => None,
        }
    }

    fn expect(&mut self, text: &str) -> Result<(), String> {
        if self.current_text() == Some(text) {
            self.index += 1;
            Ok(())
        } else {
            Err(format!("expected `{text}`"))
        }
    }

    fn current_text(&self) -> Option<&str> {
        self.tokens.get(self.index).map(|token| token.text.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::parse_model;

    #[test]
    fn parses_provider_fragment() {
        let source = r#"
            enum Phase { proposed, authorized, dispatched }
            system Provider {
              state phase: Phase = proposed
              state armed: Bool = false
              capability call: Once<Effect>
              action dispatch {
                require phase == authorized and armed
                consume call
                set phase = dispatched
              }
              safety Armed = always(phase == dispatched implies armed)
              observe phase, armed
            }
        "#;
        let model = parse_model(source).unwrap();
        assert_eq!(model.system_name, "Provider");
        assert_eq!(model.states.len(), 2);
        assert_eq!(model.actions.len(), 1);
        assert_eq!(model.properties.len(), 1);
        assert_eq!(model.observations, ["phase", "armed"]);
    }
}

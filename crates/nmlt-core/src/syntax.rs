use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::cst::{GreenElement, GreenNode, GreenToken, SyntaxKind};
use crate::lexer::{Token, TokenKind, lex_source};
use crate::{Diagnostic, Span};

/// Maximum number of recursively nested module wrappers accepted by the
/// lossless parser before bounded recovery replaces the next module with one
/// explicit error node.
pub const MAX_MODULE_NESTING_DEPTH: usize = 256;

/// A syntactically recognized top-level system declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemDecl {
    pub name: String,
    pub span: Span,
}

/// The compatibility result returned by [`parse_source`].
///
/// This remains a syntax-only result. It is not name resolution, type checking,
/// model checking, or proof evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedFile {
    pub systems: Vec<SystemDecl>,
}

/// A lossless, recovery-capable parse of one source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxParse {
    root: GreenNode,
    systems: Vec<SystemDecl>,
    diagnostics: Vec<Diagnostic>,
}

impl SyntaxParse {
    #[must_use]
    pub const fn root(&self) -> &GreenNode {
        &self.root
    }

    #[must_use]
    pub fn systems(&self) -> &[SystemDecl] {
        &self.systems
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    /// Reconstruct the exact input from the immutable token leaves.
    #[must_use]
    pub fn reconstruct(&self) -> String {
        self.root.reconstruct()
    }
}

/// Parse an NMLT file into an immutable, lossless concrete syntax tree.
///
/// Recovery nodes retain malformed input. Every returned diagnostic has a
/// source-valid half-open byte span, including empty-file and end-of-file
/// conditions. A returned tree is a syntactic artifact only; callers must
/// inspect `diagnostics()` and must not interpret parse success as semantic
/// assurance.
#[must_use]
pub fn parse_cst(source: &str) -> SyntaxParse {
    let lexed = lex_source(source);
    let mut parser = Parser::new(source, &lexed.tokens);
    parser.parse_file();

    let mut diagnostics = lexed.diagnostics;
    diagnostics.extend(
        lexed
            .tokens
            .iter()
            .filter(|token| token.kind == TokenKind::Unknown)
            .map(|token| {
                Diagnostic::error(
                    "NMLT1003",
                    format!("unsupported character `{}`", token.text(source)),
                    Some(token.span),
                )
            }),
    );
    diagnostics.extend(validate_delimiters(&lexed.tokens));
    diagnostics.append(&mut parser.diagnostics);
    if source.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "NMLT0001",
            "source file is empty",
            Some(Span::new(0, 0)),
        ));
    }
    normalize_diagnostics(&mut diagnostics);
    debug_assert_diagnostic_spans(source, &diagnostics);

    let root = fold_events(source, &lexed.tokens, parser.events);
    debug_assert_eq!(root.text_len(), source.len());
    debug_assert_eq!(root.reconstruct(), source);

    SyntaxParse {
        root,
        systems: parser.systems,
        diagnostics,
    }
}

/// Parse a complete model while preserving the original structural API.
///
/// Unlike [`parse_cst`], this compatibility wrapper requires at least one
/// system and returns no recovered tree when diagnostics are present.
pub fn parse_source(source: &str) -> Result<ParsedFile, Vec<Diagnostic>> {
    let parsed = parse_cst(source);
    let mut diagnostics = parsed.diagnostics;
    if parsed.systems.is_empty() && !diagnostics.iter().any(|item| item.code == "NMLT0001") {
        diagnostics.push(Diagnostic::error(
            "NMLT0005",
            "no `system` declaration found",
            Some(Span::new(source.len(), source.len())),
        ));
    }
    normalize_diagnostics(&mut diagnostics);
    debug_assert_diagnostic_spans(source, &diagnostics);

    if diagnostics.is_empty() {
        Ok(ParsedFile {
            systems: parsed.systems,
        })
    } else {
        Err(diagnostics)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Event {
    StartNode(SyntaxKind),
    FinishNode,
    Token(usize),
}

struct OpenNode {
    kind: SyntaxKind,
    children: Vec<GreenElement>,
}

fn fold_events(source: &str, tokens: &[Token], events: Vec<Event>) -> GreenNode {
    let mut stack: Vec<OpenNode> = Vec::new();
    let mut root = None;
    let mut next_token = 0;

    for event in events {
        match event {
            Event::StartNode(kind) => stack.push(OpenNode {
                kind,
                children: Vec::new(),
            }),
            Event::Token(index) => {
                debug_assert_eq!(index, next_token, "parser events reordered a token");
                next_token += 1;
                let token = tokens[index];
                let leaf = GreenToken::new(token.kind, token.text(source));
                stack
                    .last_mut()
                    .expect("a token event must be inside the source node")
                    .children
                    .push(GreenElement::Token(leaf));
            }
            Event::FinishNode => {
                let open = stack.pop().expect("finish event must match a start event");
                let node = GreenNode::new(open.kind, open.children);
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(GreenElement::Node(node));
                } else {
                    assert!(
                        root.replace(node).is_none(),
                        "only one root node is allowed"
                    );
                }
            }
        }
    }

    debug_assert_eq!(
        next_token,
        tokens.len(),
        "every lexical token must be emitted"
    );
    debug_assert!(stack.is_empty(), "all parser nodes must be closed");
    root.expect("the parser always emits a source node")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpressionEnd {
    Line,
    SystemMember,
    Statement,
}

struct Parser<'source, 'tokens> {
    source: &'source str,
    tokens: &'tokens [Token],
    cursor: usize,
    events: Vec<Event>,
    diagnostics: Vec<Diagnostic>,
    systems: Vec<SystemDecl>,
    system_name_scopes: Vec<BTreeSet<String>>,
    module_depth: usize,
    last_end: usize,
}

impl<'source, 'tokens> Parser<'source, 'tokens> {
    fn new(source: &'source str, tokens: &'tokens [Token]) -> Self {
        Self {
            source,
            tokens,
            cursor: 0,
            events: Vec::new(),
            diagnostics: Vec::new(),
            systems: Vec::new(),
            system_name_scopes: vec![BTreeSet::new()],
            module_depth: 0,
            last_end: 0,
        }
    }

    fn parse_file(&mut self) {
        self.start(SyntaxKind::SourceFile);
        while !self.at_end() {
            if self.at_trivia() {
                self.bump();
            } else if !self.parse_top_level_item() {
                self.error_and_recover_line("NMLT2001", "expected a top-level declaration");
            }
        }
        self.finish();
    }

    fn parse_top_level_item(&mut self) -> bool {
        if self.at_keyword("module") {
            self.parse_module_decl();
        } else if self.at_keyword("import") {
            self.parse_import_decl();
        } else if self.at_keyword("data") {
            self.parse_line_decl(SyntaxKind::DataDecl);
        } else if self.at_keyword("type") {
            self.parse_line_decl(SyntaxKind::TypeDecl);
        } else if self.at_keyword("record") {
            self.parse_header_or_block_decl(SyntaxKind::RecordDecl);
        } else if self.at_keyword("fn") {
            self.parse_header_or_block_decl(SyntaxKind::FunctionDecl);
        } else if self.at_keyword("enum") {
            self.parse_enum_decl();
        } else if self.at_keyword("system") {
            self.parse_system_decl();
        } else {
            return false;
        }
        true
    }

    fn parse_module_decl(&mut self) {
        if self.module_depth >= MAX_MODULE_NESTING_DEPTH {
            self.error_at_current(
                "NMLT2014",
                format!("module nesting exceeds the maximum depth of {MAX_MODULE_NESTING_DEPTH}"),
            );
            self.consume_overdepth_module_as_error();
            return;
        }

        self.module_depth += 1;
        self.parse_module_decl_within_limit();
        self.module_depth -= 1;
    }

    fn parse_module_decl_within_limit(&mut self) {
        self.start(SyntaxKind::ModuleDecl);
        self.bump();
        self.expect_identifier("NMLT2002", "expected a module name");
        self.bump_trivia();

        if !self.eat_kind(TokenKind::LeftBrace) {
            self.error_at_current("NMLT2003", "expected `{` to start module body");
            self.parse_line_tail();
            self.finish();
            return;
        }

        self.system_name_scopes.push(BTreeSet::new());
        while !self.at_end() && !self.at_significant_kind(TokenKind::RightBrace) {
            if self.at_trivia() {
                self.bump();
            } else if !self.parse_top_level_item() {
                self.error_and_recover_line("NMLT2001", "expected a declaration in module body");
            }
        }
        if self.at_significant_kind(TokenKind::RightBrace) {
            self.bump_trivia();
            self.bump();
        }
        self.system_name_scopes
            .pop()
            .expect("module parsing always has a nested system-name scope");
        self.finish();
    }

    fn consume_overdepth_module_as_error(&mut self) {
        self.start(SyntaxKind::Error);
        self.bump();
        self.expect_identifier("NMLT2002", "expected a module name");
        self.bump_trivia();
        if self.at_kind(TokenKind::LeftBrace) {
            self.consume_balanced_braces();
        } else {
            self.parse_line_tail_before_close();
        }
        self.finish();
    }

    fn parse_import_decl(&mut self) {
        self.start(SyntaxKind::ImportDecl);
        self.bump();
        let target = self.expect_identifier("NMLT2002", "expected an imported module name");
        self.bump_inline_trivia();
        if target.is_some()
            && !self.at_end()
            && !self.is_line_break()
            && !self.at_kind(TokenKind::RightBrace)
        {
            self.error_at_current("NMLT2012", "unexpected tokens after imported module name");
        }
        self.parse_line_tail_before_close();
        self.finish();
    }

    fn parse_line_decl(&mut self, kind: SyntaxKind) {
        self.start(kind);
        self.bump();
        self.parse_line_tail();
        self.finish();
    }

    fn parse_header_or_block_decl(&mut self, kind: SyntaxKind) {
        self.start(kind);
        self.bump();
        let mut delimiters = Vec::new();
        while !self.at_end() {
            if self.is_line_break() && delimiters.is_empty() {
                self.bump();
                break;
            }
            let token_kind = self.current_kind();
            self.bump();
            update_delimiter_stack(&mut delimiters, token_kind);
            if token_kind == TokenKind::RightBrace && delimiters.is_empty() {
                break;
            }
        }
        self.finish();
    }

    fn parse_enum_decl(&mut self) {
        self.start(SyntaxKind::EnumDecl);
        self.bump();
        self.expect_identifier("NMLT2002", "expected an enum name");
        self.bump_trivia();
        if !self.eat_kind(TokenKind::LeftBrace) {
            self.error_at_current("NMLT2003", "expected `{` to start enum body");
            self.finish();
            return;
        }

        while !self.at_end() && !self.at_significant_kind(TokenKind::RightBrace) {
            self.bump_trivia();
            if self.at_end() || self.at_kind(TokenKind::RightBrace) {
                break;
            }
            if self.at_kind(TokenKind::Identifier) {
                self.start(SyntaxKind::EnumVariant);
                self.bump();
                self.bump_trivia();
                if self.at_text(",") {
                    self.bump();
                }
                self.finish();
            } else {
                self.error_and_recover_enum();
            }
        }
        if self.at_significant_kind(TokenKind::RightBrace) {
            self.bump_trivia();
            self.bump();
        }
        self.finish();
    }

    fn parse_system_decl(&mut self) {
        let start = self.current_span().start;
        self.start(SyntaxKind::SystemDecl);
        self.bump();
        let name = self.expect_identifier("NMLT0003", "expected a system name after `system`");
        self.bump_trivia();
        if self.at_kind(TokenKind::LeftParen) {
            self.parse_parameter_list();
            self.bump_trivia();
        }

        if !self.at_kind(TokenKind::LeftBrace) {
            let display_name = name.as_ref().map_or("<missing>", |(name, _)| name.as_str());
            self.error_at_current("NMLT0004", format!("system `{display_name}` has no body"));
            self.finish();
            return;
        }

        self.start(SyntaxKind::SystemBody);
        self.bump();
        self.parse_system_body();
        self.finish();
        let end = self.last_end;
        self.finish();

        if let Some((name, name_span)) = name {
            let current_scope = self
                .system_name_scopes
                .last_mut()
                .expect("the source file always has a system-name scope");
            if current_scope.insert(name.clone()) {
                self.systems.push(SystemDecl {
                    name,
                    span: Span::new(start, end),
                });
            } else {
                self.diagnostics.push(Diagnostic::error(
                    "NMLT0006",
                    format!("duplicate system declaration `{name}`"),
                    Some(name_span),
                ));
            }
        }
    }

    fn parse_system_body(&mut self) {
        while !self.at_end() && !self.at_significant_kind(TokenKind::RightBrace) {
            if self.at_trivia() {
                self.bump();
                continue;
            }

            if self.at_keyword("const") {
                self.parse_binding_decl(SyntaxKind::ConstDecl, false, false);
            } else if self.at_keyword("input") {
                self.parse_binding_decl(SyntaxKind::InputDecl, false, false);
            } else if self.at_keyword("state") {
                self.parse_binding_decl(SyntaxKind::StateDecl, true, false);
            } else if self.at_keyword("capability") {
                self.parse_binding_decl(SyntaxKind::CapabilityDecl, false, true);
            } else if self.at_keyword("port") {
                self.parse_port_decl();
            } else if self.at_keyword("action") {
                self.parse_action_decl();
            } else if self.at_keyword("safety") {
                self.parse_property_decl(SyntaxKind::SafetyDecl);
            } else if self.at_keyword("temporal") {
                self.parse_property_decl(SyntaxKind::TemporalDecl);
            } else if self.at_keyword("resource") {
                self.parse_property_decl(SyntaxKind::ResourceDecl);
            } else if self.at_keyword("observe") {
                self.parse_observation_decl(SyntaxKind::ObserveDecl);
            } else if self.at_keyword("hide") {
                self.parse_observation_decl(SyntaxKind::HideDecl);
            } else {
                self.error_and_recover_line("NMLT2005", "expected a system member declaration");
            }
        }
        if self.at_significant_kind(TokenKind::RightBrace) {
            self.bump_trivia();
            self.bump();
        }
    }

    fn parse_binding_decl(&mut self, kind: SyntaxKind, require_value: bool, multiline_type: bool) {
        self.start(kind);
        self.bump();
        self.expect_identifier("NMLT2002", "expected a declaration name");
        self.bump_inline_trivia();
        if !self.eat_text(":") {
            self.error_at_current("NMLT2006", "expected `:` before the declared type");
        }

        self.start(SyntaxKind::TypeExpr);
        let has_type = if multiline_type {
            self.consume_expression_tokens(ExpressionEnd::SystemMember, true)
        } else {
            self.consume_type_until_value_or_line()
        };
        self.finish();
        if !has_type {
            self.error_at_current("NMLT2006", "expected a type expression");
        }

        self.bump_inline_trivia();
        if self.eat_text("=") {
            self.parse_expression(ExpressionEnd::Line);
        } else if require_value {
            self.error_at_current("NMLT2007", "expected `=` in state declaration");
        }
        self.finish();
    }

    fn parse_port_decl(&mut self) {
        self.start(SyntaxKind::PortDecl);
        self.bump();
        self.expect_identifier("NMLT2002", "expected `input` or `output` after `port`");
        self.expect_identifier("NMLT2002", "expected a port name");
        self.bump_inline_trivia();
        if !self.eat_text(":") {
            self.error_at_current("NMLT2006", "expected `:` before the port type");
        }
        self.start(SyntaxKind::TypeExpr);
        let has_type = self.consume_type_until_value_or_line();
        self.finish();
        if !has_type {
            self.error_at_current("NMLT2006", "expected a port type");
        }
        self.finish();
    }

    fn parse_action_decl(&mut self) {
        self.start(SyntaxKind::ActionDecl);
        self.bump();
        self.expect_identifier("NMLT2002", "expected an action name");
        self.bump_trivia();
        if self.at_kind(TokenKind::LeftParen) {
            self.parse_parameter_list();
            self.bump_trivia();
        }
        if self.at_keyword("grade") {
            self.parse_grade_clause();
            self.bump_trivia();
        }
        if !self.at_kind(TokenKind::LeftBrace) {
            self.error_at_current("NMLT2008", "expected `{` to start action body");
            self.finish();
            return;
        }

        self.start(SyntaxKind::ActionBody);
        self.bump();
        while !self.at_end() && !self.at_significant_kind(TokenKind::RightBrace) {
            if self.at_trivia() {
                self.bump();
            } else if self.at_keyword("require") {
                self.parse_action_statement(SyntaxKind::RequireStmt);
            } else if self.at_keyword("set") {
                self.parse_update_statement();
            } else if self.at_keyword("emit") {
                self.parse_action_statement(SyntaxKind::EmitStmt);
            } else if self.at_keyword("consume") {
                self.parse_action_statement(SyntaxKind::ConsumeStmt);
            } else {
                self.error_and_recover_statement();
            }
        }
        if self.at_significant_kind(TokenKind::RightBrace) {
            self.bump_trivia();
            self.bump();
        }
        self.finish();
        self.finish();
    }

    fn parse_parameter_list(&mut self) {
        self.start(SyntaxKind::ParameterList);
        self.bump();
        while !self.at_end() && !self.at_significant_kind(TokenKind::RightParen) {
            self.bump_trivia();
            if self.at_end() || self.at_kind(TokenKind::RightParen) {
                break;
            }
            self.start(SyntaxKind::Parameter);
            self.expect_identifier("NMLT2002", "expected a parameter name");
            self.bump_inline_trivia();
            if !self.eat_text(":") {
                self.error_at_current("NMLT2006", "expected `:` before the parameter type");
            }
            self.start(SyntaxKind::TypeExpr);
            let has_type = self.consume_parameter_type();
            self.finish();
            if !has_type {
                self.error_at_current("NMLT2006", "expected a parameter type");
            }
            self.bump_inline_trivia();
            if self.at_text(",") {
                self.bump();
            }
            self.finish();
        }
        if self.at_significant_kind(TokenKind::RightParen) {
            self.bump_trivia();
            self.bump();
        }
        self.finish();
    }

    fn parse_grade_clause(&mut self) {
        self.start(SyntaxKind::GradeClause);
        self.bump();
        self.bump_trivia();
        if !self.at_kind(TokenKind::LeftBrace) {
            self.error_at_current("NMLT2003", "expected `{` after `grade`");
            self.finish();
            return;
        }
        self.consume_balanced_braces();
        self.finish();
    }

    fn parse_action_statement(&mut self, kind: SyntaxKind) {
        self.start(kind);
        let keyword_end = self.current_span().end;
        self.bump();
        let has_expression = self.parse_expression(ExpressionEnd::Statement);
        if !has_expression {
            self.diagnostics.push(Diagnostic::error(
                "NMLT2011",
                "expected an expression after action statement keyword",
                Some(Span::new(keyword_end, keyword_end)),
            ));
        }
        self.finish();
    }

    fn parse_update_statement(&mut self) {
        self.start(SyntaxKind::UpdateStmt);
        self.bump();
        self.start(SyntaxKind::Expr);
        let has_target = self.consume_update_target();
        self.finish();
        if !has_target {
            self.error_at_current("NMLT2010", "expected an update target after `set`");
        }
        self.bump_inline_trivia();
        if !self.eat_text("=") {
            self.error_at_current("NMLT2010", "expected `=` in update statement");
        } else if !self.parse_expression(ExpressionEnd::Statement) {
            self.error_at_current("NMLT2011", "expected an update value after `=`");
        }
        self.finish();
    }

    fn parse_property_decl(&mut self, kind: SyntaxKind) {
        self.start(kind);
        self.bump();
        self.expect_identifier("NMLT2002", "expected a property name");
        self.bump_inline_trivia();
        if !self.eat_text("=") {
            self.error_at_current("NMLT2007", "expected `=` before the property expression");
        }
        if !self.parse_expression(ExpressionEnd::SystemMember) {
            self.error_at_current("NMLT2011", "expected a property expression");
        }
        self.finish();
    }

    fn parse_observation_decl(&mut self, kind: SyntaxKind) {
        self.start(kind);
        self.bump();
        if !self.parse_expression(ExpressionEnd::Line) {
            self.error_at_current("NMLT2011", "expected at least one observed name");
        }
        self.finish();
    }

    fn parse_expression(&mut self, end: ExpressionEnd) -> bool {
        self.start(SyntaxKind::Expr);
        let has_expression = self.consume_expression_tokens(end, false);
        self.finish();
        if end == ExpressionEnd::Statement && self.at_statement_terminator() {
            self.bump();
        }
        has_expression
    }

    fn consume_expression_tokens(&mut self, end: ExpressionEnd, stop_before_equals: bool) -> bool {
        let mut delimiters = Vec::new();
        let mut has_significant = false;

        while !self.at_end() {
            if end == ExpressionEnd::Statement && self.at_semicolon_punctuation() {
                if delimiters.is_empty() && self.at_statement_terminator() {
                    break;
                }

                self.error_at_current(
                    "NMLT2013",
                    "`;` must be a standalone action statement terminator",
                );
                self.bump();
                if delimiters.is_empty() {
                    break;
                }
                continue;
            }

            if delimiters.is_empty() {
                if self.at_kind(TokenKind::RightBrace) {
                    break;
                }
                if stop_before_equals && self.at_text("=") {
                    break;
                }
                if self.is_line_break() {
                    self.bump();
                    if end != ExpressionEnd::SystemMember || self.next_is_expression_continuation()
                    {
                        if end == ExpressionEnd::Line || end == ExpressionEnd::Statement {
                            break;
                        }
                        continue;
                    }
                    break;
                }
            }

            let kind = self.current_kind();
            if !kind.is_trivia() {
                has_significant = true;
            }
            self.bump();
            update_delimiter_stack(&mut delimiters, kind);
        }
        has_significant
    }

    fn consume_type_until_value_or_line(&mut self) -> bool {
        let mut has_type = false;
        let mut delimiters = Vec::new();
        while !self.at_end() {
            if delimiters.is_empty()
                && (self.at_text("=")
                    || self.is_line_break()
                    || self.at_kind(TokenKind::RightBrace))
            {
                break;
            }
            let kind = self.current_kind();
            if !kind.is_trivia() {
                has_type = true;
            }
            self.bump();
            update_delimiter_stack(&mut delimiters, kind);
        }
        has_type
    }

    fn consume_parameter_type(&mut self) -> bool {
        let mut has_type = false;
        let mut delimiters = Vec::new();
        while !self.at_end() {
            if delimiters.is_empty()
                && (self.at_text(",")
                    || self.at_kind(TokenKind::RightParen)
                    || self.is_line_break())
            {
                break;
            }
            let kind = self.current_kind();
            if !kind.is_trivia() {
                has_type = true;
            }
            self.bump();
            update_delimiter_stack(&mut delimiters, kind);
        }
        has_type
    }

    fn consume_update_target(&mut self) -> bool {
        let mut has_target = false;
        let mut delimiters = Vec::new();
        while !self.at_end() {
            if delimiters.is_empty()
                && (self.at_text("=")
                    || self.at_statement_terminator()
                    || self.is_line_break()
                    || self.at_kind(TokenKind::RightBrace))
            {
                break;
            }
            let kind = self.current_kind();
            if !kind.is_trivia() {
                has_target = true;
            }
            self.bump();
            update_delimiter_stack(&mut delimiters, kind);
        }
        has_target
    }

    fn consume_balanced_braces(&mut self) {
        let mut depth = 0_usize;
        while !self.at_end() {
            let kind = self.current_kind();
            self.bump();
            match kind {
                TokenKind::LeftBrace => depth += 1,
                TokenKind::RightBrace => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    fn parse_line_tail(&mut self) {
        while !self.at_end() {
            let is_end = self.is_line_break();
            self.bump();
            if is_end {
                break;
            }
        }
    }

    fn parse_line_tail_before_close(&mut self) {
        while !self.at_end() && !self.at_kind(TokenKind::RightBrace) {
            let is_end = self.is_line_break();
            self.bump();
            if is_end {
                break;
            }
        }
    }

    fn error_and_recover_line(&mut self, code: &'static str, message: &'static str) {
        self.diagnostics
            .push(Diagnostic::error(code, message, Some(self.current_span())));
        self.start(SyntaxKind::Error);
        let mut consumed = false;
        while !self.at_end() {
            let is_end = self.is_line_break();
            let is_system_close = self.at_kind(TokenKind::RightBrace);
            if is_system_close {
                // System/module loops stop before their own close. At the file
                // root an unmatched close must still make recovery progress.
                if !consumed {
                    self.bump();
                }
                break;
            }
            self.bump();
            consumed = true;
            if is_end {
                break;
            }
        }
        self.finish();
    }

    fn error_and_recover_enum(&mut self) {
        self.diagnostics.push(Diagnostic::error(
            "NMLT2004",
            "expected an enum variant",
            Some(self.current_span()),
        ));
        self.start(SyntaxKind::Error);
        while !self.at_end()
            && !self.at_kind(TokenKind::RightBrace)
            && !self.at_text(",")
            && !self.is_line_break()
        {
            self.bump();
        }
        if self.at_text(",") || self.is_line_break() {
            self.bump();
        }
        self.finish();
    }

    fn error_and_recover_statement(&mut self) {
        self.diagnostics.push(Diagnostic::error(
            "NMLT2009",
            "expected `require`, `set`, `emit`, or `consume` in action body",
            Some(self.current_span()),
        ));
        self.start(SyntaxKind::Error);
        while !self.at_end()
            && !self.at_kind(TokenKind::RightBrace)
            && !self.at_statement_terminator()
            && !self.is_line_break()
        {
            self.bump();
        }
        if self.at_statement_terminator() || self.is_line_break() {
            self.bump();
        }
        self.finish();
    }

    fn next_is_expression_continuation(&self) -> bool {
        let Some(index) = self.next_significant_index() else {
            return false;
        };
        let token = self.tokens[index];
        token.kind != TokenKind::RightBrace
            && !(token.kind == TokenKind::Identifier
                && is_system_member_keyword(token.text(self.source)))
    }

    fn expect_identifier(
        &mut self,
        code: &'static str,
        message: &'static str,
    ) -> Option<(String, Span)> {
        self.bump_inline_trivia();
        if self.at_kind(TokenKind::Identifier) {
            let token = self.tokens[self.cursor];
            let name = token.text(self.source).to_owned();
            self.bump();
            Some((name, token.span))
        } else {
            self.error_at_current(code, message);
            None
        }
    }

    fn error_at_current(&mut self, code: &'static str, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(
            code,
            message,
            Some(self.current_or_eof_span()),
        ));
    }

    fn start(&mut self, kind: SyntaxKind) {
        self.events.push(Event::StartNode(kind));
    }

    fn finish(&mut self) {
        self.events.push(Event::FinishNode);
    }

    fn bump(&mut self) {
        debug_assert!(!self.at_end());
        self.last_end = self.tokens[self.cursor].span.end;
        self.events.push(Event::Token(self.cursor));
        self.cursor += 1;
    }

    fn bump_trivia(&mut self) {
        while self.at_trivia() {
            self.bump();
        }
    }

    fn bump_inline_trivia(&mut self) {
        while self.at_trivia() && !self.is_line_break() {
            self.bump();
        }
    }

    fn eat_kind(&mut self, kind: TokenKind) -> bool {
        if self.at_kind(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_text(&mut self, text: &str) -> bool {
        if self.at_text(text) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn at_trivia(&self) -> bool {
        !self.at_end() && self.current_kind().is_trivia()
    }

    fn at_kind(&self, kind: TokenKind) -> bool {
        !self.at_end() && self.current_kind() == kind
    }

    fn at_significant_kind(&self, kind: TokenKind) -> bool {
        self.next_significant_index()
            .is_some_and(|index| self.tokens[index].kind == kind)
    }

    fn at_keyword(&self, keyword: &str) -> bool {
        !self.at_end()
            && self.current_kind() == TokenKind::Identifier
            && self.current_text() == keyword
    }

    fn at_text(&self, text: &str) -> bool {
        !self.at_end() && self.current_text() == text
    }

    fn at_statement_terminator(&self) -> bool {
        !self.at_end()
            && self.current_kind() == TokenKind::Punctuation
            && self.current_text() == ";"
    }

    fn at_semicolon_punctuation(&self) -> bool {
        !self.at_end()
            && self.current_kind() == TokenKind::Punctuation
            && self.current_text().contains(';')
    }

    fn current_kind(&self) -> TokenKind {
        self.tokens[self.cursor].kind
    }

    fn current_text(&self) -> &str {
        self.tokens[self.cursor].text(self.source)
    }

    fn current_span(&self) -> Span {
        self.tokens[self.cursor].span
    }

    fn current_or_eof_span(&self) -> Span {
        if self.at_end() {
            Span::new(self.source.len(), self.source.len())
        } else {
            self.current_span()
        }
    }

    fn is_line_break(&self) -> bool {
        !self.at_end()
            && self.current_kind() == TokenKind::Whitespace
            && self.current_text().contains(['\r', '\n'])
    }

    fn next_significant_index(&self) -> Option<usize> {
        (self.cursor..self.tokens.len()).find(|index| !self.tokens[*index].kind.is_trivia())
    }
}

fn is_system_member_keyword(text: &str) -> bool {
    matches!(
        text,
        "const"
            | "input"
            | "state"
            | "capability"
            | "port"
            | "action"
            | "safety"
            | "temporal"
            | "resource"
            | "observe"
            | "hide"
    )
}

fn update_delimiter_stack(stack: &mut Vec<TokenKind>, kind: TokenKind) {
    if is_open(kind) {
        stack.push(kind);
    } else if is_close(kind)
        && stack
            .last()
            .is_some_and(|open| matching_close(*open) == kind)
    {
        stack.pop();
    }
}

fn validate_delimiters(tokens: &[Token]) -> Vec<Diagnostic> {
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
                format!(
                    "unmatched closing delimiter `{}`",
                    delimiter_text(token.kind)
                ),
                Some(token.span),
            ));
            continue;
        };
        if matching_close(open) != token.kind {
            diagnostics.push(Diagnostic::error(
                "NMLT0002",
                format!(
                    "mismatched closing delimiter: expected `{}`, found `{}`",
                    delimiter_text(matching_close(open)),
                    delimiter_text(token.kind)
                ),
                Some(token.span),
            ));
        }
    }

    for (kind, span) in stack {
        diagnostics.push(Diagnostic::error(
            "NMLT0002",
            format!("unclosed opening delimiter `{}`", delimiter_text(kind)),
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
        _ => unreachable!("only opening delimiters have a matching close"),
    }
}

fn delimiter_text(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::LeftBrace => "{",
        TokenKind::RightBrace => "}",
        TokenKind::LeftParen => "(",
        TokenKind::RightParen => ")",
        TokenKind::LeftBracket => "[",
        TokenKind::RightBracket => "]",
        _ => unreachable!("only delimiter tokens have delimiter text"),
    }
}

fn normalize_diagnostics(diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.sort_by(compare_diagnostics);
    diagnostics.dedup();
}

fn debug_assert_diagnostic_spans(source: &str, diagnostics: &[Diagnostic]) {
    debug_assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.span.is_some_and(|span| {
            span.start <= span.end
                && span.end <= source.len()
                && source.is_char_boundary(span.start)
                && source.is_char_boundary(span.end)
        })
    }));
}

fn compare_diagnostics(left: &Diagnostic, right: &Diagnostic) -> Ordering {
    let left_position = left.span.map_or(usize::MAX, |span| span.start);
    let right_position = right.span.map_or(usize::MAX, |span| span.start);
    left_position
        .cmp(&right_position)
        .then_with(|| left.code.cmp(right.code))
        .then_with(|| left.message.cmp(&right.message))
}

#[cfg(test)]
mod tests {
    use crate::{Span, SyntaxKind};

    use super::{MAX_MODULE_NESTING_DEPTH, parse_cst, parse_source};

    fn nested_modules(depth: usize) -> String {
        let openings = (0..depth)
            .map(|index| format!("module M{index} {{"))
            .collect::<String>();
        format!("{openings}system S {{}}{}", "}".repeat(depth))
    }

    #[test]
    fn parses_a_system_and_nested_blocks() {
        let parsed = parse_source("system Clock { action Tick { set bit = true } }").unwrap();
        assert_eq!(parsed.systems.len(), 1);
        assert_eq!(parsed.systems[0].name, "Clock");
    }

    #[test]
    fn ignores_system_text_in_comments_and_strings() {
        let parsed =
            parse_source("// system Fake {}\ndata Note = \"system AlsoFake {}\"\nsystem Real {}")
                .unwrap();
        assert_eq!(parsed.systems.len(), 1);
        assert_eq!(parsed.systems[0].name, "Real");
    }

    #[test]
    fn creates_structural_nodes_without_losing_trivia() {
        let source = "enum Bit { off, on }\n\nsystem Clock {\n  state bit: Bit = off\n  action tick { set bit = on }\n  safety Safe = always(bit == off or bit == on)\n}\n";
        let parsed = parse_cst(source);
        assert!(
            parsed.diagnostics().is_empty(),
            "{:?}",
            parsed.diagnostics()
        );
        assert_eq!(parsed.reconstruct(), source);
        assert_eq!(parsed.root().text_len(), source.len());
        assert_eq!(parsed.root().descendants(SyntaxKind::EnumDecl).len(), 1);
        assert_eq!(parsed.root().descendants(SyntaxKind::StateDecl).len(), 1);
        assert_eq!(parsed.root().descendants(SyntaxKind::ActionDecl).len(), 1);
        assert_eq!(parsed.root().descendants(SyntaxKind::SafetyDecl).len(), 1);
    }

    #[test]
    fn recognizes_phase_one_mathematical_declaration_shells() {
        let source = concat!(
            "module Math {\n",
            "  import Base\n",
            "  data Maybe = none | some(Nat)\n",
            "  type Count = Nat\n",
            "  record Pair { left: Nat, right: Nat }\n",
            "  fn add(left: Nat, right: Nat) = left + right\n",
            "  system UsesMath {}\n",
            "}\n",
        );
        let parsed = parse_cst(source);
        assert!(parsed.diagnostics().is_empty());
        assert_eq!(parsed.reconstruct(), source);
        for kind in [
            SyntaxKind::ModuleDecl,
            SyntaxKind::ImportDecl,
            SyntaxKind::DataDecl,
            SyntaxKind::TypeDecl,
            SyntaxKind::RecordDecl,
            SyntaxKind::FunctionDecl,
        ] {
            assert_eq!(parsed.root().descendants(kind).len(), 1, "{kind:?}");
        }
    }

    #[test]
    fn recovers_without_accepting_the_recovered_parse() {
        let source = "system Broken {\n  state x: Nat\n  action go { mystery x; set x = 1 }\n  state y: Nat = 0\n}\n";
        let parsed = parse_cst(source);
        assert_eq!(parsed.reconstruct(), source);
        assert_eq!(parsed.root().descendants(SyntaxKind::StateDecl).len(), 2);
        assert_eq!(parsed.root().descendants(SyntaxKind::Error).len(), 1);
        assert_eq!(
            parsed
                .diagnostics()
                .iter()
                .map(|item| item.code)
                .collect::<Vec<_>>(),
            ["NMLT2007", "NMLT2009"]
        );
        assert!(parse_source(source).is_err());
    }

    #[test]
    fn rejects_duplicate_systems() {
        let diagnostics = parse_source("system Same {} system Same {}").unwrap_err();
        assert!(diagnostics.iter().any(|item| item.code == "NMLT0006"));
    }

    #[test]
    fn module_nesting_is_bounded_before_recursive_descent() {
        let at_limit = nested_modules(MAX_MODULE_NESTING_DEPTH);
        let parsed = parse_cst(&at_limit);
        assert!(
            parsed.diagnostics().is_empty(),
            "{:?}",
            parsed.diagnostics()
        );
        assert_eq!(parsed.reconstruct(), at_limit);
        assert_eq!(
            parsed.root().descendants(SyntaxKind::ModuleDecl).len(),
            MAX_MODULE_NESTING_DEPTH
        );

        let above_limit = nested_modules(MAX_MODULE_NESTING_DEPTH + 1);
        let parsed = parse_cst(&above_limit);
        assert_eq!(parsed.reconstruct(), above_limit);
        let depth_errors = parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == "NMLT2014")
            .collect::<Vec<_>>();
        assert_eq!(depth_errors.len(), 1);
        let rejected_start = above_limit
            .find(&format!("module M{MAX_MODULE_NESTING_DEPTH} {{"))
            .expect("the boundary module is present");
        assert_eq!(
            depth_errors[0].span,
            Some(Span::new(rejected_start, rejected_start + "module".len(),))
        );
        assert_eq!(parsed.root().descendants(SyntaxKind::Error).len(), 1);
        assert_eq!(
            parsed.root().descendants(SyntaxKind::ModuleDecl).len(),
            MAX_MODULE_NESTING_DEPTH
        );

        let openings = (0..MAX_MODULE_NESTING_DEPTH)
            .map(|index| format!("module M{index} {{"))
            .collect::<String>();
        let newline_body = format!(
            "{openings}module TooDeep\n{{ system Hidden {{}} }}\nsystem Sibling {{}}{}",
            "}".repeat(MAX_MODULE_NESTING_DEPTH)
        );
        let parsed = parse_cst(&newline_body);
        assert_eq!(parsed.reconstruct(), newline_body);
        assert_eq!(
            parsed
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.code == "NMLT2014")
                .count(),
            1
        );
        assert_eq!(parsed.root().descendants(SyntaxKind::Error).len(), 1);
        assert_eq!(parsed.root().descendants(SyntaxKind::SystemDecl).len(), 1);
        assert_eq!(parsed.systems()[0].name, "Sibling");
    }

    #[test]
    fn rejects_unbalanced_or_mismatched_delimiters() {
        for source in [
            "system Broken {",
            "system Broken { action x(] }",
            "}",
            "junk }",
        ] {
            let parsed = parse_cst(source);
            assert_eq!(parsed.reconstruct(), source);
            assert!(
                parsed
                    .diagnostics()
                    .iter()
                    .any(|item| item.code == "NMLT0002")
            );
        }
    }

    #[test]
    fn compatibility_parser_requires_a_system() {
        let diagnostics = parse_source("data Bool = True | False").unwrap_err();
        assert!(diagnostics.iter().any(|item| item.code == "NMLT0005"));
        assert!(
            parse_cst("data Bool = True | False")
                .diagnostics()
                .is_empty()
        );
    }

    #[test]
    fn preserves_the_empty_source_diagnostic_for_whitespace() {
        let diagnostics = parse_source(" \r\n\t").unwrap_err();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "NMLT0001");
        assert_eq!(parse_cst(" \r\n\t").reconstruct(), " \r\n\t");
    }
}

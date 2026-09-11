//! Interpret the canonical declaration projection; never rescan top-level declarations.
use crate::*;
use nmlt_core::{
    SyntaxKind, TokenKind, UntypedDeclaration, lex_source, parse_cst, project_untyped,
};
use std::collections::BTreeSet;

#[derive(Clone)]
struct Token {
    text: String,
    kind: TokenKind,
    span: Location,
}
struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    end: Location,
    depth: usize,
    nodes: usize,
}

pub(super) fn source(source: &str) -> Result<RawSource, Diagnostic> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(error(
            Location {
                source: 0,
                start: 0,
                end: 0,
            },
            "workflow source exceeds 128 KiB",
        ));
    }
    let parsed = parse_cst(source);
    if let Some(diagnostic) = parsed.diagnostics().first() {
        return Err(diagnostic.clone().into());
    }
    let projected = project_untyped(&parsed);
    if !projected.is_structurally_complete() {
        return Err(error(
            Location {
                source: 0,
                start: 0,
                end: 0,
            },
            "incomplete canonical source projection",
        ));
    }
    let mut result = RawSource {
        functions: vec![],
        records: vec![],
        imports: vec![],
        modules: vec![],
    };
    declarations(&projected.file.declarations, "", 0, &mut result)?;
    Ok(result)
}

fn declarations(
    items: &[UntypedDeclaration],
    module: &str,
    depth: usize,
    out: &mut RawSource,
) -> Result<(), Diagnostic> {
    let mut modules = BTreeSet::new();
    for item in items {
        match item {
            UntypedDeclaration::Import(import) => {
                if depth != 0 {
                    return Err(error(
                        import.span.into(),
                        "workflow imports must be at file root",
                    ));
                }
                let name = import
                    .module
                    .as_ref()
                    .ok_or_else(|| error(import.span.into(), "missing imported module"))?;
                if out.imports.iter().any(|(n, _)| n == &name.text) {
                    return Err(error(name.span.into(), "duplicate import"));
                }
                out.imports.push((name.text.clone(), name.span.into()));
            }
            UntypedDeclaration::Module(m) => {
                if depth == 8 {
                    return Err(error(
                        m.span.into(),
                        "workflow module nesting exceeds eight",
                    ));
                }
                let name = m
                    .name
                    .as_ref()
                    .ok_or_else(|| error(m.span.into(), "missing module name"))?;
                if !modules.insert(name.text.clone()) {
                    return Err(error(name.span.into(), "duplicate module"));
                }
                out.modules
                    .push((qualify(module, &name.text), name.span.into()));
                declarations(
                    &m.declarations,
                    &qualify(module, &name.text),
                    depth + 1,
                    out,
                )?;
            }
            UntypedDeclaration::Unsupported(raw) if raw.kind == SyntaxKind::FunctionDecl => {
                if out.functions.len() == MAX_FUNCTIONS {
                    return Err(error(
                        raw.source.span.into(),
                        "workflow exceeds 64 functions",
                    ));
                }
                let mut parser = Parser::new(&raw.source.text, raw.source.span.start);
                out.functions.push(parser.function(module)?);
            }
            UntypedDeclaration::Unsupported(raw) if raw.kind == SyntaxKind::RecordDecl => {
                if out.records.len() == 64 {
                    return Err(error(raw.source.span.into(), "workflow exceeds 64 records"));
                }
                let mut parser = Parser::new(&raw.source.text, raw.source.span.start);
                out.records.push(parser.record(module)?);
            }
            _ => {
                return Err(error(
                    item.span().into(),
                    "workflow route accepts only fn/record declarations, file-root imports, and local module wrappers; unsupported declarations are never omitted",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn qualify(module: &str, name: &str) -> String {
    if module.is_empty() {
        name.into()
    } else {
        format!("{module}.{name}")
    }
}

impl Parser {
    fn new(source: &str, base: usize) -> Self {
        let mut tokens = vec![];
        for token in lex_source(source)
            .tokens
            .into_iter()
            .filter(|t| !t.kind.is_trivia())
        {
            let text = token.text(source);
            if token.kind == TokenKind::Punctuation {
                let mut offset = 0;
                while offset < text.len() {
                    let tail = &text[offset..];
                    let len = if ["->", "=>", "==", "!=", "<=", ">="]
                        .iter()
                        .any(|op| tail.starts_with(op))
                    {
                        2
                    } else {
                        1
                    };
                    tokens.push(Token {
                        text: tail[..len].into(),
                        kind: token.kind,
                        span: Location {
                            source: 0,
                            start: base + token.span.start + offset,
                            end: base + token.span.start + offset + len,
                        },
                    });
                    offset += len;
                }
            } else {
                tokens.push(Token {
                    text: text.into(),
                    kind: token.kind,
                    span: Location {
                        source: 0,
                        start: base + token.span.start,
                        end: base + token.span.end,
                    },
                });
            }
        }
        Self {
            tokens,
            cursor: 0,
            end: Location {
                source: 0,
                start: base + source.len(),
                end: base + source.len(),
            },
            depth: 0,
            nodes: 0,
        }
    }
    fn span(&self) -> Location {
        self.tokens.get(self.cursor).map_or(self.end, |t| t.span)
    }
    fn peek(&self) -> &str {
        self.tokens.get(self.cursor).map_or("", |t| t.text.as_str())
    }
    fn eat(&mut self, text: &str) -> bool {
        if self.peek() == text {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, text: &str) -> Result<(), Diagnostic> {
        if self.eat(text) {
            Ok(())
        } else {
            Err(error(
                self.span(),
                format!("expected `{text}`, found `{}`", self.peek()),
            ))
        }
    }
    fn name(&mut self) -> Result<String, Diagnostic> {
        let Some(token) = self.tokens.get(self.cursor) else {
            return Err(error(self.end, "expected a name"));
        };
        if token.kind != TokenKind::Identifier
            || [
                "fn",
                "let",
                "if",
                "else",
                "match",
                "true",
                "false",
                "and",
                "or",
                "not",
                "Ok",
                "Err",
                "record",
                "new",
                "fold",
                "length",
                "get",
                "job_square",
                "job_start_square",
                "job_start_lean",
                "job_start_lean_check",
                "job_start_lean_project",
                "job_poll",
                "job_cancel",
                "job_collect",
            ]
            .contains(&token.text.as_str())
        {
            return Err(error(token.span, "expected a non-reserved identifier"));
        }
        let text = token.text.clone();
        self.cursor += 1;
        Ok(text)
    }
    fn ty(&mut self, depth: usize) -> Result<Type, Diagnostic> {
        if depth > 8 {
            return Err(error(self.span(), "type nesting exceeds eight"));
        }
        let name = self.name()?;
        Ok(match name.as_str() {
            "Bool" => Type::Bool,
            "Int" => Type::Int,
            "Text" => Type::Text,
            "Outcome" | "List" | "Job" => {
                self.expect("<")?;
                let inner = self.ty(depth + 1)?;
                self.expect(">")?;
                if name == "Outcome" {
                    Type::Outcome(Box::new(inner))
                } else if name == "Job" {
                    Type::Job(Box::new(inner))
                } else {
                    Type::List(Box::new(inner))
                }
            }
            _ => {
                let mut name = name;
                while self.eat(".") {
                    name.push('.');
                    name.push_str(&self.name()?);
                }
                Type::Record(name)
            }
        })
    }
    fn record(&mut self, module: &str) -> Result<RawRecord, Diagnostic> {
        let span = self.span();
        self.expect("record")?;
        let local = self.name()?;
        if ["Int", "Bool", "Text", "Outcome", "List", "Job"].contains(&local.as_str()) {
            return Err(error(span, "record name conflicts with a built-in type"));
        }
        let name = qualify(module, &local);
        self.expect("{")?;
        let mut fields = vec![];
        let mut names = BTreeSet::new();
        while !self.eat("}") {
            if fields.len() == 16 {
                return Err(error(self.span(), "record exceeds 16 fields"));
            }
            let at = self.span();
            let name = self.name()?;
            if !names.insert(name.clone()) {
                return Err(error(at, "duplicate record field"));
            }
            self.expect(":")?;
            fields.push(Parameter {
                name,
                ty: self.ty(0)?,
            });
            if self.eat("}") {
                break;
            }
            self.expect(",")?;
        }
        if self.cursor != self.tokens.len() {
            return Err(error(self.span(), "unexpected trailing record tokens"));
        }
        Ok(RawRecord {
            name,
            module: module.into(),
            span,
            fields,
        })
    }
    fn function(&mut self, module: &str) -> Result<RawFunction, Diagnostic> {
        let span = self.span();
        self.expect("fn")?;
        let name = qualify(module, &self.name()?);
        self.expect("(")?;
        let mut parameters = vec![];
        let mut names = BTreeSet::new();
        if !self.eat(")") {
            loop {
                if parameters.len() == 16 {
                    return Err(error(self.span(), "function exceeds 16 parameters"));
                }
                let at = self.span();
                let name = self.name()?;
                if !names.insert(name.clone()) {
                    return Err(error(at, "duplicate parameter"));
                }
                self.expect(":")?;
                let ty = self.ty(0)?;
                parameters.push(Parameter { name, ty });
                if self.eat(")") {
                    break;
                }
                self.expect(",")?;
            }
        }
        self.expect("->")?;
        let result = self.ty(0)?;
        let body = self.block()?;
        if self.cursor != self.tokens.len() {
            return Err(error(self.span(), "unexpected trailing function tokens"));
        }
        Ok(RawFunction {
            name,
            module: module.into(),
            span,
            parameters,
            result,
            body,
        })
    }
    fn block(&mut self) -> Result<Expr, Diagnostic> {
        self.expect("{")?;
        let value = self.expr(0)?;
        self.expect("}")?;
        Ok(value)
    }
    fn node(&mut self, span: Location, kind: ExprKind) -> Result<Expr, Diagnostic> {
        let depth = 1 + match &kind {
            ExprKind::Literal(_) => 0,
            ExprKind::Name(name) => name.matches('.').count(),
            ExprKind::Call(_, args) | ExprKind::List(args) => {
                args.iter().map(|a| a.depth).max().unwrap_or(0)
            }
            ExprKind::Record(_, fields) => fields.iter().map(|(_, a)| a.depth).max().unwrap_or(0),
            ExprKind::Fold(a, b, _, _, c) => a.depth.max(b.depth).max(c.depth),
            ExprKind::Let(_, _, a, b) | ExprKind::Binary(_, a, b) => a.depth.max(b.depth),
            ExprKind::If(a, b, c) | ExprKind::Match(a, _, b, _, c) => {
                a.depth.max(b.depth).max(c.depth)
            }
            ExprKind::Unary(_, a) | ExprKind::Field(a, _) => a.depth,
        };
        if depth > MAX_DEPTH || self.nodes == 2048 {
            return Err(error(span, "expression depth/node bound exceeded"));
        }
        self.nodes += 1;
        Ok(Expr { span, kind, depth })
    }
    fn expr(&mut self, min: u8) -> Result<Expr, Diagnostic> {
        if self.depth == MAX_DEPTH {
            return Err(error(self.span(), "expression parser depth bound exceeded"));
        }
        self.depth += 1;
        let result = self.expr_inner(min);
        self.depth -= 1;
        result
    }
    fn expr_inner(&mut self, min: u8) -> Result<Expr, Diagnostic> {
        let start = self.span();
        let kind = if self.eat("new") {
            let mut name = self.name()?;
            while self.eat(".") {
                name.push('.');
                name.push_str(&self.name()?);
            }
            self.expect("{")?;
            let mut fields = vec![];
            let mut names = BTreeSet::new();
            while !self.eat("}") {
                if fields.len() == 16 {
                    return Err(error(self.span(), "record exceeds 16 fields"));
                }
                let at = self.span();
                let field = self.name()?;
                if !names.insert(field.clone()) {
                    return Err(error(at, "duplicate record initializer"));
                }
                self.expect(":")?;
                fields.push((field, self.expr(0)?));
                if self.eat("}") {
                    break;
                }
                self.expect(",")?;
            }
            ExprKind::Record(name, fields)
        } else if self.eat("[") {
            let mut items = vec![];
            while !self.eat("]") {
                if items.len() == MAX_LIST_ITEMS {
                    return Err(error(self.span(), "list exceeds 256 items"));
                }
                items.push(self.expr(0)?);
                if self.eat("]") {
                    break;
                }
                self.expect(",")?;
            }
            ExprKind::List(items)
        } else if self.eat("fold") {
            self.expect("(")?;
            let items = self.expr(0)?;
            self.expect(",")?;
            let initial = self.expr(0)?;
            self.expect(",")?;
            let acc = self.name()?;
            self.expect(",")?;
            let item = self.name()?;
            if acc == item {
                return Err(error(start, "fold binders must differ"));
            }
            self.expect("=>")?;
            let body = self.expr(0)?;
            self.expect(")")?;
            ExprKind::Fold(
                Box::new(items),
                Box::new(initial),
                acc,
                item,
                Box::new(body),
            )
        } else if self.eat("let") {
            let name = self.name()?;
            let annotation = if self.eat(":") {
                Some(self.ty(0)?)
            } else {
                None
            };
            self.expect("=")?;
            let value = self.expr(0)?;
            self.expect(";")?;
            ExprKind::Let(name, annotation, Box::new(value), Box::new(self.expr(0)?))
        } else if self.eat("if") {
            let condition = self.expr(0)?;
            let yes = self.block()?;
            self.expect("else")?;
            let no = self.block()?;
            ExprKind::If(Box::new(condition), Box::new(yes), Box::new(no))
        } else if self.eat("match") {
            let value = self.expr(0)?;
            self.expect("{")?;
            let mut ok = None;
            let mut err = None;
            for _ in 0..2 {
                let at = self.span();
                let tag = self.peek().to_owned();
                if !["Ok", "Err"].contains(&tag.as_str()) {
                    return Err(error(at, "match needs exactly one Ok and one Err arm"));
                }
                self.cursor += 1;
                self.expect("(")?;
                let binder = self.name()?;
                self.expect(")")?;
                self.expect("=>")?;
                let body = if self.peek() == "{" {
                    self.block()?
                } else {
                    self.expr(0)?
                };
                let slot = if tag == "Ok" { &mut ok } else { &mut err };
                if slot.replace((binder, body)).is_some() {
                    return Err(error(at, "duplicate match arm"));
                }
                if self.peek() != "}" {
                    self.expect(",")?;
                }
            }
            self.eat(",");
            self.expect("}")?;
            let (oname, obody) = ok.ok_or_else(|| error(start, "missing Ok arm"))?;
            let (ename, ebody) = err.ok_or_else(|| error(start, "missing Err arm"))?;
            ExprKind::Match(
                Box::new(value),
                oname,
                Box::new(obody),
                ename,
                Box::new(ebody),
            )
        } else if ["-", "not"].contains(&self.peek()) {
            let op = self.peek().to_owned();
            self.cursor += 1;
            // Parse the full negative literal, including i64::MIN, before conversion.
            if op == "-"
                && self
                    .tokens
                    .get(self.cursor)
                    .is_some_and(|t| t.kind == TokenKind::Integer)
            {
                let magnitude = self
                    .peek()
                    .replace('_', "")
                    .parse::<u64>()
                    .map_err(|_| error(start, "Int literal out of range"))?;
                let value = i64::try_from(-(magnitude as i128))
                    .map_err(|_| error(start, "Int literal out of range"))?;
                self.cursor += 1;
                ExprKind::Literal(Value::Int(value))
            } else {
                ExprKind::Unary(op, Box::new(self.expr(6)?))
            }
        } else if self.eat("(") {
            let inner = self.expr(0)?;
            self.expect(")")?;
            inner.kind
        } else if self.peek() == "{" {
            self.block()?.kind
        } else {
            let token = self
                .tokens
                .get(self.cursor)
                .cloned()
                .ok_or_else(|| error(start, "expected expression"))?;
            self.cursor += 1;
            match token.kind {
                TokenKind::Integer => ExprKind::Literal(Value::Int(
                    token
                        .text
                        .replace('_', "")
                        .parse()
                        .map_err(|_| error(token.span, "Int literal out of range"))?,
                )),
                TokenKind::String => {
                    let text: String = serde_json::from_str(&token.text).map_err(|_| {
                        error(token.span, "Text literal must use JSON string escapes")
                    })?;
                    if text.len() > 4096 {
                        return Err(error(token.span, "Text exceeds 4096 bytes"));
                    }
                    ExprKind::Literal(Value::Text(text))
                }
                TokenKind::Identifier if token.text == "true" || token.text == "false" => {
                    ExprKind::Literal(Value::Bool(token.text == "true"))
                }
                TokenKind::Identifier => {
                    self.cursor -= 1;
                    let mut name = if [
                        "Ok",
                        "Err",
                        "length",
                        "get",
                        "job_square",
                        "job_start_square",
                        "job_start_lean",
                        "job_start_lean_check",
                        "job_start_lean_project",
                        "job_poll",
                        "job_cancel",
                        "job_collect",
                    ]
                    .contains(&self.peek())
                    {
                        self.cursor += 1;
                        token.text
                    } else {
                        self.name()?
                    };
                    while self.eat(".") {
                        name.push('.');
                        name.push_str(&self.name()?);
                    }
                    if self.eat("(") {
                        let mut args = vec![];
                        if !self.eat(")") {
                            loop {
                                if args.len() == 16 {
                                    return Err(error(self.span(), "call exceeds 16 arguments"));
                                }
                                args.push(self.expr(0)?);
                                if self.eat(")") {
                                    break;
                                }
                                self.expect(",")?;
                            }
                        }
                        ExprKind::Call(name, args)
                    } else {
                        ExprKind::Name(name)
                    }
                }
                _ => return Err(error(token.span, "expected workflow expression")),
            }
        };
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .map_or(start.end, |t| t.span.end);
        let mut left = self.node(
            Location {
                source: 0,
                start: start.start,
                end,
            },
            kind,
        )?;
        let mut chain = 0;
        loop {
            if self.eat(".") {
                let field = self.name()?;
                let span = Location {
                    source: 0,
                    start: left.span.start,
                    end: self.tokens[self.cursor - 1].span.end,
                };
                left = self.node(span, ExprKind::Field(Box::new(left), field))?;
                continue;
            }
            let precedence = match self.peek() {
                "or" => 1,
                "and" => 2,
                "==" | "!=" | "<" | ">" | "<=" | ">=" => 3,
                "+" | "-" => 4,
                "*" => 5,
                _ => 0,
            };
            if precedence == 0 || precedence < min {
                break;
            }
            chain += 1;
            if chain == MAX_DEPTH {
                return Err(error(start, "operator chain exceeds depth bound"));
            }
            let op = self.peek().to_owned();
            self.cursor += 1;
            let right = self.expr(precedence + 1)?;
            let span = Location {
                source: 0,
                start: left.span.start,
                end: right.span.end,
            };
            left = self.node(span, ExprKind::Binary(op, Box::new(left), Box::new(right)))?;
        }
        Ok(left)
    }
}

//! Compile source safety declarations through the canonical source projection.
use crate::behavior::{BehaviorDiagnostic, compile_with_properties, reject};
use nmlt_core::{TokenKind, lex_source};
use nmlt_ir::{
    BehaviorCoreProgram, CoreBehaviorTerm as Term, SafetyPredicate as Predicate, SafetyProgram,
    SafetyProperty,
};
use std::collections::BTreeSet;

pub fn compile_safety(
    repository_path: impl Into<String>,
    bytes: impl Into<Vec<u8>>,
) -> Result<SafetyProgram, BehaviorDiagnostic> {
    let bytes = bytes.into();
    if bytes.len() > 131_072 {
        return Err(reject("NMLT-SAFETY-BOUND", "safety source exceeds 128 KiB"));
    }
    let (core, raw) = compile_with_properties(repository_path.into(), bytes, true, true)?;
    if raw.is_empty() || raw.len() > 64 {
        return Err(reject(
            "NMLT-SAFETY-COUNT",
            "safety source needs 1..64 safety declarations",
        ));
    }
    let mut properties = vec![];
    let mut names = BTreeSet::new();
    for (system, raw) in raw {
        let name = raw
            .name
            .ok_or_else(|| reject("NMLT-SAFETY-NAME", "missing property name"))?
            .text;
        if !names.insert((system.clone(), name.clone())) {
            return Err(reject("NMLT-SAFETY-NAME", "duplicate safety property"));
        }
        let expression = raw
            .expression
            .ok_or_else(|| reject("NMLT-SAFETY-EXPR", "missing property expression"))?
            .source;
        if name.len() > 256 || raw.span.end - raw.span.start > 8192 {
            return Err(reject(
                "NMLT-SAFETY-BOUND",
                "safety declaration or name exceeds bound",
            )
            .at(raw.span));
        }
        let predicate =
            parse_predicate(&expression.text, &core, &system).map_err(|e| e.at(expression.span))?;
        properties.push(SafetyProperty {
            system,
            name,
            expression: expression.text,
            expression_start: expression.span.start,
            expression_end: expression.span.end,
            declaration_start: raw.span.start,
            declaration_end: raw.span.end,
            predicate,
        });
    }
    properties.sort_by(|a, b| (&a.system, &a.name).cmp(&(&b.system, &b.name)));
    Ok(SafetyProgram { core, properties })
}

pub fn parse_predicate(
    source: &str,
    core: &BehaviorCoreProgram,
    system: &str,
) -> Result<Predicate, BehaviorDiagnostic> {
    if source.len() > 4096 {
        return Err(reject("NMLT-SAFETY-BOUND", "predicate exceeds 4096 bytes"));
    }
    let lexed = lex_source(source);
    if !lexed.diagnostics.is_empty() {
        return Err(reject("NMLT-SAFETY-SYNTAX", "malformed predicate token"));
    }
    let tokens: Vec<_> = lexed
        .tokens
        .iter()
        .filter(|t| !t.kind.is_trivia())
        .map(|t| t.text(source))
        .collect();
    if tokens.len() > 256
        || lexed.tokens.iter().any(|t| {
            matches!(
                t.kind,
                TokenKind::Unknown | TokenKind::Error | TokenKind::String | TokenKind::Integer
            )
        })
    {
        return Err(reject(
            "NMLT-SAFETY-SYNTAX",
            "predicate token or size is outside the finite safety grammar",
        ));
    }
    let mut parser = Parser {
        tokens,
        cursor: 0,
        core,
        system,
    };
    parser.expect("always")?;
    parser.expect("(")?;
    let result = parser.implies(64)?;
    parser.expect(")")?;
    if parser.cursor != parser.tokens.len() {
        return Err(parser.error("unsupported trailing predicate syntax"));
    }
    Ok(result)
}

struct Parser<'a> {
    tokens: Vec<&'a str>,
    cursor: usize,
    core: &'a BehaviorCoreProgram,
    system: &'a str,
}
impl Parser<'_> {
    fn error(&self, message: &str) -> BehaviorDiagnostic {
        reject("NMLT-SAFETY-SYNTAX", message)
    }
    fn take(&mut self, token: &str) -> bool {
        if self.tokens.get(self.cursor) == Some(&token) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, token: &str) -> Result<(), BehaviorDiagnostic> {
        if self.take(token) {
            Ok(())
        } else {
            Err(reject(
                "NMLT-SAFETY-SYNTAX",
                format!("expected `{token}` in safety expression"),
            ))
        }
    }
    fn depth(&self, depth: usize) -> Result<usize, BehaviorDiagnostic> {
        depth
            .checked_sub(1)
            .ok_or_else(|| reject("NMLT-SAFETY-BOUND", "predicate nesting exceeds bound"))
    }
    fn implies(&mut self, depth: usize) -> Result<Predicate, BehaviorDiagnostic> {
        let depth = self.depth(depth)?;
        let left = self.or(depth)?;
        if self.take("implies") {
            Ok(Predicate::Implies {
                left: Box::new(left),
                right: Box::new(self.implies(depth)?),
            })
        } else {
            Ok(left)
        }
    }
    fn or(&mut self, depth: usize) -> Result<Predicate, BehaviorDiagnostic> {
        let mut left = self.and(depth)?;
        let mut remaining = depth;
        while self.take("or") || self.take("||") {
            remaining = self.depth(remaining)?;
            left = Predicate::Or {
                left: Box::new(left),
                right: Box::new(self.and(remaining)?),
            };
        }
        Ok(left)
    }
    fn and(&mut self, depth: usize) -> Result<Predicate, BehaviorDiagnostic> {
        let mut left = self.not(depth)?;
        let mut remaining = depth;
        while self.take("and") || self.take("&&") {
            remaining = self.depth(remaining)?;
            left = Predicate::And {
                left: Box::new(left),
                right: Box::new(self.not(remaining)?),
            };
        }
        Ok(left)
    }
    fn not(&mut self, depth: usize) -> Result<Predicate, BehaviorDiagnostic> {
        let depth = self.depth(depth)?;
        if self.take("not") || self.take("!") {
            return Ok(Predicate::Not {
                value: Box::new(self.not(depth)?),
            });
        }
        if self.take("(") {
            let result = self.implies(depth)?;
            self.expect(")")?;
            return Ok(result);
        }
        let left = self.value()?;
        let equal = self.take("==");
        let different = !equal && self.take("!=");
        if equal || different {
            let right = self.value()?;
            if left.ty() != right.ty() {
                return Err(self
                    .error("equality operands must have the same finite type")
                    .mismatch(left.ty(), right.ty()));
            }
            let comparison = Predicate::Boolean {
                term: Term::Equal {
                    r#type: "Bool".into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
            return Ok(if different {
                Predicate::Not {
                    value: Box::new(comparison),
                }
            } else {
                comparison
            });
        }
        if left.ty() != "Bool" {
            return Err(self
                .error("a safety predicate must have type Bool")
                .mismatch("Bool", left.ty()));
        }
        Ok(Predicate::Boolean { term: left })
    }
    fn value(&mut self) -> Result<Term, BehaviorDiagnostic> {
        let token = self
            .tokens
            .get(self.cursor)
            .copied()
            .ok_or_else(|| self.error("missing predicate value"))?;
        self.cursor += 1;
        match token {
            "true" | "false" => {
                return Ok(Term::Bool {
                    r#type: "Bool".into(),
                    value: token == "true",
                });
            }
            "unit" => {
                return Ok(Term::Unit {
                    r#type: "Unit".into(),
                });
            }
            _ => {}
        }
        let mut name = token.to_owned();
        while self.take(".") {
            let next = self
                .tokens
                .get(self.cursor)
                .copied()
                .ok_or_else(|| self.error("missing qualified name"))?;
            self.cursor += 1;
            name.push('.');
            name.push_str(next);
        }
        let system = self
            .core
            .systems
            .get(self.system)
            .ok_or_else(|| self.error("unknown predicate system"))?;
        if let Some(field) = system.state.get(&name) {
            return Ok(Term::Read {
                r#type: field.ty.clone(),
                field: name,
            });
        }
        if let Some((ty, constructor)) = name.rsplit_once('.')
            && self
                .core
                .enums
                .get(ty)
                .is_some_and(|variants| variants.contains(constructor))
        {
            return Ok(Term::Enum {
                r#type: ty.into(),
                constructor: constructor.into(),
            });
        }
        Err(reject(
            "NMLT-SAFETY-NAME",
            format!("unknown finite field or qualified enum value `{name}`"),
        ))
    }
}

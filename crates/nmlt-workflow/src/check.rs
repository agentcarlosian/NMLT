use crate::*;
use std::collections::{BTreeMap, BTreeSet};

/// Compile every declaration and branch, including unused functions. This is
/// Rust acceptance for the pure workflow profile, not the M9 or Lean checker.
pub fn compile(source: &str) -> Result<Program, Diagnostic> {
    let parsed = parse::source(source)?;
    if let Some((_, span)) = parsed.imports.first() {
        return Err(error(
            *span,
            "imports require compile_package or the workflow CLI",
        ));
    }
    compile_raw(
        parsed,
        &[Scope {
            root: String::new(),
            imports: BTreeSet::new(),
            libraries: BTreeSet::new(),
        }],
        vec![SourceIdentity {
            path: "source.nmlt".into(),
            bytes: source.len(),
            source_sha256: digest(source.as_bytes()),
        }],
    )
}

pub(super) fn compile_raw(
    parsed: RawSource,
    scopes: &[Scope],
    sources: Vec<SourceIdentity>,
) -> Result<Program, Diagnostic> {
    if parsed.functions.is_empty() {
        return Err(error(
            Location {
                source: 0,
                start: 0,
                end: 0,
            },
            "workflow requires a function",
        ));
    }
    if parsed.functions.len() > MAX_FUNCTIONS || parsed.records.len() > 64 {
        return Err(error(
            Location {
                source: 0,
                start: 0,
                end: 0,
            },
            "package exceeds 64 functions or records",
        ));
    }
    let mut records = BTreeMap::new();
    for r in &parsed.records {
        if records.insert(r.name.clone(), r.fields.clone()).is_some() {
            return Err(error(r.span, format!("duplicate record `{}`", r.name)));
        }
    }
    for r in &parsed.records {
        let fields = r
            .fields
            .iter()
            .map(|f| {
                Ok(Parameter {
                    name: f.name.clone(),
                    ty: resolve_type(&f.ty, &r.module, &scopes[r.span.source], &records, r.span)?,
                })
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        records.insert(r.name.clone(), fields);
    }
    let mut heights = BTreeMap::new();
    for r in &parsed.records {
        if records[&r.name]
            .iter()
            .any(|f| matches!(f.ty, Type::Job(_)))
        {
            return Err(error(r.span, "Job handles cannot be record fields"));
        }
        type_height(
            &Type::Record(r.name.clone()),
            &records,
            &mut heights,
            &mut BTreeSet::new(),
            r.span,
        )?;
    }
    let mut raw = parsed.functions;
    for f in &mut raw {
        for p in &mut f.parameters {
            p.ty = resolve_type(&p.ty, &f.module, &scopes[f.span.source], &records, f.span)?;
            type_height(&p.ty, &records, &mut heights, &mut BTreeSet::new(), f.span)?;
        }
        f.result = resolve_type(
            &f.result,
            &f.module,
            &scopes[f.span.source],
            &records,
            f.span,
        )?;
        type_height(
            &f.result,
            &records,
            &mut heights,
            &mut BTreeSet::new(),
            f.span,
        )?;
    }
    let mut names = BTreeMap::new();
    for (index, f) in raw.iter().enumerate() {
        if names.insert(f.name.clone(), index).is_some() {
            return Err(error(f.span, format!("duplicate function `{}`", f.name)));
        }
    }
    let mut functions = vec![];
    let mut edges = vec![];
    for f in &raw {
        let mut checker = Checker {
            raw: &raw,
            names: &names,
            module: &f.module,
            scope: &scopes[f.span.source],
            records: &records,
            heights: heights.clone(),
            calls: BTreeSet::new(),
            jobs: false,
            async_jobs: false,
            lean_jobs: false,
            project_jobs: false,
        };
        let mut env = f
            .parameters
            .iter()
            .map(|p| (p.name.clone(), p.ty.clone()))
            .collect();
        let body = checker
            .term(&f.body, Some(&f.result), &mut env, 0)
            .map_err(|mut d| {
                d.related
                    .push((f.span, format!("in function `{}` declared here", f.name)));
                d
            })?;
        affine::check(&body, &f.parameters).map_err(|mut d| {
            d.related
                .push((f.span, format!("in function `{}` declared here", f.name)));
            d
        })?;
        functions.push(Function {
            name: f.name.clone(),
            parameters: f.parameters.clone(),
            result: f.result.clone(),
            body,
            jobs: checker.jobs,
            async_jobs: checker.async_jobs,
            lean_jobs: checker.lean_jobs,
            project_jobs: checker.project_jobs,
        });
        edges.push(checker.calls);
    }
    let mut marks = vec![0; raw.len()];
    for index in 0..raw.len() {
        acyclic(index, &edges, &mut marks, &raw)?;
    }
    // At most one propagation per function on the already acyclic call graph.
    for _ in 0..functions.len() {
        for i in 0..functions.len() {
            functions[i].jobs |= edges[i].iter().any(|j| functions[*j].jobs);
            functions[i].async_jobs |= edges[i].iter().any(|j| functions[*j].async_jobs);
            functions[i].lean_jobs |= edges[i].iter().any(|j| functions[*j].lean_jobs);
            functions[i].project_jobs |= edges[i].iter().any(|j| functions[*j].project_jobs);
        }
    }
    Ok(Program {
        functions,
        records,
        sources,
    })
}

fn resolve_name<T>(
    name: &str,
    module: &str,
    scope: &Scope,
    names: &BTreeMap<String, T>,
) -> Option<String> {
    if name.contains('.') {
        let first = name.split('.').next().unwrap();
        if scope.imports.contains(first) {
            return names.contains_key(name).then(|| name.into());
        }
        let local = parse::qualify(&scope.root, name);
        // Root-file qualified names must not bypass direct-import requirements
        // merely because another dependency loaded the named library.
        if scope.root.is_empty() && scope.libraries.contains(first) {
            return None;
        }
        names.contains_key(&local).then_some(local)
    } else {
        let local = parse::qualify(module, name);
        if names.contains_key(&local) {
            Some(local)
        } else {
            let root = parse::qualify(&scope.root, name);
            names.contains_key(&root).then_some(root)
        }
    }
}
fn resolve_type(
    ty: &Type,
    module: &str,
    scope: &Scope,
    records: &BTreeMap<String, Vec<Parameter>>,
    span: Location,
) -> Result<Type, Diagnostic> {
    Ok(match ty {
        Type::Record(name) => Type::Record(
            resolve_name(name, module, scope, records)
                .ok_or_else(|| error(span, format!("unknown record type `{name}`")))?,
        ),
        Type::List(t) => Type::List(Box::new(resolve_type(t, module, scope, records, span)?)),
        Type::Outcome(t) => Type::Outcome(Box::new(resolve_type(t, module, scope, records, span)?)),
        _ => ty.clone(),
    })
}
fn type_height(
    ty: &Type,
    records: &BTreeMap<String, Vec<Parameter>>,
    heights: &mut BTreeMap<String, usize>,
    active: &mut BTreeSet<String>,
    span: Location,
) -> Result<usize, Diagnostic> {
    let height = match ty {
        Type::Outcome(t) | Type::List(t) => {
            if matches!(**t, Type::Job(_)) {
                return Err(error(
                    span,
                    "Job handles cannot be stored in outcomes or lists",
                ));
            }
            1 + type_height(t, records, heights, active, span)?
        }
        Type::Job(t) => {
            if !matches!(**t, Type::Int | Type::Text) {
                return Err(error(span, "Job output must be Int or Text"));
            }
            1
        }
        Type::Record(name) => {
            if let Some(h) = heights.get(name) {
                return Ok(*h);
            }
            if !active.insert(name.clone()) {
                return Err(error(span, "recursive record types are unsupported"));
            }
            let mut h = 0;
            for f in &records[name] {
                h = h.max(1 + type_height(&f.ty, records, heights, active, span)?);
            }
            active.remove(name);
            heights.insert(name.clone(), h);
            h
        }
        _ => 0,
    };
    if height > 16 {
        return Err(error(span, "expanded type nesting exceeds sixteen"));
    }
    Ok(height)
}

fn acyclic(
    index: usize,
    edges: &[BTreeSet<usize>],
    marks: &mut [u8],
    raw: &[RawFunction],
) -> Result<(), Diagnostic> {
    if marks[index] == 1 {
        return Err(error(
            raw[index].span,
            "recursive function cycle is outside the pure workflow profile",
        ));
    }
    if marks[index] == 2 {
        return Ok(());
    }
    marks[index] = 1;
    for child in &edges[index] {
        acyclic(*child, edges, marks, raw)?;
    }
    marks[index] = 2;
    Ok(())
}

struct Checker<'a> {
    raw: &'a [RawFunction],
    names: &'a BTreeMap<String, usize>,
    module: &'a str,
    scope: &'a Scope,
    records: &'a BTreeMap<String, Vec<Parameter>>,
    heights: BTreeMap<String, usize>,
    calls: BTreeSet<usize>,
    jobs: bool,
    async_jobs: bool,
    lean_jobs: bool,
    project_jobs: bool,
}
impl Checker<'_> {
    fn term(
        &mut self,
        expr: &Expr,
        expected: Option<&Type>,
        env: &mut Vec<(String, Type)>,
        depth: usize,
    ) -> Result<Typed, Diagnostic> {
        if depth > MAX_DEPTH {
            return Err(error(expr.span, "lowered expression exceeds depth bound"));
        }
        let (ty, kind) = match &expr.kind {
            ExprKind::Literal(value) => {
                let ty = match value {
                    Value::Bool(_) => Type::Bool,
                    Value::Int(_) => Type::Int,
                    Value::Text(_) => Type::Text,
                    _ => unreachable!(),
                };
                (ty, TypedKind::Literal(value.clone()))
            }
            ExprKind::Name(name) => {
                let mut path = name.split('.');
                let root = path.next().unwrap();
                if path.clone().count() >= MAX_DEPTH {
                    return Err(error(expr.span, "field path exceeds depth bound"));
                }
                let index = env.iter().rposition(|(n, _)| n == root).ok_or_else(|| {
                    error(
                        expr.span,
                        format!("unbound local `{name}`; functions require a call"),
                    )
                })?;
                let mut value = Typed {
                    span: expr.span,
                    ty: env[index].1.clone(),
                    kind: TypedKind::Local(index),
                };
                for field in path {
                    value = self.field(value, field, expr.span)?;
                }
                (value.ty, value.kind)
            }
            ExprKind::Field(value, field) => {
                let value = self.term(value, None, env, depth + 1)?;
                let value = self.field(value, field, expr.span)?;
                (value.ty, value.kind)
            }
            ExprKind::Record(name, fields) => {
                let name = resolve_name(name, self.module, self.scope, self.records)
                    .ok_or_else(|| error(expr.span, format!("unknown record `{name}`")))?;
                let definition = &self.records[&name];
                if fields.len() != definition.len() {
                    return Err(error(
                        expr.span,
                        "record requires exactly its declared fields",
                    ));
                }
                let mut typed = vec![];
                // Preserve source evaluation order; names bind fields independently of order.
                for (name, value) in fields {
                    let field = definition
                        .iter()
                        .find(|f| &f.name == name)
                        .ok_or_else(|| error(value.span, format!("unknown field `{name}`")))?;
                    typed.push((
                        name.clone(),
                        self.term(value, Some(&field.ty), env, depth + 1)?,
                    ));
                }
                (Type::Record(name.clone()), TypedKind::Record(name, typed))
            }
            ExprKind::List(items) => {
                let mut inner = match expected {
                    Some(Type::List(t)) => Some(*t.clone()),
                    _ => None,
                };
                let mut typed = vec![];
                for item in items {
                    let item = self.term(item, inner.as_ref(), env, depth + 1)?;
                    inner = Some(item.ty.clone());
                    typed.push(item);
                }
                let inner = inner.ok_or_else(|| {
                    error(
                        expr.span,
                        "empty list needs an expected List<T>; annotate the binding",
                    )
                })?;
                (Type::List(Box::new(inner)), TypedKind::List(typed))
            }
            ExprKind::Fold(items, initial, acc, item, body) => {
                let items = self.term(items, None, env, depth + 1)?;
                let Type::List(inner) = &items.ty else {
                    return Err(error(items.span, "fold requires a List<T>"));
                };
                let initial = self.term(initial, expected, env, depth + 1)?;
                env.push((acc.clone(), initial.ty.clone()));
                env.push((item.clone(), *inner.clone()));
                let body = self.term(body, Some(&initial.ty), env, depth + 1)?;
                env.pop();
                env.pop();
                (
                    initial.ty.clone(),
                    TypedKind::Fold(Box::new(items), Box::new(initial), Box::new(body)),
                )
            }
            ExprKind::Call(name, args)
                if name == "job_start_lean_check" || name == "job_start_lean_project" =>
            {
                if args.len() != 2 {
                    return Err(error(
                        expr.span,
                        "Lean check requires statement/registered project alias and proof Text arguments",
                    ));
                }
                let statement = self.term(&args[0], Some(&Type::Text), env, depth + 1)?;
                let proof = self.term(&args[1], Some(&Type::Text), env, depth + 1)?;
                self.jobs = true;
                self.async_jobs = true;
                let project = name == "job_start_lean_project";
                self.lean_jobs |= !project;
                self.project_jobs |= project;
                (
                    Type::Job(Box::new(Type::Text)),
                    if project {
                        TypedKind::JobLeanProject(Box::new(statement), Box::new(proof))
                    } else {
                        TypedKind::JobLeanCheck(Box::new(statement), Box::new(proof))
                    },
                )
            }
            ExprKind::Call(name, args)
                if name == "job_start_square" || name == "job_start_lean" =>
            {
                if args.len() != 1 {
                    return Err(error(expr.span, "job start requires one argument"));
                }
                let lean = name == "job_start_lean";
                let ty = if lean { Type::Text } else { Type::Int };
                let input = self.term(&args[0], Some(&ty), env, depth + 1)?;
                if lean
                    && !matches!(&input.kind, TypedKind::Literal(Value::Text(s)) if ["wrong_term", "existing_lemma", "induction", "admitted"].contains(&s.as_str()))
                {
                    return Err(error(
                        input.span,
                        "Lean strategy must be a supported literal: wrong_term, existing_lemma, induction, admitted",
                    ));
                }
                self.jobs = true;
                self.async_jobs = true;
                self.lean_jobs |= lean;
                (
                    Type::Job(Box::new(ty)),
                    TypedKind::JobStart(Box::new(input), lean),
                )
            }
            ExprKind::Call(name, args)
                if ["job_poll", "job_cancel", "job_collect"].contains(&name.as_str()) =>
            {
                if args.len() != 1 {
                    return Err(error(expr.span, "job control requires one named handle"));
                }
                let handle = self.term(&args[0], None, env, depth + 1)?;
                let (Type::Job(inner), TypedKind::Local(index)) = (&handle.ty, &handle.kind) else {
                    return Err(error(
                        expr.span,
                        "job control requires a scoped Job handle by name",
                    ));
                };
                let op = match name.as_str() {
                    "job_poll" => JobOperation::Poll,
                    "job_cancel" => JobOperation::Cancel,
                    _ => JobOperation::Collect,
                };
                self.jobs = true;
                self.async_jobs = true;
                let result = if op == JobOperation::Collect {
                    Type::Outcome(inner.clone())
                } else {
                    Type::Bool
                };
                (result, TypedKind::JobControl(op, *index))
            }
            ExprKind::Call(name, args) if name == "job_square" => {
                if args.len() != 1 {
                    return Err(error(expr.span, "job_square requires one Int argument"));
                }
                let input = self.term(&args[0], Some(&Type::Int), env, depth + 1)?;
                self.jobs = true;
                (
                    Type::Outcome(Box::new(Type::Int)),
                    TypedKind::JobSquare(Box::new(input)),
                )
            }
            ExprKind::Call(name, args) if name == "length" || name == "get" => {
                let arity = if name == "length" { 1 } else { 2 };
                if args.len() != arity {
                    return Err(error(
                        expr.span,
                        format!("{name} requires {arity} arguments"),
                    ));
                }
                let items = self.term(&args[0], None, env, depth + 1)?;
                let Type::List(inner) = &items.ty else {
                    return Err(error(items.span, format!("{name} requires a List<T>")));
                };
                if name == "length" {
                    (Type::Int, TypedKind::Length(Box::new(items)))
                } else {
                    let ty = Type::Outcome(inner.clone());
                    let index = self.term(&args[1], Some(&Type::Int), env, depth + 1)?;
                    (ty, TypedKind::Get(Box::new(items), Box::new(index)))
                }
            }
            ExprKind::Call(name, args) if name == "Ok" || name == "Err" => {
                if args.len() != 1 {
                    return Err(error(expr.span, "Ok/Err requires one argument"));
                }
                let inner = match expected {
                    Some(Type::Outcome(t)) => Some(t.as_ref()),
                    _ => None,
                };
                if name == "Ok" {
                    let value = self.term(&args[0], inner, env, depth + 1)?;
                    (
                        Type::Outcome(Box::new(value.ty.clone())),
                        TypedKind::Ok(Box::new(value)),
                    )
                } else {
                    let inner = inner.ok_or_else(|| {
                        error(
                            expr.span,
                            "Err needs an expected Outcome<T>; annotate the let binding",
                        )
                    })?;
                    let value = self.term(&args[0], Some(&Type::Text), env, depth + 1)?;
                    (
                        Type::Outcome(Box::new(inner.clone())),
                        TypedKind::Err(Box::new(value)),
                    )
                }
            }
            ExprKind::Call(name, args) => {
                let index = resolve_name(name, self.module, self.scope, self.names)
                    .map(|name| self.names[&name])
                    .ok_or_else(|| error(expr.span, format!("unknown function `{name}`")))?;
                let signature = &self.raw[index];
                if args.len() != signature.parameters.len() {
                    return Err(error(
                        expr.span,
                        format!(
                            "`{name}` expects {} arguments, received {}",
                            signature.parameters.len(),
                            args.len()
                        ),
                    ));
                }
                let parameters = signature.parameters.clone();
                let result = signature.result.clone();
                self.calls.insert(index);
                let typed = args
                    .iter()
                    .zip(parameters)
                    .map(|(arg, p)| self.term(arg, Some(&p.ty), env, depth + 1))
                    .collect::<Result<_, _>>()?;
                (result, TypedKind::Call(index, typed))
            }
            ExprKind::Let(name, annotation, value, body) => {
                let annotation = annotation
                    .as_ref()
                    .map(|t| resolve_type(t, self.module, self.scope, self.records, expr.span))
                    .transpose()?;
                if let Some(t) = &annotation {
                    type_height(
                        t,
                        self.records,
                        &mut BTreeMap::new(),
                        &mut BTreeSet::new(),
                        expr.span,
                    )?;
                }
                let value = self.term(value, annotation.as_ref(), env, depth + 1)?;
                env.push((name.clone(), value.ty.clone()));
                let body = self.term(body, expected, env, depth + 1)?;
                env.pop();
                (
                    body.ty.clone(),
                    TypedKind::Let(Box::new(value), Box::new(body)),
                )
            }
            ExprKind::If(condition, yes, no) => {
                let condition = self.term(condition, Some(&Type::Bool), env, depth + 1)?;
                let yes = self.term(yes, expected, env, depth + 1)?;
                let no = self.term(no, Some(&yes.ty), env, depth + 1)?;
                (
                    yes.ty.clone(),
                    TypedKind::If(Box::new(condition), Box::new(yes), Box::new(no)),
                )
            }
            ExprKind::Match(value, oname, ok, ename, err) => {
                let value = self.term(value, None, env, depth + 1)?;
                let Type::Outcome(inner) = &value.ty else {
                    return Err(error(
                        value.span,
                        "match scrutinee must have type Outcome<T>",
                    ));
                };
                env.push((oname.clone(), *inner.clone()));
                let ok = self.term(ok, expected, env, depth + 1)?;
                env.pop();
                env.push((ename.clone(), Type::Text));
                let err = self.term(err, Some(&ok.ty), env, depth + 1)?;
                env.pop();
                (
                    ok.ty.clone(),
                    TypedKind::Match(Box::new(value), Box::new(ok), Box::new(err)),
                )
            }
            ExprKind::Unary(op, value) => {
                let ty = if op == "not" { Type::Bool } else { Type::Int };
                let value = self.term(value, Some(&ty), env, depth + 1)?;
                (ty, TypedKind::Unary(op.clone(), Box::new(value)))
            }
            ExprKind::Binary(op, left, right) => {
                let ty = match op.as_str() {
                    "and" | "or" => Some(Type::Bool),
                    "==" | "!=" => None,
                    _ => Some(Type::Int),
                };
                let left = self.term(left, ty.as_ref(), env, depth + 1)?;
                if !matches!(left.ty, Type::Bool | Type::Int | Type::Text) {
                    return Err(error(expr.span, "operators require scalar operands"));
                }
                let right = self.term(right, Some(&left.ty), env, depth + 1)?;
                let result = if ["+", "-", "*"].contains(&op.as_str()) {
                    Type::Int
                } else {
                    Type::Bool
                };
                (
                    result,
                    TypedKind::Binary(op.clone(), Box::new(left), Box::new(right)),
                )
            }
        };
        type_height(
            &ty,
            self.records,
            &mut self.heights,
            &mut BTreeSet::new(),
            expr.span,
        )?;
        if expected.is_some_and(|e| *e != ty) {
            let mut diagnostic = error(
                expr.span,
                format!("expected {:?}, received {ty:?}", expected.unwrap()),
            );
            diagnostic.expected = expected.cloned().map(Box::new);
            diagnostic.actual = Some(Box::new(ty));
            return Err(diagnostic);
        }
        Ok(Typed {
            span: expr.span,
            ty,
            kind,
        })
    }
    fn field(&self, value: Typed, field: &str, span: Location) -> Result<Typed, Diagnostic> {
        let Type::Record(name) = &value.ty else {
            return Err(error(span, "field access requires a record"));
        };
        let ty = self.records[name]
            .iter()
            .find(|f| f.name == field)
            .ok_or_else(|| error(span, format!("unknown field `{field}` on `{name}`")))?
            .ty
            .clone();
        Ok(Typed {
            span,
            ty,
            kind: TypedKind::Field(Box::new(value), field.into()),
        })
    }
}

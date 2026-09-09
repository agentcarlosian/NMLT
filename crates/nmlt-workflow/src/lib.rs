//! Executable-only workflow functions with an explicit host effect boundary.
#![forbid(unsafe_code)]

use nmlt_core::Span;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod affine;
mod check;
mod eval;
mod package;
mod parse;
mod value;
pub use check::compile;
pub use eval::{
    Execution, JobError, JobHost, JobOperation, JobRequest, Stop, execute, execute_with_host,
    validate_inputs,
};
pub use package::{PackageError, compile_package};

pub const MAX_SOURCE_BYTES: usize = 131_072;
pub(crate) const MAX_FUNCTIONS: usize = 64;
pub(crate) const MAX_DEPTH: usize = 48;
pub const MAX_LIST_ITEMS: usize = 256;
pub const MAX_VALUE_NODES: usize = 4096;
pub const MAX_VALUE_BYTES: usize = 65_536;
pub const MAX_VALUE_WORK: usize = 1_000_000;
pub const MAX_PACKAGE_FILES: usize = 32;
pub const MAX_PACKAGE_BYTES: usize = 1_048_576;

/// Workflow diagnostics retain a source index in addition to the core span.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub source: usize,
    pub diagnostic: nmlt_core::Diagnostic,
    pub expected: Option<Box<Type>>,
    pub actual: Option<Box<Type>>,
    pub related: Vec<(Location, String)>,
}
impl std::ops::Deref for Diagnostic {
    type Target = nmlt_core::Diagnostic;
    fn deref(&self) -> &Self::Target {
        &self.diagnostic
    }
}
impl From<nmlt_core::Diagnostic> for Diagnostic {
    fn from(diagnostic: nmlt_core::Diagnostic) -> Self {
        Self {
            source: 0,
            diagnostic,
            expected: None,
            actual: None,
            related: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub path: String,
    pub bytes: usize,
    pub source_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "inner",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Type {
    Bool,
    Int,
    Text,
    Outcome(Box<Type>),
    List(Box<Type>),
    Record(String),
    Job(Box<Type>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Text(String),
    Ok(Box<Value>),
    Err(String),
    List(Vec<Value>),
    Record {
        name: String,
        fields: BTreeMap<String, Value>,
    },
}

impl Value {
    pub fn conforms(&self, ty: &Type) -> bool {
        value::conforms(self, ty, &BTreeMap::new())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub source: usize,
    pub start: usize,
    pub end: usize,
}
impl From<Span> for Location {
    fn from(span: Span) -> Self {
        Self {
            source: 0,
            start: span.start,
            end: span.end,
        }
    }
}
impl From<Location> for Span {
    fn from(span: Location) -> Self {
        Self::new(span.start, span.end)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

/// Only compilation constructs this value. No unchecked deserialization path.
#[derive(Clone, Debug, Serialize)]
pub struct Program {
    functions: Vec<Function>,
    records: BTreeMap<String, Vec<Parameter>>,
    sources: Vec<SourceIdentity>,
}
impl Program {
    pub fn can_run_as_entry(&self, entry: &str) -> bool {
        self.functions
            .iter()
            .find(|f| f.name == entry)
            .is_some_and(|f| {
                !matches!(f.result, Type::Job(_))
                    && f.parameters.iter().all(|p| !matches!(p.ty, Type::Job(_)))
            })
    }
    pub fn requires_async_jobs(&self, entry: &str) -> Result<bool, String> {
        self.functions
            .iter()
            .find(|f| f.name == entry)
            .map(|f| f.async_jobs)
            .ok_or_else(|| format!("unknown workflow entry `{entry}`"))
    }
    pub fn requires_lean_jobs(&self, entry: &str) -> Result<bool, String> {
        self.functions
            .iter()
            .find(|f| f.name == entry)
            .map(|f| f.lean_jobs)
            .ok_or_else(|| format!("unknown workflow entry `{entry}`"))
    }
    /// Conservative, transitive effect summary, including unselected branches.
    pub fn requires_jobs(&self, entry: &str) -> Result<bool, String> {
        self.functions
            .iter()
            .find(|f| f.name == entry)
            .map(|f| f.jobs)
            .ok_or_else(|| format!("unknown workflow entry `{entry}`"))
    }
    pub fn sources(&self) -> &[SourceIdentity] {
        &self.sources
    }
    pub fn source_path(&self, location: Location) -> Option<&str> {
        self.sources.get(location.source).map(|s| s.path.as_str())
    }
    pub fn records(&self) -> &BTreeMap<String, Vec<Parameter>> {
        &self.records
    }
    pub fn conforms(&self, value: &Value, ty: &Type) -> bool {
        value::conforms(value, ty, &self.records)
    }
    /// Decode ordinary JSON according to an entry signature; record names are
    /// supplied by the compiled type, never accepted from an untyped tag.
    pub fn input_value(&self, ty: &Type, json: &serde_json::Value) -> Result<Value, String> {
        value::decode(self, ty, json)
    }
    pub fn entries(&self) -> impl Iterator<Item = (&str, &[Parameter], &Type)> {
        self.functions
            .iter()
            .map(|f| (f.name.as_str(), f.parameters.as_slice(), &f.result))
    }
    pub fn identity(&self) -> String {
        digest(&serde_json::to_vec(self).expect("typed program serialization"))
    }
}

pub fn digest(bytes: &[u8]) -> String {
    nmlt_hir::sha256_bytes(bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}

fn error(span: Location, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        source: span.source,
        diagnostic: nmlt_core::Diagnostic::error("NMLT-WORKFLOW", message, Some(span.into())),
        expected: None,
        actual: None,
        related: vec![],
    }
}

#[derive(Clone, Debug, Serialize)]
struct Function {
    name: String,
    parameters: Vec<Parameter>,
    result: Type,
    body: Typed,
    jobs: bool,
    async_jobs: bool,
    lean_jobs: bool,
}
#[derive(Clone, Debug, Serialize)]
struct Typed {
    span: Location,
    ty: Type,
    kind: TypedKind,
}
#[derive(Clone, Debug, Serialize)]
enum TypedKind {
    JobStart(Box<Typed>, bool),
    JobLeanCheck(Box<Typed>, Box<Typed>),
    JobControl(JobOperation, usize),
    JobSquare(Box<Typed>),
    Literal(Value),
    Local(usize),
    Call(usize, Vec<Typed>),
    Let(Box<Typed>, Box<Typed>),
    If(Box<Typed>, Box<Typed>, Box<Typed>),
    Match(Box<Typed>, Box<Typed>, Box<Typed>),
    Ok(Box<Typed>),
    Err(Box<Typed>),
    Unary(String, Box<Typed>),
    Binary(String, Box<Typed>, Box<Typed>),
    Record(String, Vec<(String, Typed)>),
    Field(Box<Typed>, String),
    List(Vec<Typed>),
    Length(Box<Typed>),
    Get(Box<Typed>, Box<Typed>),
    Fold(Box<Typed>, Box<Typed>, Box<Typed>),
}

#[derive(Clone, Debug)]
struct RawRecord {
    name: String,
    module: String,
    span: Location,
    fields: Vec<Parameter>,
}
struct RawSource {
    functions: Vec<RawFunction>,
    records: Vec<RawRecord>,
    imports: Vec<(String, Location)>,
    modules: Vec<(String, Location)>,
}
struct Scope {
    root: String,
    imports: std::collections::BTreeSet<String>,
    libraries: std::collections::BTreeSet<String>,
}
#[derive(Clone, Debug)]
struct RawFunction {
    name: String,
    module: String,
    span: Location,
    parameters: Vec<Parameter>,
    result: Type,
    body: Expr,
}
#[derive(Clone, Debug)]
struct Expr {
    span: Location,
    kind: ExprKind,
    depth: usize,
}
#[derive(Clone, Debug)]
enum ExprKind {
    Literal(Value),
    Name(String),
    Call(String, Vec<Expr>),
    Let(String, Option<Type>, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Match(Box<Expr>, String, Box<Expr>, String, Box<Expr>),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Record(String, Vec<(String, Expr)>),
    Field(Box<Expr>, String),
    List(Vec<Expr>),
    Fold(Box<Expr>, Box<Expr>, String, String, Box<Expr>),
}

pub type Inputs = BTreeMap<String, Value>;

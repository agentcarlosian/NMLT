//! Typed behavioral core and deterministic finite-state engine.
//!
//! The implemented fragment is intentionally narrower than the complete NMLT
//! surface language. Unsupported constructs are rejected rather than assigned
//! an approximate meaning.

pub mod ast;
pub mod model;
pub mod parser;
pub mod types;

pub use ast::{
    Action, BinaryOp, Expr, Model, Property, PropertyKind, StateVar, Type, UnaryOp, Value,
};
pub use model::{
    CheckConfig, CheckReport, PropertyResult, ResultClass, Trace, TraceStep, check_model,
};
pub use parser::parse_model;
pub use types::{TypedModel, type_check};

/// Parse and type-check the executable NMLT fragment.
pub fn compile(source: &str) -> Result<TypedModel, Vec<String>> {
    let model = parse_model(source)?;
    type_check(model)
}

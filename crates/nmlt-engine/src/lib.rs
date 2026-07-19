//! Typed behavioral core and deterministic finite-state engine.
//!
//! The implemented fragment is intentionally narrower than the complete NMLT
//! surface language. Unsupported constructs are rejected rather than assigned
//! an approximate meaning.

pub mod ast;
mod checked;
pub mod model;
pub mod types;

pub use ast::{
    Action, BinaryOp, Expr, Model, Property, PropertyKind, StateVar, Type, UnaryOp, Value,
};
pub use checked::from_checked;
pub use model::{
    CheckConfig, CheckReport, PropertyResult, ResultClass, Trace, TraceStep, check_model,
};
pub use types::{SemanticBinding, TypedModel};

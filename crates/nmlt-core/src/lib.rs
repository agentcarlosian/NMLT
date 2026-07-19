//! Shared foundations for the pre-alpha NMLT frontend.
//!
//! This crate currently provides lossless lexing, structural source scanning,
//! and evidence scaffolding. It does not implement NMLT's proposed type system
//! or semantics.

pub mod diagnostic;
pub mod evidence;
pub mod lexer;
pub mod syntax;

pub use diagnostic::{Diagnostic, Severity, Span};
pub use evidence::{EvidenceManifest, EvidenceResult};
pub use lexer::{LexedFile, Token, TokenKind, lex_source};
pub use syntax::{ParsedFile, SystemDecl, parse_source};

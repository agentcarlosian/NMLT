//! Shared foundations for the pre-alpha NMLT frontend.
//!
//! This crate currently provides lossless lexing, an immutable concrete syntax
//! tree, recovery-capable syntactic parsing, conservative formatting, and
//! evidence scaffolding. It does not implement NMLT's proposed type system,
//! transition semantics, model checking, or proof checking.

pub mod cst;
pub mod diagnostic;
pub mod evidence;
pub mod formatter;
pub mod lexer;
pub mod syntax;
pub mod untyped;

pub use cst::{
    GreenElement, GreenNode, GreenToken, SpannedGreenNode, SpannedGreenToken, SyntaxKind,
};
pub use diagnostic::{Diagnostic, Severity, Span, render_diagnostic_snapshot};
pub use evidence::{EvidenceManifest, EvidenceResult};
pub use formatter::{FormatMode, FormatOutput, format_cst, format_source};
pub use lexer::{LexedFile, Token, TokenKind, lex_source};
pub use syntax::{ParsedFile, SyntaxParse, SystemDecl, parse_cst, parse_source};
pub use untyped::{
    BindingKind, ObservationKind, ProjectionIssue, ProjectionIssueKind, PropertyKind, RawTerm,
    SpannedText, UntypedAction, UntypedBinding, UntypedErrorNode, UntypedFile, UntypedMember,
    UntypedObservation, UntypedParameter, UntypedPort, UntypedProjection, UntypedProperty,
    UntypedStatement, UntypedSurfaceNode, UntypedSystem, UntypedUpdateTarget, project_untyped,
};

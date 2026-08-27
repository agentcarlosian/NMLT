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
    BindingKind, HideSort, M9SurfaceIssue, ObservationKind, ProjectionCoverage, ProjectionIssue,
    ProjectionIssueKind, PropertyKind, RawTerm, SpannedText, SurfaceOrigin, SurfacePolarity,
    SurfaceWire, UntypedAction, UntypedBinding, UntypedCompose, UntypedComposeItem, UntypedConnect,
    UntypedDeclaration, UntypedEnum, UntypedEnumItem, UntypedEnumVariant, UntypedErrorNode,
    UntypedFile, UntypedImport, UntypedMember, UntypedModule, UntypedObservation, UntypedParameter,
    UntypedParameterItem, UntypedPort, UntypedProjection, UntypedProperty, UntypedStatement,
    UntypedSurfaceNode, UntypedSystem, UntypedUpdateTarget, hidden_action_names,
    hidden_wired_actions, non_complementary_port_wires, non_complementary_surface_wires,
    port_polarity, project_untyped, surface_connections, surface_endpoint_wires,
    surface_endpoint_wires_for_left, surface_port_polarities, surface_wired_action_pairs,
    surface_wired_action_pairs_for_left, surface_wires, surface_wires_in_compose,
};

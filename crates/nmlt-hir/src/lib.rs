//! Experimental deterministic resolution boundary for NMLT.
//!
//! This crate implements the first M9-003 resolver boundary: closed acyclic
//! imports, typed definition namespaces, stable content-derived IDs, and
//! direct-import name lookup. [`project_source_module`] is the canonical
//! repository source adapter: it invokes `nmlt-core`'s lossless parser and
//! complete hierarchical projection rather than implementing another parser.
//!
//! The public boundary seals projected imports and definitions to exact bytes
//! and binds them in [`ResolutionId`], but it does not independently read back
//! the trusted parser/projector. The filesystem loader also remains responsible
//! for resolving symlinks before passing portable repository-relative paths.
//! Raw-term reference origins and local binders do not yet have RFC 0013's
//! complete `ResolutionMap`. A [`ResolvedProgram`] is not a typed core program,
//! a proof certificate, or semantic acceptance.

#![forbid(unsafe_code)]

mod hir;
mod identity;
mod model;
mod resolve_terms;
mod resolver;
mod surface;
mod term;

pub use hir::{
    HirBinaryOp, HirNode, HirNodeKind, HirRoot, HirUnaryOp, LocalBinder, ResolutionEntry,
    ResolutionMap, ResolvedRef, SemanticPath, SemanticPathSegment,
};
pub use identity::{
    DefId, LocalId, ModuleId, ModuleMapId, NodeId, ResolutionId, SourceId, SourceSetEntry,
    SourceSetId, SourceSetIdentityError, sha256_bytes,
};
pub(crate) use model::{DeclarationInput, ImportInput, ModuleInput};
pub use model::{
    DeclarationKey, DefPath, DefPathSegment, NameReference, Namespace, ProjectedModule,
    ProjectionIssue, ProjectionIssueKind, ResolvedDeclaration, ResolvedImport, ResolvedModule,
    ResolvedProgram, SemanticRole, SourceSpan,
};
pub use resolve_terms::verify_resolution_readback;
pub use resolver::{
    DefPathViolation, DefinitionCandidate, LookupError, PathViolation, ResolveError,
    ResourceDimension, SourceEncodingViolation, resolve_modules,
};
pub use surface::project_source_module;

#[cfg(test)]
mod resolution_tests;

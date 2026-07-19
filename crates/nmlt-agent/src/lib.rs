//! Authority-bounded agentic repair primitives for NMLT.
//!
//! An assistant implemented with this crate proposes edits. It never decides
//! whether a claim is true and cannot write trusted artifacts or checker
//! results. See RFC 0011.

pub mod artifact;
pub mod assistant;
pub mod authority;
pub mod digest;
pub mod evaluation;
pub mod feedback;
pub mod graph;
pub mod mutation;

pub use artifact::{ArtifactRole, ArtifactSet, TrustedArtifact};
pub use assistant::{AssistantInput, DeterministicAssistant, RepairAssistant};
pub use authority::{
    AppliedCandidates, AuthorityError, ByteSpan, CandidateFile, Edit, EditPolicy, Proposal,
    apply_proposal, validate_proposal,
};
pub use evaluation::{EvaluationMetrics, EvaluationReport, evaluate_held_out_suite};
pub use feedback::{
    CheckResult, CounterexampleStep, Feedback, ParseDiagnostic, ResultClass, TypeDiagnostic,
};
pub use graph::{ArtifactGraph, ArtifactNode, Edge};
pub use mutation::{MutationDescriptor, MutationKind};

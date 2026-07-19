//! Bidirectional elaboration from resolved HIR into NMLT's explicit typed core.
//!
//! Successful elaboration returns a structurally validated `CoreProgram` and
//! a canonical, rule-explicit derivation DAG. The artifact is inspectable but
//! is not kernel acceptance and cannot construct `CheckedProgram`.

#![forbid(unsafe_code)]

mod elaborate;
mod identity;
mod model;

pub use elaborate::{ElaborationError, elaborate};
pub use identity::{DerivationNodeId, ElaborationCertificateId, ResourcePolicyId, RulesetBundleId};
pub use model::{
    DerivationConclusion, DerivationNode, DerivationWitness, ElaborationArtifact, ElaborationRule,
    JudgmentKind, ObligationKey,
};

//! Neutral M9 elaboration-certificate data and producer-side identity encoding.
//!
//! These structures are untrusted evidence syntax. They grant no checked
//! status; `nmlt-kernel` independently reconstructs their identities and
//! judgments before it can create `CheckedProgram`.

#![forbid(unsafe_code)]

mod identity;
mod model;

pub use identity::{
    DerivationNodeId, ElaborationCertificateId, ResourcePolicyId, RulesetBundleId, certificate_id,
    make_derivation_node, resource_policy_id, ruleset_bundle_id,
};
pub use model::{
    DerivationConclusion, DerivationNode, DerivationWitness, ElaborationArtifact, ElaborationRule,
    JudgmentKind, ObligationKey,
};

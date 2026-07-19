//! Finite verification conditions, independent engines, checked certificates, and
//! fail-closed evidence composition for NMLT.
//!
//! The crate intentionally supports one small domain: finite Boolean transition
//! systems with state invariants. That restriction lets the implementation make
//! the boundary between exhaustive evidence, bounded evidence, and proof evidence
//! exact rather than aspirational.

#![forbid(unsafe_code)]

pub mod certificate;
pub mod evidence;
pub mod identity;
pub mod inductive;
pub mod ir;
pub mod proof;
pub mod reachability;
pub mod smt;
pub mod test_hook;

pub use certificate::{
    CertificateError, FiniteInvariantCertificate, check_finite_invariant_certificate,
};
pub use evidence::{
    BackendIdentity, CompositeEvidence, EngineBinding, NormalizedClass, NormalizedResult,
    RawEngineResult, RawStatus, ResultScope, TrustedComponent, compose_evidence, normalize_result,
};
pub use identity::{IdentityError, Sha256Id};
pub use inductive::InductiveEngine;
pub use ir::{
    BoolExpr, FiniteSafetyVc, IrError, StateRef, VerificationConfig, VerificationIdentity,
};
pub use proof::{
    PROOF_ASSISTANT_PROTOCOL, ProofAssistant, ProofAssistantExport, ProofAssistantReturn,
    export_lean4_inductiveness, normalize_proof_assistant_return,
};
pub use reachability::{ReachabilityEngine, ReachabilityLimits};
pub use smt::{
    SMT_LIB_VERSION, SmtBackendReturn, SmtProtocolError, SmtStatus, encode_inductiveness_query,
    normalize_smt_return, parse_smt_status,
};
pub use test_hook::{ModelTestPlan, ModelTestReport, run_model_based_tests};

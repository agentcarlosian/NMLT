//! Executable, finite-state candidates for NMLT temporal and refinement semantics.
//!
//! This crate deliberately makes a narrow claim: its algorithms decide properties of
//! the finite graphs supplied to them. They are not a proof of the source language,
//! elaborator, or an unbounded implementation.

#![forbid(unsafe_code)]

pub mod certificate;
pub mod graph;
pub mod observation;
pub mod open;
pub mod open_congruence;
pub mod open_contract;
pub mod open_encoding;
pub mod open_refinement;
pub mod open_resources;
pub mod refinement;
pub mod runtime;
pub mod temporal;

pub use certificate::{
    CertificateIssue, CertificateReport, CoinductiveCertificate, CoinductiveCertificateChecker,
    StatePair,
};
pub use graph::{
    FiniteGraph, GraphError, ModelState, StateId, Transition, TransitionId, TransitionKind, Value,
};
pub use observation::{
    ActionHiding, ActionProjectionError, ObservationError, ObservationMap, stutter_equivalent,
    stutter_project,
};
pub use open::{
    ActionPolarity, ActionSignature, CompatibilityChecker, CompatibilityIssue, CompatibilityReport,
    CompositionError, CompositionLimits, CompositionSpec, CongruenceIssue, CongruenceReport,
    CongruenceSpec, Connection as OpenConnection, ContractLink, DEFAULT_MAX_COMPOSED_STATES,
    DEFAULT_MAX_COMPOSED_TRANSITIONS, DEFAULT_MAX_COMPOSITION_WORK_ITEMS,
    Interface as OpenInterface, InterfaceBuildError, OpenRefinementCongruenceChecker, OpenSystem,
    OpenSystemIssue, Side, compose as compose_open_systems,
    compose_with_limits as compose_open_systems_with_limits,
};
pub use open_congruence::{
    FiniteStateInvariant, InvariantTransportReport, TwoSidedCongruenceChecker,
    TwoSidedCongruenceIssue, TwoSidedCongruenceReport, TwoSidedCongruenceSpec,
    TwoSidedResourceSpec,
};
pub use open_contract::{
    FiniteContract, FiniteContractError, PayloadPredicate, PayloadPredicateError,
    PayloadPredicateId, PayloadType, PayloadTypeError, PayloadTypeId,
};
pub use open_encoding::{
    CanonicalActionEncoding, CanonicalCongruenceEncoding, CanonicalConnectionEncoding,
    CanonicalEncodingValidator, CanonicalGradeEncoding, CanonicalRefinementEncoding,
    CanonicalResourceEncoding, CanonicalSide, CanonicalSystemEncoding, CanonicalValidationIssue,
    CanonicalValidationReport, EncodingCorrespondenceChecker, EncodingCorrespondenceIssue,
    EncodingCorrespondenceReport,
};
pub use open_refinement::{
    OpenRefinementChecker, OpenRefinementIssue, OpenRefinementReport, RefinementCompositionError,
    compose_refinement_specs, identity_refinement_spec,
};
pub use open_resources::{
    ActionResourceProfile, ResourceCompositionChecker, ResourceCompositionIssue,
    ResourceCompositionReport, ResourceProfileError, ResourceRefinementChecker,
    ResourceRefinementIssue, ResourceRefinementReport, SystemResourceProfile,
    mapped_product_resource_refinement,
};
pub use refinement::{
    RefinementChecker, RefinementMismatch, RefinementMismatchKind, RefinementReport, RefinementSpec,
};
pub use runtime::{
    JournalAction, JournalRecord, JournalValue, RuntimeIssue, RuntimeIssueKind, RuntimeMapping,
    RuntimeReport, RuntimeTraceAdapter, RuntimeUncertainty, RuntimeUncertaintyKind, RuntimeVerdict,
};
pub use temporal::{CheckOutcome, Fairness, FairnessKind, FairnessSet, Lasso, TemporalChecker};

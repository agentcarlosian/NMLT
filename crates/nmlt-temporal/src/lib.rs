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
    OpenSystemIssue, SafetyContract, Side, compose as compose_open_systems,
    compose_with_limits as compose_open_systems_with_limits,
};
pub use refinement::{
    RefinementChecker, RefinementMismatch, RefinementMismatchKind, RefinementReport, RefinementSpec,
};
pub use runtime::{
    JournalAction, JournalRecord, JournalValue, RuntimeIssue, RuntimeIssueKind, RuntimeMapping,
    RuntimeReport, RuntimeTraceAdapter, RuntimeUncertainty, RuntimeUncertaintyKind, RuntimeVerdict,
};
pub use temporal::{CheckOutcome, Fairness, FairnessKind, FairnessSet, Lasso, TemporalChecker};

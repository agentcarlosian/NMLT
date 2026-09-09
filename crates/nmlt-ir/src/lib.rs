//! Explicit typed core representation for the first NMLT M9 slice.
//!
//! `CoreProgram::new` validates graph closure, structural typing, system
//! indexing, action frames, local scope, canonical integers, and resource
//! ceilings before assigning `CoreProgramId`. This is still an inspectable
//! artifact, not an elaboration certificate or a kernel-accepted
//! `CheckedProgram`.

#![forbid(unsafe_code)]

mod behavior;
mod execution;
mod identity;
mod invariant;
mod model;
mod validate;

pub use behavior::{
    BEHAVIOR_CORE_SCHEMA, BEHAVIOR_CORE_V2_SCHEMA, BehaviorCoreProgram, CoreBehaviorAction,
    CoreBehaviorBinding, CoreBehaviorState, CoreBehaviorSystem, CoreBehaviorTerm, CoreComposition,
    CoreConnection, CorePort, CorePortDirection, CoreRefinement, CoreResourceProfile,
};
pub use execution::{ExecutionAction, ExecutionPath, ExecutionState};
pub use invariant::{
    SafetyArtifact, SafetyClaim, SafetyPredicate, SafetyProgram, SafetyProperty, SafetyWitness,
};

pub use identity::{CoreIdentityError, CoreNodeId, CoreProgramId};
pub use model::{
    CoreAction, CoreActionParameter, CoreBinaryOp, CoreCapability, CoreEnum, CoreModule,
    CoreObservation, CoreProgram, CoreProperty, CorePropertyKind, CoreStateField, CoreSystem,
    CoreTerm, CoreTermKind, CoreType, CoreUnaryOp,
};
pub use validate::{CoreResourceDimension, CoreValidationError};

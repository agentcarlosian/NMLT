//! Exact finite safety predicates and untrusted closure/path witnesses.
use crate::{BehaviorCoreProgram, CoreBehaviorTerm, ExecutionPath, ExecutionState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SafetyPredicate {
    Boolean { term: CoreBehaviorTerm },
    Not { value: Box<Self> },
    And { left: Box<Self>, right: Box<Self> },
    Or { left: Box<Self>, right: Box<Self> },
    Implies { left: Box<Self>, right: Box<Self> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyProperty {
    pub system: String,
    pub name: String,
    pub expression: String,
    pub expression_start: usize,
    pub expression_end: usize,
    pub declaration_start: usize,
    pub declaration_end: usize,
    pub predicate: SafetyPredicate,
}

#[derive(Clone, Debug)]
pub struct SafetyProgram {
    pub core: BehaviorCoreProgram,
    pub properties: Vec<SafetyProperty>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyArtifact {
    pub schema: String,
    pub core_sha256: String,
    pub source_sha256: String,
    pub behavior: String,
    pub property: SafetyProperty,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SafetyClaim {
    Invariant { states: Vec<ExecutionState> },
    Counterexample { path: ExecutionPath },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyWitness {
    pub schema: String,
    pub core_sha256: String,
    pub invariant_sha256: String,
    pub claim: SafetyClaim,
}

//! Bounded execution of the finite v2 slice. This is not a host-effect runtime
//! or a Lean checker. Scheduling is executable-only policy over shared steps.

use std::collections::BTreeMap;

use nmlt_ir::BehaviorCoreProgram;
use serde::{Deserialize, Serialize};

use crate::{EvalError, EvalState, prepare};

/// Limit retained trace size in the first local execution profile.
pub const MAX_RUN_STEPS: usize = 10_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case", deny_unknown_fields)]
pub enum Schedule {
    /// Lexicographically first enabled label; makes no fairness claim.
    FirstEnabled,
    /// Stop after this exact sequence, or at its first unavailable action.
    Actions { labels: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunConfig {
    pub max_steps: usize,
    pub schedule: Schedule,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RunOutcome {
    /// No enabled step; this is not a claim of successful work or liveness.
    Quiescent,
    ScheduleComplete,
    StepLimit,
    ActionUnavailable {
        requested: String,
        enabled: Vec<String>,
    },
    GradeOverflow {
        atom: String,
    },
}

impl RunOutcome {
    #[must_use]
    pub fn completed(&self) -> bool {
        matches!(self, Self::Quiescent | Self::ScheduleComplete)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunStep {
    pub label: String,
    pub after: EvalState,
    pub model_grade: BTreeMap<String, u64>,
    pub transfers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrace {
    pub initial: EvalState,
    pub steps: Vec<RunStep>,
    pub outcome: RunOutcome,
    /// Additive model annotations, not observed expenditure or reservations.
    pub model_grade: BTreeMap<String, u64>,
}

pub fn execute(
    program: &BehaviorCoreProgram,
    behavior: &str,
    config: &RunConfig,
) -> Result<RunTrace, EvalError> {
    if program.schema != nmlt_ir::BEHAVIOR_CORE_V2_SCHEMA {
        return Err(EvalError(
            "local execution requires behavior-core-v2".into(),
        ));
    }
    if !(1..=MAX_RUN_STEPS).contains(&config.max_steps) {
        return Err(EvalError(format!(
            "max_steps must be between 1 and {MAX_RUN_STEPS}"
        )));
    }
    if let Schedule::Actions { labels } = &config.schedule
        && (labels.len() > MAX_RUN_STEPS || labels.iter().any(|s| s.trim().is_empty()))
    {
        return Err(EvalError("invalid or oversized action schedule".into()));
    }
    let prepared = prepare(program, behavior)?;
    let mut current = prepared.initial.clone();
    let mut result = RunTrace {
        initial: prepared.initial,
        steps: Vec::new(),
        outcome: RunOutcome::StepLimit,
        model_grade: BTreeMap::new(),
    };
    result.outcome = loop {
        let count = result.steps.len();
        if let Schedule::Actions { labels } = &config.schedule
            && count == labels.len()
        {
            break RunOutcome::ScheduleComplete;
        }
        let mut candidates = (prepared.successors)(&current)?;
        candidates.sort_by(|a, b| a.label.cmp(&b.label));
        // Match exploration's duplicate-edge policy without constructing a graph.
        candidates.dedup_by(|a, b| {
            a.label == b.label
                && a.state == b.state
                && a.resources == b.resources
                && a.transfers == b.transfers
        });
        if matches!(config.schedule, Schedule::FirstEnabled) && candidates.is_empty() {
            break RunOutcome::Quiescent;
        }
        if count == config.max_steps {
            break RunOutcome::StepLimit;
        }
        let candidate = match &config.schedule {
            Schedule::FirstEnabled => &candidates[0],
            Schedule::Actions { labels } => {
                let Some(candidate) = candidates.iter().find(|c| c.label == labels[count]) else {
                    break RunOutcome::ActionUnavailable {
                        requested: labels[count].clone(),
                        enabled: candidates.iter().map(|c| c.label.clone()).collect(),
                    };
                };
                candidate
            }
        };
        if candidates
            .iter()
            .filter(|c| c.label == candidate.label)
            .count()
            != 1
        {
            return Err(EvalError(format!(
                "ambiguous enabled label '{}'",
                candidate.label
            )));
        }
        let mut total = result.model_grade.clone();
        let mut overflow = None;
        for (atom, amount) in &candidate.resources.grade {
            let value = total.entry(atom.clone()).or_default();
            if let Some(sum) = value.checked_add(*amount) {
                *value = sum;
            } else {
                overflow = Some(atom.clone());
                break;
            }
        }
        if let Some(atom) = overflow {
            break RunOutcome::GradeOverflow { atom };
        }
        current = candidate.state.clone();
        result.steps.push(RunStep {
            label: candidate.label.clone(),
            after: current.clone(),
            model_grade: candidate.resources.grade.clone(),
            transfers: candidate.transfers.clone(),
        });
        result.model_grade = total;
    };
    Ok(result)
}

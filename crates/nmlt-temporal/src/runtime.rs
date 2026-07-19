use crate::graph::{FiniteGraph, StateId, TransitionKind, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalValue {
    Known(Value),
    Unknown,
}

impl From<Value> for JournalValue {
    fn from(value: Value) -> Self {
        Self::Known(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalAction {
    /// Required for the first snapshot, before any transition has occurred.
    Initial,
    Action(String),
    IdentityStutter,
    /// The journal did not reveal which transition occurred.
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalRecord {
    pub sequence: u64,
    pub action: JournalAction,
    pub observations: BTreeMap<String, JournalValue>,
}

/// Maps required model-state fields to concrete journal field names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeMapping {
    pub model_to_journal: BTreeMap<String, String>,
}

impl RuntimeMapping {
    pub fn new<I, S, T>(fields: I) -> Self
    where
        I: IntoIterator<Item = (S, T)>,
        S: Into<String>,
        T: Into<String>,
    {
        Self {
            model_to_journal: fields
                .into_iter()
                .map(|(model, journal)| (model.into(), journal.into()))
                .collect(),
        }
    }

    pub fn identity<'a, I>(fields: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        Self::new(fields.into_iter().map(|field| (field, field)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeVerdict {
    /// A fully observed finite trace has at least one matching model path.
    Accepted,
    /// Known journal data contradicts every matching model path.
    Rejected,
    /// The finite trace remains compatible, but omitted/unknown data prevents acceptance.
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeIssueKind {
    EmptyJournal,
    MappingFieldMissing,
    FirstRecordIsNotInitial,
    UnexpectedInitialMarker,
    NonConsecutiveSequence,
    NoMatchingInitialState,
    NoMatchingTransition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeIssue {
    pub kind: RuntimeIssueKind,
    pub record_index: Option<usize>,
    pub sequence: Option<u64>,
    pub candidates_before: Vec<StateId>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeUncertaintyKind {
    MissingObservation,
    UnknownObservation,
    UnknownAction,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeUncertainty {
    pub record_index: usize,
    pub kind: RuntimeUncertaintyKind,
    pub field: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeReport {
    pub verdict: RuntimeVerdict,
    pub candidate_states: Vec<Vec<StateId>>,
    pub issue: Option<RuntimeIssue>,
    pub uncertainties: Vec<RuntimeUncertainty>,
}

pub struct RuntimeTraceAdapter<'a> {
    graph: FiniteGraph,
    mapping: &'a RuntimeMapping,
}

impl<'a> RuntimeTraceAdapter<'a> {
    pub fn new(graph: &FiniteGraph, mapping: &'a RuntimeMapping) -> Self {
        Self {
            graph: graph.stutter_closed(),
            mapping,
        }
    }

    /// Checks a journal as an exact sequence of model steps.
    ///
    /// This is finite-prefix conformance, not a liveness proof. Missing fields and
    /// explicitly unknown values are never treated as evidence of equality.
    pub fn check(&self, journal: &[JournalRecord]) -> RuntimeReport {
        if journal.is_empty() {
            return rejected(RuntimeIssue {
                kind: RuntimeIssueKind::EmptyJournal,
                record_index: None,
                sequence: None,
                candidates_before: Vec::new(),
                message: "a runtime journal must contain an initial snapshot".to_owned(),
            });
        }

        for model_field in self.mapping.model_to_journal.keys() {
            if let Some(state) = self
                .graph
                .states()
                .iter()
                .position(|state| !state.contains_key(model_field))
            {
                return rejected(RuntimeIssue {
                    kind: RuntimeIssueKind::MappingFieldMissing,
                    record_index: None,
                    sequence: None,
                    candidates_before: Vec::new(),
                    message: format!(
                        "runtime mapping requires model field {model_field:?}, missing at state {state}"
                    ),
                });
            }
        }

        if journal[0].action != JournalAction::Initial {
            return rejected(RuntimeIssue {
                kind: RuntimeIssueKind::FirstRecordIsNotInitial,
                record_index: Some(0),
                sequence: Some(journal[0].sequence),
                candidates_before: Vec::new(),
                message: "record 0 must use the initial marker".to_owned(),
            });
        }

        let mut uncertainties = BTreeSet::new();
        collect_observation_uncertainty(&mut uncertainties, self.mapping, 0, &journal[0]);
        let mut candidates: BTreeSet<StateId> = self
            .graph
            .initial_states()
            .iter()
            .copied()
            .filter(|&state| self.observations_match(state, &journal[0]))
            .collect();
        if candidates.is_empty() {
            return RuntimeReport {
                verdict: RuntimeVerdict::Rejected,
                candidate_states: vec![Vec::new()],
                issue: Some(RuntimeIssue {
                    kind: RuntimeIssueKind::NoMatchingInitialState,
                    record_index: Some(0),
                    sequence: Some(journal[0].sequence),
                    candidates_before: self.graph.initial_states().to_vec(),
                    message: "known initial observations match no model initial state".to_owned(),
                }),
                uncertainties: uncertainties.into_iter().collect(),
            };
        }

        let mut snapshots = vec![candidates.iter().copied().collect()];
        for index in 1..journal.len() {
            let record = &journal[index];
            if record.action == JournalAction::Initial {
                return RuntimeReport {
                    verdict: RuntimeVerdict::Rejected,
                    candidate_states: snapshots,
                    issue: Some(RuntimeIssue {
                        kind: RuntimeIssueKind::UnexpectedInitialMarker,
                        record_index: Some(index),
                        sequence: Some(record.sequence),
                        candidates_before: candidates.iter().copied().collect(),
                        message: "the initial marker may appear only at record 0".to_owned(),
                    }),
                    uncertainties: uncertainties.into_iter().collect(),
                };
            }
            let expected_sequence = journal[index - 1].sequence.checked_add(1);
            if expected_sequence != Some(record.sequence) {
                let expectation = expected_sequence.map_or_else(
                    || "no successor because the previous sequence is u64::MAX".to_owned(),
                    |expected| expected.to_string(),
                );
                return RuntimeReport {
                    verdict: RuntimeVerdict::Rejected,
                    candidate_states: snapshots,
                    issue: Some(RuntimeIssue {
                        kind: RuntimeIssueKind::NonConsecutiveSequence,
                        record_index: Some(index),
                        sequence: Some(record.sequence),
                        candidates_before: candidates.iter().copied().collect(),
                        message: format!(
                            "record {index} has sequence {}, expected {expectation} for exact-step conformance",
                            record.sequence
                        ),
                    }),
                    uncertainties: uncertainties.into_iter().collect(),
                };
            }

            if record.action == JournalAction::Unknown {
                uncertainties.insert(RuntimeUncertainty {
                    record_index: index,
                    kind: RuntimeUncertaintyKind::UnknownAction,
                    field: None,
                });
            }
            collect_observation_uncertainty(&mut uncertainties, self.mapping, index, record);

            let mut next = BTreeSet::new();
            for &candidate in &candidates {
                for &transition in self.graph.outgoing_ids(candidate) {
                    let edge = self.graph.transition(transition);
                    if action_matches(&record.action, &edge.kind)
                        && self.observations_match(edge.to, record)
                    {
                        next.insert(edge.to);
                    }
                }
            }
            if next.is_empty() {
                return RuntimeReport {
                    verdict: RuntimeVerdict::Rejected,
                    candidate_states: snapshots,
                    issue: Some(RuntimeIssue {
                        kind: RuntimeIssueKind::NoMatchingTransition,
                        record_index: Some(index),
                        sequence: Some(record.sequence),
                        candidates_before: candidates.iter().copied().collect(),
                        message: format!(
                            "record {index} action/observations match no transition from candidate states {:?}",
                            candidates
                        ),
                    }),
                    uncertainties: uncertainties.into_iter().collect(),
                };
            }
            candidates = next;
            snapshots.push(candidates.iter().copied().collect());
        }

        RuntimeReport {
            verdict: if uncertainties.is_empty() {
                RuntimeVerdict::Accepted
            } else {
                RuntimeVerdict::Unknown
            },
            candidate_states: snapshots,
            issue: None,
            uncertainties: uncertainties.into_iter().collect(),
        }
    }

    fn observations_match(&self, state: StateId, record: &JournalRecord) -> bool {
        self.mapping
            .model_to_journal
            .iter()
            .all(
                |(model_field, journal_field)| match record.observations.get(journal_field) {
                    None | Some(JournalValue::Unknown) => true,
                    Some(JournalValue::Known(observed)) => {
                        self.graph.state(state).get(model_field) == Some(observed)
                    }
                },
            )
    }
}

fn action_matches(observed: &JournalAction, model: &TransitionKind) -> bool {
    match observed {
        JournalAction::Initial => false,
        JournalAction::Action(action) => model.action() == Some(action),
        JournalAction::IdentityStutter => matches!(model, TransitionKind::IdentityStutter),
        JournalAction::Unknown => true,
    }
}

fn collect_observation_uncertainty(
    output: &mut BTreeSet<RuntimeUncertainty>,
    mapping: &RuntimeMapping,
    record_index: usize,
    record: &JournalRecord,
) {
    for journal_field in mapping.model_to_journal.values() {
        match record.observations.get(journal_field) {
            None => {
                output.insert(RuntimeUncertainty {
                    record_index,
                    kind: RuntimeUncertaintyKind::MissingObservation,
                    field: Some(journal_field.clone()),
                });
            }
            Some(JournalValue::Unknown) => {
                output.insert(RuntimeUncertainty {
                    record_index,
                    kind: RuntimeUncertaintyKind::UnknownObservation,
                    field: Some(journal_field.clone()),
                });
            }
            Some(JournalValue::Known(_)) => {}
        }
    }
}

fn rejected(issue: RuntimeIssue) -> RuntimeReport {
    RuntimeReport {
        verdict: RuntimeVerdict::Rejected,
        candidate_states: Vec::new(),
        issue: Some(issue),
        uncertainties: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ModelState, Transition};

    fn state(phase: &str, count: i64) -> ModelState {
        BTreeMap::from([
            ("phase".to_owned(), Value::from(phase)),
            ("count".to_owned(), Value::Int(count)),
        ])
    }

    fn record(
        sequence: u64,
        action: JournalAction,
        phase: Option<JournalValue>,
        count: Option<JournalValue>,
    ) -> JournalRecord {
        let mut observations = BTreeMap::new();
        if let Some(phase) = phase {
            observations.insert("phase".to_owned(), phase);
        }
        if let Some(count) = count {
            observations.insert("count".to_owned(), count);
        }
        JournalRecord {
            sequence,
            action,
            observations,
        }
    }

    fn fixture() -> (FiniteGraph, RuntimeMapping) {
        (
            FiniteGraph::new(
                vec![state("idle", 0), state("sent", 1)],
                vec![0],
                vec![Transition::action(0, "send", 1)],
            )
            .unwrap(),
            RuntimeMapping::identity(["phase", "count"]),
        )
    }

    #[test]
    fn accepts_a_fully_observed_matching_trace() {
        let (graph, mapping) = fixture();
        let journal = vec![
            record(
                10,
                JournalAction::Initial,
                Some(Value::from("idle").into()),
                Some(Value::Int(0).into()),
            ),
            record(
                11,
                JournalAction::Action("send".to_owned()),
                Some(Value::from("sent").into()),
                Some(Value::Int(1).into()),
            ),
        ];
        let report = RuntimeTraceAdapter::new(&graph, &mapping).check(&journal);
        assert_eq!(report.verdict, RuntimeVerdict::Accepted);
        assert_eq!(report.candidate_states, vec![vec![0], vec![1]]);
    }

    #[test]
    fn preserves_unknown_for_missing_or_unknown_data() {
        let (graph, mapping) = fixture();
        let journal = vec![
            record(
                0,
                JournalAction::Initial,
                Some(Value::from("idle").into()),
                None,
            ),
            record(
                1,
                JournalAction::Unknown,
                Some(JournalValue::Unknown),
                Some(Value::Int(1).into()),
            ),
        ];
        let report = RuntimeTraceAdapter::new(&graph, &mapping).check(&journal);
        assert_eq!(report.verdict, RuntimeVerdict::Unknown);
        assert!(report.issue.is_none());
        assert_eq!(report.uncertainties.len(), 3);
    }

    #[test]
    fn known_contradiction_rejects_even_with_other_unknown_fields() {
        let (graph, mapping) = fixture();
        let journal = vec![
            record(
                0,
                JournalAction::Initial,
                Some(Value::from("idle").into()),
                None,
            ),
            record(
                1,
                JournalAction::Action("send".to_owned()),
                Some(Value::from("idle").into()),
                None,
            ),
        ];
        let report = RuntimeTraceAdapter::new(&graph, &mapping).check(&journal);
        assert_eq!(report.verdict, RuntimeVerdict::Rejected);
        let issue = report.issue.unwrap();
        assert_eq!(issue.kind, RuntimeIssueKind::NoMatchingTransition);
        assert_eq!(issue.record_index, Some(1));
        assert_eq!(issue.candidates_before, vec![0]);
    }

    #[test]
    fn sequence_gap_is_localized() {
        let (graph, mapping) = fixture();
        let journal = vec![
            record(
                20,
                JournalAction::Initial,
                Some(Value::from("idle").into()),
                Some(Value::Int(0).into()),
            ),
            record(
                22,
                JournalAction::Action("send".to_owned()),
                Some(Value::from("sent").into()),
                Some(Value::Int(1).into()),
            ),
        ];
        let report = RuntimeTraceAdapter::new(&graph, &mapping).check(&journal);
        assert_eq!(report.verdict, RuntimeVerdict::Rejected);
        assert_eq!(
            report.issue.unwrap().kind,
            RuntimeIssueKind::NonConsecutiveSequence
        );
    }

    #[test]
    fn maximum_sequence_has_no_wrapping_or_saturating_successor() {
        let (graph, mapping) = fixture();
        let report = RuntimeTraceAdapter::new(&graph, &mapping).check(&[
            record(
                u64::MAX,
                JournalAction::Initial,
                Some(Value::from("idle").into()),
                Some(Value::Int(0).into()),
            ),
            record(
                u64::MAX,
                JournalAction::Action("send".to_owned()),
                Some(Value::from("sent").into()),
                Some(Value::Int(1).into()),
            ),
        ]);
        assert_eq!(report.verdict, RuntimeVerdict::Rejected);
        let issue = report.issue.expect("overflowing sequence is localized");
        assert_eq!(issue.kind, RuntimeIssueKind::NonConsecutiveSequence);
        assert_eq!(issue.record_index, Some(1));
        assert!(issue.message.contains("u64::MAX"));
    }

    #[test]
    fn explicit_identity_stutter_matches_only_temporal_closure() {
        let (_graph, mapping) = fixture();
        let journal = vec![
            record(
                0,
                JournalAction::Initial,
                Some(Value::from("sent").into()),
                Some(Value::Int(1).into()),
            ),
            record(
                1,
                JournalAction::IdentityStutter,
                Some(Value::from("sent").into()),
                Some(Value::Int(1).into()),
            ),
        ];
        // State 1 is not initial, so use a graph rooted there for this check.
        let terminal = FiniteGraph::new(vec![state("sent", 1)], vec![0], vec![]).unwrap();
        let report = RuntimeTraceAdapter::new(&terminal, &mapping).check(&journal);
        assert_eq!(report.verdict, RuntimeVerdict::Accepted);
    }
}

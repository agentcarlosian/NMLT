use crate::graph::{FiniteGraph, StateId, TransitionId, TransitionKind};
use crate::observation::{ActionHiding, ObservationMap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefinementSpec {
    /// Total concrete-state-indexed mapping into the abstract graph.
    pub state_map: Vec<StateId>,
    pub concrete_observation: ObservationMap,
    pub abstract_observation: ObservationMap,
    pub actions: ActionHiding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefinementMismatchKind {
    StateMapLength,
    AbstractStateOutOfRange,
    InitialStateNotPreserved,
    ObservationUndefined,
    ObservationMismatch,
    ActionUnmapped,
    HiddenStepChangesAbstractState,
    VisibleStepMissing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefinementMismatch {
    pub kind: RefinementMismatchKind,
    pub concrete_state: Option<StateId>,
    pub concrete_transition: Option<TransitionId>,
    pub mapped_from: Option<StateId>,
    pub mapped_to: Option<StateId>,
    pub expected_abstract_action: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefinementReport {
    pub accepted: bool,
    pub checked_states: usize,
    pub checked_transitions: usize,
    pub mismatches: Vec<RefinementMismatch>,
}

pub struct RefinementChecker;

impl RefinementChecker {
    /// Checks a finite one-step forward simulation with observation preservation.
    ///
    /// Acceptance does not establish liveness refinement: fairness transport, hidden
    /// divergence, resource effects, and environment assumptions are separate proof
    /// obligations.
    pub fn check(
        concrete: &FiniteGraph,
        abstract_graph: &FiniteGraph,
        spec: &RefinementSpec,
    ) -> RefinementReport {
        let mut mismatches = Vec::new();

        if spec.state_map.len() != concrete.states().len() {
            mismatches.push(RefinementMismatch {
                kind: RefinementMismatchKind::StateMapLength,
                concrete_state: None,
                concrete_transition: None,
                mapped_from: None,
                mapped_to: None,
                expected_abstract_action: None,
                message: format!(
                    "state map has {} entries for {} concrete states",
                    spec.state_map.len(),
                    concrete.states().len()
                ),
            });
        }

        for concrete_state in 0..concrete.states().len() {
            let Some(&abstract_state) = spec.state_map.get(concrete_state) else {
                continue;
            };
            if abstract_state >= abstract_graph.states().len() {
                mismatches.push(RefinementMismatch {
                    kind: RefinementMismatchKind::AbstractStateOutOfRange,
                    concrete_state: Some(concrete_state),
                    concrete_transition: None,
                    mapped_from: Some(abstract_state),
                    mapped_to: None,
                    expected_abstract_action: None,
                    message: format!(
                        "concrete state {concrete_state} maps to missing abstract state {abstract_state}"
                    ),
                });
                continue;
            }

            let concrete_observation = spec
                .concrete_observation
                .observe(concrete.state(concrete_state));
            let abstract_observation = spec
                .abstract_observation
                .observe(abstract_graph.state(abstract_state));
            match (concrete_observation, abstract_observation) {
                (Ok(concrete_value), Ok(abstract_value)) if concrete_value != abstract_value => {
                    mismatches.push(RefinementMismatch {
                        kind: RefinementMismatchKind::ObservationMismatch,
                        concrete_state: Some(concrete_state),
                        concrete_transition: None,
                        mapped_from: Some(abstract_state),
                        mapped_to: None,
                        expected_abstract_action: None,
                        message: format!(
                            "observation of concrete state {concrete_state} differs from mapped abstract state {abstract_state}: {concrete_value:?} != {abstract_value:?}"
                        ),
                    });
                }
                (Err(error), _) | (_, Err(error)) => {
                    mismatches.push(RefinementMismatch {
                        kind: RefinementMismatchKind::ObservationUndefined,
                        concrete_state: Some(concrete_state),
                        concrete_transition: None,
                        mapped_from: Some(abstract_state),
                        mapped_to: None,
                        expected_abstract_action: None,
                        message: format!(
                            "observation is undefined at concrete state {concrete_state} / abstract state {abstract_state}: {error}"
                        ),
                    });
                }
                _ => {}
            }
        }

        for &concrete_initial in concrete.initial_states() {
            let Some(&mapped) = spec.state_map.get(concrete_initial) else {
                continue;
            };
            if mapped < abstract_graph.states().len()
                && !abstract_graph.initial_states().contains(&mapped)
            {
                mismatches.push(RefinementMismatch {
                    kind: RefinementMismatchKind::InitialStateNotPreserved,
                    concrete_state: Some(concrete_initial),
                    concrete_transition: None,
                    mapped_from: Some(mapped),
                    mapped_to: None,
                    expected_abstract_action: None,
                    message: format!(
                        "concrete initial state {concrete_initial} maps to non-initial abstract state {mapped}"
                    ),
                });
            }
        }

        for (transition_id, transition) in concrete.transitions().iter().enumerate() {
            if matches!(transition.kind, TransitionKind::IdentityStutter) {
                continue;
            }
            let (Some(&mapped_from), Some(&mapped_to)) = (
                spec.state_map.get(transition.from),
                spec.state_map.get(transition.to),
            ) else {
                continue;
            };
            if mapped_from >= abstract_graph.states().len()
                || mapped_to >= abstract_graph.states().len()
            {
                continue;
            }
            let action = transition
                .kind
                .action()
                .expect("non-stutter transition is an action");
            match spec.actions.get(action) {
                None => mismatches.push(RefinementMismatch {
                    kind: RefinementMismatchKind::ActionUnmapped,
                    concrete_state: Some(transition.from),
                    concrete_transition: Some(transition_id),
                    mapped_from: Some(mapped_from),
                    mapped_to: Some(mapped_to),
                    expected_abstract_action: None,
                    message: format!(
                        "concrete transition {transition_id} uses action {action:?}, which has no refinement mapping"
                    ),
                }),
                Some(None) if mapped_from != mapped_to => {
                    mismatches.push(RefinementMismatch {
                        kind: RefinementMismatchKind::HiddenStepChangesAbstractState,
                        concrete_state: Some(transition.from),
                        concrete_transition: Some(transition_id),
                        mapped_from: Some(mapped_from),
                        mapped_to: Some(mapped_to),
                        expected_abstract_action: None,
                        message: format!(
                            "hidden concrete transition {transition_id} maps abstract state {mapped_from} to distinct state {mapped_to}"
                        ),
                    });
                }
                Some(None) => {}
                Some(Some(abstract_action)) => {
                    let matching = abstract_graph
                        .outgoing_ids(mapped_from)
                        .iter()
                        .copied()
                        .any(|abstract_transition| {
                            let edge = abstract_graph.transition(abstract_transition);
                            edge.to == mapped_to
                                && edge.kind.action() == Some(abstract_action)
                        });
                    if !matching {
                        mismatches.push(RefinementMismatch {
                            kind: RefinementMismatchKind::VisibleStepMissing,
                            concrete_state: Some(transition.from),
                            concrete_transition: Some(transition_id),
                            mapped_from: Some(mapped_from),
                            mapped_to: Some(mapped_to),
                            expected_abstract_action: Some(abstract_action.to_owned()),
                            message: format!(
                                "concrete transition {transition_id} requires abstract step {mapped_from} -[{abstract_action}]-> {mapped_to}, but none exists"
                            ),
                        });
                    }
                }
            }
        }

        RefinementReport {
            accepted: mismatches.is_empty(),
            checked_states: concrete.states().len(),
            checked_transitions: concrete.transitions().len(),
            mismatches,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ModelState, Transition, Value};
    use std::collections::BTreeMap;

    fn state(value: bool, secret: i64) -> ModelState {
        BTreeMap::from([
            ("visible".to_owned(), Value::Bool(value)),
            ("secret".to_owned(), Value::Int(secret)),
        ])
    }

    fn spec(state_map: Vec<StateId>) -> RefinementSpec {
        RefinementSpec {
            state_map,
            concrete_observation: ObservationMap::identity(["visible"]),
            abstract_observation: ObservationMap::identity(["visible"]),
            actions: ActionHiding::new([("cache", None::<&str>), ("publish", Some("commit"))]),
        }
    }

    #[test]
    fn accepts_hidden_stutter_and_visible_simulation() {
        let concrete = FiniteGraph::new(
            vec![state(false, 0), state(false, 1), state(true, 1)],
            vec![0],
            vec![
                Transition::action(0, "cache", 1),
                Transition::action(1, "publish", 2),
            ],
        )
        .unwrap();
        let abstract_graph = FiniteGraph::new(
            vec![state(false, 99), state(true, 99)],
            vec![0],
            vec![Transition::action(0, "commit", 1)],
        )
        .unwrap();

        let report = RefinementChecker::check(&concrete, &abstract_graph, &spec(vec![0, 0, 1]));
        assert!(report.accepted, "{:#?}", report.mismatches);
    }

    #[test]
    fn localizes_hidden_step_that_changes_abstract_state() {
        let concrete = FiniteGraph::new(
            vec![state(false, 0), state(true, 0)],
            vec![0],
            vec![Transition::action(0, "cache", 1)],
        )
        .unwrap();
        let abstract_graph =
            FiniteGraph::new(vec![state(false, 0), state(true, 0)], vec![0], vec![]).unwrap();
        let report = RefinementChecker::check(&concrete, &abstract_graph, &spec(vec![0, 1]));

        assert!(!report.accepted);
        assert!(report.mismatches.iter().any(|mismatch| {
            mismatch.kind == RefinementMismatchKind::HiddenStepChangesAbstractState
                && mismatch.concrete_transition == Some(0)
                && mismatch.mapped_from == Some(0)
                && mismatch.mapped_to == Some(1)
        }));
    }

    #[test]
    fn localizes_missing_visible_abstract_step() {
        let concrete = FiniteGraph::new(
            vec![state(false, 0), state(true, 0)],
            vec![0],
            vec![Transition::action(0, "publish", 1)],
        )
        .unwrap();
        let abstract_graph = FiniteGraph::new(
            vec![state(false, 0), state(true, 0)],
            vec![0],
            vec![Transition::action(0, "wrong", 1)],
        )
        .unwrap();
        let report = RefinementChecker::check(&concrete, &abstract_graph, &spec(vec![0, 1]));

        let mismatch = report
            .mismatches
            .iter()
            .find(|mismatch| mismatch.kind == RefinementMismatchKind::VisibleStepMissing)
            .unwrap();
        assert_eq!(mismatch.concrete_transition, Some(0));
        assert_eq!(mismatch.expected_abstract_action.as_deref(), Some("commit"));
    }

    #[test]
    fn rejects_equal_shape_with_different_observation() {
        let concrete = FiniteGraph::new(vec![state(true, 0)], vec![0], vec![]).unwrap();
        let abstract_graph = FiniteGraph::new(vec![state(false, 0)], vec![0], vec![]).unwrap();
        let report = RefinementChecker::check(&concrete, &abstract_graph, &spec(vec![0]));
        assert!(
            report
                .mismatches
                .iter()
                .any(|mismatch| mismatch.kind == RefinementMismatchKind::ObservationMismatch)
        );
    }
}

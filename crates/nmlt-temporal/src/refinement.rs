use crate::graph::{FiniteGraph, ModelState, StateId, TransitionId, TransitionKind};
use crate::observation::{ActionHiding, ObservationError, ObservationMap, stutter_expands};
use std::fmt;

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

/// Witness that a finite concrete observation word stutter-expands a finite
/// abstract observation word along the mapped path.
///
/// This is a kernel regression against Lean
/// `NMLT.weakRefines_finite_observation_trace_inclusion` / `StutterExpands`.
/// It is not a proof of that lemma.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationTraceInclusion {
    pub concrete_path: Vec<StateId>,
    pub abstract_path: Vec<StateId>,
    pub concrete_observations: Vec<ModelState>,
    pub abstract_observations: Vec<ModelState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationTraceError {
    EmptyPath,
    ConcreteStateOutOfRange(StateId),
    StateMapTooShort {
        concrete: StateId,
        map_len: usize,
    },
    AbstractStateOutOfRange {
        concrete: StateId,
        mapped: StateId,
    },
    NoConcreteStep {
        from: StateId,
        to: StateId,
    },
    ActionUnmapped {
        from: StateId,
        to: StateId,
        action: String,
    },
    HiddenStepChangesAbstractState {
        from: StateId,
        to: StateId,
        mapped_from: StateId,
        mapped_to: StateId,
    },
    VisibleStepMissing {
        from: StateId,
        to: StateId,
        mapped_from: StateId,
        mapped_to: StateId,
        abstract_action: String,
    },
    ObservationUndefined {
        state: StateId,
        source: &'static str,
        error: ObservationError,
    },
    ObservationMismatch {
        concrete_state: StateId,
        mapped: StateId,
        concrete_observation: ModelState,
        abstract_observation: ModelState,
    },
    DoesNotStutterExpand {
        abstract_observations: Vec<ModelState>,
        concrete_observations: Vec<ModelState>,
    },
}

impl fmt::Display for ObservationTraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPath => write!(
                f,
                "observation-trace inclusion requires a non-empty concrete path"
            ),
            Self::ConcreteStateOutOfRange(state) => {
                write!(f, "concrete path refers to missing state {state}")
            }
            Self::StateMapTooShort { concrete, map_len } => write!(
                f,
                "state map of length {map_len} has no entry for concrete state {concrete}"
            ),
            Self::AbstractStateOutOfRange { concrete, mapped } => write!(
                f,
                "concrete state {concrete} maps to missing abstract state {mapped}"
            ),
            Self::NoConcreteStep { from, to } => {
                write!(f, "no concrete step from {from} to {to} on the given path")
            }
            Self::ActionUnmapped { from, to, action } => write!(
                f,
                "concrete step {from} -[{action}]-> {to} has no refinement mapping"
            ),
            Self::HiddenStepChangesAbstractState {
                from,
                to,
                mapped_from,
                mapped_to,
            } => write!(
                f,
                "hidden concrete step {from} -> {to} maps abstract state {mapped_from} to distinct state {mapped_to}"
            ),
            Self::VisibleStepMissing {
                from,
                to,
                mapped_from,
                mapped_to,
                abstract_action,
            } => write!(
                f,
                "visible concrete step {from} -> {to} requires abstract step {mapped_from} -[{abstract_action}]-> {mapped_to}, but none exists"
            ),
            Self::ObservationUndefined {
                state,
                source,
                error,
            } => write!(
                f,
                "{source} observation is undefined at state {state}: {error}"
            ),
            Self::ObservationMismatch {
                concrete_state,
                mapped,
                concrete_observation,
                abstract_observation,
            } => write!(
                f,
                "observation of concrete state {concrete_state} differs from mapped abstract state {mapped}: {concrete_observation:?} != {abstract_observation:?}"
            ),
            Self::DoesNotStutterExpand {
                abstract_observations,
                concrete_observations,
            } => write!(
                f,
                "concrete observation word {concrete_observations:?} is not a stutter-expansion of abstract word {abstract_observations:?}"
            ),
        }
    }
}

impl std::error::Error for ObservationTraceError {}

/// Finite observation-trace inclusion for a concrete path under a one-step
/// refinement map with stuttering.
///
/// Hidden steps stay on the current abstract state (repeat its observation);
/// visible steps take exactly one abstract step. Acceptance means the concrete
/// observation word is a `stutter_expands` of the abstract word along the
/// mapped path, matching Lean `StutterExpands`. Not a proof of
/// `weakRefines_finite_observation_trace_inclusion`, not LTL, not infinite
/// traces, fairness, or liveness.
pub fn observation_trace_inclusion(
    concrete: &FiniteGraph,
    abstract_graph: &FiniteGraph,
    spec: &RefinementSpec,
    concrete_path: &[StateId],
) -> Result<ObservationTraceInclusion, ObservationTraceError> {
    if concrete_path.is_empty() {
        return Err(ObservationTraceError::EmptyPath);
    }

    let mut concrete_observations = Vec::with_capacity(concrete_path.len());
    for &state in concrete_path {
        if state >= concrete.states().len() {
            return Err(ObservationTraceError::ConcreteStateOutOfRange(state));
        }
        let mapped = mapped_state(spec, abstract_graph, state)?;
        let concrete_obs = spec
            .concrete_observation
            .observe(concrete.state(state))
            .map_err(|error| ObservationTraceError::ObservationUndefined {
                state,
                source: "concrete",
                error,
            })?;
        let abstract_obs = spec
            .abstract_observation
            .observe(abstract_graph.state(mapped))
            .map_err(|error| ObservationTraceError::ObservationUndefined {
                state: mapped,
                source: "abstract",
                error,
            })?;
        if concrete_obs != abstract_obs {
            return Err(ObservationTraceError::ObservationMismatch {
                concrete_state: state,
                mapped,
                concrete_observation: concrete_obs,
                abstract_observation: abstract_obs,
            });
        }
        concrete_observations.push(concrete_obs);
    }

    let start = concrete_path[0];
    let mut abstract_path = vec![mapped_state(spec, abstract_graph, start)?];
    for window in concrete_path.windows(2) {
        let from = window[0];
        let to = window[1];
        let mapped_from = mapped_state(spec, abstract_graph, from)?;
        let mapped_to = mapped_state(spec, abstract_graph, to)?;
        if classify_path_step(
            concrete,
            abstract_graph,
            spec,
            from,
            to,
            mapped_from,
            mapped_to,
        )? {
            abstract_path.push(mapped_to);
        }
    }

    let mut abstract_observations = Vec::with_capacity(abstract_path.len());
    for &mapped in &abstract_path {
        abstract_observations.push(
            spec.abstract_observation
                .observe(abstract_graph.state(mapped))
                .map_err(|error| ObservationTraceError::ObservationUndefined {
                    state: mapped,
                    source: "abstract",
                    error,
                })?,
        );
    }

    if !stutter_expands(&abstract_observations, &concrete_observations) {
        return Err(ObservationTraceError::DoesNotStutterExpand {
            abstract_observations,
            concrete_observations,
        });
    }

    Ok(ObservationTraceInclusion {
        concrete_path: concrete_path.to_vec(),
        abstract_path,
        concrete_observations,
        abstract_observations,
    })
}

fn mapped_state(
    spec: &RefinementSpec,
    abstract_graph: &FiniteGraph,
    concrete_state: StateId,
) -> Result<StateId, ObservationTraceError> {
    let Some(&mapped) = spec.state_map.get(concrete_state) else {
        return Err(ObservationTraceError::StateMapTooShort {
            concrete: concrete_state,
            map_len: spec.state_map.len(),
        });
    };
    if mapped >= abstract_graph.states().len() {
        return Err(ObservationTraceError::AbstractStateOutOfRange {
            concrete: concrete_state,
            mapped,
        });
    }
    Ok(mapped)
}

/// Returns `true` when the witnessing concrete step is visible (one abstract
/// step) and `false` when it is hidden or identity stutter (repeat abstract obs).
fn classify_path_step(
    concrete: &FiniteGraph,
    abstract_graph: &FiniteGraph,
    spec: &RefinementSpec,
    from: StateId,
    to: StateId,
    mapped_from: StateId,
    mapped_to: StateId,
) -> Result<bool, ObservationTraceError> {
    let mut first_error = None;
    for &transition_id in concrete.outgoing_ids(from) {
        let edge = concrete.transition(transition_id);
        if edge.to != to {
            continue;
        }
        match &edge.kind {
            TransitionKind::IdentityStutter => {
                if mapped_from == mapped_to {
                    return Ok(false);
                }
                first_error.get_or_insert(ObservationTraceError::HiddenStepChangesAbstractState {
                    from,
                    to,
                    mapped_from,
                    mapped_to,
                });
            }
            TransitionKind::Action(action) => match spec.actions.get(action) {
                None => {
                    first_error.get_or_insert(ObservationTraceError::ActionUnmapped {
                        from,
                        to,
                        action: action.clone(),
                    });
                }
                Some(None) => {
                    if mapped_from == mapped_to {
                        return Ok(false);
                    }
                    first_error.get_or_insert(
                        ObservationTraceError::HiddenStepChangesAbstractState {
                            from,
                            to,
                            mapped_from,
                            mapped_to,
                        },
                    );
                }
                Some(Some(abstract_action)) => {
                    let matching = abstract_graph
                        .outgoing_ids(mapped_from)
                        .iter()
                        .copied()
                        .any(|abstract_transition| {
                            let abstract_edge = abstract_graph.transition(abstract_transition);
                            abstract_edge.to == mapped_to
                                && abstract_edge.kind.action() == Some(abstract_action)
                        });
                    if matching {
                        return Ok(true);
                    }
                    first_error.get_or_insert(ObservationTraceError::VisibleStepMissing {
                        from,
                        to,
                        mapped_from,
                        mapped_to,
                        abstract_action: abstract_action.to_owned(),
                    });
                }
            },
        }
    }
    Err(first_error.unwrap_or(ObservationTraceError::NoConcreteStep { from, to }))
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

    fn ping_spec() -> RefinementSpec {
        RefinementSpec {
            state_map: vec![0],
            concrete_observation: ObservationMap::identity(["visible"]),
            abstract_observation: ObservationMap::identity(["visible"]),
            actions: ActionHiding::from_hide_actions(["ping"], [] as [(&str, &str); 0]),
        }
    }

    fn bounded_walks(graph: &FiniteGraph, max_steps: usize) -> Vec<Vec<StateId>> {
        let mut walks = Vec::new();
        fn rec(
            graph: &FiniteGraph,
            max_steps: usize,
            path: &mut Vec<StateId>,
            walks: &mut Vec<Vec<StateId>>,
        ) {
            walks.push(path.clone());
            if path.len() > max_steps {
                return;
            }
            let from = *path.last().expect("walks start at a state");
            for &transition_id in graph.outgoing_ids(from) {
                path.push(graph.transition(transition_id).to);
                rec(graph, max_steps, path, walks);
                path.pop();
            }
        }
        for &initial in graph.initial_states() {
            let mut path = vec![initial];
            rec(graph, max_steps, &mut path, &mut walks);
        }
        walks
    }

    #[test]
    fn hidden_ping_loop_stutter_expands_one_state_abstract() {
        // Paper 1 concrete sender: one state, hidden ping self-loop; abstract is silent.
        let concrete = FiniteGraph::new(
            vec![state(false, 0)],
            vec![0],
            vec![Transition::action(0, "ping", 0)],
        )
        .unwrap();
        let abstract_graph = FiniteGraph::new(vec![state(false, 99)], vec![0], vec![]).unwrap();
        let spec = ping_spec();

        let report = RefinementChecker::check(&concrete, &abstract_graph, &spec);
        assert!(report.accepted, "{:#?}", report.mismatches);

        let inclusion = observation_trace_inclusion(&concrete, &abstract_graph, &spec, &[0, 0])
            .expect("hidden ping path");
        assert_eq!(inclusion.abstract_path, vec![0]);
        assert_eq!(inclusion.concrete_observations.len(), 2);
        assert_eq!(inclusion.abstract_observations.len(), 1);
        assert_eq!(
            inclusion.concrete_observations[0],
            inclusion.abstract_observations[0]
        );
        assert!(stutter_expands(
            &inclusion.abstract_observations,
            &inclusion.concrete_observations
        ));

        for walk in bounded_walks(&concrete, 2) {
            observation_trace_inclusion(&concrete, &abstract_graph, &spec, &walk)
                .unwrap_or_else(|error| panic!("accepted refinement, walk {walk:?}: {error}"));
        }
    }

    #[test]
    fn visible_send_commit_path_stutter_expands() {
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
        let spec = spec(vec![0, 0, 1]);

        let report = RefinementChecker::check(&concrete, &abstract_graph, &spec);
        assert!(report.accepted, "{:#?}", report.mismatches);

        let inclusion =
            observation_trace_inclusion(&concrete, &abstract_graph, &spec, &[0, 1, 2]).unwrap();
        assert_eq!(inclusion.abstract_path, vec![0, 1]);
        assert_eq!(inclusion.concrete_observations.len(), 3);
        assert_eq!(inclusion.abstract_observations.len(), 2);
        assert!(stutter_expands(
            &inclusion.abstract_observations,
            &inclusion.concrete_observations
        ));

        for walk in bounded_walks(&concrete, 2) {
            observation_trace_inclusion(&concrete, &abstract_graph, &spec, &walk)
                .unwrap_or_else(|error| panic!("accepted refinement, walk {walk:?}: {error}"));
        }
    }

    #[test]
    fn hidden_step_that_changes_observation_fails_trace_inclusion() {
        let concrete = FiniteGraph::new(
            vec![state(false, 0), state(true, 0)],
            vec![0],
            vec![Transition::action(0, "cache", 1)],
        )
        .unwrap();
        let abstract_graph =
            FiniteGraph::new(vec![state(false, 0), state(true, 0)], vec![0], vec![]).unwrap();
        let spec = spec(vec![0, 1]);

        let report = RefinementChecker::check(&concrete, &abstract_graph, &spec);
        assert!(report.mismatches.iter().any(|mismatch| {
            mismatch.kind == RefinementMismatchKind::HiddenStepChangesAbstractState
        }));

        let error = observation_trace_inclusion(&concrete, &abstract_graph, &spec, &[0, 1])
            .expect_err("hidden step that changes the mapped state");
        assert!(matches!(
            error,
            ObservationTraceError::HiddenStepChangesAbstractState {
                from: 0,
                to: 1,
                mapped_from: 0,
                mapped_to: 1,
            }
        ));
    }
}

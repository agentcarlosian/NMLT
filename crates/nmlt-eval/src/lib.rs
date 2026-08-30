//! A deliberately non-authoritative evaluator for inspectable behavior cores.
//!
//! Exploration is a language aid. It does not return verification result
//! classes and cannot establish a semantic theorem.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use nmlt_ir::{
    BehaviorCoreProgram, CoreBehaviorAction, CoreBehaviorSystem, CoreBehaviorTerm, CoreComposition,
    CorePortDirection, CoreResourceProfile,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExploreConfig {
    pub max_states: usize,
}

impl Default for ExploreConfig {
    fn default() -> Self {
        Self { max_states: 128 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvalState {
    pub values: BTreeMap<String, bool>,
    pub authority: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvalTransition {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub grade: BTreeMap<String, u64>,
    pub transfers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exploration {
    pub behavior: String,
    pub states: Vec<EvalState>,
    pub transitions: Vec<EvalTransition>,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvalError(String);

impl fmt::Display for EvalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for EvalError {}

pub fn explore(
    program: &BehaviorCoreProgram,
    behavior: &str,
    config: ExploreConfig,
) -> Result<Exploration, EvalError> {
    if config.max_states == 0 {
        return Err(EvalError("max_states must be positive".to_owned()));
    }
    if let Some(system) = program.systems.get(behavior) {
        explore_system(system, config)
    } else if let Some(composition) = program.compositions.get(behavior) {
        explore_composition(program, composition, config)
    } else {
        Err(EvalError(format!("unknown behavior '{behavior}'")))
    }
}

fn explore_system(
    system: &CoreBehaviorSystem,
    config: ExploreConfig,
) -> Result<Exploration, EvalError> {
    let initial = initial_state(system, None)?;
    explore_graph(system.name.clone(), initial, config, |state| {
        system
            .actions
            .values()
            .filter(|action| enabled(action, state, &system.name))
            .map(|action| {
                let mut after = state.clone();
                apply(action, &mut after, &system.name)?;
                apply_open_resources(action, &mut after, &system.name);
                Ok(Candidate {
                    state: after,
                    label: format!("{}.{}", system.name, action.name),
                    resources: action.resources.clone(),
                    transfers: action.resources.transfers.iter().cloned().collect(),
                })
            })
            .collect()
    })
}

fn explore_composition(
    program: &BehaviorCoreProgram,
    composition: &CoreComposition,
    config: ExploreConfig,
) -> Result<Exploration, EvalError> {
    let left = &program.systems[&composition.left_system];
    let right = &program.systems[&composition.right_system];
    let mut initial = initial_state(left, Some(&left.name))?;
    let right_initial = initial_state(right, Some(&right.name))?;
    initial.values.extend(right_initial.values);
    initial.authority.extend(right_initial.authority);
    let connected = composition
        .connections
        .iter()
        .flat_map(|connection| {
            [
                (
                    connection.left_system.clone(),
                    connection.left_action.clone(),
                ),
                (
                    connection.right_system.clone(),
                    connection.right_action.clone(),
                ),
            ]
        })
        .collect::<BTreeSet<_>>();

    explore_graph(composition.name.clone(), initial, config, |state| {
        let mut candidates = Vec::new();
        for connection in &composition.connections {
            let left_system = &program.systems[&connection.left_system];
            let right_system = &program.systems[&connection.right_system];
            let left_action = &left_system.actions[&connection.left_action];
            let right_action = &right_system.actions[&connection.right_action];
            if enabled(left_action, state, &left_system.name)
                && enabled(right_action, state, &right_system.name)
            {
                let mut after = state.clone();
                apply(left_action, &mut after, &left_system.name)?;
                apply(right_action, &mut after, &right_system.name)?;
                apply_synchronized_resources(
                    left_action,
                    &left_system.name,
                    right_action,
                    &right_system.name,
                    &mut after,
                );
                let resources = left_action.resources.parallel(&right_action.resources);
                let (sender_name, transfers) =
                    if left_action.direction == Some(CorePortDirection::Output) {
                        (&left_system.name, &left_action.resources.transfers)
                    } else {
                        (&right_system.name, &right_action.resources.transfers)
                    };
                let receiver_name = if sender_name == &left_system.name {
                    &right_system.name
                } else {
                    &left_system.name
                };
                candidates.push(Candidate {
                    state: after,
                    label: format!(
                        "{}.{}|{}.{}",
                        left_system.name, left_action.name, right_system.name, right_action.name
                    ),
                    resources,
                    transfers: transfers
                        .iter()
                        .map(|name| format!("{name}: {sender_name} -> {receiver_name}"))
                        .collect(),
                });
            }
        }
        for system in [left, right] {
            for action in system.actions.values() {
                if connected.contains(&(system.name.clone(), action.name.clone()))
                    || !enabled(action, state, &system.name)
                {
                    continue;
                }
                let mut after = state.clone();
                apply(action, &mut after, &system.name)?;
                apply_open_resources(action, &mut after, &system.name);
                candidates.push(Candidate {
                    state: after,
                    label: format!("{}.{}", system.name, action.name),
                    resources: action.resources.clone(),
                    transfers: action.resources.transfers.iter().cloned().collect(),
                });
            }
        }
        Ok(candidates)
    })
}

#[derive(Clone)]
struct Candidate {
    state: EvalState,
    label: String,
    resources: CoreResourceProfile,
    transfers: Vec<String>,
}

fn explore_graph(
    behavior: String,
    initial: EvalState,
    config: ExploreConfig,
    mut successors: impl FnMut(&EvalState) -> Result<Vec<Candidate>, EvalError>,
) -> Result<Exploration, EvalError> {
    let mut states = vec![initial.clone()];
    let mut index = BTreeMap::from([(initial, 0_usize)]);
    let mut queue = VecDeque::from([0_usize]);
    let mut transitions = Vec::new();
    let mut truncated = false;
    while let Some(from) = queue.pop_front() {
        for candidate in successors(&states[from])? {
            let to = if let Some(existing) = index.get(&candidate.state) {
                *existing
            } else if states.len() < config.max_states {
                let next = states.len();
                index.insert(candidate.state.clone(), next);
                states.push(candidate.state);
                queue.push_back(next);
                next
            } else {
                truncated = true;
                continue;
            };
            transitions.push(EvalTransition {
                from,
                to,
                label: candidate.label,
                grade: candidate.resources.grade,
                transfers: candidate.transfers,
            });
        }
    }
    transitions.sort_by(|left, right| {
        (left.from, &left.label, left.to).cmp(&(right.from, &right.label, right.to))
    });
    transitions.dedup();
    Ok(Exploration {
        behavior,
        states,
        transitions,
        truncated,
    })
}

fn initial_state(
    system: &CoreBehaviorSystem,
    prefix: Option<&str>,
) -> Result<EvalState, EvalError> {
    let values = system
        .state
        .values()
        .map(|field| {
            let name = match prefix {
                Some(prefix) => format!("{prefix}.{}", field.name),
                None => field.name.clone(),
            };
            match &field.initial_ast {
                CoreBehaviorTerm::Bool { value, .. } => Ok((name, *value)),
                _ => Err(EvalError(format!(
                    "reference evaluator supports Bool initializers only: {}.{}",
                    system.name, field.name
                ))),
            }
        })
        .collect::<Result<_, _>>()?;
    let authority = system
        .capabilities
        .keys()
        .map(|capability| (capability.clone(), system.name.clone()))
        .collect();
    Ok(EvalState { values, authority })
}

fn enabled(action: &CoreBehaviorAction, state: &EvalState, system: &str) -> bool {
    let guards_hold = action
        .guard_ast
        .iter()
        .all(|guard| eval_bool(guard, state, system).unwrap_or(false));
    let authority_available = action
        .resources
        .requires
        .iter()
        .chain(&action.resources.consumes)
        .chain(&action.resources.transfers)
        .all(|capability| {
            state
                .authority
                .get(capability)
                .is_some_and(|owner| owner == system)
        });
    guards_hold && authority_available
}

fn apply(
    action: &CoreBehaviorAction,
    state: &mut EvalState,
    system: &str,
) -> Result<(), EvalError> {
    let before = state.clone();
    for (field, term) in &action.update_ast {
        let value = eval_bool(term, &before, system).ok_or_else(|| {
            EvalError(format!(
                "unsupported Bool update AST in {system}.{} for field {field}",
                action.name,
            ))
        })?;
        let qualified = qualify(&before, system, field);
        state.values.insert(qualified, value);
    }
    for capability in &action.resources.consumes {
        state.authority.remove(capability);
    }
    Ok(())
}

fn apply_open_resources(action: &CoreBehaviorAction, state: &mut EvalState, system: &str) {
    for capability in &action.resources.transfers {
        state.authority.remove(capability);
    }
    for capability in &action.resources.receives {
        state
            .authority
            .entry(capability.clone())
            .or_insert_with(|| system.to_owned());
    }
}

fn apply_synchronized_resources(
    left: &CoreBehaviorAction,
    left_system: &str,
    right: &CoreBehaviorAction,
    right_system: &str,
    state: &mut EvalState,
) {
    let (sender, sender_system, receiver_system) =
        if left.direction == Some(CorePortDirection::Output) {
            (left, left_system, right_system)
        } else {
            (right, right_system, left_system)
        };
    for capability in &sender.resources.transfers {
        if state
            .authority
            .get(capability)
            .is_some_and(|owner| owner == sender_system)
        {
            state
                .authority
                .insert(capability.clone(), receiver_system.to_owned());
        }
    }
}

fn eval_bool(term: &CoreBehaviorTerm, state: &EvalState, system: &str) -> Option<bool> {
    match term {
        CoreBehaviorTerm::Bool { value, .. } => Some(*value),
        CoreBehaviorTerm::Read { field, .. } => {
            let key = qualify(state, system, field);
            state.values.get(&key).copied()
        }
        CoreBehaviorTerm::Not { value, .. } => eval_bool(value, state, system).map(|value| !value),
        CoreBehaviorTerm::Equal { left, right, .. } => {
            Some(eval_bool(left, state, system)? == eval_bool(right, state, system)?)
        }
        CoreBehaviorTerm::Unit { .. } | CoreBehaviorTerm::Enum { .. } => None,
    }
}

fn qualify(state: &EvalState, system: &str, field: &str) -> String {
    let qualified = format!("{system}.{field}");
    if state.values.contains_key(&qualified) {
        qualified
    } else {
        field.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nmlt_ir::{
        BEHAVIOR_CORE_SCHEMA, CoreBehaviorState, CoreBehaviorTerm, CoreConnection, CorePort,
    };

    #[test]
    fn reference_exploration_reports_sync_grade_and_transfer() {
        let send = CoreBehaviorAction {
            name: "send".to_owned(),
            direction: Some(CorePortDirection::Output),
            parameters: Vec::new(),
            guards: Vec::new(),
            guard_ast: Vec::new(),
            updates: BTreeMap::new(),
            update_ast: BTreeMap::new(),
            outputs: vec!["permit".to_owned()],
            hidden: false,
            resources: CoreResourceProfile {
                transfers: BTreeSet::from(["permit".to_owned()]),
                grade: BTreeMap::from([("work".to_owned(), 1)]),
                ..CoreResourceProfile::default()
            },
        };
        let receive = CoreBehaviorAction {
            name: "receive".to_owned(),
            direction: Some(CorePortDirection::Input),
            parameters: Vec::new(),
            guards: Vec::new(),
            guard_ast: Vec::new(),
            updates: BTreeMap::from([("bit".to_owned(), "true".to_owned())]),
            update_ast: BTreeMap::from([(
                "bit".to_owned(),
                CoreBehaviorTerm::Bool {
                    r#type: "Bool".to_owned(),
                    value: true,
                },
            )]),
            outputs: Vec::new(),
            hidden: false,
            resources: CoreResourceProfile {
                receives: BTreeSet::from(["permit".to_owned()]),
                grade: BTreeMap::from([("work".to_owned(), 2)]),
                ..CoreResourceProfile::default()
            },
        };
        let sender = CoreBehaviorSystem {
            name: "Sender".to_owned(),
            state: BTreeMap::from([(
                "unit".to_owned(),
                CoreBehaviorState {
                    name: "unit".to_owned(),
                    ty: "Bool".to_owned(),
                    initializer: "false".to_owned(),
                    initial_ast: CoreBehaviorTerm::Bool {
                        r#type: "Bool".to_owned(),
                        value: false,
                    },
                },
            )]),
            capabilities: BTreeMap::from([("permit".to_owned(), "Once<Unit>".to_owned())]),
            ports: BTreeMap::from([(
                "send".to_owned(),
                CorePort {
                    name: "send".to_owned(),
                    direction: CorePortDirection::Output,
                    payload_type: "Once<Unit>".to_owned(),
                },
            )]),
            actions: BTreeMap::from([("send".to_owned(), send)]),
            observations: vec!["unit".to_owned()],
        };
        let receiver = CoreBehaviorSystem {
            name: "Receiver".to_owned(),
            state: BTreeMap::from([(
                "bit".to_owned(),
                CoreBehaviorState {
                    name: "bit".to_owned(),
                    ty: "Bool".to_owned(),
                    initializer: "false".to_owned(),
                    initial_ast: CoreBehaviorTerm::Bool {
                        r#type: "Bool".to_owned(),
                        value: false,
                    },
                },
            )]),
            capabilities: BTreeMap::new(),
            ports: BTreeMap::from([(
                "receive".to_owned(),
                CorePort {
                    name: "receive".to_owned(),
                    direction: CorePortDirection::Input,
                    payload_type: "Once<Unit>".to_owned(),
                },
            )]),
            actions: BTreeMap::from([("receive".to_owned(), receive)]),
            observations: vec!["bit".to_owned()],
        };
        let program = BehaviorCoreProgram {
            schema: BEHAVIOR_CORE_SCHEMA.to_owned(),
            source_path: "test.nmlt".to_owned(),
            source_sha256: "0".repeat(64),
            enums: BTreeMap::new(),
            systems: BTreeMap::from([
                ("Receiver".to_owned(), receiver),
                ("Sender".to_owned(), sender),
            ]),
            compositions: BTreeMap::from([(
                "Network".to_owned(),
                CoreComposition {
                    name: "Network".to_owned(),
                    left_system: "Receiver".to_owned(),
                    right_system: "Sender".to_owned(),
                    connections: vec![CoreConnection {
                        left_system: "Sender".to_owned(),
                        left_action: "send".to_owned(),
                        right_system: "Receiver".to_owned(),
                        right_action: "receive".to_owned(),
                    }],
                },
            )]),
            refinements: Vec::new(),
        };
        let result = explore(&program, "Network", ExploreConfig::default()).unwrap();
        assert!(
            result
                .states
                .iter()
                .any(|state| state.values["Receiver.bit"])
        );
        assert!(result.states.iter().any(|state| {
            state.authority.get("permit").map(String::as_str) == Some("Receiver")
        }));
        assert!(result.transitions.iter().any(|transition| {
            transition.grade.get("work") == Some(&3)
                && transition.transfers == ["permit: Sender -> Receiver"]
        }));
    }
}

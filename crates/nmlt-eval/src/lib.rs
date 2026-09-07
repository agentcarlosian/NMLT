//! A deliberately non-authoritative evaluator for inspectable behavior cores.
//!
//! Exploration and bounded finite execution share their step operations. They
//! do not return verification results or establish semantic theorems.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use nmlt_ir::{
    BehaviorCoreProgram, CoreBehaviorAction, CoreBehaviorSystem, CoreBehaviorTerm, CoreComposition,
    CorePortDirection, CoreResourceProfile,
};

mod execution;
mod interpreter;
mod value;
pub use execution::execution_path;
pub use interpreter::{MAX_RUN_STEPS, RunConfig, RunOutcome, RunStep, RunTrace, Schedule, execute};

pub use value::EvalValue;
use value::eval_value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExploreConfig {
    pub max_states: usize,
}

impl Default for ExploreConfig {
    fn default() -> Self {
        Self { max_states: 128 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalState {
    pub values: BTreeMap<String, EvalValue>,
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
    let prepared = prepare(program, behavior)?;
    explore_graph(
        behavior.to_owned(),
        prepared.initial,
        config,
        prepared.successors,
    )
}

type Successors<'a> = Box<dyn Fn(&EvalState) -> Result<Vec<Candidate>, EvalError> + 'a>;

/// One initializer and successor operation shared by graph search and execution.
struct PreparedBehavior<'a> {
    initial: EvalState,
    successors: Successors<'a>,
}

fn prepare<'a>(
    program: &'a BehaviorCoreProgram,
    behavior: &str,
) -> Result<PreparedBehavior<'a>, EvalError> {
    let dynamic = program.schema == nmlt_ir::BEHAVIOR_CORE_V2_SCHEMA;
    if dynamic {
        program.validate_execution_maps().map_err(EvalError)?;
    }
    if let Some(system) = program.systems.get(behavior) {
        prepare_system(system, &program.enums, dynamic)
    } else if let Some(composition) = program.compositions.get(behavior) {
        prepare_composition(program, composition)
    } else {
        Err(EvalError(format!("unknown behavior '{behavior}'")))
    }
}

fn prepare_system<'a>(
    system: &'a CoreBehaviorSystem,
    enums: &'a BTreeMap<String, BTreeSet<String>>,
    dynamic: bool,
) -> Result<PreparedBehavior<'a>, EvalError> {
    let initial = initial_state(system, None, enums)?;
    validate_action_terms(system, &initial, enums)?;
    let successors = Box::new(move |state: &EvalState| {
        system
            .actions
            .values()
            .filter(|action| {
                enabled(action, state, &system.name, enums)
                    && (!dynamic || local_enabled(action, state, &system.name))
            })
            .map(|action| {
                let mut after = state.clone();
                apply(action, &mut after, &system.name, enums)?;
                apply_open_resources(action, &mut after, &system.name);
                Ok(Candidate {
                    state: after,
                    label: format!("{}.{}", system.name, action.name),
                    resources: action.resources.clone(),
                    transfers: action.resources.transfers.iter().cloned().collect(),
                })
            })
            .collect()
    });
    Ok(PreparedBehavior {
        initial,
        successors,
    })
}

fn prepare_composition<'a>(
    program: &'a BehaviorCoreProgram,
    composition: &'a CoreComposition,
) -> Result<PreparedBehavior<'a>, EvalError> {
    let enums = &program.enums;
    let dynamic = program.schema == nmlt_ir::BEHAVIOR_CORE_V2_SCHEMA;
    let resolve_system = |name: &str| {
        program.systems.get(name).ok_or_else(|| {
            EvalError(format!(
                "composition '{}' names unknown system '{name}'",
                composition.name
            ))
        })
    };
    let left = resolve_system(&composition.left_system)?;
    let right = resolve_system(&composition.right_system)?;
    if composition.left_system == composition.right_system {
        return Err(EvalError(format!(
            "composition '{}' must contain two distinct systems",
            composition.name
        )));
    }
    let connections = composition
        .connections
        .iter()
        .map(|connection| {
            let (left_system, right_system) = if connection.left_system == composition.left_system
                && connection.right_system == composition.right_system
            {
                (left, right)
            } else if connection.left_system == composition.right_system
                && connection.right_system == composition.left_system
            {
                (right, left)
            } else {
                return Err(EvalError(format!(
                    "connection '{}.{}|{}.{}' in composition '{}' must join '{}' and '{}'",
                    connection.left_system,
                    connection.left_action,
                    connection.right_system,
                    connection.right_action,
                    composition.name,
                    composition.left_system,
                    composition.right_system,
                )));
            };
            Ok(ResolvedConnection {
                left_system,
                left_action: resolve_action(
                    left_system,
                    &connection.left_action,
                    &composition.name,
                )?,
                right_system,
                right_action: resolve_action(
                    right_system,
                    &connection.right_action,
                    &composition.name,
                )?,
            })
        })
        .collect::<Result<Vec<_>, EvalError>>()?;
    let mut initial = initial_state(left, Some(&left.name), enums)?;
    let right_initial = initial_state(right, Some(&right.name), enums)?;
    validate_action_terms(left, &initial, enums)?;
    validate_action_terms(right, &right_initial, enums)?;
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

    let successors = Box::new(move |state: &EvalState| {
        let mut candidates = Vec::new();
        for &ResolvedConnection {
            left_system,
            left_action,
            right_system,
            right_action,
        } in &connections
        {
            if enabled(left_action, state, &left_system.name, enums)
                && enabled(right_action, state, &right_system.name, enums)
                && (!dynamic
                    || synchronized_enabled(
                        left_action,
                        &left_system.name,
                        right_action,
                        &right_system.name,
                        state,
                    ))
            {
                let mut after = state.clone();
                apply(left_action, &mut after, &left_system.name, enums)?;
                apply(right_action, &mut after, &right_system.name, enums)?;
                apply_synchronized_resources(
                    left_action,
                    &left_system.name,
                    right_action,
                    &right_system.name,
                    &mut after,
                );
                let resources = left_action
                    .resources
                    .parallel(&right_action.resources)
                    .map_err(|error| {
                        EvalError(format!(
                            "composition '{}' at '{}.{}|{}.{}': {error}",
                            composition.name,
                            left_system.name,
                            left_action.name,
                            right_system.name,
                            right_action.name,
                        ))
                    })?;
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
                    || !enabled(action, state, &system.name, enums)
                    || (dynamic && !local_enabled(action, state, &system.name))
                {
                    continue;
                }
                let mut after = state.clone();
                apply(action, &mut after, &system.name, enums)?;
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
    });
    Ok(PreparedBehavior {
        initial,
        successors,
    })
}

struct ResolvedConnection<'a> {
    left_system: &'a CoreBehaviorSystem,
    left_action: &'a CoreBehaviorAction,
    right_system: &'a CoreBehaviorSystem,
    right_action: &'a CoreBehaviorAction,
}

fn resolve_action<'a>(
    system: &'a CoreBehaviorSystem,
    name: &str,
    composition: &str,
) -> Result<&'a CoreBehaviorAction, EvalError> {
    system.actions.get(name).ok_or_else(|| {
        EvalError(format!(
            "composition '{composition}' names unknown action '{}.{name}'",
            system.name
        ))
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
    enums: &BTreeMap<String, BTreeSet<String>>,
) -> Result<EvalState, EvalError> {
    let empty = EvalState {
        values: BTreeMap::new(),
        authority: BTreeMap::new(),
    };
    let values = system
        .state
        .values()
        .map(|field| {
            let name = match prefix {
                Some(prefix) => format!("{prefix}.{}", field.name),
                None => field.name.clone(),
            };
            let value = eval_value(&field.initial_ast, &empty, &system.name, enums)
                .filter(|value| value.type_name() == field.ty)
                .ok_or_else(|| {
                    EvalError(format!(
                        "expected a closed initializer of declared Bool/Unit/enum type: {}.{}",
                        system.name, field.name
                    ))
                })?;
            Ok((name, value))
        })
        .collect::<Result<_, _>>()?;
    let authority = system
        .capabilities
        .keys()
        .map(|capability| (capability.clone(), system.name.clone()))
        .collect();
    Ok(EvalState { values, authority })
}

fn validate_action_terms(
    system: &CoreBehaviorSystem,
    initial: &EvalState,
    enums: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), EvalError> {
    for action in system.actions.values() {
        for guard in &action.guard_ast {
            if eval_bool(guard, initial, &system.name, enums).is_none() {
                return Err(EvalError(format!(
                    "invalid Bool guard AST in {}.{}",
                    system.name, action.name
                )));
            }
        }
        for (field, term) in &action.update_ast {
            let declared = system.state.get(field).ok_or_else(|| {
                EvalError(format!(
                    "unknown update field {field} in {}.{}",
                    system.name, action.name
                ))
            })?;
            if eval_value(term, initial, &system.name, enums)
                .is_none_or(|value| value.type_name() != declared.ty)
            {
                return Err(EvalError(format!(
                    "invalid update AST in {}.{} for field {field}; expected {}",
                    system.name, action.name, declared.ty
                )));
            }
        }
    }
    Ok(())
}

fn enabled(
    action: &CoreBehaviorAction,
    state: &EvalState,
    system: &str,
    enums: &BTreeMap<String, BTreeSet<String>>,
) -> bool {
    let guards_hold = action
        .guard_ast
        .iter()
        .all(|guard| eval_bool(guard, state, system, enums).unwrap_or(false));
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

fn affine_enabled(action: &CoreBehaviorAction, state: &EvalState, actor: &str) -> bool {
    let p = &action.resources;
    p.receives
        .iter()
        .all(|cap| state.authority.get(cap).is_none_or(|owner| owner != actor))
        && p.consumes.is_disjoint(&p.transfers)
        && p.consumes.is_disjoint(&p.receives)
        && p.transfers.is_disjoint(&p.receives)
}

fn local_enabled(action: &CoreBehaviorAction, state: &EvalState, actor: &str) -> bool {
    affine_enabled(action, state, actor)
        && action.resources.transfers.is_empty()
        && action.resources.receives.is_empty()
}

fn synchronized_enabled(
    left: &CoreBehaviorAction,
    left_owner: &str,
    right: &CoreBehaviorAction,
    right_owner: &str,
    state: &EvalState,
) -> bool {
    let l = &left.resources;
    let r = &right.resources;
    left_owner != right_owner
        && affine_enabled(left, state, left_owner)
        && affine_enabled(right, state, right_owner)
        && l.transfers == r.receives
        && r.transfers == l.receives
        && l.relies.is_subset(&r.guarantees)
        && r.relies.is_subset(&l.guarantees)
}

fn apply(
    action: &CoreBehaviorAction,
    state: &mut EvalState,
    system: &str,
    enums: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), EvalError> {
    let before = state.clone();
    for (field, term) in &action.update_ast {
        let value = eval_value(term, &before, system, enums).ok_or_else(|| {
            EvalError(format!(
                "invalid finite update AST in {system}.{} for field {field}",
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

fn eval_bool(
    term: &CoreBehaviorTerm,
    state: &EvalState,
    system: &str,
    enums: &BTreeMap<String, BTreeSet<String>>,
) -> Option<bool> {
    match eval_value(term, state, system, enums)? {
        EvalValue::Bool(value) => Some(value),
        EvalValue::Unit | EvalValue::Enum { .. } => None,
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

    fn resource_sync_program() -> BehaviorCoreProgram {
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
        BehaviorCoreProgram {
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
            known_capabilities: BTreeMap::new(),
            initial_authority: BTreeMap::new(),
        }
    }

    #[test]
    fn reference_exploration_reports_sync_grade_and_transfer() {
        let program = resource_sync_program();
        let result = explore(&program, "Network", ExploreConfig::default()).unwrap();
        assert!(
            result
                .states
                .iter()
                .any(|state| state.values["Receiver.bit"] == EvalValue::Bool(true))
        );
        assert!(result.states.iter().any(|state| {
            state.authority.get("permit").map(String::as_str) == Some("Receiver")
        }));
        assert!(result.transitions.iter().any(|transition| {
            transition.grade.get("work") == Some(&3)
                && transition.transfers == ["permit: Sender -> Receiver"]
        }));
    }

    fn assert_exploration_error(program: &BehaviorCoreProgram, message: &str) {
        let decoded = BehaviorCoreProgram::from_canonical_json(&program.to_json_pretty())
            .expect("canonical representation");
        for program in [program, &decoded] {
            let error = explore(program, "Network", ExploreConfig::default())
                .expect_err("invalid composition must return an error");
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    #[test]
    fn composition_resolves_both_component_systems() {
        for left in [true, false] {
            let mut program = resource_sync_program();
            let composition = program.compositions.get_mut("Network").unwrap();
            if left {
                composition.left_system = "Unknown".to_owned();
            } else {
                composition.right_system = "Unknown".to_owned();
            }
            assert_exploration_error(&program, "unknown system 'Unknown'");
        }

        let mut program = resource_sync_program();
        let composition = program.compositions.get_mut("Network").unwrap();
        composition
            .left_system
            .clone_from(&composition.right_system);
        assert_exploration_error(&program, "must contain two distinct systems");
    }

    #[test]
    fn connection_endpoints_must_join_the_declared_components() {
        for (left, right) in [
            ("Unknown", "Receiver"),
            ("Sender", "Unknown"),
            ("Other", "Receiver"),
            ("Sender", "Other"),
            ("Sender", "Sender"),
            ("Receiver", "Receiver"),
        ] {
            let mut program = resource_sync_program();
            let mut other = program.systems["Sender"].clone();
            other.name = "Other".to_owned();
            program.systems.insert("Other".to_owned(), other);
            let composition = program.compositions.get_mut("Network").unwrap();
            composition.connections[0].left_system = left.to_owned();
            composition.connections[0].right_system = right.to_owned();
            assert_exploration_error(&program, "must join 'Receiver' and 'Sender'");
        }
    }

    #[test]
    fn connection_actions_are_resolved_before_exploration() {
        for left in [true, false] {
            let mut program = resource_sync_program();
            // Even a later, disabled connection must have valid action references.
            program
                .systems
                .get_mut("Sender")
                .unwrap()
                .actions
                .get_mut("send")
                .unwrap()
                .guard_ast
                .push(bool_term(false));
            let composition = program.compositions.get_mut("Network").unwrap();
            let mut connection = composition.connections[0].clone();
            if left {
                connection.left_action = "unknown".to_owned();
            } else {
                connection.right_action = "unknown".to_owned();
            }
            composition.connections.push(connection);
            let action = if left {
                "Sender.unknown"
            } else {
                "Receiver.unknown"
            };
            assert_exploration_error(&program, &format!("unknown action '{action}'"));
        }
    }

    #[test]
    fn composition_accepts_both_connection_orientations() {
        let mut program = resource_sync_program();
        let before = explore(&program, "Network", ExploreConfig::default()).unwrap();
        let composition = program.compositions.get_mut("Network").unwrap();
        let connection = &mut composition.connections[0];
        std::mem::swap(&mut connection.left_system, &mut connection.right_system);
        std::mem::swap(&mut connection.left_action, &mut connection.right_action);
        let after = explore(&program, "Network", ExploreConfig::default()).unwrap();
        assert_eq!(after.states, before.states);
        assert_eq!(after.transitions.len(), before.transitions.len());
        assert!(after.transitions.iter().any(|transition| {
            transition.label == "Receiver.receive|Sender.send"
                && transition.grade.get("work") == Some(&3)
                && transition.transfers == ["permit: Sender -> Receiver"]
        }));
    }

    #[test]
    fn synchronized_grade_overflow_returns_an_eval_error() {
        let mut program = resource_sync_program();
        program
            .systems
            .get_mut("Sender")
            .unwrap()
            .actions
            .get_mut("send")
            .unwrap()
            .resources
            .grade
            .insert("work".to_owned(), u64::MAX);
        assert_exploration_error(
            &program,
            "composition 'Network' at 'Sender.send|Receiver.receive': parallel resource grade overflow for 'work'",
        );
    }

    fn bool_term(value: bool) -> CoreBehaviorTerm {
        CoreBehaviorTerm::Bool {
            r#type: "Bool".to_owned(),
            value,
        }
    }

    #[test]
    fn initial_state_evaluates_closed_boolean_terms() {
        let mut program = resource_sync_program();
        let receiver = program.systems.get_mut("Receiver").unwrap();
        let bit = receiver.state.get_mut("bit").unwrap();
        bit.initial_ast = CoreBehaviorTerm::Not {
            r#type: "Bool".to_owned(),
            value: Box::new(CoreBehaviorTerm::Equal {
                r#type: "Bool".to_owned(),
                left: Box::new(bool_term(true)),
                right: Box::new(bool_term(false)),
            }),
        };
        let mut literal = bit.clone();
        literal.name = "literal".to_owned();
        literal.initial_ast = bool_term(false);
        receiver.state.insert("literal".to_owned(), literal);

        let result = explore(&program, "Receiver", ExploreConfig::default()).unwrap();
        assert_eq!(result.states[0].values["bit"], EvalValue::Bool(true));
        assert_eq!(result.states[0].values["literal"], EvalValue::Bool(false));

        let result = explore(&program, "Network", ExploreConfig::default()).unwrap();
        assert_eq!(
            result.states[0].values["Receiver.bit"],
            EvalValue::Bool(true)
        );
        assert_eq!(
            result.states[0].values["Receiver.literal"],
            EvalValue::Bool(false)
        );
    }

    #[test]
    fn initial_state_rejects_open_terms_and_unsupported_state_types() {
        for (ty, term) in [
            (
                "Bool",
                CoreBehaviorTerm::Read {
                    r#type: "Bool".to_owned(),
                    field: "bit".to_owned(),
                },
            ),
            (
                "Bool",
                CoreBehaviorTerm::Unit {
                    r#type: "Unit".to_owned(),
                },
            ),
            (
                "Color",
                CoreBehaviorTerm::Enum {
                    r#type: "Color".to_owned(),
                    constructor: "Red".to_owned(),
                },
            ),
            ("Unit", bool_term(false)),
        ] {
            let mut program = resource_sync_program();
            let receiver = program.systems.get_mut("Receiver").unwrap();
            let bit = receiver.state.get_mut("bit").unwrap();
            bit.ty = ty.to_owned();
            bit.initial_ast = term;
            let error = explore(&program, "Receiver", ExploreConfig::default())
                .expect_err("unsupported initializer must return an error");
            assert!(
                error
                    .to_string()
                    .contains("closed initializer of declared Bool/Unit/enum type")
            );
        }
    }
}

#[cfg(test)]
mod value_tests;

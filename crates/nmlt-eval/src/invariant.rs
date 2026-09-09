//! Candidate safety evidence from the common exploration step implementation.
use super::{EvalError, EvalState, EvalValue, ExploreConfig, explore, value::eval_value};
use nmlt_ir::{
    BehaviorCoreProgram, CoreBehaviorSystem, SafetyClaim, SafetyPredicate, SafetyProperty,
};
use std::collections::VecDeque;

pub fn evaluate_predicate(
    core: &BehaviorCoreProgram,
    property: &SafetyProperty,
    state: &EvalState,
) -> Result<bool, EvalError> {
    fn go(
        core: &BehaviorCoreProgram,
        system: &str,
        p: &SafetyPredicate,
        state: &EvalState,
        depth: usize,
    ) -> Result<bool, EvalError> {
        let depth = depth
            .checked_sub(1)
            .ok_or_else(|| EvalError("predicate depth exceeds 64".into()))?;
        Ok(match p {
            SafetyPredicate::Boolean { term } => {
                let Some(EvalValue::Bool(value)) = eval_value(term, state, system, &core.enums)
                else {
                    return Err(EvalError("predicate has an invalid finite term".into()));
                };
                value
            }
            SafetyPredicate::Not { value } => !go(core, system, value, state, depth)?,
            SafetyPredicate::And { left, right } => {
                go(core, system, left, state, depth)? && go(core, system, right, state, depth)?
            }
            SafetyPredicate::Or { left, right } => {
                go(core, system, left, state, depth)? || go(core, system, right, state, depth)?
            }
            SafetyPredicate::Implies { left, right } => {
                !go(core, system, left, state, depth)? || go(core, system, right, state, depth)?
            }
        })
    }
    go(core, &property.system, &property.predicate, state, 64)
}

/// Bound the complete Lean semantic universe, not only the Rust visited prefix.
pub fn safety_universe(
    core: &BehaviorCoreProgram,
    behavior: &str,
) -> Result<(usize, usize), EvalError> {
    let fail = || EvalError("safety checking exceeds the finite universe/action bound".into());
    let composition = core
        .compositions
        .get(behavior)
        .ok_or_else(|| EvalError("safety requires a binary composition".into()))?;
    let left = core
        .systems
        .get(&composition.left_system)
        .ok_or_else(fail)?;
    let right = core
        .systems
        .get(&composition.right_system)
        .ok_or_else(fail)?;
    let size = |s: &CoreBehaviorSystem| -> Result<usize, EvalError> {
        let mut count = 1usize;
        for field in s.state.values() {
            let values = match field.ty.as_str() {
                "Bool" => 2,
                "Unit" => 1,
                name => core.enums.get(name).ok_or_else(fail)?.len(),
            };
            count = count
                .checked_mul(values)
                .filter(|n| (1..=512).contains(n))
                .ok_or_else(fail)?;
        }
        Ok(count + 1)
    };
    let capabilities = core.initial_authority.get(behavior).ok_or_else(fail)?.len();
    if capabilities > 4 {
        return Err(fail());
    }
    let worlds = 3usize.pow(capabilities as u32 + 1);
    let states = size(left)?
        .checked_mul(size(right)?)
        .and_then(|n| n.checked_mul(worlds))
        .filter(|n| *n <= 512)
        .ok_or_else(fail)?;
    let l = left.actions.len() + 1;
    let r = right.actions.len() + 1;
    let actions = l
        .checked_mul(r)
        .and_then(|n| n.checked_add(l + r))
        .filter(|n| *n <= 256)
        .ok_or_else(fail)?;
    Ok((states, actions))
}

pub fn safety_claim(
    core: &BehaviorCoreProgram,
    behavior: &str,
    property: &SafetyProperty,
    core_sha256: String,
    max_states: usize,
) -> Result<SafetyClaim, EvalError> {
    if !(1..=256).contains(&max_states) {
        return Err(EvalError("safety max-states must be in 1..256".into()));
    }
    let (universe, actions) = safety_universe(core, behavior)?;
    let composition = core
        .compositions
        .get(behavior)
        .ok_or_else(|| EvalError("unknown composition".into()))?;
    if property.system != composition.left_system && property.system != composition.right_system {
        return Err(EvalError(
            "selected property is not on a leaf of this composition".into(),
        ));
    }
    let graph = explore(core, behavior, ExploreConfig { max_states })?;
    let mut violation = None;
    for (index, state) in graph.states.iter().enumerate() {
        if !evaluate_predicate(core, property, state)? {
            violation = Some(index);
            break;
        }
    }
    if let Some(mut index) = violation {
        let mut parents = vec![None; graph.states.len()];
        let mut seen = vec![false; graph.states.len()];
        seen[0] = true;
        let mut queue = VecDeque::from([0]);
        while let Some(before) = queue.pop_front() {
            for edge in graph.transitions.iter().filter(|e| e.from == before) {
                if !seen[edge.to] {
                    seen[edge.to] = true;
                    parents[edge.to] = Some((before, edge.label.clone()));
                    queue.push_back(edge.to);
                }
            }
        }
        let mut labels = vec![];
        while index != 0 {
            let (previous, label) = parents[index]
                .as_ref()
                .ok_or_else(|| EvalError("counterexample has no initialized path".into()))?;
            labels.push(label.clone());
            index = *previous;
        }
        labels.reverse();
        return Ok(SafetyClaim::Counterexample {
            path: super::execution_path(core, &graph, core_sha256, &labels)?,
        });
    }
    if graph.truncated {
        return Err(EvalError(
            "incomplete safety exploration: max-states exhausted without a counterexample".into(),
        ));
    }
    if universe * actions * graph.states.len() > 1_000_000 {
        return Err(EvalError(
            "safety preservation exceeds one million finite obligations".into(),
        ));
    }
    Ok(SafetyClaim::Invariant {
        states: graph
            .states
            .iter()
            .map(|state| super::encode_execution_state(core, behavior, state))
            .collect::<Result<_, _>>()?,
    })
}

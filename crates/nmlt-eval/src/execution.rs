use super::{EvalError, EvalState, EvalValue, Exploration};
use nmlt_ir::{
    BehaviorCoreProgram, CoreBehaviorSystem, ExecutionAction, ExecutionPath, ExecutionState,
};

/// Materialize a selected path from reference exploration. Lean must check it.
pub fn execution_path(
    program: &BehaviorCoreProgram,
    graph: &Exploration,
    artifact_sha256: String,
    labels: &[String],
) -> Result<ExecutionPath, EvalError> {
    if program.schema != nmlt_ir::BEHAVIOR_CORE_V2_SCHEMA {
        return Err(EvalError(
            "execution witnesses require behavior-core-v2".into(),
        ));
    }
    let composition = program
        .compositions
        .get(&graph.behavior)
        .ok_or_else(|| EvalError("execution witness requires a binary composition".into()))?;
    let left = program
        .systems
        .get(&composition.left_system)
        .ok_or_else(|| EvalError("unknown left leaf".into()))?;
    let right = program
        .systems
        .get(&composition.right_system)
        .ok_or_else(|| EvalError("unknown right leaf".into()))?;
    let initial_authority = program
        .initial_authority
        .get(&graph.behavior)
        .ok_or_else(|| EvalError("missing initial authority".into()))?;
    let encode = |state: &EvalState| -> Result<ExecutionState, EvalError> {
        Ok(ExecutionState {
            left: control_index(program, left, state)?,
            right: control_index(program, right, state)?,
            authority: initial_authority
                .keys()
                .map(|cap| (cap.clone(), state.authority.get(cap).cloned()))
                .collect(),
        })
    };
    let initial = graph
        .states
        .first()
        .ok_or_else(|| EvalError("empty exploration".into()))?;
    let mut path = ExecutionPath {
        schema: "behavior-execution-v1".into(),
        artifact_sha256,
        behavior: graph.behavior.clone(),
        states: vec![encode(initial)?],
        actions: vec![],
    };
    let mut current = 0;
    for label in labels {
        let candidates: Vec<_> = graph
            .transitions
            .iter()
            .filter(|edge| edge.from == current && edge.label == *label)
            .collect();
        let [edge] = candidates.as_slice() else {
            return Err(EvalError(format!(
                "expected one enabled transition '{label}' from state {current}"
            )));
        };
        let mut action = ExecutionAction {
            left: None,
            right: None,
        };
        for endpoint in label.split('|') {
            if let Some(name) = endpoint.strip_prefix(&format!("{}.", left.name)) {
                action.left = Some(name.into());
            } else if let Some(name) = endpoint.strip_prefix(&format!("{}.", right.name)) {
                action.right = Some(name.into());
            } else {
                return Err(EvalError("unknown transition participant".into()));
            }
        }
        current = edge.to;
        path.actions.push(action);
        path.states
            .push(encode(graph.states.get(current).ok_or_else(|| {
                EvalError("transition target is outside the graph".into())
            })?)?);
    }
    Ok(path)
}

fn control_index(
    program: &BehaviorCoreProgram,
    system: &CoreBehaviorSystem,
    state: &EvalState,
) -> Result<usize, EvalError> {
    let mut index: usize = 0;
    for (name, declaration) in &system.state {
        let value = state
            .values
            .get(&format!("{}.{name}", system.name))
            .ok_or_else(|| EvalError("missing control value".into()))?;
        let (size, offset) = match value {
            EvalValue::Bool(value) if declaration.ty == "Bool" => (2, usize::from(*value)),
            EvalValue::Unit if declaration.ty == "Unit" => (1, 0),
            EvalValue::Enum {
                type_name,
                constructor,
            } if *type_name == declaration.ty => {
                let variants = program
                    .enums
                    .get(type_name)
                    .ok_or_else(|| EvalError("unknown enum type".into()))?;
                let offset = variants
                    .iter()
                    .position(|v| v == constructor)
                    .ok_or_else(|| EvalError("invalid enum constructor".into()))?;
                (variants.len(), offset)
            }
            _ => {
                return Err(EvalError(
                    "state value does not match its finite domain".into(),
                ));
            }
        };
        index = index
            .checked_mul(size)
            .and_then(|v| v.checked_add(offset))
            .ok_or_else(|| EvalError("control index overflow".into()))?;
    }
    Ok(index)
}

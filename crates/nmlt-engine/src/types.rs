use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{BinaryOp, Expr, Model, Type, UnaryOp, Value};

/// A model after deterministic name, type, frame, capability, and property
/// indexing checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedModel {
    pub model: Model,
    pub state_types: BTreeMap<String, Type>,
    pub frames: BTreeMap<String, Vec<String>>,
    pub property_behavior: BTreeMap<String, String>,
}

pub fn type_check(mut model: Model) -> Result<TypedModel, Vec<String>> {
    let mut errors = Vec::new();
    let mut state_types = BTreeMap::new();
    for state in &model.states {
        if state_types
            .insert(state.name.clone(), state.ty.clone())
            .is_some()
        {
            errors.push(format!(
                "NMLT2101 at {}: duplicate state `{}`",
                state.span.start, state.name
            ));
        }
    }

    let mut action_names = BTreeSet::new();
    for action in &model.actions {
        if !action_names.insert(action.name.clone()) {
            errors.push(format!(
                "NMLT2102 at {}: duplicate action `{}`",
                action.span.start, action.name
            ));
        }
    }

    let mut capabilities = BTreeSet::new();
    for capability in &model.capabilities {
        if !capabilities.insert(capability.clone()) {
            errors.push(format!("NMLT2103: duplicate capability `{capability}`"));
        }
    }

    // Constructor names and state names occupy distinct source namespaces.
    // The executable AST records the distinction explicitly after contextual
    // elaboration so runtime lookup can never guess. This matters for models
    // such as ProviderAttempt, whose `dispatched` constructor and Boolean
    // `dispatched` state intentionally share a spelling.
    for state in &mut model.states {
        resolve_contextual_symbols(&mut state.initial, Some(&state.ty), &state_types);
    }
    for action in &mut model.actions {
        for guard in &mut action.guards {
            resolve_contextual_symbols(guard, Some(&Type::Bool), &state_types);
        }
        for update in &mut action.updates {
            let expected = state_types.get(&update.target);
            resolve_contextual_symbols(&mut update.value, expected, &state_types);
        }
    }
    for property in &mut model.properties {
        resolve_contextual_symbols(&mut property.expression, Some(&Type::Bool), &state_types);
    }

    for state in &model.states {
        match expression_type(&state.initial, &state_types, &action_names) {
            Ok(actual) if compatible(&state.ty, &actual) => {}
            Ok(actual) => errors.push(format!(
                "NMLT2104 at {}: initializer for `{}` has type {actual:?}, expected {:?}",
                state.span.start, state.name, state.ty
            )),
            Err(error) => errors.push(error),
        }
        if state.ty == Type::Nat
            && matches!(&state.initial, Expr::Value(Value::Int(value)) if *value < 0)
        {
            errors.push(format!(
                "NMLT2105 at {}: natural state `{}` cannot start negative",
                state.span.start, state.name
            ));
        }
    }

    let all_states = state_types.keys().cloned().collect::<BTreeSet<_>>();
    let mut frames = BTreeMap::new();
    for action in &model.actions {
        for guard in &action.guards {
            match expression_type(guard, &state_types, &action_names) {
                Ok(Type::Bool) => {}
                Ok(actual) => errors.push(format!(
                    "NMLT2106 at {}: guard in `{}` has type {actual:?}, expected Bool",
                    action.span.start, action.name
                )),
                Err(error) => errors.push(error),
            }
        }

        let mut written = BTreeSet::new();
        for update in &action.updates {
            let Some(expected) = state_types.get(&update.target) else {
                errors.push(format!(
                    "NMLT2107 at {}: update target `{}` is not declared state",
                    update.span.start, update.target
                ));
                continue;
            };
            if !written.insert(update.target.clone()) {
                errors.push(format!(
                    "NMLT2108 at {}: action `{}` updates `{}` more than once",
                    update.span.start, action.name, update.target
                ));
            }
            match expression_type(&update.value, &state_types, &action_names) {
                Ok(actual) if compatible(expected, &actual) => {}
                Ok(actual) => errors.push(format!(
                    "NMLT2109 at {}: update of `{}` has type {actual:?}, expected {expected:?}",
                    update.span.start, update.target
                )),
                Err(error) => errors.push(error),
            }
        }
        let mut consumed = BTreeSet::new();
        for capability in &action.consumes {
            if !capabilities.contains(capability) {
                errors.push(format!(
                    "NMLT2110 at {}: action `{}` consumes undeclared capability `{capability}`",
                    action.span.start, action.name
                ));
            }
            if !consumed.insert(capability) {
                errors.push(format!(
                    "NMLT2111 at {}: linear capability `{capability}` is consumed twice in `{}`",
                    action.span.start, action.name
                ));
            }
        }
        frames.insert(
            action.name.clone(),
            all_states.difference(&written).cloned().collect(),
        );
    }

    let mut property_names = BTreeSet::new();
    let mut property_behavior = BTreeMap::new();
    for property in &model.properties {
        if !property_names.insert(property.name.clone()) {
            errors.push(format!(
                "NMLT2112 at {}: duplicate property `{}`",
                property.span.start, property.name
            ));
        }
        match expression_type(&property.expression, &state_types, &action_names) {
            Ok(Type::Bool) => {}
            Ok(actual) => errors.push(format!(
                "NMLT2113 at {}: property `{}` has type {actual:?}, expected Bool",
                property.span.start, property.name
            )),
            Err(error) => errors.push(error),
        }
        property_behavior.insert(property.name.clone(), model.system_name.clone());
    }

    for observation in &model.observations {
        if !state_types.contains_key(observation) {
            errors.push(format!(
                "NMLT2114: observation `{observation}` is not declared state"
            ));
        }
    }

    if errors.is_empty() {
        Ok(TypedModel {
            model,
            state_types,
            frames,
            property_behavior,
        })
    } else {
        Err(errors)
    }
}

fn resolve_contextual_symbols(
    expression: &mut Expr,
    expected: Option<&Type>,
    states: &BTreeMap<String, Type>,
) {
    if let (Expr::Name(name), Some(Type::Named(expected_name))) = (&*expression, expected) {
        if !matches!(states.get(name), Some(Type::Named(actual)) if actual == expected_name) {
            *expression = Expr::Value(Value::Symbol(name.clone()));
        }
        return;
    }

    match expression {
        Expr::Unary { operand, .. } => resolve_contextual_symbols(operand, None, states),
        Expr::Binary { op, left, right } => {
            if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
                let left_type = expression_type(left, states, &BTreeSet::new()).ok();
                let right_type = expression_type(right, states, &BTreeSet::new()).ok();
                match (left_type.as_ref(), right_type.as_ref()) {
                    (Some(named @ Type::Named(_)), Some(actual)) if !compatible(named, actual) => {
                        resolve_contextual_symbols(right, Some(named), states);
                    }
                    (Some(actual), Some(named @ Type::Named(_))) if !compatible(named, actual) => {
                        resolve_contextual_symbols(left, Some(named), states);
                    }
                    _ => {}
                }
            }
            resolve_contextual_symbols(left, None, states);
            resolve_contextual_symbols(right, None, states);
        }
        Expr::Call { arguments, .. } => {
            for argument in arguments {
                resolve_contextual_symbols(argument, None, states);
            }
        }
        Expr::Value(_) | Expr::Name(_) => {}
    }
}

fn expression_type(
    expression: &Expr,
    states: &BTreeMap<String, Type>,
    actions: &BTreeSet<String>,
) -> Result<Type, String> {
    match expression {
        Expr::Value(Value::Bool(_)) => Ok(Type::Bool),
        Expr::Value(Value::Int(_)) => Ok(Type::Int),
        Expr::Value(Value::Symbol(_)) => Ok(Type::Symbol),
        Expr::Name(name) => Ok(states.get(name).cloned().unwrap_or(Type::Symbol)),
        Expr::Unary { op, operand } => {
            let operand = expression_type(operand, states, actions)?;
            match (op, operand) {
                (UnaryOp::Not, Type::Bool) => Ok(Type::Bool),
                (UnaryOp::Negate, Type::Int | Type::Nat) => Ok(Type::Int),
                (op, actual) => Err(format!(
                    "NMLT2115: unary operator {op:?} does not accept {actual:?}"
                )),
            }
        }
        Expr::Binary { op, left, right } => {
            let left = expression_type(left, states, actions)?;
            let right = expression_type(right, states, actions)?;
            match op {
                BinaryOp::Implies | BinaryOp::Or | BinaryOp::And => {
                    if left == Type::Bool && right == Type::Bool {
                        Ok(Type::Bool)
                    } else {
                        Err(format!(
                            "NMLT2116: Boolean operator {op:?} received {left:?} and {right:?}"
                        ))
                    }
                }
                BinaryOp::Equal | BinaryOp::NotEqual => {
                    if compatible(&left, &right) {
                        Ok(Type::Bool)
                    } else {
                        Err(format!(
                            "NMLT2117: equality compares incompatible {left:?} and {right:?}"
                        ))
                    }
                }
                BinaryOp::Greater
                | BinaryOp::GreaterEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual => {
                    if numeric(&left) && numeric(&right) {
                        Ok(Type::Bool)
                    } else {
                        Err(format!(
                            "NMLT2118: comparison requires numbers, received {left:?} and {right:?}"
                        ))
                    }
                }
                BinaryOp::Add | BinaryOp::Subtract => {
                    if numeric(&left) && numeric(&right) {
                        Ok(Type::Int)
                    } else {
                        Err(format!(
                            "NMLT2119: arithmetic requires numbers, received {left:?} and {right:?}"
                        ))
                    }
                }
            }
        }
        Expr::Call { name, arguments } => match name.as_str() {
            "always" | "next" | "eventually" => {
                if arguments.len() != 1 {
                    return Err(format!("NMLT2120: `{name}` expects one argument"));
                }
                let actual = expression_type(&arguments[0], states, actions)?;
                if actual == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    Err(format!(
                        "NMLT2121: `{name}` requires Bool, received {actual:?}"
                    ))
                }
            }
            "enabled" => {
                let [Expr::Name(action)] = arguments.as_slice() else {
                    return Err("NMLT2122: `enabled` expects exactly one action name".to_owned());
                };
                if actions.contains(action) {
                    Ok(Type::Bool)
                } else {
                    Err(format!("NMLT2123: unknown action `{action}` in `enabled`"))
                }
            }
            _ => Err(format!("NMLT2124: unsupported executable call `{name}`")),
        },
    }
}

fn compatible(expected: &Type, actual: &Type) -> bool {
    expected == actual
        || matches!(
            (expected, actual),
            (Type::Nat, Type::Int) | (Type::Int, Type::Nat)
        )
        || matches!((expected, actual), (Type::Named(_), Type::Symbol))
        || matches!((expected, actual), (Type::Symbol, Type::Named(_)))
}

fn numeric(ty: &Type) -> bool {
    matches!(ty, Type::Nat | Type::Int)
}

#[cfg(test)]
mod tests {
    use crate::{compile, parse_model};

    #[test]
    fn derives_explicit_frames() {
        let typed = compile(
            "system S {\n state x: Nat = 0\n state ok: Bool = false\n action inc { set x = x + 1 }\n safety Safe = always(x >= 0)\n }",
        )
        .unwrap();
        assert_eq!(typed.frames["inc"], ["ok"]);
    }

    #[test]
    fn rejects_duplicate_linear_consumption() {
        let model = parse_model(
            "system S {\n state x: Nat = 0\n capability c: Once<E>\n action fire { consume c; consume c; set x = 1 }\n safety Safe = always(x >= 0)\n }",
        )
        .unwrap();
        let errors = super::type_check(model).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("consumed twice")));
    }

    #[test]
    fn rejects_cross_system_or_unknown_property_names_by_construction() {
        let typed =
            compile("system A {\n state x: Bool = false\n safety P = always(not x)\n }").unwrap();
        assert_eq!(typed.property_behavior["P"], "A");
        assert!(!typed.property_behavior.contains_key("B.P"));
    }
}

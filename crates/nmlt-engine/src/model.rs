use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::ast::{Action, BinaryOp, Expr, Property, UnaryOp, Value};
use crate::types::TypedModel;

/// Explicit limits are part of the meaning of a bounded result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckConfig {
    pub max_states: usize,
    pub max_depth: usize,
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            max_states: 10_000,
            max_depth: 100,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultClass {
    ModelChecked,
    Refuted,
    Unknown,
}

impl ResultClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModelChecked => "model_checked",
            Self::Refuted => "refuted",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceStep {
    pub index: usize,
    /// `None` identifies the initial state; `Some("stutter")` is explicit.
    pub action: Option<String>,
    pub state: BTreeMap<String, Value>,
    pub available_capabilities: BTreeSet<String>,
    /// Actions enabled in this state under the exact guards and affine
    /// capability availability. This makes zero-transition state violations
    /// inspectable instead of requiring an inferred outgoing edge.
    pub enabled_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trace {
    pub steps: Vec<TraceStep>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyResult {
    pub property: String,
    pub class: ResultClass,
    pub reason: String,
    pub witness: Option<Trace>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckReport {
    pub semantic_binding: crate::SemanticBinding,
    pub system: String,
    pub class: ResultClass,
    pub complete: bool,
    pub explored_states: usize,
    pub explored_transitions: usize,
    pub config: CheckConfig,
    pub properties: Vec<PropertyResult>,
}

impl CheckReport {
    /// Deterministic structured output. Map and set ordering is canonicalized by
    /// their `BTree*` representation; this is the stable counterexample
    /// renderer, not a cryptographic evidence envelope.
    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        let properties = self
            .properties
            .iter()
            .map(property_json)
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            concat!(
                "{{\n",
                "  \"schema_version\": \"1.1.0\",\n",
                "  \"semantic_binding\": {},\n",
                "  \"system\": {},\n",
                "  \"result\": {},\n",
                "  \"complete\": {},\n",
                "  \"explored_states\": {},\n",
                "  \"explored_transitions\": {},\n",
                "  \"bounds\": {{ \"max_states\": {}, \"max_depth\": {} }},\n",
                "  \"properties\": [\n{}\n  ]\n",
                "}}"
            ),
            semantic_binding_json(&self.semantic_binding),
            json_string(&self.system),
            json_string(self.class.as_str()),
            self.complete,
            self.explored_states,
            self.explored_transitions,
            self.config.max_states,
            self.config.max_depth,
            indent(&properties, 4),
        )
    }
}

fn semantic_binding_json(binding: &crate::SemanticBinding) -> String {
    format!(
        concat!(
            "{{ \"source_set_id\": {}, \"module_map_id\": {}, ",
            "\"surface_program_id\": {}, \"resolved_hir_id\": {}, ",
            "\"core_program_id\": {}, \"ruleset_bundle_id\": {}, ",
            "\"resource_policy_id\": {}, \"certificate_id\": {}, ",
            "\"kernel_profile_id\": {} }}"
        ),
        json_string(&binding.source_set_id),
        json_string(&binding.module_map_id),
        json_string(&binding.surface_program_id),
        json_string(&binding.resolved_hir_id),
        json_string(&binding.core_program_id),
        json_string(&binding.ruleset_bundle_id),
        json_string(&binding.resource_policy_id),
        json_string(&binding.certificate_id),
        json_string(&binding.kernel_profile_id),
    )
}

fn property_json(property: &PropertyResult) -> String {
    let witness = property
        .witness
        .as_ref()
        .map_or_else(|| "null".to_owned(), trace_json);
    format!(
        concat!(
            "{{\n",
            "  \"property\": {},\n",
            "  \"result\": {},\n",
            "  \"reason\": {},\n",
            "  \"witness\": {}\n",
            "}}"
        ),
        json_string(&property.property),
        json_string(property.class.as_str()),
        json_string(&property.reason),
        indent(&witness, 2).trim_start(),
    )
}

fn trace_json(trace: &Trace) -> String {
    let steps = trace
        .steps
        .iter()
        .map(|step| {
            let action = step
                .action
                .as_ref()
                .map_or_else(|| "null".to_owned(), |action| json_string(action));
            let state = step
                .state
                .iter()
                .map(|(name, value)| format!("{}: {}", json_string(name), value_json(value)))
                .collect::<Vec<_>>()
                .join(", ");
            let capabilities = step
                .available_capabilities
                .iter()
                .map(|item| json_string(item))
                .collect::<Vec<_>>()
                .join(", ");
            let enabled_actions = step
                .enabled_actions
                .iter()
                .map(|item| json_string(item))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                concat!(
                    "{{ \"index\": {}, \"action\": {}, \"state\": {{ {} }}, ",
                    "\"available_capabilities\": [{}], \"enabled_actions\": [{}] }}"
                ),
                step.index, action, state, capabilities, enabled_actions
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!("{{\n  \"steps\": [\n{}\n  ]\n}}", indent(&steps, 4))
}

fn value_json(value: &Value) -> String {
    match value {
        Value::Bool(value) => value.to_string(),
        Value::Int(value) => value.to_string(),
        Value::Symbol(value) => json_string(value),
    }
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                use std::fmt::Write;
                write!(output, "\\u{:04x}", u32::from(character))
                    .expect("writing to String cannot fail");
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn indent(value: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RuntimeState {
    values: BTreeMap<String, Value>,
    available_capabilities: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct Node {
    state: RuntimeState,
    depth: usize,
    parent: Option<(usize, String)>,
}

/// Exhaustively explore the reachable finite graph up to the declared bounds.
///
/// A `model_checked` result is returned only when the frontier is exhausted.
/// Hitting either bound produces `unknown`, never success.
pub fn check_model(typed: &TypedModel, config: CheckConfig) -> Result<CheckReport, Vec<String>> {
    if config.max_states == 0 {
        return Err(vec!["NMLT2201: max_states must be positive".to_owned()]);
    }
    if typed.model.properties.is_empty() {
        return Err(vec![
            "NMLT2202: model checking requires at least one declared property".to_owned(),
        ]);
    }

    let initial = initialize(typed)?;
    let mut nodes = vec![Node {
        state: initial.clone(),
        depth: 0,
        parent: None,
    }];
    let mut indices = BTreeMap::from([(initial, 0usize)]);
    let mut frontier = VecDeque::from([0usize]);
    let mut property_results = typed
        .model
        .properties
        .iter()
        .map(|property| PropertyResult {
            property: property.name.clone(),
            class: ResultClass::ModelChecked,
            reason: "holds on every reachable state and transition".to_owned(),
            witness: None,
        })
        .collect::<Vec<_>>();
    let mut explored_transitions = 0usize;
    let mut complete = true;

    while let Some(node_index) = frontier.pop_front() {
        let node = nodes[node_index].clone();
        check_state_properties(
            typed,
            &nodes,
            node_index,
            &node.state,
            &mut property_results,
        )?;

        let mut successors = Vec::new();
        for action in &typed.model.actions {
            if action_enabled(action, &node.state, typed).map_err(|error| vec![error])? {
                let next = apply_action(action, &node.state, typed)?;
                successors.push((action.name.clone(), next));
            }
        }
        successors.sort_by(|(left_action, left_state), (right_action, right_state)| {
            left_action
                .cmp(right_action)
                .then_with(|| left_state.cmp(right_state))
        });

        if successors.is_empty() {
            // NMLT behaviors are total through stuttering. The stutter edge is
            // semantically observable to `next`, but it does not enlarge BFS.
            explored_transitions += 1;
            check_transition_properties(
                typed,
                &nodes,
                node_index,
                "stutter",
                &node.state,
                &node.state,
                &mut property_results,
            )?;
        }

        for (action, successor) in successors {
            explored_transitions += 1;
            check_transition_properties(
                typed,
                &nodes,
                node_index,
                &action,
                &node.state,
                &successor,
                &mut property_results,
            )?;

            if !indices.contains_key(&successor) {
                if node.depth >= config.max_depth || nodes.len() >= config.max_states {
                    complete = false;
                    continue;
                }
                let successor_index = nodes.len();
                indices.insert(successor.clone(), successor_index);
                nodes.push(Node {
                    state: successor,
                    depth: node.depth + 1,
                    parent: Some((node_index, action)),
                });
                frontier.push_back(successor_index);
            }
        }

        if property_results
            .iter()
            .all(|result| result.class == ResultClass::Refuted)
        {
            complete = frontier.is_empty();
            break;
        }
    }

    for (property, result) in typed.model.properties.iter().zip(&mut property_results) {
        if result.class == ResultClass::Refuted {
            continue;
        }
        if contains_eventuality(&property.expression) {
            result.class = ResultClass::Unknown;
            result.reason = "eventuality checking requires the Phase 4 lasso engine".to_owned();
        } else if !complete {
            result.class = ResultClass::Unknown;
            result.reason = format!(
                "reachable frontier exceeded max_states={} or max_depth={}",
                config.max_states, config.max_depth
            );
        }
    }

    let class = if property_results
        .iter()
        .any(|result| result.class == ResultClass::Refuted)
    {
        ResultClass::Refuted
    } else if !complete
        || property_results
            .iter()
            .any(|result| result.class == ResultClass::Unknown)
    {
        ResultClass::Unknown
    } else {
        ResultClass::ModelChecked
    };

    Ok(CheckReport {
        semantic_binding: typed.semantic_binding.clone(),
        system: typed.model.system_name.clone(),
        class,
        complete,
        explored_states: nodes.len(),
        explored_transitions,
        config,
        properties: property_results,
    })
}

fn initialize(typed: &TypedModel) -> Result<RuntimeState, Vec<String>> {
    let mut state = RuntimeState {
        values: BTreeMap::new(),
        available_capabilities: typed.model.capabilities.iter().cloned().collect(),
    };
    let mut errors = Vec::new();
    for variable in &typed.model.states {
        match evaluate(&variable.initial, &state, None, typed) {
            Ok(value) => {
                if variable.ty == crate::ast::Type::Nat
                    && matches!(value, Value::Int(number) if number < 0)
                {
                    errors.push(format!(
                        "NMLT2202: natural state `{}` initialized below zero",
                        variable.name
                    ));
                } else {
                    state.values.insert(variable.name.clone(), value);
                }
            }
            Err(error) => errors.push(format!(
                "NMLT2203: cannot initialize `{}`: {error}",
                variable.name
            )),
        }
    }
    if errors.is_empty() {
        Ok(state)
    } else {
        Err(errors)
    }
}

fn action_enabled(
    action: &Action,
    state: &RuntimeState,
    typed: &TypedModel,
) -> Result<bool, String> {
    if action
        .consumes
        .iter()
        .any(|capability| !state.available_capabilities.contains(capability))
    {
        return Ok(false);
    }
    for guard in &action.guards {
        if !as_bool(evaluate(guard, state, None, typed)?)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn apply_action(
    action: &Action,
    state: &RuntimeState,
    typed: &TypedModel,
) -> Result<RuntimeState, Vec<String>> {
    let mut next = state.clone();
    let mut errors = Vec::new();
    let mut updates = Vec::new();
    for update in &action.updates {
        match evaluate(&update.value, state, None, typed) {
            Ok(value) => {
                if typed.state_types.get(&update.target) == Some(&crate::ast::Type::Nat)
                    && matches!(value, Value::Int(number) if number < 0)
                {
                    errors.push(format!(
                        "NMLT2204: action `{}` makes natural state `{}` negative",
                        action.name, update.target
                    ));
                } else {
                    updates.push((update.target.clone(), value));
                }
            }
            Err(error) => errors.push(format!(
                "NMLT2205: action `{}` cannot update `{}`: {error}",
                action.name, update.target
            )),
        }
    }
    for capability in &action.consumes {
        if !next.available_capabilities.remove(capability) {
            errors.push(format!(
                "NMLT2206: action `{}` consumes unavailable capability `{capability}`",
                action.name
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    for (target, value) in updates {
        next.values.insert(target, value);
    }
    Ok(next)
}

fn check_state_properties(
    typed: &TypedModel,
    nodes: &[Node],
    node_index: usize,
    state: &RuntimeState,
    results: &mut [PropertyResult],
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for (property, result) in typed.model.properties.iter().zip(results) {
        if result.class == ResultClass::Refuted
            || contains_next(&property.expression)
            || contains_eventuality(&property.expression)
        {
            continue;
        }
        match evaluate_property(property, state, None, typed) {
            Ok(true) => {}
            Ok(false) => {
                result.class = ResultClass::Refuted;
                result.reason = format!(
                    "property `{}` is false in reachable state {node_index}",
                    property.name
                );
                result.witness = Some(
                    trace_to(nodes, node_index, None, typed)
                        .map_err(|error| vec![format!("NMLT2210: {error}")])?,
                );
            }
            Err(error) => errors.push(format!("NMLT2207 in `{}`: {error}", property.name)),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[allow(clippy::too_many_arguments)]
fn check_transition_properties(
    typed: &TypedModel,
    nodes: &[Node],
    node_index: usize,
    action: &str,
    source: &RuntimeState,
    target: &RuntimeState,
    results: &mut [PropertyResult],
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for (property, result) in typed.model.properties.iter().zip(results) {
        if result.class == ResultClass::Refuted
            || !contains_next(&property.expression)
            || contains_eventuality(&property.expression)
        {
            continue;
        }
        match evaluate_property(property, source, Some(target), typed) {
            Ok(true) => {}
            Ok(false) => {
                result.class = ResultClass::Refuted;
                result.reason = format!(
                    "property `{}` is false across `{action}` transition",
                    property.name
                );
                result.witness = Some(
                    trace_to(
                        nodes,
                        node_index,
                        Some((action.to_owned(), target.clone())),
                        typed,
                    )
                    .map_err(|error| vec![format!("NMLT2211: {error}")])?,
                );
            }
            Err(error) => errors.push(format!("NMLT2208 in `{}`: {error}", property.name)),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn evaluate_property(
    property: &Property,
    state: &RuntimeState,
    successor: Option<&RuntimeState>,
    typed: &TypedModel,
) -> Result<bool, String> {
    let expression = match &property.expression {
        Expr::Call { name, arguments } if name == "always" => &arguments[0],
        expression => expression,
    };
    as_bool(evaluate(expression, state, successor, typed)?)
}

fn evaluate(
    expression: &Expr,
    state: &RuntimeState,
    successor: Option<&RuntimeState>,
    typed: &TypedModel,
) -> Result<Value, String> {
    match expression {
        Expr::Value(value) => Ok(value.clone()),
        Expr::Name(name) => Ok(state
            .values
            .get(name)
            .cloned()
            .unwrap_or_else(|| Value::Symbol(name.clone()))),
        Expr::Unary { op, operand } => {
            let operand = evaluate(operand, state, successor, typed)?;
            match (op, operand) {
                (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
                (UnaryOp::Negate, Value::Int(value)) => value
                    .checked_neg()
                    .map(Value::Int)
                    .ok_or_else(|| "integer negation overflow".to_owned()),
                (op, value) => Err(format!("operator {op:?} cannot evaluate {value:?}")),
            }
        }
        Expr::Binary { op, left, right } => {
            if *op == BinaryOp::Implies {
                let left = as_bool(evaluate(left, state, successor, typed)?)?;
                return if left {
                    Ok(Value::Bool(as_bool(evaluate(
                        right, state, successor, typed,
                    )?)?))
                } else {
                    Ok(Value::Bool(true))
                };
            }
            if *op == BinaryOp::And {
                let left = as_bool(evaluate(left, state, successor, typed)?)?;
                return if left {
                    Ok(Value::Bool(as_bool(evaluate(
                        right, state, successor, typed,
                    )?)?))
                } else {
                    Ok(Value::Bool(false))
                };
            }
            if *op == BinaryOp::Or {
                let left = as_bool(evaluate(left, state, successor, typed)?)?;
                return if left {
                    Ok(Value::Bool(true))
                } else {
                    Ok(Value::Bool(as_bool(evaluate(
                        right, state, successor, typed,
                    )?)?))
                };
            }
            let left = evaluate(left, state, successor, typed)?;
            let right = evaluate(right, state, successor, typed)?;
            evaluate_binary(*op, left, right)
        }
        Expr::Call { name, arguments } => match name.as_str() {
            "always" => evaluate(&arguments[0], state, successor, typed),
            "next" => {
                let next = successor.ok_or_else(|| {
                    "`next` requires transition semantics (including stutter)".to_owned()
                })?;
                evaluate(&arguments[0], next, Some(next), typed)
            }
            "enabled" => {
                let [Expr::Name(action_name)] = arguments.as_slice() else {
                    return Err("`enabled` requires one action name".to_owned());
                };
                let action = typed
                    .model
                    .actions
                    .iter()
                    .find(|action| action.name == *action_name)
                    .ok_or_else(|| format!("unknown action `{action_name}`"))?;
                Ok(Value::Bool(action_enabled(action, state, typed)?))
            }
            "eventually" => Err("eventuality requires lasso evaluation".to_owned()),
            other => Err(format!("unsupported executable call `{other}`")),
        },
    }
}

fn evaluate_binary(op: BinaryOp, left: Value, right: Value) -> Result<Value, String> {
    match op {
        BinaryOp::Equal => Ok(Value::Bool(left == right)),
        BinaryOp::NotEqual => Ok(Value::Bool(left != right)),
        BinaryOp::Greater
        | BinaryOp::GreaterEqual
        | BinaryOp::Less
        | BinaryOp::LessEqual
        | BinaryOp::Add
        | BinaryOp::Subtract
        | BinaryOp::Multiply => {
            let (Value::Int(left), Value::Int(right)) = (left, right) else {
                return Err(format!("numeric operator {op:?} requires integers"));
            };
            match op {
                BinaryOp::Greater => Ok(Value::Bool(left > right)),
                BinaryOp::GreaterEqual => Ok(Value::Bool(left >= right)),
                BinaryOp::Less => Ok(Value::Bool(left < right)),
                BinaryOp::LessEqual => Ok(Value::Bool(left <= right)),
                BinaryOp::Add => left
                    .checked_add(right)
                    .map(Value::Int)
                    .ok_or_else(|| "integer addition overflow".to_owned()),
                BinaryOp::Subtract => left
                    .checked_sub(right)
                    .map(Value::Int)
                    .ok_or_else(|| "integer subtraction overflow".to_owned()),
                BinaryOp::Multiply => left
                    .checked_mul(right)
                    .map(Value::Int)
                    .ok_or_else(|| "integer multiplication overflow".to_owned()),
                _ => unreachable!(),
            }
        }
        BinaryOp::Implies | BinaryOp::Or | BinaryOp::And => {
            unreachable!("short-circuit Boolean operators are handled before this function")
        }
    }
}

fn as_bool(value: Value) -> Result<bool, String> {
    match value {
        Value::Bool(value) => Ok(value),
        other => Err(format!("expected Bool, received {other:?}")),
    }
}

fn contains_next(expression: &Expr) -> bool {
    contains_call(expression, "next")
}

fn contains_eventuality(expression: &Expr) -> bool {
    contains_call(expression, "eventually")
}

fn contains_call(expression: &Expr, expected: &str) -> bool {
    match expression {
        Expr::Unary { operand, .. } => contains_call(operand, expected),
        Expr::Binary { left, right, .. } => {
            contains_call(left, expected) || contains_call(right, expected)
        }
        Expr::Call { name, arguments } => {
            name == expected
                || arguments
                    .iter()
                    .any(|argument| contains_call(argument, expected))
        }
        Expr::Value(_) | Expr::Name(_) => false,
    }
}

fn enabled_action_names(
    state: &RuntimeState,
    typed: &TypedModel,
) -> Result<BTreeSet<String>, String> {
    typed
        .model
        .actions
        .iter()
        .filter_map(|action| match action_enabled(action, state, typed) {
            Ok(true) => Some(Ok(action.name.clone())),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

fn trace_to(
    nodes: &[Node],
    index: usize,
    tail: Option<(String, RuntimeState)>,
    typed: &TypedModel,
) -> Result<Trace, String> {
    let mut chain = Vec::new();
    let mut cursor = index;
    loop {
        chain.push(cursor);
        let Some((parent, _)) = &nodes[cursor].parent else {
            break;
        };
        cursor = *parent;
    }
    chain.reverse();
    let mut steps = chain
        .into_iter()
        .enumerate()
        .map(|(step_index, node_index)| {
            Ok(TraceStep {
                index: step_index,
                action: nodes[node_index]
                    .parent
                    .as_ref()
                    .map(|(_, action)| action.clone()),
                state: nodes[node_index].state.values.clone(),
                available_capabilities: nodes[node_index].state.available_capabilities.clone(),
                enabled_actions: enabled_action_names(&nodes[node_index].state, typed)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if let Some((action, state)) = tail {
        let enabled_actions = enabled_action_names(&state, typed)?;
        steps.push(TraceStep {
            index: steps.len(),
            action: Some(action),
            state: state.values,
            available_capabilities: state.available_capabilities,
            enabled_actions,
        });
    }
    Ok(Trace { steps })
}

#[cfg(test)]
mod tests {
    use crate::{TypedModel, from_checked, model::check_model};

    use super::{CheckConfig, ResultClass};

    fn compile(source: &str) -> Result<TypedModel, String> {
        let checked = nmlt_compile::compile_single("Test", "tests/test.nmlt", source)
            .map_err(|error| error.to_string())?;
        from_checked(&checked).map_err(|errors| errors.join("; "))
    }

    #[test]
    fn finds_a_deterministic_safety_counterexample() {
        let typed = compile(
            "system S {\n state armed: Bool = false\n state sent: Bool = false\n action send { set sent = true }\n safety Safe = always(sent implies armed)\n }",
        )
        .unwrap();
        let report = check_model(&typed, CheckConfig::default()).unwrap();
        assert_eq!(report.class, ResultClass::Refuted);
        let trace = report.properties[0].witness.as_ref().unwrap();
        assert_eq!(trace.steps.len(), 2);
        assert_eq!(trace.steps[1].action.as_deref(), Some("send"));
    }

    #[test]
    fn bound_exhaustion_is_unknown() {
        let typed = compile(
            "system S {\n state n: Nat = 0\n action inc { set n = n + 1 }\n safety Safe = always(n >= 0)\n }",
        )
        .unwrap();
        let report = check_model(
            &typed,
            CheckConfig {
                max_states: 3,
                max_depth: 10,
            },
        )
        .unwrap();
        assert_eq!(report.class, ResultClass::Unknown);
        assert!(!report.complete);
    }

    #[test]
    fn propertyless_model_cannot_receive_vacuous_model_checked_evidence() {
        let typed = compile("system S {\n state safe: Bool = true\n }").unwrap();
        let errors = check_model(&typed, CheckConfig::default()).unwrap_err();
        assert_eq!(
            errors,
            vec!["NMLT2202: model checking requires at least one declared property"]
        );
    }

    #[test]
    fn missing_updates_are_frames_and_capabilities_are_consumed() {
        let typed = compile(
            "system S {\n state x: Bool = false\n state y: Bool = true\n capability once: Once<E>\n action flip { consume once; set x = true }\n safety Y = always(y)\n }",
        )
        .unwrap();
        let report = check_model(&typed, CheckConfig::default()).unwrap();
        assert_eq!(report.class, ResultClass::ModelChecked);
        assert!(report.complete);
        assert_eq!(report.explored_states, 2);
    }

    #[test]
    fn updates_read_the_same_frozen_pre_state() {
        let typed = compile(
            "system S {\n state left: Nat = 0\n state right: Nat = 1\n action swap { set left = right; set right = left }\n safety Values = always(left >= 0 and right >= 0)\n }",
        )
        .unwrap();
        let initial = super::initialize(&typed).unwrap();
        let successor = super::apply_action(&typed.model.actions[0], &initial, &typed).unwrap();
        assert_eq!(successor.values["left"], crate::Value::Int(1));
        assert_eq!(successor.values["right"], crate::Value::Int(0));
    }

    #[test]
    fn lean_provider_vector_matches_the_rust_fragment() {
        // Mirrors `NMLT.Core.Provider`: this is a checked correspondence
        // vector, not a proof that the compiler implements the Lean model.
        let typed = compile(
            "system Provider {\n state authorized: Bool = true\n state dispatched: Bool = false\n capability attempt: Once<Effect>\n action dispatch { require authorized; consume attempt; set dispatched = true }\n safety Safe = always(dispatched implies authorized)\n }",
        )
        .unwrap();
        let initial = super::initialize(&typed).unwrap();
        let successor = super::apply_action(&typed.model.actions[0], &initial, &typed).unwrap();
        assert_eq!(successor.values["authorized"], crate::Value::Bool(true));
        assert_eq!(successor.values["dispatched"], crate::Value::Bool(true));
        assert!(!successor.available_capabilities.contains("attempt"));
        let report = check_model(&typed, CheckConfig::default()).unwrap();
        assert_eq!(report.class, ResultClass::ModelChecked);
        assert!(report.complete);
    }

    #[test]
    fn next_is_evaluated_only_after_the_source_antecedent() {
        let typed = compile(
            "enum Phase { dispatched, indeterminate }\nsystem S {\n state phase: Phase = dispatched\n action lose { set phase = indeterminate }\n action retry { require phase == indeterminate; set phase = indeterminate }\n temporal NoRetry = always(phase == indeterminate implies next(not enabled(retry)))\n }",
        )
        .unwrap();
        let source = super::initialize(&typed).unwrap();
        let lose = &typed.model.actions[0];
        let target = super::apply_action(lose, &source, &typed).unwrap();
        assert!(
            super::evaluate_property(&typed.model.properties[0], &source, Some(&target), &typed)
                .unwrap(),
            "source-state antecedent must remain false: {:#?}",
            typed.model.properties[0].expression
        );
    }

    #[test]
    fn current_state_enabledness_rejects_a_one_shot_blind_replay() {
        let old = compile(
            "enum Phase { indeterminate, reconciled }\nsystem OneShotReplay {\n state phase: Phase = indeterminate\n state dispatch_count: Nat = 1\n action dispatch { require phase == indeterminate; set dispatch_count = dispatch_count + 1; set phase = reconciled }\n temporal NoBlindReplay = always(phase == indeterminate implies next(not enabled(dispatch)))\n }",
        )
        .unwrap();
        let old_report = check_model(&old, CheckConfig::default()).unwrap();
        assert_eq!(
            old_report.class,
            ResultClass::ModelChecked,
            "the regression control must expose the off-by-one old formula"
        );

        let corrected = compile(
            "enum Phase { indeterminate, reconciled }\nsystem OneShotReplay {\n state phase: Phase = indeterminate\n state dispatch_count: Nat = 1\n action dispatch { require phase == indeterminate; set dispatch_count = dispatch_count + 1; set phase = reconciled }\n temporal NoBlindReplay = always(phase == indeterminate implies not enabled(dispatch))\n }",
        )
        .unwrap();
        let report = check_model(&corrected, CheckConfig::default()).unwrap();
        assert_eq!(report.class, ResultClass::Refuted);
        let witness = report.properties[0].witness.as_ref().unwrap();
        assert_eq!(witness.steps.len(), 1);
        assert_eq!(witness.steps[0].action, None);
        assert!(witness.steps[0].enabled_actions.contains("dispatch"));
    }

    #[test]
    fn structured_report_json_is_deterministic_and_parse_shaped() {
        let typed =
            compile("system S {\n state safe: Bool = true\n safety Safe = always(safe)\n }")
                .unwrap();
        let report = check_model(&typed, CheckConfig::default()).unwrap();
        let json = report.to_json_pretty();
        assert_eq!(json, report.to_json_pretty());
        assert!(json.contains("\"schema_version\": \"1.1.0\""));
        assert!(json.contains("\"result\": \"model_checked\""));
        assert!(json.contains("\"witness\": null"));
    }
}

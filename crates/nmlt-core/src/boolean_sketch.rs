//! Paper 1 boolean finite-graph *sketch*, not a compiler.
//!
//! Walks one [`UntypedSystem`] and, for a tiny Bool fragment, enumerates
//! reachable assignments as states with action-named transitions. Anything
//! outside the fragment is a hard error. This is not source-to-LTS, not M9
//! compile, and not composition of graphs from surface `compose`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use crate::untyped::{
    BindingKind, ObservationKind, RawTerm, UntypedAction, UntypedMember, UntypedObservation,
    UntypedStatement, UntypedSystem, UntypedUpdateTarget, hidden_action_names,
};
use crate::{Span, TokenKind, lex_source};

/// One reachable boolean assignment. Keys are state-field names.
pub type SketchState = BTreeMap<String, bool>;

/// Action-labelled edge on a [`BooleanSketch`].
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct SketchTransition {
    pub from: usize,
    pub action: String,
    pub to: usize,
}

/// Reachable Bool assignments and action names for one system.
///
/// Not a `nmlt-temporal::FiniteGraph`. Names match the Paper 1 OpenSystem
/// fixtures (ping loop; Lean receive false→true). An unguarded
/// `set ident = true|false` (no `require`) is enabled in every reachable
/// assignment, so the OpenSystem-receptive dual can loop receive at bit=true.
/// Hidden action names are recorded but not projected; surface `compose` is
/// out of scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BooleanSketch {
    pub system_name: String,
    pub fields: Vec<String>,
    pub states: Vec<SketchState>,
    pub initial: usize,
    pub transitions: Vec<SketchTransition>,
    pub action_names: Vec<String>,
    pub hidden_actions: Vec<String>,
    pub observed_fields: Vec<String>,
}

/// Fail-closed rejection of a construct outside the boolean sketch fragment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BooleanSketchError {
    pub code: &'static str,
    pub feature: &'static str,
    pub span: Span,
}

impl fmt::Display for BooleanSketchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: Paper 1 boolean sketch rejects {}",
            self.code, self.feature
        )
    }
}

impl std::error::Error for BooleanSketchError {}

const CODE_NAT: &str = "NMLT-SKETCH-NAT";
const CODE_UNSUPPORTED: &str = "NMLT-SKETCH-UNSUPPORTED";

fn reject(code: &'static str, feature: &'static str, span: Span) -> BooleanSketchError {
    BooleanSketchError {
        code,
        feature,
        span,
    }
}

enum CompiledStmt {
    Require { field: String, value: bool },
    SetLiteral { field: String, value: bool },
    SetIdentity,
}

struct CompiledAction {
    name: String,
    statements: Vec<CompiledStmt>,
}

/// Sketch reachable Bool assignments of `system`.
///
/// Supported: `state name: Bool = true|false`; action bodies of
/// `require ident == true|false`, `set ident = ident` (same name), and
/// `set ident = true|false`; `observe` of those fields; `hide action`.
/// Optional `action input|output` polarity is ignored (names only).
/// No `require` means the action is enabled in every reachable assignment
/// (from init: `set bit = true` yields false→true then true→true).
///
/// Refused: Nat, params, consume, capability, state-field hide, emit, ports,
/// properties, grades, non-literal inits, non-identity copies, and any other
/// statement or member.
pub fn sketch_boolean_system(system: &UntypedSystem) -> Result<BooleanSketch, BooleanSketchError> {
    let system_name = system
        .name
        .as_ref()
        .map(|name| name.text.clone())
        .ok_or_else(|| reject(CODE_UNSUPPORTED, "unnamed system", system.span))?;

    if let Some(parameter) = system.parameters.first() {
        return Err(reject(
            CODE_UNSUPPORTED,
            "system parameter",
            parameter.span(),
        ));
    }

    let mut fields = Vec::new();
    let mut initial_assignment = SketchState::new();
    let mut field_set = BTreeSet::new();
    let mut raw_actions = Vec::new();
    let mut observations = Vec::new();

    for member in &system.members {
        match member {
            UntypedMember::Binding(binding) => match binding.kind {
                BindingKind::State => {
                    collect_bool_state(
                        binding,
                        &mut fields,
                        &mut field_set,
                        &mut initial_assignment,
                    )?;
                }
                BindingKind::Capability => {
                    return Err(reject(CODE_UNSUPPORTED, "capability", binding.span));
                }
                BindingKind::Const => {
                    return Err(reject(CODE_UNSUPPORTED, "system constant", binding.span));
                }
                BindingKind::Input => {
                    return Err(reject(CODE_UNSUPPORTED, "system input", binding.span));
                }
            },
            UntypedMember::Action(action) => raw_actions.push(action),
            UntypedMember::Observation(observation) => observations.push(observation),
            UntypedMember::Port(port) => {
                return Err(reject(CODE_UNSUPPORTED, "port declaration", port.span));
            }
            UntypedMember::Property(property) => {
                return Err(reject(
                    CODE_UNSUPPORTED,
                    "property declaration",
                    property.span,
                ));
            }
            UntypedMember::SurfaceOnly(node) => {
                return Err(reject(
                    CODE_UNSUPPORTED,
                    "unsupported system member",
                    node.source.span,
                ));
            }
            UntypedMember::Error(node) => {
                return Err(reject(
                    CODE_UNSUPPORTED,
                    "recovered system member",
                    node.source.span,
                ));
            }
        }
    }

    let mut observed_fields = Vec::new();
    for observation in observations {
        collect_observation(observation, &field_set)?;
        if observation.kind == ObservationKind::Observe {
            for name in &observation.names {
                if !observed_fields
                    .iter()
                    .any(|existing| existing == &name.text)
                {
                    observed_fields.push(name.text.clone());
                }
            }
        }
    }

    let mut actions = Vec::new();
    for action in raw_actions {
        actions.push(compile_action(action, &field_set)?);
    }

    let hidden_actions = hidden_action_names(system)
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let action_name_set = actions
        .iter()
        .map(|action| action.name.as_str())
        .collect::<BTreeSet<_>>();
    for hidden in &hidden_actions {
        if !action_name_set.contains(hidden.as_str()) {
            return Err(reject(
                CODE_UNSUPPORTED,
                "hide of unknown action",
                system.span,
            ));
        }
    }

    let (states, transitions) = enumerate_reachable(&initial_assignment, &actions);
    let action_names = actions.into_iter().map(|action| action.name).collect();

    Ok(BooleanSketch {
        system_name,
        fields,
        states,
        initial: 0,
        transitions,
        action_names,
        hidden_actions,
        observed_fields,
    })
}

fn collect_bool_state(
    binding: &crate::untyped::UntypedBinding,
    fields: &mut Vec<String>,
    field_set: &mut BTreeSet<String>,
    initial: &mut SketchState,
) -> Result<(), BooleanSketchError> {
    let name = binding
        .name
        .as_ref()
        .map(|name| name.text.clone())
        .ok_or_else(|| reject(CODE_UNSUPPORTED, "unnamed state field", binding.span))?;
    let declared_type = binding
        .declared_type
        .as_ref()
        .ok_or_else(|| reject(CODE_UNSUPPORTED, "state field without type", binding.span))?;
    if is_nat_type(declared_type) {
        return Err(reject(
            CODE_NAT,
            "Nat state field",
            declared_type.source.span,
        ));
    }
    if !is_bool_type(declared_type) {
        return Err(reject(
            CODE_UNSUPPORTED,
            "non-Bool state field",
            declared_type.source.span,
        ));
    }
    let initializer = binding.initializer.as_ref().ok_or_else(|| {
        reject(
            CODE_UNSUPPORTED,
            "state field without initializer",
            binding.span,
        )
    })?;
    let value = parse_bool_literal(initializer).ok_or_else(|| {
        reject(
            CODE_UNSUPPORTED,
            "state initializer that is not true or false",
            initializer.source.span,
        )
    })?;
    if !field_set.insert(name.clone()) {
        return Err(reject(
            CODE_UNSUPPORTED,
            "duplicate state field",
            binding.span,
        ));
    }
    fields.push(name.clone());
    initial.insert(name, value);
    Ok(())
}

fn collect_observation(
    observation: &UntypedObservation,
    field_set: &BTreeSet<String>,
) -> Result<(), BooleanSketchError> {
    match observation.kind {
        ObservationKind::Hide => {
            if observation.hides_actions() {
                Ok(())
            } else {
                Err(reject(
                    CODE_UNSUPPORTED,
                    "hide of state fields",
                    observation.span,
                ))
            }
        }
        ObservationKind::Observe => {
            let Some(expression) = observation.expression.as_ref() else {
                return Err(reject(CODE_UNSUPPORTED, "empty observe", observation.span));
            };
            if !is_ident_list(expression) {
                return Err(reject(
                    CODE_UNSUPPORTED,
                    "observe expression outside field-name list",
                    expression.source.span,
                ));
            }
            for name in &observation.names {
                if !field_set.contains(&name.text) {
                    return Err(reject(
                        CODE_UNSUPPORTED,
                        "observe of unknown state field",
                        name.span,
                    ));
                }
            }
            Ok(())
        }
    }
}

fn compile_action(
    action: &UntypedAction,
    field_set: &BTreeSet<String>,
) -> Result<CompiledAction, BooleanSketchError> {
    let name = action
        .name
        .as_ref()
        .map(|name| name.text.clone())
        .ok_or_else(|| reject(CODE_UNSUPPORTED, "unnamed action", action.span))?;
    if let Some(parameter) = action.parameters.first() {
        return Err(reject(
            CODE_UNSUPPORTED,
            "action parameter",
            parameter.span(),
        ));
    }
    if let Some(grade) = &action.grade {
        return Err(reject(CODE_UNSUPPORTED, "action grade", grade.span));
    }
    let mut statements = Vec::new();
    for statement in &action.statements {
        statements.push(compile_statement(statement, field_set)?);
    }
    Ok(CompiledAction { name, statements })
}

fn compile_statement(
    statement: &UntypedStatement,
    field_set: &BTreeSet<String>,
) -> Result<CompiledStmt, BooleanSketchError> {
    match statement {
        UntypedStatement::Require { condition, span } => {
            let (field, value) = parse_require(condition).ok_or_else(|| {
                reject(
                    CODE_UNSUPPORTED,
                    "require outside ident == true|false",
                    *span,
                )
            })?;
            if !field_set.contains(&field) {
                return Err(reject(
                    CODE_UNSUPPORTED,
                    "require of unknown state field",
                    condition.source.span,
                ));
            }
            Ok(CompiledStmt::Require { field, value })
        }
        UntypedStatement::Update {
            target,
            value,
            span,
        } => {
            let field = match target {
                UntypedUpdateTarget::Location { root, source, .. } if root.text == source.text => {
                    root.text.clone()
                }
                UntypedUpdateTarget::Location { source, .. }
                | UntypedUpdateTarget::Unsupported { source, .. } => {
                    return Err(reject(
                        CODE_UNSUPPORTED,
                        "update target outside a plain state field",
                        source.span,
                    ));
                }
            };
            if !field_set.contains(&field) {
                return Err(reject(
                    CODE_UNSUPPORTED,
                    "update of unknown state field",
                    *span,
                ));
            }
            if let Some(literal) = parse_bool_literal(value) {
                Ok(CompiledStmt::SetLiteral {
                    field,
                    value: literal,
                })
            } else if is_same_ident(value, &field) {
                Ok(CompiledStmt::SetIdentity)
            } else {
                Err(reject(
                    CODE_UNSUPPORTED,
                    "set outside identity or true|false",
                    value.source.span,
                ))
            }
        }
        UntypedStatement::Consume { span, .. } => {
            Err(reject(CODE_UNSUPPORTED, "consume statement", *span))
        }
        UntypedStatement::Emit { span, .. } => {
            Err(reject(CODE_UNSUPPORTED, "emit statement", *span))
        }
        UntypedStatement::SurfaceOnly(node) => Err(reject(
            CODE_UNSUPPORTED,
            "unsupported action statement",
            node.source.span,
        )),
        UntypedStatement::Error(node) => Err(reject(
            CODE_UNSUPPORTED,
            "recovered action statement",
            node.source.span,
        )),
    }
}

fn enumerate_reachable(
    initial: &SketchState,
    actions: &[CompiledAction],
) -> (Vec<SketchState>, Vec<SketchTransition>) {
    let mut states = vec![initial.clone()];
    let mut index_by_state = BTreeMap::from([(initial.clone(), 0_usize)]);
    let mut queue = VecDeque::from([0_usize]);
    let mut transitions = Vec::new();

    while let Some(from) = queue.pop_front() {
        let current = states[from].clone();
        for action in actions {
            let Some(next) = apply_action(&current, &action.statements) else {
                continue;
            };
            let to = if let Some(&existing) = index_by_state.get(&next) {
                existing
            } else {
                let to = states.len();
                index_by_state.insert(next.clone(), to);
                states.push(next);
                queue.push_back(to);
                to
            };
            transitions.push(SketchTransition {
                from,
                action: action.name.clone(),
                to,
            });
        }
    }

    transitions.sort();
    transitions.dedup();
    (states, transitions)
}

fn apply_action(assignment: &SketchState, statements: &[CompiledStmt]) -> Option<SketchState> {
    let mut working = assignment.clone();
    for statement in statements {
        match statement {
            CompiledStmt::Require { field, value } => {
                if working.get(field) != Some(value) {
                    return None;
                }
            }
            CompiledStmt::SetLiteral { field, value } => {
                working.insert(field.clone(), *value);
            }
            CompiledStmt::SetIdentity => {}
        }
    }
    Some(working)
}

fn significant_tokens(source: &str) -> Vec<(TokenKind, &str)> {
    lex_source(source)
        .tokens
        .iter()
        .filter(|token| !token.kind.is_trivia())
        .map(|token| (token.kind, token.text(source)))
        .collect()
}

fn is_bool_type(term: &RawTerm) -> bool {
    matches!(
        significant_tokens(&term.source.text).as_slice(),
        [(TokenKind::Identifier, "Bool")]
    )
}

fn is_nat_type(term: &RawTerm) -> bool {
    matches!(
        significant_tokens(&term.source.text).as_slice(),
        [(TokenKind::Identifier, "Nat")]
    )
}

fn parse_bool_literal(term: &RawTerm) -> Option<bool> {
    match significant_tokens(&term.source.text).as_slice() {
        [(TokenKind::Identifier, "true")] => Some(true),
        [(TokenKind::Identifier, "false")] => Some(false),
        _ => None,
    }
}

fn parse_require(term: &RawTerm) -> Option<(String, bool)> {
    match significant_tokens(&term.source.text).as_slice() {
        [
            (TokenKind::Identifier, field),
            (TokenKind::Punctuation, "=="),
            (TokenKind::Identifier, "true"),
        ] => Some(((*field).to_owned(), true)),
        [
            (TokenKind::Identifier, field),
            (TokenKind::Punctuation, "=="),
            (TokenKind::Identifier, "false"),
        ] => Some(((*field).to_owned(), false)),
        _ => None,
    }
}

fn is_same_ident(term: &RawTerm, field: &str) -> bool {
    matches!(
        significant_tokens(&term.source.text).as_slice(),
        [(TokenKind::Identifier, name)] if *name == field
    )
}

fn is_ident_list(term: &RawTerm) -> bool {
    let tokens = significant_tokens(&term.source.text);
    if tokens.is_empty() {
        return false;
    }
    let mut expect_ident = true;
    for (kind, text) in tokens {
        if expect_ident {
            if kind != TokenKind::Identifier {
                return false;
            }
            expect_ident = false;
        } else if kind == TokenKind::Punctuation && text == "," {
            expect_ident = true;
        } else {
            return false;
        }
    }
    !expect_ident
}

//! Structural validation for explicit typed core.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use nmlt_hir::{DefId, LocalId, ModuleId};

use crate::identity::CoreNodeId;
use crate::model::{
    CoreBinaryOp, CoreProgram, CorePropertyKind, CoreTerm, CoreTermKind, CoreType, CoreUnaryOp,
};

const MAX_MODULES: usize = 256;
const MAX_TERMS: usize = 262_144;
const MAX_TERM_DEPTH: usize = 256;
const MAX_INTEGER_MAGNITUDE: usize = 4_096;
const MAX_TOTAL_INTEGER_BYTES: usize = 16 * 1024 * 1024;
const MAX_CONTEXT_ENTRIES: usize = 65_536;
pub(crate) const MAX_CANONICAL_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreResourceDimension {
    Modules,
    Terms,
    TermDepth,
    IntegerMagnitudeBytes,
    TotalIntegerBytes,
    ContextEntries,
    CanonicalBytes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreValidationError {
    EmptyProgram,
    ResourceExceeded {
        dimension: CoreResourceDimension,
        actual: usize,
        maximum: usize,
    },
    DuplicateCoreNode(CoreNodeId),
    MissingImport(ModuleId),
    KeyMismatch {
        context: &'static str,
    },
    DuplicateDefinition(DefId),
    DuplicateLocal(LocalId),
    UnknownDefinition {
        context: &'static str,
        definition: DefId,
    },
    MissingTerm {
        context: &'static str,
        term: CoreNodeId,
    },
    InvalidType {
        context: &'static str,
        ty: CoreType,
    },
    TypeMismatch {
        context: &'static str,
        expected: CoreType,
        actual: CoreType,
    },
    InvalidTerm {
        term: CoreNodeId,
        reason: &'static str,
    },
    OwnerMismatch {
        term: CoreNodeId,
        expected: DefId,
        actual: DefId,
    },
    SystemMismatch {
        context: &'static str,
        expected: DefId,
        actual: DefId,
    },
    InvalidActionFrame {
        action: DefId,
    },
    Cycle(CoreNodeId),
    UnreachableTerm(CoreNodeId),
}

impl fmt::Display for CoreValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => formatter.write_str("typed core contains no modules"),
            Self::ResourceExceeded {
                dimension,
                actual,
                maximum,
            } => write!(
                formatter,
                "typed-core resource {dimension:?} is {actual}, above maximum {maximum}"
            ),
            Self::DuplicateCoreNode(id) => write!(formatter, "duplicate core node `{id}`"),
            Self::MissingImport(id) => {
                write!(formatter, "typed core imports missing module `{id}`")
            }
            Self::KeyMismatch { context } => write!(
                formatter,
                "map key disagrees with embedded {context} identity"
            ),
            Self::DuplicateDefinition(id) => {
                write!(formatter, "definition `{id}` occurs more than once")
            }
            Self::DuplicateLocal(id) => {
                write!(formatter, "local binder `{id}` occurs more than once")
            }
            Self::UnknownDefinition {
                context,
                definition,
            } => write!(
                formatter,
                "{context} references unknown definition `{definition}`"
            ),
            Self::MissingTerm { context, term } => {
                write!(formatter, "{context} references missing core term `{term}`")
            }
            Self::InvalidType { context, ty } => {
                write!(formatter, "invalid type {ty:?} for {context}")
            }
            Self::TypeMismatch {
                context,
                expected,
                actual,
            } => write!(
                formatter,
                "{context} expects {expected:?}, found {actual:?}"
            ),
            Self::InvalidTerm { term, reason } => {
                write!(formatter, "invalid core term `{term}`: {reason}")
            }
            Self::OwnerMismatch {
                term,
                expected,
                actual,
            } => write!(
                formatter,
                "core term `{term}` owner is `{actual}`, expected `{expected}`"
            ),
            Self::SystemMismatch {
                context,
                expected,
                actual,
            } => write!(
                formatter,
                "{context} uses system `{actual}`, expected `{expected}`"
            ),
            Self::InvalidActionFrame { action } => write!(
                formatter,
                "action `{action}` updates/frames are not an exact state partition"
            ),
            Self::Cycle(id) => write!(
                formatter,
                "typed-core term graph has a cycle through `{id}`"
            ),
            Self::UnreachableTerm(id) => write!(
                formatter,
                "typed-core term `{id}` is unreachable from a semantic root"
            ),
        }
    }
}

impl std::error::Error for CoreValidationError {}

#[derive(Default)]
struct Index {
    systems: BTreeSet<DefId>,
    enumerations: BTreeMap<DefId, BTreeSet<DefId>>,
    states: BTreeMap<DefId, (DefId, CoreType)>,
    actions: BTreeMap<DefId, DefId>,
    capabilities: BTreeMap<DefId, DefId>,
    protocols: BTreeSet<nmlt_hir::NodeId>,
    locals: BTreeMap<LocalId, (DefId, CoreType)>,
}

pub(crate) fn validate_program(program: &CoreProgram) -> Result<(), CoreValidationError> {
    resource(
        CoreResourceDimension::Modules,
        program.modules.len(),
        MAX_MODULES,
    )?;
    resource(CoreResourceDimension::Terms, program.terms.len(), MAX_TERMS)?;
    if program.modules.is_empty() {
        return Err(CoreValidationError::EmptyProgram);
    }

    for (id, module) in &program.modules {
        if *id != module.id {
            return Err(CoreValidationError::KeyMismatch { context: "module" });
        }
        for import in &module.imports {
            if !program.modules.contains_key(import) {
                return Err(CoreValidationError::MissingImport(*import));
            }
        }
    }

    let index = build_index(program)?;
    validate_types(program, &index)?;
    validate_terms(program, &index)?;
    validate_roots(program, &index)?;
    validate_graph(program)?;
    Ok(())
}

fn build_index(program: &CoreProgram) -> Result<Index, CoreValidationError> {
    let mut index = Index::default();
    let mut definitions = BTreeSet::new();
    for module in program.modules.values() {
        for (id, enumeration) in &module.enumerations {
            key(*id == enumeration.id, "enumeration")?;
            insert_definition(&mut definitions, *id)?;
            for constructor in &enumeration.constructors {
                insert_definition(&mut definitions, *constructor)?;
            }
            index
                .enumerations
                .insert(*id, enumeration.constructors.clone());
        }
        for (system_id, system) in &module.systems {
            key(*system_id == system.id, "system")?;
            insert_definition(&mut definitions, *system_id)?;
            index.systems.insert(*system_id);
            for (state_id, state) in &system.state {
                key(*state_id == state.id, "state")?;
                insert_definition(&mut definitions, *state_id)?;
                index
                    .states
                    .insert(*state_id, (*system_id, state.ty.clone()));
            }
            for (capability_id, capability) in &system.capabilities {
                key(*capability_id == capability.id, "capability")?;
                insert_definition(&mut definitions, *capability_id)?;
                index.capabilities.insert(*capability_id, *system_id);
                index.protocols.insert(capability.protocol);
            }
            for (action_id, action) in &system.actions {
                key(*action_id == action.id, "action")?;
                insert_definition(&mut definitions, *action_id)?;
                index.actions.insert(*action_id, *system_id);
                for (local_id, parameter) in &action.parameters {
                    key(*local_id == parameter.id, "action parameter")?;
                    if index
                        .locals
                        .insert(*local_id, (*action_id, parameter.ty.clone()))
                        .is_some()
                    {
                        return Err(CoreValidationError::DuplicateLocal(*local_id));
                    }
                }
            }
            for (property_id, property) in &system.properties {
                key(*property_id == property.id, "property")?;
                insert_definition(&mut definitions, *property_id)?;
            }
        }
    }
    resource(
        CoreResourceDimension::ContextEntries,
        definitions.len() + index.locals.len(),
        MAX_CONTEXT_ENTRIES,
    )?;
    Ok(index)
}

fn validate_types(program: &CoreProgram, index: &Index) -> Result<(), CoreValidationError> {
    for module in program.modules.values() {
        for system in module.systems.values() {
            for state in system.state.values() {
                validate_type(&state.ty, index, "state field")?;
                if !state.ty.is_scalar() {
                    return Err(CoreValidationError::InvalidType {
                        context: "state field",
                        ty: state.ty.clone(),
                    });
                }
            }
            for action in system.actions.values() {
                for parameter in action.parameters.values() {
                    validate_type(&parameter.ty, index, "action parameter")?;
                    if !parameter.ty.is_scalar() {
                        return Err(CoreValidationError::InvalidType {
                            context: "action parameter",
                            ty: parameter.ty.clone(),
                        });
                    }
                }
            }
        }
    }
    for term in program.terms.values() {
        validate_type(&term.ty, index, "term annotation")?;
    }
    Ok(())
}

fn validate_type(
    ty: &CoreType,
    index: &Index,
    context: &'static str,
) -> Result<(), CoreValidationError> {
    match ty {
        CoreType::Enum(id) if !index.enumerations.contains_key(id) => {
            Err(CoreValidationError::UnknownDefinition {
                context,
                definition: *id,
            })
        }
        CoreType::StateProp { system } | CoreType::TemporalProp { system }
            if !index.systems.contains(system) =>
        {
            Err(CoreValidationError::UnknownDefinition {
                context,
                definition: *system,
            })
        }
        CoreType::Once { protocol } if !index.protocols.contains(protocol) => {
            Err(CoreValidationError::InvalidType {
                context,
                ty: ty.clone(),
            })
        }
        _ => Ok(()),
    }
}

fn validate_terms(program: &CoreProgram, index: &Index) -> Result<(), CoreValidationError> {
    let mut total_integer_bytes = 0usize;
    for (id, term) in &program.terms {
        if *id != term.id {
            return Err(CoreValidationError::KeyMismatch { context: "term" });
        }
        match &term.kind {
            CoreTermKind::Bool(_) => expect_annotation(term, CoreType::Bool, "Boolean literal")?,
            CoreTermKind::Nat { magnitude } => {
                validate_magnitude(*id, magnitude, false)?;
                total_integer_bytes = total_integer_bytes.saturating_add(magnitude.len());
                expect_annotation(term, CoreType::Nat, "natural literal")?;
            }
            CoreTermKind::Int {
                negative,
                magnitude,
            } => {
                validate_magnitude(*id, magnitude, *negative)?;
                total_integer_bytes = total_integer_bytes.saturating_add(magnitude.len());
                expect_annotation(term, CoreType::Int, "integer literal")?;
            }
            CoreTermKind::Local(local) => {
                let Some((owner, ty)) = index.locals.get(local) else {
                    return Err(CoreValidationError::InvalidTerm {
                        term: *id,
                        reason: "unknown local binder",
                    });
                };
                if *owner != term.owner {
                    return Err(CoreValidationError::OwnerMismatch {
                        term: *id,
                        expected: *owner,
                        actual: term.owner,
                    });
                }
                expect_annotation(term, ty.clone(), "local reference")?;
            }
            CoreTermKind::State { system, state } => {
                let Some((actual_system, ty)) = index.states.get(state) else {
                    return Err(CoreValidationError::UnknownDefinition {
                        context: "state reference",
                        definition: *state,
                    });
                };
                same_system("state reference", *system, *actual_system)?;
                expect_annotation(term, ty.clone(), "state reference")?;
            }
            CoreTermKind::Constructor {
                enumeration,
                constructor,
            } => {
                let Some(constructors) = index.enumerations.get(enumeration) else {
                    return Err(CoreValidationError::UnknownDefinition {
                        context: "constructor type",
                        definition: *enumeration,
                    });
                };
                if !constructors.contains(constructor) {
                    return Err(CoreValidationError::UnknownDefinition {
                        context: "constructor",
                        definition: *constructor,
                    });
                }
                expect_annotation(term, CoreType::Enum(*enumeration), "constructor")?;
            }
            CoreTermKind::Unary { operator, operand } => {
                let operand = child(program, *operand, "unary operand")?;
                same_owner(term, operand)?;
                let expected = match operator {
                    CoreUnaryOp::Not => CoreType::Bool,
                    CoreUnaryOp::Negate => CoreType::Int,
                };
                expect_type(operand, &expected, "unary operand")?;
                expect_annotation(term, expected, "unary result")?;
            }
            CoreTermKind::Binary {
                operator,
                left,
                right,
            } => validate_binary(program, term, *operator, *left, *right)?,
            CoreTermKind::IntFromNat { operand } => {
                let operand = child(program, *operand, "to_int operand")?;
                same_owner(term, operand)?;
                expect_type(operand, &CoreType::Nat, "to_int operand")?;
                expect_annotation(term, CoreType::Int, "to_int result")?;
            }
            CoreTermKind::StatePredicate { system, condition } => {
                require_system(index, *system, "state predicate")?;
                let condition = child(program, *condition, "state predicate condition")?;
                same_owner(term, condition)?;
                expect_type(condition, &CoreType::Bool, "state predicate condition")?;
                expect_annotation(
                    term,
                    CoreType::StateProp { system: *system },
                    "state predicate",
                )?;
            }
            CoreTermKind::Always { system, property }
            | CoreTermKind::Eventually { system, property }
            | CoreTermKind::Next { system, property } => {
                validate_temporal_unary(program, index, term, *system, *property)?;
            }
            CoreTermKind::Until {
                system,
                left,
                right,
            } => {
                require_system(index, *system, "until")?;
                for id in [*left, *right] {
                    let child = child(program, id, "until operand")?;
                    same_owner(term, child)?;
                    expect_formula(child, *system, "until operand")?;
                }
                expect_annotation(term, CoreType::TemporalProp { system: *system }, "until")?;
            }
            CoreTermKind::Enabled { system, action }
            | CoreTermKind::ActionOccurred { system, action } => {
                let Some(actual_system) = index.actions.get(action) else {
                    return Err(CoreValidationError::UnknownDefinition {
                        context: "action predicate",
                        definition: *action,
                    });
                };
                same_system("action predicate", *system, *actual_system)?;
                expect_annotation(
                    term,
                    CoreType::StateProp { system: *system },
                    "action predicate",
                )?;
            }
        }
    }
    resource(
        CoreResourceDimension::TotalIntegerBytes,
        total_integer_bytes,
        MAX_TOTAL_INTEGER_BYTES,
    )
}

fn validate_binary(
    program: &CoreProgram,
    term: &CoreTerm,
    operator: CoreBinaryOp,
    left: CoreNodeId,
    right: CoreNodeId,
) -> Result<(), CoreValidationError> {
    let left = child(program, left, "binary left operand")?;
    let right = child(program, right, "binary right operand")?;
    same_owner(term, left)?;
    same_owner(term, right)?;
    match operator {
        CoreBinaryOp::Or | CoreBinaryOp::And | CoreBinaryOp::Implies => {
            expect_type(left, &CoreType::Bool, "logical left operand")?;
            expect_type(right, &CoreType::Bool, "logical right operand")?;
            expect_annotation(term, CoreType::Bool, "logical result")
        }
        CoreBinaryOp::Equal | CoreBinaryOp::NotEqual => {
            if left.ty != right.ty || !left.ty.is_scalar() {
                return Err(CoreValidationError::InvalidTerm {
                    term: term.id,
                    reason: "equality operands must have the same scalar type",
                });
            }
            expect_annotation(term, CoreType::Bool, "equality result")
        }
        CoreBinaryOp::Less
        | CoreBinaryOp::LessEqual
        | CoreBinaryOp::Greater
        | CoreBinaryOp::GreaterEqual => {
            if left.ty != right.ty || !matches!(left.ty, CoreType::Nat | CoreType::Int) {
                return Err(CoreValidationError::InvalidTerm {
                    term: term.id,
                    reason: "comparison operands must have the same numeric type",
                });
            }
            expect_annotation(term, CoreType::Bool, "comparison result")
        }
        CoreBinaryOp::Add | CoreBinaryOp::Multiply => {
            if left.ty != right.ty || !matches!(left.ty, CoreType::Nat | CoreType::Int) {
                return Err(CoreValidationError::InvalidTerm {
                    term: term.id,
                    reason: "arithmetic operands must have the same numeric type",
                });
            }
            expect_annotation(term, left.ty.clone(), "arithmetic result")
        }
        CoreBinaryOp::Subtract => {
            expect_type(left, &CoreType::Int, "subtraction left operand")?;
            expect_type(right, &CoreType::Int, "subtraction right operand")?;
            expect_annotation(term, CoreType::Int, "subtraction result")
        }
    }
}

fn validate_temporal_unary(
    program: &CoreProgram,
    index: &Index,
    term: &CoreTerm,
    system: DefId,
    property: CoreNodeId,
) -> Result<(), CoreValidationError> {
    require_system(index, system, "temporal operator")?;
    let property = child(program, property, "temporal operand")?;
    same_owner(term, property)?;
    expect_formula(property, system, "temporal operand")?;
    expect_annotation(term, CoreType::TemporalProp { system }, "temporal result")
}

fn validate_roots(program: &CoreProgram, index: &Index) -> Result<(), CoreValidationError> {
    for module in program.modules.values() {
        for system in module.systems.values() {
            let all_state: BTreeSet<_> = system.state.keys().copied().collect();
            for state in system.state.values() {
                let term = owned_root(program, state.initializer, state.id, "state initializer")?;
                expect_type(term, &state.ty, "state initializer")?;
            }
            for action in system.actions.values() {
                same_system("action", system.id, action.system)?;
                let writes: BTreeSet<_> = action.updates.keys().copied().collect();
                if !writes.is_disjoint(&action.frames)
                    || writes
                        .union(&action.frames)
                        .copied()
                        .collect::<BTreeSet<_>>()
                        != all_state
                {
                    return Err(CoreValidationError::InvalidActionFrame { action: action.id });
                }
                for guard in &action.guards {
                    expect_type(
                        owned_root(program, *guard, action.id, "action guard")?,
                        &CoreType::Bool,
                        "action guard",
                    )?;
                }
                for (state_id, value) in &action.updates {
                    let Some((actual_system, ty)) = index.states.get(state_id) else {
                        return Err(CoreValidationError::UnknownDefinition {
                            context: "action update",
                            definition: *state_id,
                        });
                    };
                    same_system("action update", system.id, *actual_system)?;
                    expect_type(
                        owned_root(program, *value, action.id, "action update")?,
                        ty,
                        "action update",
                    )?;
                }
                for output in &action.outputs {
                    let output = owned_root(program, *output, action.id, "action output")?;
                    if !output.ty.is_scalar() {
                        return Err(CoreValidationError::InvalidType {
                            context: "action output",
                            ty: output.ty.clone(),
                        });
                    }
                }
                for capability in &action.consumes {
                    let Some(actual_system) = index.capabilities.get(capability) else {
                        return Err(CoreValidationError::UnknownDefinition {
                            context: "capability consumption",
                            definition: *capability,
                        });
                    };
                    same_system("capability consumption", system.id, *actual_system)?;
                }
            }
            for property in system.properties.values() {
                same_system("property", system.id, property.system)?;
                let body = owned_root(program, property.body, property.id, "property body")?;
                let expected = CoreType::TemporalProp { system: system.id };
                expect_type(
                    body,
                    &expected,
                    match property.kind {
                        CorePropertyKind::Safety => "safety property",
                        CorePropertyKind::Temporal => "temporal property",
                    },
                )?;
            }
            for observation in &system.observations {
                for item in &observation.items {
                    let term = owned_root(program, *item, observation.owner, "observation item")?;
                    if !term.ty.is_scalar() {
                        return Err(CoreValidationError::InvalidType {
                            context: "observation item",
                            ty: term.ty.clone(),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_graph(program: &CoreProgram) -> Result<(), CoreValidationError> {
    let mut roots = Vec::new();
    for module in program.modules.values() {
        for system in module.systems.values() {
            roots.extend(system.state.values().map(|state| state.initializer));
            for action in system.actions.values() {
                roots.extend(action.guards.iter().copied());
                roots.extend(action.updates.values().copied());
                roots.extend(action.outputs.iter().copied());
            }
            roots.extend(system.properties.values().map(|property| property.body));
            for observation in &system.observations {
                roots.extend(observation.items.iter().copied());
            }
        }
    }
    let mut permanent = BTreeSet::new();
    let mut active = BTreeSet::new();
    for root in roots {
        let mut stack = vec![(root, 1_usize, false)];
        while let Some((id, depth, exiting)) = stack.pop() {
            if exiting {
                active.remove(&id);
                permanent.insert(id);
                continue;
            }
            if permanent.contains(&id) {
                continue;
            }
            resource(CoreResourceDimension::TermDepth, depth, MAX_TERM_DEPTH)?;
            if !active.insert(id) {
                return Err(CoreValidationError::Cycle(id));
            }
            let term = child(program, id, "term graph")?;
            stack.push((id, depth, true));
            for child_id in children(&term.kind).into_iter().rev() {
                stack.push((child_id, depth + 1, false));
            }
        }
    }
    if let Some(unreachable) = program.terms.keys().find(|id| !permanent.contains(id)) {
        return Err(CoreValidationError::UnreachableTerm(*unreachable));
    }
    Ok(())
}

fn children(kind: &CoreTermKind) -> Vec<CoreNodeId> {
    match kind {
        CoreTermKind::Unary { operand, .. } | CoreTermKind::IntFromNat { operand } => {
            vec![*operand]
        }
        CoreTermKind::Binary { left, right, .. } | CoreTermKind::Until { left, right, .. } => {
            vec![*left, *right]
        }
        CoreTermKind::StatePredicate { condition, .. } => vec![*condition],
        CoreTermKind::Always { property, .. }
        | CoreTermKind::Eventually { property, .. }
        | CoreTermKind::Next { property, .. } => vec![*property],
        _ => Vec::new(),
    }
}

fn validate_magnitude(
    id: CoreNodeId,
    magnitude: &[u8],
    negative: bool,
) -> Result<(), CoreValidationError> {
    resource(
        CoreResourceDimension::IntegerMagnitudeBytes,
        magnitude.len(),
        MAX_INTEGER_MAGNITUDE,
    )?;
    if magnitude.is_empty() || (magnitude.len() > 1 && magnitude[0] == 0) {
        return Err(CoreValidationError::InvalidTerm {
            term: id,
            reason: "integer magnitude is not minimal unsigned big-endian",
        });
    }
    if negative && magnitude == [0] {
        return Err(CoreValidationError::InvalidTerm {
            term: id,
            reason: "negative zero is forbidden",
        });
    }
    Ok(())
}

fn child<'a>(
    program: &'a CoreProgram,
    id: CoreNodeId,
    context: &'static str,
) -> Result<&'a CoreTerm, CoreValidationError> {
    program
        .terms
        .get(&id)
        .ok_or(CoreValidationError::MissingTerm { context, term: id })
}

fn owned_root<'a>(
    program: &'a CoreProgram,
    id: CoreNodeId,
    owner: DefId,
    context: &'static str,
) -> Result<&'a CoreTerm, CoreValidationError> {
    let term = child(program, id, context)?;
    if term.owner != owner {
        return Err(CoreValidationError::OwnerMismatch {
            term: id,
            expected: owner,
            actual: term.owner,
        });
    }
    Ok(term)
}

fn same_owner(parent: &CoreTerm, child: &CoreTerm) -> Result<(), CoreValidationError> {
    if parent.owner != child.owner {
        return Err(CoreValidationError::OwnerMismatch {
            term: child.id,
            expected: parent.owner,
            actual: child.owner,
        });
    }
    Ok(())
}

fn expect_annotation(
    term: &CoreTerm,
    expected: CoreType,
    context: &'static str,
) -> Result<(), CoreValidationError> {
    expect_type(term, &expected, context)
}

fn expect_type(
    term: &CoreTerm,
    expected: &CoreType,
    context: &'static str,
) -> Result<(), CoreValidationError> {
    if term.ty != *expected {
        return Err(CoreValidationError::TypeMismatch {
            context,
            expected: expected.clone(),
            actual: term.ty.clone(),
        });
    }
    Ok(())
}

fn expect_formula(
    term: &CoreTerm,
    system: DefId,
    context: &'static str,
) -> Result<(), CoreValidationError> {
    if term.ty != (CoreType::StateProp { system }) && term.ty != (CoreType::TemporalProp { system })
    {
        return Err(CoreValidationError::InvalidType {
            context,
            ty: term.ty.clone(),
        });
    }
    Ok(())
}

fn same_system(
    context: &'static str,
    expected: DefId,
    actual: DefId,
) -> Result<(), CoreValidationError> {
    if expected != actual {
        return Err(CoreValidationError::SystemMismatch {
            context,
            expected,
            actual,
        });
    }
    Ok(())
}

fn require_system(
    index: &Index,
    system: DefId,
    context: &'static str,
) -> Result<(), CoreValidationError> {
    if !index.systems.contains(&system) {
        return Err(CoreValidationError::UnknownDefinition {
            context,
            definition: system,
        });
    }
    Ok(())
}

fn insert_definition(
    definitions: &mut BTreeSet<DefId>,
    id: DefId,
) -> Result<(), CoreValidationError> {
    if !definitions.insert(id) {
        return Err(CoreValidationError::DuplicateDefinition(id));
    }
    Ok(())
}

fn key(valid: bool, context: &'static str) -> Result<(), CoreValidationError> {
    if !valid {
        return Err(CoreValidationError::KeyMismatch { context });
    }
    Ok(())
}

fn resource(
    dimension: CoreResourceDimension,
    actual: usize,
    maximum: usize,
) -> Result<(), CoreValidationError> {
    if actual > maximum {
        return Err(CoreValidationError::ResourceExceeded {
            dimension,
            actual,
            maximum,
        });
    }
    Ok(())
}

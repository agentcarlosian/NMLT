use std::collections::BTreeMap;

use nmlt_core::Span;
use nmlt_hir::{DefId, LocalId, ResolvedProgram, SourceSpan};
use nmlt_ir::{
    CoreBinaryOp, CoreNodeId, CoreProgram, CorePropertyKind, CoreTermKind, CoreType, CoreUnaryOp,
};
use nmlt_kernel::CheckedProgram;

use crate::ast::{
    Action, BinaryOp, Expr, Model, Property, PropertyKind, StateVar, Type, UnaryOp, Update, Value,
};
use crate::types::{SemanticBinding, TypedModel};

struct Names {
    definitions: BTreeMap<DefId, (String, Span)>,
    locals: BTreeMap<LocalId, String>,
}

impl Names {
    fn new(hir: &ResolvedProgram) -> Self {
        let mut definitions = BTreeMap::new();
        let mut locals = BTreeMap::new();
        for module in hir.modules().values() {
            for declaration in module.declarations().values() {
                let name = declaration
                    .key()
                    .path
                    .segments
                    .last()
                    .map(|segment| segment.name.clone())
                    .unwrap_or_default();
                definitions.insert(declaration.id(), (name, span(declaration.span())));
            }
            for binder in module.local_binders().values() {
                locals.insert(binder.id(), binder.name().to_owned());
            }
        }
        Self {
            definitions,
            locals,
        }
    }

    fn definition(&self, id: DefId) -> Result<&str, String> {
        self.definitions
            .get(&id)
            .map(|(name, _)| name.as_str())
            .ok_or_else(|| format!("NMLT_ENGINE_MISSING_NAME: definition {id}"))
    }

    fn definition_span(&self, id: DefId) -> Result<Span, String> {
        self.definitions
            .get(&id)
            .map(|(_, span)| *span)
            .ok_or_else(|| format!("NMLT_ENGINE_MISSING_NAME: definition {id}"))
    }

    fn local(&self, id: LocalId) -> Result<&str, String> {
        self.locals
            .get(&id)
            .map(String::as_str)
            .ok_or_else(|| format!("NMLT_ENGINE_MISSING_NAME: local {id}"))
    }
}

/// Adapt one kernel-accepted program into the bounded engine's runtime model.
///
/// This function performs no parsing, name resolution, or type inference. It
/// is a total structural projection for the executable M9-v1 subset and fails
/// explicitly where the finite engine has no execution policy.
pub fn from_checked(checked: &CheckedProgram) -> Result<TypedModel, Vec<String>> {
    adapt_checked(checked).map_err(|error| vec![error])
}

fn adapt_checked(checked: &CheckedProgram) -> Result<TypedModel, String> {
    let hir = checked.resolved_program();
    let core = checked.core_program();
    let names = Names::new(hir);
    let systems = core
        .modules()
        .values()
        .flat_map(|module| module.systems().values())
        .collect::<Vec<_>>();
    let [system] = systems.as_slice() else {
        return Err(
            "NMLT_ENGINE_SYSTEM_CARDINALITY: bounded execution requires exactly one system"
                .to_owned(),
        );
    };
    let system_name = names.definition(system.id())?.to_owned();
    let state_types = system
        .state()
        .values()
        .map(|field| {
            Ok((
                names.definition(field.id())?.to_owned(),
                runtime_type(field.ty(), &names)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let states = system
        .state()
        .values()
        .map(|field| {
            Ok(StateVar {
                name: names.definition(field.id())?.to_owned(),
                ty: runtime_type(field.ty(), &names)?,
                initial: expression(core, field.initializer(), &names)?,
                span: names.definition_span(field.id())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let capabilities = system
        .capabilities()
        .keys()
        .map(|id| names.definition(*id).map(str::to_owned))
        .collect::<Result<Vec<_>, String>>()?;
    let actions = system
        .actions()
        .values()
        .map(|action| {
            if !action.parameters().is_empty() {
                return Err(format!(
                    "NMLT_ENGINE_ACTION_PARAMETERS: action `{}` requires an explicit finite input-domain policy",
                    names.definition(action.id())?
                ));
            }
            Ok(Action {
                name: names.definition(action.id())?.to_owned(),
                guards: action
                    .guards()
                    .iter()
                    .map(|node| expression(core, *node, &names))
                    .collect::<Result<Vec<_>, _>>()?,
                updates: action
                    .updates()
                    .iter()
                    .map(|(target, value)| {
                        Ok(Update {
                            target: names.definition(*target)?.to_owned(),
                            value: expression(core, *value, &names)?,
                            span: names.definition_span(action.id())?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?,
                consumes: action
                    .consumes()
                    .iter()
                    .map(|id| names.definition(*id).map(str::to_owned))
                    .collect::<Result<Vec<_>, _>>()?,
                span: names.definition_span(action.id())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let properties = system
        .properties()
        .values()
        .map(|property| {
            Ok(Property {
                name: names.definition(property.id())?.to_owned(),
                kind: match property.kind() {
                    CorePropertyKind::Safety => PropertyKind::Safety,
                    CorePropertyKind::Temporal => PropertyKind::Temporal,
                },
                expression: expression(core, property.body(), &names)?,
                span: names.definition_span(property.id())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let observations = system
        .observations()
        .iter()
        .flat_map(|observation| observation.items())
        .map(|node| match core.terms()[node].kind() {
            CoreTermKind::State { state, .. } => names.definition(*state).map(str::to_owned),
            _ => Err("NMLT_ENGINE_OBSERVATION: only state-field observations execute".to_owned()),
        })
        .collect::<Result<Vec<_>, String>>()?;
    let frames = system
        .actions()
        .values()
        .map(|action| {
            Ok((
                names.definition(action.id())?.to_owned(),
                action
                    .frames()
                    .iter()
                    .map(|id| names.definition(*id).map(str::to_owned))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let property_behavior = properties
        .iter()
        .map(|property| (property.name.clone(), system_name.clone()))
        .collect();
    let system_span = names.definition_span(system.id())?;
    Ok(TypedModel {
        model: Model {
            system_name,
            states,
            capabilities,
            actions,
            properties,
            observations,
            span: system_span,
        },
        state_types,
        frames,
        property_behavior,
        semantic_binding: SemanticBinding {
            source_set_id: hir.source_set_id().to_string(),
            module_map_id: hir.module_map_id().to_string(),
            surface_program_id: hir.surface_program_id().to_string(),
            resolved_hir_id: hir.resolution_id().to_string(),
            core_program_id: core.id().to_string(),
            ruleset_bundle_id: digest_id(
                "nmlt-ruleset-bundle-v1:sha256:",
                checked.ruleset_bundle_digest(),
            ),
            resource_policy_id: digest_id(
                "nmlt-kernel-policy-v1:sha256:",
                checked.resource_policy_digest(),
            ),
            certificate_id: digest_id(
                "nmlt-elaboration-certificate-v1:sha256:",
                checked.certificate_digest(),
            ),
            kernel_profile_id: checked.kernel_profile_id().to_string(),
        },
    })
}

fn digest_id(prefix: &str, digest: &[u8; 32]) -> String {
    use std::fmt::Write;

    let mut value = String::with_capacity(prefix.len() + 64);
    value.push_str(prefix);
    for byte in digest {
        write!(value, "{byte:02x}").expect("writing to String cannot fail");
    }
    value
}

fn runtime_type(ty: &CoreType, names: &Names) -> Result<Type, String> {
    match ty {
        CoreType::Bool => Ok(Type::Bool),
        CoreType::Nat => Ok(Type::Nat),
        CoreType::Int => Ok(Type::Int),
        CoreType::Enum(id) => Ok(Type::Named(names.definition(*id)?.to_owned())),
        CoreType::Once { .. } | CoreType::StateProp { .. } | CoreType::TemporalProp { .. } => Err(
            "NMLT_ENGINE_NONSCALAR_STATE: bounded runtime state must use scalar values".to_owned(),
        ),
    }
}

fn expression(core: &CoreProgram, id: CoreNodeId, names: &Names) -> Result<Expr, String> {
    let term = core
        .terms()
        .get(&id)
        .ok_or_else(|| format!("NMLT_ENGINE_MISSING_TERM: {id}"))?;
    match term.kind() {
        CoreTermKind::Bool(value) => Ok(Expr::Value(Value::Bool(*value))),
        CoreTermKind::Nat { magnitude } => Ok(Expr::Value(Value::Int(magnitude_i64(
            false, magnitude,
        )?))),
        CoreTermKind::Int {
            negative,
            magnitude,
        } => Ok(Expr::Value(Value::Int(magnitude_i64(
            *negative, magnitude,
        )?))),
        CoreTermKind::Local(local) => Ok(Expr::Name(names.local(*local)?.to_owned())),
        CoreTermKind::State { state, .. } => {
            Ok(Expr::Name(names.definition(*state)?.to_owned()))
        }
        CoreTermKind::Constructor { constructor, .. } => Ok(Expr::Value(Value::Symbol(
            names.definition(*constructor)?.to_owned(),
        ))),
        CoreTermKind::Unary { operator, operand } => Ok(Expr::Unary {
            op: match operator {
                CoreUnaryOp::Not => UnaryOp::Not,
                CoreUnaryOp::Negate => UnaryOp::Negate,
            },
            operand: Box::new(expression(core, *operand, names)?),
        }),
        CoreTermKind::Binary {
            operator,
            left,
            right,
        } => Ok(Expr::Binary {
            op: binary(*operator),
            left: Box::new(expression(core, *left, names)?),
            right: Box::new(expression(core, *right, names)?),
        }),
        CoreTermKind::IntFromNat { operand } => expression(core, *operand, names),
        CoreTermKind::StatePredicate { condition, .. } => expression(core, *condition, names),
        CoreTermKind::Always { property, .. } => call("always", core, [*property], names),
        CoreTermKind::Eventually { property, .. } => call("eventually", core, [*property], names),
        CoreTermKind::Next { property, .. } => call("next", core, [*property], names),
        CoreTermKind::Until { .. } => Err(
            "NMLT_ENGINE_UNTIL: lasso-based until execution belongs to the temporal engine"
                .to_owned(),
        ),
        CoreTermKind::Enabled { action, .. } => Ok(Expr::Call {
            name: "enabled".to_owned(),
            arguments: vec![Expr::Name(names.definition(*action)?.to_owned())],
        }),
        CoreTermKind::ActionOccurred { .. } => Err(
            "NMLT_ENGINE_ACTION_OCCURRED: transition-label properties are not in the bounded engine adapter"
                .to_owned(),
        ),
    }
}

fn call(
    name: &str,
    core: &CoreProgram,
    arguments: impl IntoIterator<Item = CoreNodeId>,
    names: &Names,
) -> Result<Expr, String> {
    Ok(Expr::Call {
        name: name.to_owned(),
        arguments: arguments
            .into_iter()
            .map(|id| expression(core, id, names))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

const fn binary(operator: CoreBinaryOp) -> BinaryOp {
    match operator {
        CoreBinaryOp::Or => BinaryOp::Or,
        CoreBinaryOp::And => BinaryOp::And,
        CoreBinaryOp::Implies => BinaryOp::Implies,
        CoreBinaryOp::Equal => BinaryOp::Equal,
        CoreBinaryOp::NotEqual => BinaryOp::NotEqual,
        CoreBinaryOp::Less => BinaryOp::Less,
        CoreBinaryOp::LessEqual => BinaryOp::LessEqual,
        CoreBinaryOp::Greater => BinaryOp::Greater,
        CoreBinaryOp::GreaterEqual => BinaryOp::GreaterEqual,
        CoreBinaryOp::Add => BinaryOp::Add,
        CoreBinaryOp::Subtract => BinaryOp::Subtract,
        CoreBinaryOp::Multiply => BinaryOp::Multiply,
    }
}

fn magnitude_i64(negative: bool, bytes: &[u8]) -> Result<i64, String> {
    let mut magnitude = 0_u64;
    for byte in bytes {
        magnitude = magnitude
            .checked_mul(256)
            .and_then(|value| value.checked_add(u64::from(*byte)))
            .ok_or_else(|| "NMLT_ENGINE_INTEGER_RANGE: integer exceeds u64".to_owned())?;
    }
    if negative {
        let limit = (i64::MAX as u64) + 1;
        if magnitude > limit {
            return Err("NMLT_ENGINE_INTEGER_RANGE: integer is below i64 minimum".to_owned());
        }
        if magnitude == limit {
            Ok(i64::MIN)
        } else {
            Ok(-(magnitude as i64))
        }
    } else {
        i64::try_from(magnitude)
            .map_err(|_| "NMLT_ENGINE_INTEGER_RANGE: integer exceeds i64 maximum".to_owned())
    }
}

const fn span(value: SourceSpan) -> Span {
    Span::new(value.start, value.end)
}

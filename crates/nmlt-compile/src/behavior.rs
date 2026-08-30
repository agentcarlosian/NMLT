//! Finite source-to-behavior-core elaboration for the first language slice.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use nmlt_core::{
    BindingKind, HideSort, ObservationKind, SurfacePolarity, UntypedAction, UntypedDeclaration,
    UntypedMember, UntypedRefinementItem, UntypedStatement, UntypedSystem, UntypedUpdateTarget,
    parse_cst, project_untyped,
};
use nmlt_hir::sha256_bytes;
use nmlt_ir::{
    BEHAVIOR_CORE_SCHEMA, BehaviorCoreProgram, CoreBehaviorAction, CoreBehaviorBinding,
    CoreBehaviorState, CoreBehaviorSystem, CoreBehaviorTerm, CoreComposition, CoreConnection,
    CorePort, CorePortDirection, CoreRefinement, CoreResourceProfile,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BehaviorDiagnostic {
    code: &'static str,
    message: String,
}

impl BehaviorDiagnostic {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for BehaviorDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for BehaviorDiagnostic {}

fn reject(code: &'static str, message: impl Into<String>) -> BehaviorDiagnostic {
    BehaviorDiagnostic {
        code,
        message: message.into(),
    }
}

/// Elaborate the finite behavior slice into deterministic behavior-core-v1.
pub fn compile_behavior_single(
    repository_path: impl Into<String>,
    exact_bytes: impl Into<Vec<u8>>,
) -> Result<BehaviorCoreProgram, BehaviorDiagnostic> {
    let repository_path = repository_path.into();
    let exact_bytes = exact_bytes.into();
    let source = std::str::from_utf8(&exact_bytes)
        .map_err(|_| reject("NMLT-BHV-UTF8", "behavior source is not valid UTF-8"))?;
    let parsed = parse_cst(source);
    let projection = project_untyped(&parsed);
    if !projection.issues.is_empty() || !projection.coverage.is_exact() {
        return Err(reject(
            "NMLT-BHV-PROJECTION",
            format!(
                "lossless projection is incomplete: {:?}",
                projection.issues.first().map(|issue| &issue.kind)
            ),
        ));
    }

    let mut enums = BTreeMap::new();
    collect_enums(&projection.file.declarations, &mut enums)?;
    let facts = enums
        .iter()
        .flat_map(|(name, variants)| {
            variants
                .iter()
                .map(move |variant| format!("{name}.{variant}"))
        })
        .collect::<BTreeSet<_>>();

    let mut systems = BTreeMap::new();
    for system in projection.file.systems() {
        let system = compile_system(system, &facts)?;
        if systems.insert(system.name.clone(), system).is_some() {
            return Err(reject(
                "NMLT-BHV-DUPLICATE-SYSTEM",
                "duplicate behavior system name",
            ));
        }
    }
    if systems.is_empty() {
        return Err(reject(
            "NMLT-BHV-NO-SYSTEM",
            "a behavior program must declare at least one system",
        ));
    }

    let mut compositions = BTreeMap::new();
    collect_compositions(&projection.file.declarations, &systems, &mut compositions)?;
    let refinements = compile_refinements(projection.file.refinements(), &systems)?;

    Ok(BehaviorCoreProgram {
        schema: BEHAVIOR_CORE_SCHEMA.to_owned(),
        source_path: repository_path,
        source_sha256: hex_digest(sha256_bytes(&exact_bytes)),
        enums,
        systems,
        compositions,
        refinements,
    })
}

fn collect_enums(
    declarations: &[UntypedDeclaration],
    enums: &mut BTreeMap<String, BTreeSet<String>>,
) -> Result<(), BehaviorDiagnostic> {
    for declaration in declarations {
        match declaration {
            UntypedDeclaration::Module(module) => collect_enums(&module.declarations, enums)?,
            UntypedDeclaration::Enum(enumeration) => {
                let name = required_name(
                    enumeration.name.as_ref().map(|name| name.text.as_str()),
                    "NMLT-BHV-ENUM-NAME",
                    "enum",
                )?;
                let variants = enumeration
                    .supported_variants()
                    .map(|variant| {
                        required_name(
                            variant.name.as_ref().map(|name| name.text.as_str()),
                            "NMLT-BHV-ENUM-VARIANT",
                            "enum variant",
                        )
                        .map(str::to_owned)
                    })
                    .collect::<Result<BTreeSet<_>, _>>()?;
                if enums.insert(name.to_owned(), variants).is_some() {
                    return Err(reject(
                        "NMLT-BHV-DUPLICATE-ENUM",
                        format!("duplicate enum '{name}'"),
                    ));
                }
            }
            UntypedDeclaration::Import(_)
            | UntypedDeclaration::System(_)
            | UntypedDeclaration::Compose(_)
            | UntypedDeclaration::Connect(_)
            | UntypedDeclaration::Refinement(_) => {}
            UntypedDeclaration::Unsupported(node) => {
                return Err(reject(
                    "NMLT-BHV-UNSUPPORTED-DECLARATION",
                    format!("unsupported declaration {:?}", node.kind),
                ));
            }
            UntypedDeclaration::Error(_) => {
                return Err(reject(
                    "NMLT-BHV-RECOVERY",
                    "recovered declaration cannot enter the behavior core",
                ));
            }
        }
    }
    Ok(())
}

fn compile_system(
    system: &UntypedSystem,
    facts: &BTreeSet<String>,
) -> Result<CoreBehaviorSystem, BehaviorDiagnostic> {
    let name = required_name(
        system.name.as_ref().map(|name| name.text.as_str()),
        "NMLT-BHV-SYSTEM-NAME",
        "system",
    )?
    .to_owned();
    if !system.parameters.is_empty() {
        return Err(reject(
            "NMLT-BHV-SYSTEM-PARAMETERS",
            format!("system '{name}' uses unsupported parameters"),
        ));
    }

    let mut state = BTreeMap::new();
    let mut capabilities = BTreeMap::new();
    let mut ports = BTreeMap::new();
    let mut action_sources = Vec::new();
    let mut observations = Vec::new();
    let mut hidden = BTreeSet::new();

    for member in &system.members {
        match member {
            UntypedMember::Binding(binding) => {
                let binding_name = required_name(
                    binding.name.as_ref().map(|value| value.text.as_str()),
                    "NMLT-BHV-BINDING-NAME",
                    "binding",
                )?
                .to_owned();
                let ty = raw_required(binding.declared_type.as_ref(), "NMLT-BHV-TYPE")?;
                match binding.kind {
                    BindingKind::State => {
                        validate_state_type(&ty, facts)?;
                        let initializer = raw_required(
                            binding.initializer.as_ref(),
                            "NMLT-BHV-STATE-INITIALIZER",
                        )?;
                        let initial_ast =
                            parse_behavior_term(&initializer, &ty, &BTreeMap::new(), facts)?;
                        state.insert(
                            binding_name.clone(),
                            CoreBehaviorState {
                                name: binding_name,
                                ty,
                                initializer,
                                initial_ast,
                            },
                        );
                    }
                    BindingKind::Capability => {
                        if !ty.starts_with("Once<") {
                            return Err(reject(
                                "NMLT-BHV-CAPABILITY-TYPE",
                                format!("capability '{name}.{binding_name}' must use Once<...>"),
                            ));
                        }
                        capabilities.insert(binding_name, ty);
                    }
                    BindingKind::Const | BindingKind::Input => {
                        return Err(reject(
                            "NMLT-BHV-UNSUPPORTED-BINDING",
                            format!("system '{name}' uses an unsupported binding kind"),
                        ));
                    }
                }
            }
            UntypedMember::Port(port) => {
                let port_name = required_name(
                    port.name.as_ref().map(|value| value.text.as_str()),
                    "NMLT-BHV-PORT-NAME",
                    "port",
                )?
                .to_owned();
                let direction = match port.polarity() {
                    Some(SurfacePolarity::Input) => CorePortDirection::Input,
                    Some(SurfacePolarity::Output) => CorePortDirection::Output,
                    None => {
                        return Err(reject(
                            "NMLT-BHV-PORT-DIRECTION",
                            format!("port '{name}.{port_name}' must be input or output"),
                        ));
                    }
                };
                let payload_type =
                    raw_required(port.declared_type.as_ref(), "NMLT-BHV-PORT-PAYLOAD")?;
                ports.insert(
                    port_name.clone(),
                    CorePort {
                        name: port_name,
                        direction,
                        payload_type,
                    },
                );
            }
            UntypedMember::Action(action) => action_sources.push(action),
            UntypedMember::Observation(observation) => match observation.kind {
                ObservationKind::Observe => {
                    observations.extend(observation.names.iter().map(|name| name.text.clone()));
                }
                ObservationKind::Hide if observation.hide_sort == Some(HideSort::Actions) => {
                    hidden.extend(observation.names.iter().map(|name| name.text.clone()));
                }
                ObservationKind::Hide => {
                    return Err(reject(
                        "NMLT-BHV-HIDE-KIND",
                        format!("system '{name}' may hide actions only in this slice"),
                    ));
                }
            },
            UntypedMember::Property(_) => {}
            UntypedMember::SurfaceOnly(node) => {
                return Err(reject(
                    "NMLT-BHV-UNSUPPORTED-MEMBER",
                    format!(
                        "system '{name}' contains unsupported member {:?}",
                        node.kind
                    ),
                ));
            }
            UntypedMember::Error(_) => {
                return Err(reject(
                    "NMLT-BHV-RECOVERY",
                    format!("system '{name}' contains a recovered member"),
                ));
            }
        }
    }

    for observed in &observations {
        if !state.contains_key(observed) {
            return Err(reject(
                "NMLT-BHV-OBSERVATION",
                format!("observation '{name}.{observed}' is not a state field"),
            ));
        }
    }

    let mut actions = BTreeMap::new();
    for action in action_sources {
        let action = compile_action(&name, action, &state, &capabilities, &ports, facts, &hidden)?;
        if actions.insert(action.name.clone(), action).is_some() {
            return Err(reject(
                "NMLT-BHV-DUPLICATE-ACTION",
                format!("duplicate action in system '{name}'"),
            ));
        }
    }
    for hidden_action in &hidden {
        if !actions.contains_key(hidden_action) {
            return Err(reject(
                "NMLT-BHV-HIDDEN-ACTION",
                format!("hidden action '{name}.{hidden_action}' does not exist"),
            ));
        }
    }

    Ok(CoreBehaviorSystem {
        name,
        state,
        capabilities,
        ports,
        actions,
        observations,
    })
}

fn compile_action(
    system_name: &str,
    action: &UntypedAction,
    state: &BTreeMap<String, CoreBehaviorState>,
    capabilities: &BTreeMap<String, String>,
    ports: &BTreeMap<String, CorePort>,
    facts: &BTreeSet<String>,
    hidden_actions: &BTreeSet<String>,
) -> Result<CoreBehaviorAction, BehaviorDiagnostic> {
    let name = required_name(
        action.name.as_ref().map(|name| name.text.as_str()),
        "NMLT-BHV-ACTION-NAME",
        "action",
    )?
    .to_owned();
    let direction = action.polarity.map(|polarity| match polarity {
        SurfacePolarity::Input => CorePortDirection::Input,
        SurfacePolarity::Output => CorePortDirection::Output,
    });
    match (direction, ports.get(&name)) {
        (Some(direction), Some(port)) if direction == port.direction => {}
        (Some(_), Some(_)) => {
            return Err(reject(
                "NMLT-BHV-ACTION-PORT-DIRECTION",
                format!("action and port '{system_name}.{name}' disagree on direction"),
            ));
        }
        (Some(_), None) => {
            return Err(reject(
                "NMLT-BHV-ACTION-PORT",
                format!("boundary action '{system_name}.{name}' needs a same-named port"),
            ));
        }
        (None, Some(_)) => {
            return Err(reject(
                "NMLT-BHV-ACTION-POLARITY",
                format!("port action '{system_name}.{name}' needs input/output polarity"),
            ));
        }
        (None, None) => {}
    }

    let parameters = action
        .supported_parameters()
        .map(|parameter| {
            Ok(CoreBehaviorBinding {
                name: required_name(
                    parameter.name.as_ref().map(|name| name.text.as_str()),
                    "NMLT-BHV-PARAMETER-NAME",
                    "action parameter",
                )?
                .to_owned(),
                ty: raw_required(parameter.declared_type.as_ref(), "NMLT-BHV-PARAMETER-TYPE")?,
            })
        })
        .collect::<Result<Vec<_>, BehaviorDiagnostic>>()?;

    let mut guards = Vec::new();
    let mut updates = BTreeMap::new();
    let mut outputs = Vec::new();
    let mut consumed = Vec::new();
    let mut relies = BTreeSet::new();
    let mut guarantees = BTreeSet::new();
    for statement in &action.statements {
        match statement {
            UntypedStatement::Require { condition, .. } => {
                guards.push(normalize(&condition.source.text));
            }
            UntypedStatement::Update { target, value, .. } => {
                let field = match target {
                    UntypedUpdateTarget::Location { root, source, .. }
                        if root.text == source.text =>
                    {
                        root.text.clone()
                    }
                    _ => {
                        return Err(reject(
                            "NMLT-BHV-UPDATE-TARGET",
                            format!("action '{system_name}.{name}' has a selected update"),
                        ));
                    }
                };
                if !state.contains_key(&field) {
                    return Err(reject(
                        "NMLT-BHV-UPDATE-FIELD",
                        format!("action '{system_name}.{name}' updates unknown field '{field}'"),
                    ));
                }
                if updates
                    .insert(field.clone(), normalize(&value.source.text))
                    .is_some()
                {
                    return Err(reject(
                        "NMLT-BHV-DUPLICATE-UPDATE",
                        format!("action '{system_name}.{name}' updates '{field}' twice"),
                    ));
                }
            }
            UntypedStatement::Emit { value, .. } => {
                outputs.push(normalize(&value.source.text));
            }
            UntypedStatement::Consume { capability, .. } => {
                consumed.push(normalize(&capability.source.text));
            }
            UntypedStatement::Rely { fact, .. } => {
                let fact = normalize(&fact.source.text);
                validate_fact(&fact, facts)?;
                relies.insert(fact);
            }
            UntypedStatement::Guarantee { fact, .. } => {
                let fact = normalize(&fact.source.text);
                validate_fact(&fact, facts)?;
                guarantees.insert(fact);
            }
            UntypedStatement::SurfaceOnly(_) | UntypedStatement::Error(_) => {
                return Err(reject(
                    "NMLT-BHV-ACTION-STATEMENT",
                    format!("action '{system_name}.{name}' contains unsupported syntax"),
                ));
            }
        }
    }

    let mut consume_counts = BTreeMap::<String, usize>::new();
    for capability in consumed {
        if !capabilities.contains_key(&capability) {
            return Err(reject(
                "NMLT-BHV-CAPABILITY-NAME",
                format!("action '{system_name}.{name}' consumes unknown '{capability}'"),
            ));
        }
        *consume_counts.entry(capability).or_default() += 1;
    }
    if let Some((capability, _)) = consume_counts.iter().find(|(_, count)| **count > 1) {
        return Err(reject(
            "NMLT-BHV-AFFINE-DUPLICATE",
            format!("action '{system_name}.{name}' consumes '{capability}' more than once"),
        ));
    }

    let mut resources = CoreResourceProfile {
        grade: parse_grade(action.grade.as_ref().map(|grade| grade.text.as_str()))?,
        relies,
        guarantees,
        ..CoreResourceProfile::default()
    };
    for capability in consume_counts.keys() {
        if outputs.iter().any(|output| output == capability) {
            resources.transfers.insert(capability.clone());
        } else {
            resources.consumes.insert(capability.clone());
        }
    }
    for output in &outputs {
        if capabilities.contains_key(output) && !consume_counts.contains_key(output) {
            return Err(reject(
                "NMLT-BHV-AFFINE-RETAINED",
                format!(
                    "action '{system_name}.{name}' emits affine '{output}' without consuming it"
                ),
            ));
        }
    }
    if direction == Some(CorePortDirection::Input) {
        resources.receives.extend(
            parameters
                .iter()
                .filter(|parameter| parameter.ty.starts_with("Once<"))
                .map(|parameter| parameter.name.clone()),
        );
    }

    if let Some(port) = ports.get(&name) {
        validate_payload_shape(
            system_name,
            &name,
            port,
            &parameters,
            &outputs,
            capabilities,
        )?;
    }

    let guard_ast = guards
        .iter()
        .map(|guard| parse_behavior_term(guard, "Bool", state, facts))
        .collect::<Result<Vec<_>, _>>()?;
    let update_ast = updates
        .iter()
        .map(|(field, value)| {
            let expected = &state
                .get(field)
                .expect("update field was validated before AST construction")
                .ty;
            Ok((
                field.clone(),
                parse_behavior_term(value, expected, state, facts)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, BehaviorDiagnostic>>()?;

    Ok(CoreBehaviorAction {
        name: name.clone(),
        direction,
        parameters,
        guards,
        guard_ast,
        updates,
        update_ast,
        outputs,
        hidden: hidden_actions.contains(&name),
        resources,
    })
}

fn validate_payload_shape(
    system: &str,
    action: &str,
    port: &CorePort,
    parameters: &[CoreBehaviorBinding],
    outputs: &[String],
    capabilities: &BTreeMap<String, String>,
) -> Result<(), BehaviorDiagnostic> {
    match port.direction {
        CorePortDirection::Input => {
            let accepted = (port.payload_type == "Unit" && parameters.is_empty())
                || matches!(parameters, [parameter] if parameter.ty == port.payload_type);
            if !accepted {
                return Err(reject(
                    "NMLT-BHV-INPUT-PAYLOAD",
                    format!("input '{system}.{action}' parameters do not match its port payload"),
                ));
            }
        }
        CorePortDirection::Output => {
            let accepted = (port.payload_type == "Unit" && outputs.is_empty())
                || matches!(outputs, [output] if capabilities.get(output) == Some(&port.payload_type));
            if !accepted {
                return Err(reject(
                    "NMLT-BHV-OUTPUT-PAYLOAD",
                    format!("output '{system}.{action}' does not emit its port payload"),
                ));
            }
        }
    }
    Ok(())
}

fn collect_compositions(
    declarations: &[UntypedDeclaration],
    systems: &BTreeMap<String, CoreBehaviorSystem>,
    output: &mut BTreeMap<String, CoreComposition>,
) -> Result<(), BehaviorDiagnostic> {
    for declaration in declarations {
        match declaration {
            UntypedDeclaration::Module(module) => {
                collect_compositions(&module.declarations, systems, output)?;
            }
            UntypedDeclaration::Compose(composition) => {
                let name = required_name(
                    composition.name.as_ref().map(|name| name.text.as_str()),
                    "NMLT-BHV-COMPOSE-NAME",
                    "composition",
                )?
                .to_owned();
                let mut connections = Vec::new();
                let mut component_names = BTreeSet::new();
                let mut endpoints = BTreeSet::new();
                for wire in composition.supported_connections() {
                    let left_system = required_name(
                        wire.left_system.as_ref().map(|name| name.text.as_str()),
                        "NMLT-BHV-CONNECT-ENDPOINT",
                        "left system",
                    )?;
                    let left_action_name = required_name(
                        wire.left_action.as_ref().map(|name| name.text.as_str()),
                        "NMLT-BHV-CONNECT-ENDPOINT",
                        "left action",
                    )?;
                    let right_system = required_name(
                        wire.right_system.as_ref().map(|name| name.text.as_str()),
                        "NMLT-BHV-CONNECT-ENDPOINT",
                        "right system",
                    )?;
                    let right_action_name = required_name(
                        wire.right_action.as_ref().map(|name| name.text.as_str()),
                        "NMLT-BHV-CONNECT-ENDPOINT",
                        "right action",
                    )?;
                    if left_system == right_system {
                        return Err(reject(
                            "NMLT-BHV-COMPOSE-DISTINCT",
                            format!("composition '{name}' must contain two distinct systems"),
                        ));
                    }
                    if !endpoints.insert((left_system.to_owned(), left_action_name.to_owned()))
                        || !endpoints
                            .insert((right_system.to_owned(), right_action_name.to_owned()))
                    {
                        return Err(reject(
                            "NMLT-BHV-CONNECT-ONE-TO-ONE",
                            format!("composition '{name}' reuses a connection endpoint"),
                        ));
                    }
                    validate_connection(
                        name.as_str(),
                        systems,
                        left_system,
                        left_action_name,
                        right_system,
                        right_action_name,
                    )?;
                    component_names.insert(left_system.to_owned());
                    component_names.insert(right_system.to_owned());
                    connections.push(CoreConnection {
                        left_system: left_system.to_owned(),
                        left_action: left_action_name.to_owned(),
                        right_system: right_system.to_owned(),
                        right_action: right_action_name.to_owned(),
                    });
                }
                if connections.is_empty() || component_names.len() != 2 {
                    return Err(reject(
                        "NMLT-BHV-COMPOSE-BINARY",
                        format!("composition '{name}' must wire exactly two systems"),
                    ));
                }
                let mut components = component_names.into_iter();
                let first = components.next().expect("two components");
                let second = components.next().expect("two components");
                check_capability_partition(&name, systems, &first, &second)?;
                if output
                    .insert(
                        name.clone(),
                        CoreComposition {
                            name,
                            left_system: first,
                            right_system: second,
                            connections,
                        },
                    )
                    .is_some()
                {
                    return Err(reject(
                        "NMLT-BHV-DUPLICATE-COMPOSE",
                        "duplicate composition name",
                    ));
                }
            }
            UntypedDeclaration::Connect(_) => {
                return Err(reject(
                    "NMLT-BHV-TOPLEVEL-CONNECT",
                    "connections must be nested in a named composition",
                ));
            }
            UntypedDeclaration::Import(_)
            | UntypedDeclaration::Enum(_)
            | UntypedDeclaration::System(_)
            | UntypedDeclaration::Refinement(_)
            | UntypedDeclaration::Unsupported(_)
            | UntypedDeclaration::Error(_) => {}
        }
    }
    Ok(())
}

fn validate_connection(
    composition: &str,
    systems: &BTreeMap<String, CoreBehaviorSystem>,
    left_system: &str,
    left_action_name: &str,
    right_system: &str,
    right_action_name: &str,
) -> Result<(), BehaviorDiagnostic> {
    let left = systems.get(left_system).ok_or_else(|| {
        reject(
            "NMLT-BHV-CONNECT-SYSTEM",
            format!("composition '{composition}' names unknown system '{left_system}'"),
        )
    })?;
    let right = systems.get(right_system).ok_or_else(|| {
        reject(
            "NMLT-BHV-CONNECT-SYSTEM",
            format!("composition '{composition}' names unknown system '{right_system}'"),
        )
    })?;
    let left_action = left.actions.get(left_action_name).ok_or_else(|| {
        reject(
            "NMLT-BHV-CONNECT-ACTION",
            format!("unknown action '{left_system}.{left_action_name}'"),
        )
    })?;
    let right_action = right.actions.get(right_action_name).ok_or_else(|| {
        reject(
            "NMLT-BHV-CONNECT-ACTION",
            format!("unknown action '{right_system}.{right_action_name}'"),
        )
    })?;
    let left_port = left.ports.get(&left_action.name).ok_or_else(|| {
        reject(
            "NMLT-BHV-CONNECT-PORT",
            format!("action '{left_system}.{}' has no port", left_action.name),
        )
    })?;
    let right_port = right.ports.get(&right_action.name).ok_or_else(|| {
        reject(
            "NMLT-BHV-CONNECT-PORT",
            format!("action '{right_system}.{}' has no port", right_action.name),
        )
    })?;
    if !left_port.direction.complements(right_port.direction) {
        return Err(reject(
            "NMLT-BHV-CONNECT-DIRECTION",
            format!("connection in '{composition}' does not join output to input"),
        ));
    }
    if left_port.payload_type != right_port.payload_type {
        return Err(reject(
            "NMLT-BHV-CONNECT-PAYLOAD",
            format!("connection in '{composition}' has unequal payload types"),
        ));
    }
    if left_action.hidden || right_action.hidden {
        return Err(reject(
            "NMLT-BHV-HIDDEN-BOUNDARY",
            format!("composition '{composition}' connects a hidden action"),
        ));
    }
    let (sender, receiver) = if left_port.direction == CorePortDirection::Output {
        (left_action, right_action)
    } else {
        (right_action, left_action)
    };
    if sender.resources.transfers != receiver.resources.receives {
        return Err(reject(
            "NMLT-BHV-TRANSFER-MISMATCH",
            format!("composition '{composition}' does not transfer affine authority exactly"),
        ));
    }
    if !sender
        .resources
        .relies
        .is_subset(&receiver.resources.guarantees)
        || !receiver
            .resources
            .relies
            .is_subset(&sender.resources.guarantees)
    {
        return Err(reject(
            "NMLT-BHV-RELY-UNDISCHARGED",
            format!("composition '{composition}' leaves a synchronization reliance open"),
        ));
    }
    Ok(())
}

fn check_capability_partition(
    composition: &str,
    systems: &BTreeMap<String, CoreBehaviorSystem>,
    left: &str,
    right: &str,
) -> Result<(), BehaviorDiagnostic> {
    let left_caps = systems[left].capabilities.keys().collect::<BTreeSet<_>>();
    let right_caps = systems[right].capabilities.keys().collect::<BTreeSet<_>>();
    if let Some(shared) = left_caps.intersection(&right_caps).next() {
        return Err(reject(
            "NMLT-BHV-CAPABILITY-PARTITION",
            format!("composition '{composition}' shares capability '{shared}'"),
        ));
    }
    Ok(())
}

fn compile_refinements(
    refinements: Vec<&nmlt_core::UntypedRefinement>,
    systems: &BTreeMap<String, CoreBehaviorSystem>,
) -> Result<Vec<CoreRefinement>, BehaviorDiagnostic> {
    let mut output = Vec::new();
    for refinement in refinements {
        let concrete_name = required_name(
            refinement
                .concrete_system
                .as_ref()
                .map(|name| name.text.as_str()),
            "NMLT-BHV-REFINE-SYSTEM",
            "concrete system",
        )?;
        let abstract_name = required_name(
            refinement
                .abstract_system
                .as_ref()
                .map(|name| name.text.as_str()),
            "NMLT-BHV-REFINE-SYSTEM",
            "abstract system",
        )?;
        let concrete = systems.get(concrete_name).ok_or_else(|| {
            reject(
                "NMLT-BHV-REFINE-SYSTEM",
                format!("unknown concrete system '{concrete_name}'"),
            )
        })?;
        let abstract_system = systems.get(abstract_name).ok_or_else(|| {
            reject(
                "NMLT-BHV-REFINE-SYSTEM",
                format!("unknown abstract system '{abstract_name}'"),
            )
        })?;
        let mut state_map = BTreeMap::new();
        let mut hidden_actions = BTreeSet::new();
        for item in &refinement.items {
            match item {
                UntypedRefinementItem::StateMap(mapping) => {
                    let concrete_field = required_name(
                        mapping
                            .concrete_field
                            .as_ref()
                            .map(|name| name.text.as_str()),
                        "NMLT-BHV-STATE-MAP",
                        "concrete state field",
                    )?;
                    let abstract_field = required_name(
                        mapping
                            .abstract_field
                            .as_ref()
                            .map(|name| name.text.as_str()),
                        "NMLT-BHV-STATE-MAP",
                        "abstract state field",
                    )?;
                    let concrete_state = concrete.state.get(concrete_field).ok_or_else(|| {
                        reject(
                            "NMLT-BHV-STATE-MAP",
                            format!("unknown concrete state '{concrete_name}.{concrete_field}'"),
                        )
                    })?;
                    let abstract_state =
                        abstract_system.state.get(abstract_field).ok_or_else(|| {
                            reject(
                                "NMLT-BHV-STATE-MAP",
                                format!(
                                    "unknown abstract state '{abstract_name}.{abstract_field}'"
                                ),
                            )
                        })?;
                    if concrete_state.ty != abstract_state.ty {
                        return Err(reject(
                            "NMLT-BHV-STATE-MAP-TYPE",
                            "refinement state map changes the field type",
                        ));
                    }
                    if state_map
                        .insert(concrete_field.to_owned(), abstract_field.to_owned())
                        .is_some()
                    {
                        return Err(reject(
                            "NMLT-BHV-STATE-MAP-DUPLICATE",
                            "concrete state field is mapped twice",
                        ));
                    }
                }
                UntypedRefinementItem::HiddenAction(observation) => {
                    if observation.hide_sort != Some(HideSort::Actions) {
                        return Err(reject(
                            "NMLT-BHV-REFINE-HIDE",
                            "refinement may hide actions only",
                        ));
                    }
                    hidden_actions.extend(observation.names.iter().map(|name| name.text.clone()));
                }
                UntypedRefinementItem::SurfaceOnly(_) | UntypedRefinementItem::Error(_) => {
                    return Err(reject(
                        "NMLT-BHV-REFINE-BODY",
                        "unsupported or recovered refinement item",
                    ));
                }
            }
        }
        if state_map.keys().collect::<BTreeSet<_>>()
            != concrete.state.keys().collect::<BTreeSet<_>>()
            || state_map.values().collect::<BTreeSet<_>>()
                != abstract_system.state.keys().collect::<BTreeSet<_>>()
        {
            return Err(reject(
                "NMLT-BHV-STATE-MAP-INCOMPLETE",
                format!("refinement '{concrete_name} refines {abstract_name}' needs a total map"),
            ));
        }
        validate_refinement_actions(
            concrete_name,
            abstract_name,
            concrete,
            abstract_system,
            &hidden_actions,
        )?;
        output.push(CoreRefinement {
            concrete_system: concrete_name.to_owned(),
            abstract_system: abstract_name.to_owned(),
            state_map,
            hidden_actions,
        });
    }
    output.sort_by(|left, right| {
        (&left.concrete_system, &left.abstract_system)
            .cmp(&(&right.concrete_system, &right.abstract_system))
    });
    Ok(output)
}

fn validate_refinement_actions(
    concrete_name: &str,
    abstract_name: &str,
    concrete: &CoreBehaviorSystem,
    abstract_system: &CoreBehaviorSystem,
    hidden: &BTreeSet<String>,
) -> Result<(), BehaviorDiagnostic> {
    for (name, concrete_action) in &concrete.actions {
        let is_hidden = concrete_action.hidden || hidden.contains(name);
        if is_hidden {
            if concrete_action
                .updates
                .iter()
                .any(|(field, value)| field != value)
            {
                return Err(reject(
                    "NMLT-BHV-HIDDEN-STATE",
                    format!(
                        "hidden action '{concrete_name}.{name}' changes mapped observable state"
                    ),
                ));
            }
            let resources = &concrete_action.resources;
            if !resources.consumes.is_empty() {
                return Err(reject(
                    "NMLT-BHV-HIDDEN-CONSUME",
                    format!("hidden action '{concrete_name}.{name}' consumes authority"),
                ));
            }
            if !resources.transfers.is_empty()
                || !resources.receives.is_empty()
                || !resources.requires.is_empty()
            {
                return Err(reject(
                    "NMLT-BHV-HIDDEN-AUTHORITY",
                    format!("hidden action '{concrete_name}.{name}' changes authority"),
                ));
            }
            if resources.grade.values().any(|grade| *grade != 0) {
                return Err(reject(
                    "NMLT-BHV-HIDDEN-GRADE",
                    format!("hidden action '{concrete_name}.{name}' has nonzero grade"),
                ));
            }
            if !resources.relies.is_empty() {
                return Err(reject(
                    "NMLT-BHV-HIDDEN-RELY",
                    format!("hidden action '{concrete_name}.{name}' adds a reliance"),
                ));
            }
            continue;
        }
        let abstract_action = abstract_system.actions.get(name).ok_or_else(|| {
            reject(
                "NMLT-BHV-REFINE-ACTION",
                format!(
                    "visible action '{concrete_name}.{name}' has no match in '{abstract_name}'"
                ),
            )
        })?;
        if !resource_refines(&concrete_action.resources, &abstract_action.resources) {
            return Err(reject(
                "NMLT-BHV-REFINE-RESOURCES",
                format!("action '{concrete_name}.{name}' widens its resource behavior"),
            ));
        }
    }
    Ok(())
}

fn resource_refines(concrete: &CoreResourceProfile, abstract_p: &CoreResourceProfile) -> bool {
    concrete.requires.is_subset(&abstract_p.requires)
        && concrete.consumes == abstract_p.consumes
        && concrete.transfers == abstract_p.transfers
        && concrete.receives == abstract_p.receives
        && concrete
            .grade
            .iter()
            .all(|(atom, value)| *value <= abstract_p.grade.get(atom).copied().unwrap_or_default())
        && concrete.relies.is_subset(&abstract_p.relies)
        && abstract_p.guarantees.is_subset(&concrete.guarantees)
}

fn parse_grade(source: Option<&str>) -> Result<BTreeMap<String, u64>, BehaviorDiagnostic> {
    let Some(source) = source else {
        return Ok(BTreeMap::new());
    };
    let source = source.trim();
    let body = source
        .strip_prefix("grade")
        .map(str::trim)
        .and_then(|source| source.strip_prefix('{'))
        .and_then(|source| source.strip_suffix('}'))
        .ok_or_else(|| reject("NMLT-BHV-GRADE", "expected grade { atom: Nat }"))?;
    let mut grade = BTreeMap::new();
    for entry in body
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let (atom, value) = entry
            .split_once(':')
            .ok_or_else(|| reject("NMLT-BHV-GRADE", "grade entry needs atom: Nat"))?;
        let atom = atom.trim();
        if atom.is_empty()
            || !atom
                .chars()
                .all(|character| character == '_' || character.is_alphanumeric())
        {
            return Err(reject("NMLT-BHV-GRADE", "grade atom must be an identifier"));
        }
        let value = value
            .trim()
            .parse::<u64>()
            .map_err(|_| reject("NMLT-BHV-GRADE", "grade value must be a natural number"))?;
        if grade.insert(atom.to_owned(), value).is_some() {
            return Err(reject(
                "NMLT-BHV-GRADE",
                "grade atom appears more than once",
            ));
        }
    }
    Ok(grade)
}

fn validate_fact(fact: &str, facts: &BTreeSet<String>) -> Result<(), BehaviorDiagnostic> {
    if !facts.contains(fact) {
        return Err(reject(
            "NMLT-BHV-CONTRACT-FACT",
            format!("'{fact}' is not a constructor of a declared finite enum"),
        ));
    }
    Ok(())
}

fn validate_state_type(ty: &str, facts: &BTreeSet<String>) -> Result<(), BehaviorDiagnostic> {
    if ty == "Bool"
        || ty == "Unit"
        || (is_identifier(ty) && facts.iter().any(|fact| fact.starts_with(&format!("{ty}."))))
    {
        Ok(())
    } else {
        Err(reject(
            "NMLT-BHV-STATE-TYPE",
            format!("state type '{ty}' is outside Bool, Unit, and finite enums"),
        ))
    }
}

fn parse_behavior_term(
    source: &str,
    expected: &str,
    state: &BTreeMap<String, CoreBehaviorState>,
    facts: &BTreeSet<String>,
) -> Result<CoreBehaviorTerm, BehaviorDiagnostic> {
    let source = source.trim();
    if let Some((left, right)) = source.split_once("==") {
        if expected != "Bool" {
            return Err(term_error(source, expected));
        }
        let left = left.trim();
        let right = right.trim();
        let operand_type = atomic_term_type(left, state)
            .or_else(|| atomic_term_type(right, state))
            .ok_or_else(|| term_error(source, expected))?;
        let left = parse_behavior_term(left, &operand_type, state, facts)?;
        let right = parse_behavior_term(right, &operand_type, state, facts)?;
        return Ok(CoreBehaviorTerm::Equal {
            r#type: "Bool".to_owned(),
            left: Box::new(left),
            right: Box::new(right),
        });
    }
    if let Some(inner) = source.strip_prefix('!') {
        if expected != "Bool" {
            return Err(term_error(source, expected));
        }
        return Ok(CoreBehaviorTerm::Not {
            r#type: "Bool".to_owned(),
            value: Box::new(parse_behavior_term(inner, "Bool", state, facts)?),
        });
    }
    match source {
        "true" if expected == "Bool" => {
            return Ok(CoreBehaviorTerm::Bool {
                r#type: "Bool".to_owned(),
                value: true,
            });
        }
        "false" if expected == "Bool" => {
            return Ok(CoreBehaviorTerm::Bool {
                r#type: "Bool".to_owned(),
                value: false,
            });
        }
        "unit" | "()" if expected == "Unit" => {
            return Ok(CoreBehaviorTerm::Unit {
                r#type: "Unit".to_owned(),
            });
        }
        _ => {}
    }
    if let Some(field) = state.get(source) {
        if field.ty != expected {
            return Err(term_error(source, expected));
        }
        return Ok(CoreBehaviorTerm::Read {
            r#type: expected.to_owned(),
            field: source.to_owned(),
        });
    }
    if expected != "Bool" && expected != "Unit" && is_identifier_path(source) {
        let constructor = source
            .strip_prefix(&format!("{expected}."))
            .unwrap_or(source);
        if facts.contains(&format!("{expected}.{constructor}")) {
            return Ok(CoreBehaviorTerm::Enum {
                r#type: expected.to_owned(),
                constructor: constructor.to_owned(),
            });
        }
    }
    Err(term_error(source, expected))
}

fn atomic_term_type(source: &str, state: &BTreeMap<String, CoreBehaviorState>) -> Option<String> {
    let source = source.trim();
    state
        .get(source)
        .map(|field| field.ty.clone())
        .or_else(|| matches!(source, "true" | "false").then(|| "Bool".to_owned()))
}

fn term_error(source: &str, expected: &str) -> BehaviorDiagnostic {
    reject(
        "NMLT-BHV-TERM",
        format!("term '{source}' is not a supported finite {expected} expression"),
    )
}

fn is_identifier_path(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(is_identifier)
}

fn raw_required(
    value: Option<&nmlt_core::RawTerm>,
    code: &'static str,
) -> Result<String, BehaviorDiagnostic> {
    value
        .map(|value| normalize(&value.source.text))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| reject(code, "required source term is missing"))
}

fn required_name<'a>(
    value: Option<&'a str>,
    code: &'static str,
    role: &str,
) -> Result<&'a str, BehaviorDiagnostic> {
    value.ok_or_else(|| reject(code, format!("{role} name is missing")))
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|character| character == '_' || character.is_alphabetic())
        && chars.all(|character| character == '_' || character.is_alphanumeric())
}

fn hex_digest(digest: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_resource_failures_have_distinct_codes() {
        let source = br#"
system Abstract {
  state bit: Bool = false
  observe bit
}
system Concrete {
  state bit: Bool = false
  capability token: Once<Unit>
  action tick {
    consume token
  }
  observe bit
}
refine Concrete refines Abstract {
  map state bit -> bit
  hide action tick
}
"#;
        let error = compile_behavior_single("hidden.nmlt", source).unwrap_err();
        assert_eq!(error.code(), "NMLT-BHV-HIDDEN-CONSUME");
    }
}

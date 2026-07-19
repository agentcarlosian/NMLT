//! Canonical adapter from exact source bytes to resolver input.

use nmlt_core::{
    BindingKind, M9SurfaceIssue, ProjectionIssue as CoreProjectionIssue,
    ProjectionIssueKind as CoreProjectionIssueKind, RawTerm, Span, UntypedAction, UntypedBinding,
    UntypedDeclaration, UntypedEnum, UntypedMember, UntypedObservation, UntypedProjection,
    UntypedProperty, UntypedStatement, UntypedSystem, UntypedUpdateTarget, parse_cst,
    project_untyped,
};

use crate::term::{LocalBinderInput, RawTermInput, RawTermInputKind, TermRootInput};
use crate::{
    DeclarationInput, DefPath, DefPathSegment, ImportInput, ModuleInput, Namespace,
    ProjectedModule, ProjectionIssue, ProjectionIssueKind, ResourceDimension, SourceSpan,
};

/// Parse and completely project one exact source file into the M9 resolver boundary.
///
/// Invalid UTF-8 and a UTF-8 byte-order mark are left for [`crate::resolve_modules`]
/// to reject before resolution. For valid UTF-8, all frontend recovery, coverage,
/// feature-boundary, and module-layout failures become explicit projection issues;
/// the resolver therefore cannot accept a partially projected source file.
#[must_use]
pub fn project_source_module(
    logical_module: impl Into<String>,
    repository_path: impl Into<String>,
    exact_bytes: impl Into<Vec<u8>>,
) -> ProjectedModule {
    let logical_module = logical_module.into();
    let exact_bytes = exact_bytes.into();
    let mut input = ModuleInput::new(logical_module.clone(), repository_path, exact_bytes);

    if u64::try_from(input.exact_bytes.len()).unwrap_or(u64::MAX)
        > ResourceDimension::SourceBytes.maximum()
    {
        // Avoid doing frontend work above the accepted resolver bound. The
        // resolver reports the stable resource-limit error for this input.
        return ProjectedModule::from_input(input);
    }
    let Ok(source) = std::str::from_utf8(&input.exact_bytes) else {
        return ProjectedModule::from_input(input);
    };
    if input.exact_bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return ProjectedModule::from_input(input);
    }

    let parsed = parse_cst(source);
    let projection = project_untyped(&parsed);
    input
        .projection_issues
        .extend(projection.issues.iter().map(adapt_projection_issue));
    input
        .projection_issues
        .extend(projection.m9_surface_issues().iter().map(adapt_m9_issue));

    let declarations = module_declarations(&logical_module, &projection, &mut input);
    collect_declarations(declarations, &mut input);
    ProjectedModule::from_input(input)
}

fn module_declarations<'a>(
    logical_module: &str,
    projection: &'a UntypedProjection,
    input: &mut ModuleInput,
) -> &'a [UntypedDeclaration] {
    match projection.file.declarations.as_slice() {
        [UntypedDeclaration::Module(module)] => {
            if module.name.as_ref().map(|name| name.text.as_str()) != Some(logical_module) {
                let found = module
                    .name
                    .as_ref()
                    .map_or("<missing>", |name| name.text.as_str());
                input.projection_issues.push(ProjectionIssue::new(
                    ProjectionIssueKind::UnsupportedSyntax,
                    format!(
                        "NMLT-M9-MODULE-NAME-MISMATCH: source module `{found}` does not match logical module `{logical_module}`"
                    ),
                    Some(span(module.span)),
                ));
            }
            &module.declarations
        }
        declarations => declarations,
    }
}

fn collect_declarations(declarations: &[UntypedDeclaration], input: &mut ModuleInput) {
    for declaration in declarations {
        match declaration {
            UntypedDeclaration::Import(import) => {
                if let Some(module) = &import.module {
                    input
                        .imports
                        .push(ImportInput::new(module.text.clone(), span(import.span)));
                }
            }
            UntypedDeclaration::Enum(enumeration) => collect_enum(enumeration, input),
            UntypedDeclaration::System(system) => collect_system(system, input),
            UntypedDeclaration::Module(module) => {
                // Nested or mixed module layouts already carry an M9 surface issue.
                collect_declarations(&module.declarations, input);
            }
            UntypedDeclaration::Unsupported(_) | UntypedDeclaration::Error(_) => {
                // These nodes remain represented by projection issues. They never
                // become named resolver declarations.
            }
        }
    }
}

fn collect_enum(enumeration: &UntypedEnum, input: &mut ModuleInput) {
    let Some(name) = &enumeration.name else {
        return;
    };
    input.declarations.push(DeclarationInput::new(
        DefPath::top_level(Namespace::Type, name.text.clone()),
        span(enumeration.span),
    ));
    for variant in enumeration.supported_variants() {
        let Some(variant_name) = &variant.name else {
            continue;
        };
        input.declarations.push(DeclarationInput::new(
            DefPath::new([
                DefPathSegment::new(Namespace::Type, name.text.clone()),
                DefPathSegment::new(Namespace::Constructor, variant_name.text.clone()),
            ]),
            span(variant.span),
        ));
    }
}

fn collect_system(system: &UntypedSystem, input: &mut ModuleInput) {
    let Some(name) = &system.name else {
        return;
    };
    let system_name = name.text.clone();
    input.declarations.push(DeclarationInput::new(
        DefPath::top_level(Namespace::System, system_name.clone()),
        span(system.span),
    ));

    for member in &system.members {
        let (namespace, member_name, member_span) = match member {
            UntypedMember::Binding(binding) => {
                let namespace = match binding.kind {
                    BindingKind::Const => Namespace::Value,
                    BindingKind::Input => Namespace::SystemInput,
                    BindingKind::State => Namespace::State,
                    BindingKind::Capability => Namespace::Capability,
                };
                (namespace, binding.name.as_ref(), binding.span)
            }
            UntypedMember::Action(action) => (Namespace::Action, action.name.as_ref(), action.span),
            UntypedMember::Property(property) => {
                (Namespace::Property, property.name.as_ref(), property.span)
            }
            UntypedMember::Port(_)
            | UntypedMember::Observation(_)
            | UntypedMember::SurfaceOnly(_)
            | UntypedMember::Error(_) => continue,
        };
        let Some(member_name) = member_name else {
            continue;
        };
        input.declarations.push(DeclarationInput::new(
            DefPath::new([
                DefPathSegment::new(Namespace::System, system_name.clone()),
                DefPathSegment::new(namespace, member_name.text.clone()),
            ]),
            span(member_span),
        ));
    }

    for member in &system.members {
        match member {
            UntypedMember::Binding(binding) => {
                collect_binding_terms(&system_name, binding, input);
            }
            UntypedMember::Action(action) => {
                collect_action_terms(&system_name, action, input);
            }
            UntypedMember::Property(property) => {
                collect_property_term(&system_name, property, input);
            }
            UntypedMember::Observation(observation) => {
                collect_observation_term(&system_name, observation, input);
            }
            UntypedMember::Port(_) | UntypedMember::SurfaceOnly(_) | UntypedMember::Error(_) => {}
        }
    }
}

fn collect_binding_terms(system_name: &str, binding: &UntypedBinding, input: &mut ModuleInput) {
    let Some(name) = &binding.name else {
        return;
    };
    let namespace = match binding.kind {
        BindingKind::Const => Namespace::Value,
        BindingKind::Input => Namespace::SystemInput,
        BindingKind::State => Namespace::State,
        BindingKind::Capability => Namespace::Capability,
    };
    let owner = system_member_path(system_name, namespace, &name.text);
    if let Some(declared_type) = &binding.declared_type {
        input.raw_terms.push(RawTermInput::new(
            owner.clone(),
            TermRootInput::DeclaredType,
            declared_type,
            RawTermInputKind::Type,
        ));
    }
    if let Some(initializer) = &binding.initializer {
        input.raw_terms.push(RawTermInput::new(
            owner,
            TermRootInput::Initializer,
            initializer,
            RawTermInputKind::Expression,
        ));
    }
}

fn collect_action_terms(system_name: &str, action: &UntypedAction, input: &mut ModuleInput) {
    let Some(name) = &action.name else {
        return;
    };
    let owner = system_member_path(system_name, Namespace::Action, &name.text);
    for (index, parameter) in action.supported_parameters().enumerate() {
        let (Some(name), Some(declared_type)) = (&parameter.name, &parameter.declared_type) else {
            continue;
        };
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        input.local_binders.push(LocalBinderInput {
            owner: owner.clone(),
            index,
            name: name.text.clone(),
            name_span: span(name.span),
            declared_type: RawTermInput::new(
                owner.clone(),
                TermRootInput::ActionParameterType(index),
                declared_type,
                RawTermInputKind::Type,
            ),
        });
    }

    let mut guard_index = 0_u32;
    let mut update_index = 0_u32;
    let mut output_index = 0_u32;
    let mut consume_index = 0_u32;
    for statement in &action.statements {
        match statement {
            UntypedStatement::Require { condition, .. } => {
                input.raw_terms.push(RawTermInput::new(
                    owner.clone(),
                    TermRootInput::Guard(guard_index),
                    condition,
                    RawTermInputKind::Expression,
                ));
                guard_index = guard_index.saturating_add(1);
            }
            UntypedStatement::Update { target, value, .. } => {
                let target = match target {
                    UntypedUpdateTarget::Location { source, origin, .. }
                    | UntypedUpdateTarget::Unsupported { source, origin } => RawTerm {
                        source: source.clone(),
                        origin: *origin,
                    },
                };
                input.raw_terms.push(RawTermInput::new(
                    owner.clone(),
                    TermRootInput::UpdateTarget(update_index),
                    &target,
                    RawTermInputKind::UpdateTarget,
                ));
                input.raw_terms.push(RawTermInput::new(
                    owner.clone(),
                    TermRootInput::UpdateValue(update_index),
                    value,
                    RawTermInputKind::Expression,
                ));
                update_index = update_index.saturating_add(1);
            }
            UntypedStatement::Emit { value, .. } => {
                input.raw_terms.push(RawTermInput::new(
                    owner.clone(),
                    TermRootInput::Output(output_index),
                    value,
                    RawTermInputKind::Expression,
                ));
                output_index = output_index.saturating_add(1);
            }
            UntypedStatement::Consume { capability, .. } => {
                input.raw_terms.push(RawTermInput::new(
                    owner.clone(),
                    TermRootInput::Consume(consume_index),
                    capability,
                    RawTermInputKind::Consume,
                ));
                consume_index = consume_index.saturating_add(1);
            }
            UntypedStatement::SurfaceOnly(_) | UntypedStatement::Error(_) => {}
        }
    }
}

fn collect_property_term(system_name: &str, property: &UntypedProperty, input: &mut ModuleInput) {
    let (Some(name), Some(expression)) = (&property.name, &property.expression) else {
        return;
    };
    input.raw_terms.push(RawTermInput::new(
        system_member_path(system_name, Namespace::Property, &name.text),
        TermRootInput::PropertyBody,
        expression,
        RawTermInputKind::Expression,
    ));
}

fn collect_observation_term(
    system_name: &str,
    observation: &UntypedObservation,
    input: &mut ModuleInput,
) {
    let Some(expression) = &observation.expression else {
        return;
    };
    input.raw_terms.push(RawTermInput::new(
        DefPath::top_level(Namespace::System, system_name),
        TermRootInput::ObservationItems,
        expression,
        RawTermInputKind::ExpressionList,
    ));
}

fn system_member_path(system: &str, namespace: Namespace, name: &str) -> DefPath {
    DefPath::new([
        DefPathSegment::new(Namespace::System, system),
        DefPathSegment::new(namespace, name),
    ])
}

fn adapt_projection_issue(issue: &CoreProjectionIssue) -> ProjectionIssue {
    let kind = match &issue.kind {
        CoreProjectionIssueKind::MissingCoverage { .. }
        | CoreProjectionIssueKind::DuplicateCoverage { .. }
        | CoreProjectionIssueKind::CoverageOrderMismatch => ProjectionIssueKind::CoverageGap,
        CoreProjectionIssueKind::SyntaxDiagnostic { .. }
        | CoreProjectionIssueKind::MissingDiagnosticSpan { .. }
        | CoreProjectionIssueKind::RecoveryNode
        | CoreProjectionIssueKind::DuplicateDeclaration { .. }
        | CoreProjectionIssueKind::InvalidUpdateTarget
        | CoreProjectionIssueKind::MissingProjectedChild { .. }
        | CoreProjectionIssueKind::UnexpectedProjectedNode { .. } => {
            ProjectionIssueKind::RecoveryNode
        }
    };
    ProjectionIssue::new(
        kind,
        format!("nmlt-core projection failure: {:?}", issue.kind),
        Some(span(issue.span)),
    )
}

fn adapt_m9_issue(issue: &M9SurfaceIssue) -> ProjectionIssue {
    let kind = if issue.code == "NMLT-M9-SURFACE-INCOMPLETE" {
        ProjectionIssueKind::RecoveryNode
    } else {
        ProjectionIssueKind::UnsupportedSyntax
    };
    ProjectionIssue::new(
        kind,
        format!("{}: {}", issue.code, issue.feature),
        Some(span(issue.span)),
    )
}

const fn span(value: Span) -> SourceSpan {
    SourceSpan::new(value.start, value.end)
}

use std::collections::{BTreeMap, BTreeSet};

use nmlt_hir::{
    HirNodeKind, Namespace, SemanticPathSegment, project_source_module, resolve_modules,
};
use nmlt_ir::{
    CoreAction, CoreActionParameter, CoreIdentityError, CoreModule, CoreNodeId, CoreProgram,
    CoreProperty, CorePropertyKind, CoreStateField, CoreSystem, CoreTerm, CoreTermKind, CoreType,
    CoreValidationError,
};

struct Fixture {
    resolution: nmlt_hir::ResolutionId,
    module: nmlt_hir::ModuleId,
    system: nmlt_hir::DefId,
    state: nmlt_hir::DefId,
    initializer_origin: nmlt_hir::NodeId,
}

fn fixture() -> Fixture {
    let projected = project_source_module(
        "Tiny",
        "src/tiny.nmlt",
        b"system Tiny { state ready: Bool = false }\n".as_slice(),
    );
    let resolved = resolve_modules(vec![projected]).unwrap();
    let module = resolved.module("Tiny").unwrap();
    let definition = |namespace, name: &str| {
        module
            .declarations()
            .values()
            .find(|declaration| {
                declaration
                    .key()
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.namespace == namespace && segment.name == name)
            })
            .unwrap()
            .id()
    };
    let state = definition(Namespace::State, "ready");
    let initializer_origin = module
        .hir_roots()
        .iter()
        .find(|root| {
            root.owner() == state
                && root.semantic_path().segments() == [SemanticPathSegment::Initializer]
        })
        .unwrap()
        .node();
    Fixture {
        resolution: resolved.resolution_id(),
        module: module.id(),
        system: definition(Namespace::System, "Tiny"),
        state,
        initializer_origin,
    }
}

fn program(
    fixture: &Fixture,
    term: CoreTerm,
    extras: Vec<CoreTerm>,
) -> Result<CoreProgram, CoreValidationError> {
    let state = CoreStateField::new(fixture.state, CoreType::Bool, term.id());
    let system = CoreSystem::new(
        fixture.system,
        BTreeMap::from([(fixture.state, state)]),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        Vec::new(),
    );
    let module = CoreModule::new(
        fixture.module,
        BTreeSet::new(),
        BTreeMap::new(),
        BTreeMap::from([(fixture.system, system)]),
    );
    CoreProgram::new(
        fixture.resolution,
        [module],
        std::iter::once(term).chain(extras),
    )
}

#[test]
fn valid_explicit_core_is_deterministic_and_hir_bound() {
    let fixture = fixture();
    let id = CoreNodeId::from_origin(fixture.initializer_origin, &[]).unwrap();
    let term = CoreTerm::new(
        id,
        fixture.initializer_origin,
        fixture.state,
        CoreType::Bool,
        CoreTermKind::Bool(false),
    );
    let first = program(&fixture, term.clone(), Vec::new()).unwrap();
    let second = program(&fixture, term, Vec::new()).unwrap();
    assert_eq!(first.id(), second.id());
    assert_eq!(
        id.to_string(),
        "nmlt-core-node-v1:sha256:\
         8e404138e28dbe80bf6bb6498449a1e7d22b2ed17a6e66302f476380e379dbfb"
    );
    assert_eq!(
        first.id().to_string(),
        "nmlt-core-program-v1:sha256:\
         f3f3d5fc506cb71522b4559d0b8acd7ab732202a2304a86406acf68ce1bb0f4b"
    );
    assert_eq!(first.resolved_hir_id(), fixture.resolution);
    assert_eq!(first.terms()[&id].origin(), fixture.initializer_origin);
}

#[test]
fn wrong_annotations_and_unreachable_nodes_fail_closed() {
    let fixture = fixture();
    let id = CoreNodeId::from_origin(fixture.initializer_origin, &[]).unwrap();
    let mistyped = CoreTerm::new(
        id,
        fixture.initializer_origin,
        fixture.state,
        CoreType::Nat,
        CoreTermKind::Bool(false),
    );
    assert!(matches!(
        program(&fixture, mistyped, Vec::new()),
        Err(CoreValidationError::TypeMismatch { .. })
    ));

    let root = CoreTerm::new(
        id,
        fixture.initializer_origin,
        fixture.state,
        CoreType::Bool,
        CoreTermKind::Bool(false),
    );
    let extra_id = CoreNodeId::from_origin(fixture.initializer_origin, &[1]).unwrap();
    let extra = CoreTerm::new(
        extra_id,
        fixture.initializer_origin,
        fixture.state,
        CoreType::Bool,
        CoreTermKind::Bool(true),
    );
    assert_eq!(
        program(&fixture, root, vec![extra]),
        Err(CoreValidationError::UnreachableTerm(extra_id))
    );
}

#[test]
fn core_node_insertion_paths_are_bounded() {
    let fixture = fixture();
    assert!(matches!(
        CoreNodeId::from_origin(fixture.initializer_origin, &[0; 33]),
        Err(CoreIdentityError::InsertionPathTooDeep {
            actual: 33,
            maximum: 32
        })
    ));
}

#[test]
fn locals_frames_and_temporal_indices_are_explicit() {
    let source = concat!(
        "system Toggle {\n",
        "  state ready: Bool = false\n",
        "  action set(next_value: Bool) {\n",
        "    require next_value\n",
        "    set ready = next_value\n",
        "  }\n",
        "  safety Safe = always(ready)\n",
        "}\n",
    );
    let resolved = resolve_modules(vec![project_source_module(
        "Toggle",
        "src/toggle.nmlt",
        source.as_bytes(),
    )])
    .unwrap();
    let module = resolved.module("Toggle").unwrap();
    let definition = |namespace, name: &str| {
        module
            .declarations()
            .values()
            .find(|declaration| {
                declaration
                    .key()
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.namespace == namespace && segment.name == name)
            })
            .unwrap()
            .id()
    };
    let system_id = definition(Namespace::System, "Toggle");
    let state_id = definition(Namespace::State, "ready");
    let action_id = definition(Namespace::Action, "set");
    let property_id = definition(Namespace::Property, "Safe");
    let local = module.local_binders().values().next().unwrap();

    let root = |owner, first: &SemanticPathSegment| {
        module
            .hir_roots()
            .iter()
            .find(|root| {
                root.owner() == owner && root.semantic_path().segments() == [first.clone()]
            })
            .unwrap()
            .node()
    };
    let initializer_origin = root(state_id, &SemanticPathSegment::Initializer);
    let guard_origin = root(action_id, &SemanticPathSegment::Guard(0));
    let update_origin = root(action_id, &SemanticPathSegment::UpdateValue(state_id));
    let property_origin = root(property_id, &SemanticPathSegment::PropertyBody);
    let HirNodeKind::Always {
        property: state_origin,
    } = module.hir_nodes()[&property_origin].kind()
    else {
        panic!("property root must be resolved as a dedicated always node")
    };

    let direct = |origin| CoreNodeId::from_origin(origin, &[]).unwrap();
    let initializer_id = direct(initializer_origin);
    let guard_id = direct(guard_origin);
    let update_id = direct(update_origin);
    let state_ref_id = direct(*state_origin);
    let state_predicate_id = CoreNodeId::from_origin(*state_origin, &[1]).unwrap();
    let property_body_id = direct(property_origin);

    let terms = vec![
        CoreTerm::new(
            initializer_id,
            initializer_origin,
            state_id,
            CoreType::Bool,
            CoreTermKind::Bool(false),
        ),
        CoreTerm::new(
            guard_id,
            guard_origin,
            action_id,
            CoreType::Bool,
            CoreTermKind::Local(local.id()),
        ),
        CoreTerm::new(
            update_id,
            update_origin,
            action_id,
            CoreType::Bool,
            CoreTermKind::Local(local.id()),
        ),
        CoreTerm::new(
            state_ref_id,
            *state_origin,
            property_id,
            CoreType::Bool,
            CoreTermKind::State {
                system: system_id,
                state: state_id,
            },
        ),
        CoreTerm::new(
            state_predicate_id,
            *state_origin,
            property_id,
            CoreType::StateProp { system: system_id },
            CoreTermKind::StatePredicate {
                system: system_id,
                condition: state_ref_id,
            },
        ),
        CoreTerm::new(
            property_body_id,
            property_origin,
            property_id,
            CoreType::TemporalProp { system: system_id },
            CoreTermKind::Always {
                system: system_id,
                property: state_predicate_id,
            },
        ),
    ];
    let action = CoreAction::new(
        action_id,
        system_id,
        BTreeMap::from([(
            local.id(),
            CoreActionParameter::new(local.id(), CoreType::Bool),
        )]),
        vec![guard_id],
        BTreeMap::from([(state_id, update_id)]),
        BTreeSet::new(),
        Vec::new(),
        BTreeSet::new(),
    );
    let system = CoreSystem::new(
        system_id,
        BTreeMap::from([(
            state_id,
            CoreStateField::new(state_id, CoreType::Bool, initializer_id),
        )]),
        BTreeMap::new(),
        BTreeMap::from([(action_id, action)]),
        BTreeMap::from([(
            property_id,
            CoreProperty::new(
                property_id,
                system_id,
                CorePropertyKind::Safety,
                property_body_id,
            ),
        )]),
        Vec::new(),
    );
    let core = CoreProgram::new(
        resolved.resolution_id(),
        [CoreModule::new(
            module.id(),
            BTreeSet::new(),
            BTreeMap::new(),
            BTreeMap::from([(system_id, system)]),
        )],
        terms,
    )
    .unwrap();
    assert_eq!(
        core.modules()[&module.id()].systems()[&system_id].actions()[&action_id]
            .frames()
            .len(),
        0
    );
}

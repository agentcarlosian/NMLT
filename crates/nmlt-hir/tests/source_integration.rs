use nmlt_hir::{
    DeclarationKey, Namespace, ProjectionIssueKind, ResolveError, ResolvedRef, ResourceDimension,
    project_source_module, resolve_modules, verify_resolution_readback,
};

fn key(namespace: Namespace, name: &str) -> DeclarationKey {
    DeclarationKey::top_level(namespace, name)
}

#[test]
fn raw_terms_and_action_locals_produce_a_complete_resolution_map() {
    let source = concat!(
        "system Counter {\n",
        "  state current: Nat = 0\n",
        "  action advance(next_value: Nat) {\n",
        "    require next_value > current\n",
        "    set current = next_value\n",
        "    emit next_value\n",
        "  }\n",
        "  safety Monotone = always(current >= 0)\n",
        "  observe current\n",
        "}\n",
    );
    let projected = project_source_module("Counter", "src/counter.nmlt", source.as_bytes());
    assert!(projected.projection_issues().is_empty());
    let program = resolve_modules(vec![projected]).unwrap();
    verify_resolution_readback(&program).unwrap();

    let module = program.module("Counter").unwrap();
    assert_eq!(module.local_binders().len(), 1);
    assert!(!module.hir_nodes().is_empty());
    assert!(!module.hir_roots().is_empty());
    let local = module.local_binders().values().next().unwrap();
    let local_uses = program
        .resolution_map()
        .entries()
        .values()
        .filter(|entry| entry.spelling() == "next_value")
        .collect::<Vec<_>>();
    assert_eq!(local_uses.len(), 3);
    assert!(
        local_uses
            .iter()
            .all(|entry| entry.target() == &ResolvedRef::Local(local.id()))
    );
    assert!(program.resolution_map().entries().values().any(|entry| {
        entry.spelling() == "current" && matches!(entry.target(), ResolvedRef::StateField { .. })
    }));
}

#[test]
fn term_resolution_rejects_unresolved_ambiguous_and_shadowed_names() {
    let unresolved = project_source_module(
        "Unresolved",
        "src/unresolved.nmlt",
        b"system S { state x: Missing = 0 }\n".as_slice(),
    );
    assert!(matches!(
        resolve_modules(vec![unresolved]),
        Err(ResolveError::UnresolvedReference { spelling, .. }) if spelling == "Missing"
    ));

    let ambiguous = project_source_module(
        "Ambiguous",
        "src/ambiguous.nmlt",
        concat!(
            "enum Left { same }\n",
            "enum Right { same }\n",
            "system S { state x: Left = same }\n",
        )
        .as_bytes(),
    );
    assert!(matches!(
        resolve_modules(vec![ambiguous]),
        Err(ResolveError::AmbiguousReference { spelling, candidates, .. })
            if spelling == "same" && candidates.len() == 2
    ));

    let shadowed = project_source_module(
        "Shadowed",
        "src/shadowed.nmlt",
        concat!(
            "system S {\n",
            "  state value: Nat = 0\n",
            "  action use(value: Nat) { require value > 0 }\n",
            "}\n",
        )
        .as_bytes(),
    );
    assert!(matches!(
        resolve_modules(vec![shadowed]),
        Err(ResolveError::LocalShadowing { spelling, .. }) if spelling == "value"
    ));
}

#[test]
fn raw_term_grammar_rejects_open_calls_and_noncanonical_integers() {
    for (name, expression) in [("Call", "f(x)"), ("LeadingZero", "00")] {
        let source =
            format!("system S {{ state x: Nat = 0 action a {{ require {expression} }} }}\n");
        let projected = project_source_module(name, format!("src/{name}.nmlt"), source);
        assert!(matches!(
            resolve_modules(vec![projected]),
            Err(ResolveError::TermSyntax { .. })
        ));
    }
}

#[test]
fn canonical_sources_flow_through_the_lossless_frontend_into_resolution() {
    let boolean = project_source_module(
        "Boolean",
        "examples/basics/boolean_toggle.nmlt",
        include_bytes!("../../../examples/basics/boolean_toggle.nmlt").as_slice(),
    );
    let provider = project_source_module(
        "Provider",
        "examples/technicus/provider_attempt.nmlt",
        include_bytes!("../../../examples/technicus/provider_attempt.nmlt").as_slice(),
    );

    assert!(boolean.projection_issues().is_empty());
    assert!(provider.projection_issues().is_empty());
    let program = resolve_modules(vec![provider, boolean]).unwrap();

    let boolean = program.module("Boolean").unwrap();
    assert!(
        boolean
            .declarations()
            .contains_key(&key(Namespace::System, "BooleanToggle"))
    );
    let provider = program.module("Provider").unwrap();
    assert!(
        provider
            .declarations()
            .contains_key(&key(Namespace::Type, "Phase"))
    );
    assert!(provider.declarations().keys().any(|declaration| {
        declaration.path.segments.last().is_some_and(|segment| {
            segment.namespace == Namespace::Capability && segment.name == "provider_call"
        })
    }));
}

#[test]
fn wrapper_imports_and_named_declarations_reach_the_resolver() {
    let base_source = b"module Base { enum Flag { off, on } }\n";
    let app_source = concat!(
        "module App {\n",
        "  import Base\n",
        "  system Toggle {\n",
        "    state bit: Bool = false\n",
        "    action flip { set bit = not bit }\n",
        "    safety Closed = always(bit == true or bit == false)\n",
        "  }\n",
        "}\n",
    );
    let base = project_source_module("Base", "src/base.nmlt", base_source.as_slice());
    let app = project_source_module("App", "src/app.nmlt", app_source.as_bytes());

    assert!(
        base.projection_issues().is_empty(),
        "{:?}",
        base.projection_issues()
    );
    assert!(
        app.projection_issues().is_empty(),
        "{:?}",
        app.projection_issues()
    );
    let program = resolve_modules(vec![app, base]).unwrap();
    assert_eq!(program.dependency_order(), ["Base", "App"]);

    let app = program.module("App").unwrap();
    assert_eq!(app.imports()[0].logical_module(), "Base");
    for (namespace, name) in [
        (Namespace::System, "Toggle"),
        (Namespace::State, "bit"),
        (Namespace::Action, "flip"),
        (Namespace::Property, "Closed"),
    ] {
        assert!(app.declarations().keys().any(|declaration| {
            declaration
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.namespace == namespace && segment.name == name)
        }));
    }
}

#[test]
fn unsupported_or_recovered_source_fails_closed_before_resolution() {
    let unsupported = project_source_module(
        "Unsupported",
        "src/unsupported.nmlt",
        b"data Box = boxed(Nat)\nsystem S {}\n".as_slice(),
    );
    assert!(unsupported.projection_issues().iter().any(|issue| {
        issue.kind == ProjectionIssueKind::UnsupportedSyntax
            && issue.message.contains("NMLT-M9-UNSUPPORTED-DECLARATION")
    }));
    assert!(matches!(
        resolve_modules(vec![unsupported]),
        Err(ResolveError::IncompleteProjection { .. })
    ));

    let recovered = project_source_module(
        "Recovered",
        "src/recovered.nmlt",
        b"system Broken { state : Bool = false }\n".as_slice(),
    );
    assert!(matches!(
        resolve_modules(vec![recovered]),
        Err(ResolveError::IncompleteProjection { .. })
    ));
}

#[test]
fn source_module_wrapper_must_match_the_external_module_map() {
    let mismatch = project_source_module(
        "Expected",
        "src/module.nmlt",
        b"module Actual { system S {} }\n".as_slice(),
    );
    assert!(mismatch.projection_issues().iter().any(|issue| {
        issue.kind == ProjectionIssueKind::UnsupportedSyntax
            && issue.message.contains("NMLT-M9-MODULE-NAME-MISMATCH")
    }));
    assert!(matches!(
        resolve_modules(vec![mismatch]),
        Err(ResolveError::IncompleteProjection { logical_module, .. })
            if logical_module == "Expected"
    ));
}

#[test]
fn source_above_the_frozen_bound_is_not_parsed_before_resolution_rejects_it() {
    let source = vec![b'x'; ResourceDimension::SourceBytes.maximum() as usize + 1];
    let input = project_source_module("Oversized", "src/oversized.nmlt", source);
    assert!(input.projection_issues().is_empty());
    assert!(matches!(
        resolve_modules(vec![input]),
        Err(ResolveError::ResourceLimit {
            dimension: ResourceDimension::SourceBytes,
            ..
        })
    ));
}

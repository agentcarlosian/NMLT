use nmlt_core::{
    BindingKind, UntypedDeclaration, UntypedEnumItem, UntypedMember, UntypedParameterItem,
    UntypedStatement, parse_cst, project_untyped,
};

fn nested_modules(depth: usize) -> String {
    let openings = (0..depth)
        .map(|index| format!("module M{index} {{"))
        .collect::<String>();
    format!("{openings}system S {{}}{}", "}".repeat(depth))
}

#[test]
fn module_boundaries_order_imports_enums_and_parameters_are_preserved() {
    let source = concat!(
        "module App {\n",
        "  import Base\n",
        "  enum Phase { idle, ready }\n",
        "  system Main(seed: Nat) {\n",
        "    state phase: Phase = idle\n",
        "    action advance(step: Nat) { set phase = ready }\n",
        "  }\n",
        "}\n",
        "system Outside {}\n",
    );
    let parsed = parse_cst(source);
    assert!(
        parsed.diagnostics().is_empty(),
        "{:?}",
        parsed.diagnostics()
    );

    let projection = project_untyped(&parsed);
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    assert_eq!(projection.coverage.expected, projection.coverage.projected);
    assert_eq!(projection.file.declarations.len(), 2);

    let UntypedDeclaration::Module(module) = &projection.file.declarations[0] else {
        panic!("first declaration should retain the module boundary")
    };
    assert_eq!(module.name.as_ref().unwrap().text, "App");
    assert_eq!(module.declarations.len(), 3);

    let UntypedDeclaration::Import(import) = &module.declarations[0] else {
        panic!("module import should remain first")
    };
    assert_eq!(import.module.as_ref().unwrap().text, "Base");

    let UntypedDeclaration::Enum(enumeration) = &module.declarations[1] else {
        panic!("enum should remain second")
    };
    assert_eq!(enumeration.name.as_ref().unwrap().text, "Phase");
    assert_eq!(
        enumeration
            .supported_variants()
            .map(|variant| variant.name.as_ref().unwrap().text.as_str())
            .collect::<Vec<_>>(),
        ["idle", "ready"]
    );

    let UntypedDeclaration::System(system) = &module.declarations[2] else {
        panic!("system should remain third")
    };
    assert_eq!(system.parameters.len(), 1);
    assert_eq!(
        system
            .supported_parameters()
            .next()
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .text,
        "seed"
    );
    let action = system
        .members
        .iter()
        .find_map(|member| match member {
            UntypedMember::Action(action) => Some(action),
            _ => None,
        })
        .unwrap();
    assert_eq!(action.parameters.len(), 1);
    assert_eq!(
        action
            .supported_parameters()
            .next()
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .text,
        "step"
    );
    assert_eq!(projection.file.systems().len(), 2);
}

#[test]
fn statement_terminators_stay_outside_raw_expression_nodes() {
    let source = concat!(
        "system S {\n",
        "  state x: Nat = 0\n",
        "  capability cap: Once<Effect>\n",
        "  action go { require ok; set x = y; emit x; consume cap; }\n",
        "}\n",
    );
    let projection = project_untyped(&parse_cst(source));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let systems = projection.file.systems();
    let action = systems[0]
        .members
        .iter()
        .find_map(|member| match member {
            UntypedMember::Action(action) => Some(action),
            _ => None,
        })
        .unwrap();

    let UntypedStatement::Require { condition, .. } = &action.statements[0] else {
        panic!("first statement should be require")
    };
    assert_eq!(condition.source.text, "ok");

    let UntypedStatement::Update { target, value, .. } = &action.statements[1] else {
        panic!("second statement should be update")
    };
    let nmlt_core::UntypedUpdateTarget::Location { source, .. } = target else {
        panic!("update target should be a location")
    };
    assert_eq!(source.text, "x");
    assert_eq!(value.source.text, "y");

    let UntypedStatement::Emit { value, .. } = &action.statements[2] else {
        panic!("third statement should be emit")
    };
    assert_eq!(value.source.text, "x");

    let UntypedStatement::Consume { capability, .. } = &action.statements[3] else {
        panic!("fourth statement should be consume")
    };
    assert_eq!(capability.source.text, "cap");
    assert!(
        action
            .statements
            .iter()
            .all(|statement| !format!("{statement:?}").contains(";\""))
    );
}

#[test]
fn recovered_action_statement_remains_in_source_order() {
    let source = concat!(
        "system S {\n",
        "  state x: Nat = 0\n",
        "  action go { require true; mystery; set x = 1; }\n",
        "}\n",
    );
    let projection = project_untyped(&parse_cst(source));
    assert!(!projection.is_structurally_complete());
    let systems = projection.file.systems();
    let UntypedMember::Action(action) = &systems[0].members[1] else {
        panic!("second member should be action")
    };
    assert_eq!(action.statements.len(), 3);
    assert!(matches!(
        action.statements.as_slice(),
        [
            UntypedStatement::Require { .. },
            UntypedStatement::Error(_),
            UntypedStatement::Update { .. }
        ]
    ));
}

#[test]
fn missing_names_never_borrow_identifiers_from_descendants() {
    let source = concat!(
        "system S {\n",
        "  state : Nat = 0\n",
        "  action (value: Nat) {}\n",
        "  safety = always(true)\n",
        "}\n",
    );
    let projection = project_untyped(&parse_cst(source));
    assert!(!projection.is_structurally_complete());
    let systems = projection.file.systems();
    let system = systems[0];

    let UntypedMember::Binding(binding) = &system.members[0] else {
        panic!("first member should remain a binding")
    };
    assert_eq!(binding.kind, BindingKind::State);
    assert!(binding.name.is_none());

    let UntypedMember::Action(action) = &system.members[1] else {
        panic!("second member should remain an action")
    };
    assert!(action.name.is_none());
    assert_eq!(
        action
            .supported_parameters()
            .next()
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .text,
        "value"
    );

    let UntypedMember::Property(property) = &system.members[2] else {
        panic!("third member should remain a property")
    };
    assert!(property.name.is_none());
}

#[test]
fn unsupported_declarations_are_explicit_and_covered() {
    let source = "data Maybe = none | some(Nat)\nsystem S {}\n";
    let projection = project_untyped(&parse_cst(source));
    assert!(projection.is_structurally_complete());
    assert!(matches!(
        projection.file.declarations.as_slice(),
        [
            UntypedDeclaration::Unsupported(_),
            UntypedDeclaration::System(_)
        ]
    ));
    assert_eq!(projection.coverage.expected, projection.coverage.projected);
}

#[test]
fn recovered_enum_items_remain_ordered_and_covered() {
    let source = "enum Phase {\n  idle,\n  @\n  ready\n}\nsystem S {}\n";
    let projection = project_untyped(&parse_cst(source));
    assert!(!projection.is_structurally_complete());
    assert_eq!(projection.coverage.expected, projection.coverage.projected);
    assert!(projection.coverage.missing.is_empty());
    assert!(projection.coverage.duplicated.is_empty());

    let UntypedDeclaration::Enum(enumeration) = &projection.file.declarations[0] else {
        panic!("first declaration should remain an enum")
    };
    assert_eq!(enumeration.variants.len(), 3, "{:#?}", enumeration.variants);
    assert!(matches!(
        enumeration.variants.as_slice(),
        [
            UntypedEnumItem::Variant(_),
            UntypedEnumItem::Error(_),
            UntypedEnumItem::Variant(_)
        ]
    ));
    assert_eq!(
        enumeration
            .supported_variants()
            .map(|variant| variant.name.as_ref().unwrap().text.as_str())
            .collect::<Vec<_>>(),
        ["idle", "ready"]
    );
}

#[test]
fn malformed_parameters_remain_ordered_items() {
    let source = "system S(first: Nat, : Bool, last: Nat) {}\n";
    let projection = project_untyped(&parse_cst(source));
    assert!(!projection.is_structurally_complete());
    assert_eq!(projection.coverage.expected, projection.coverage.projected);

    let systems = projection.file.systems();
    assert_eq!(systems[0].parameters.len(), 3);
    assert!(
        systems[0]
            .parameters
            .iter()
            .all(|item| matches!(item, UntypedParameterItem::Parameter(_)))
    );
    assert_eq!(
        systems[0]
            .supported_parameters()
            .map(|parameter| parameter.name.as_ref().map(|name| name.text.as_str()))
            .collect::<Vec<_>>(),
        [Some("first"), None, Some("last")]
    );
}

#[test]
fn duplicate_system_names_are_scoped_to_their_modules() {
    let source = concat!(
        "module Left { system Worker {} }\n",
        "module Right { system Worker {} }\n",
    );
    let parsed = parse_cst(source);
    assert!(
        parsed.diagnostics().is_empty(),
        "{:?}",
        parsed.diagnostics()
    );
    let projection = project_untyped(&parsed);
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    assert_eq!(projection.file.systems().len(), 2);

    let duplicate = parse_cst("module Left { system Worker {} system Worker {} }");
    assert!(
        duplicate
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == "NMLT0006")
    );
}

#[test]
fn duplicate_member_detection_is_scoped_by_typed_namespace() {
    let distinct = parse_cst(concat!(
        "system S {\n",
        "  state same: Bool = false\n",
        "  action same { set same = true }\n",
        "  safety same = always(same)\n",
        "}\n",
    ));
    let projection = project_untyped(&distinct);
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );

    let duplicate_state = project_untyped(&parse_cst(concat!(
        "system S {\n",
        "  state same: Bool = false\n",
        "  state same: Bool = true\n",
        "}\n",
    )));
    assert!(!duplicate_state.is_structurally_complete());
    assert!(duplicate_state.issues.iter().any(|issue| matches!(
        &issue.kind,
        nmlt_core::ProjectionIssueKind::DuplicateDeclaration { name, .. } if name == "same"
    )));
}

#[test]
fn m9_surface_feature_boundary_is_stable_and_explicit() {
    for source in [
        include_str!("../../../examples/basics/boolean_toggle.nmlt"),
        include_str!("../../../examples/technicus/provider_attempt.nmlt"),
    ] {
        let projection = project_untyped(&parse_cst(source));
        assert!(
            projection.is_structurally_complete(),
            "{:?}",
            projection.issues
        );
        assert!(
            projection.m9_surface_issues().is_empty(),
            "{:?}",
            projection.m9_surface_issues()
        );
    }

    let outside_slice = concat!(
        "data Box = boxed(Nat)\n",
        "system Extended(seed: Nat) {\n",
        "  const limit: Nat\n",
        "  input request: Nat\n",
        "  state slots: Nat = 0\n",
        "  port output effects: Nat\n",
        "  action use(cost: Nat) grade {cost: cost} { set slots[0] = cost }\n",
        "  resource Budget = always(true)\n",
        "  hide slots\n",
        "}\n",
    );
    let projection = project_untyped(&parse_cst(outside_slice));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let codes = projection
        .m9_surface_issues()
        .into_iter()
        .map(|issue| issue.code)
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        [
            "NMLT-M9-UNSUPPORTED-DECLARATION",
            "NMLT-M9-SYSTEM-PARAMETER",
            "NMLT-M9-SYSTEM-CONSTANT",
            "NMLT-M9-SYSTEM-INPUT",
            "NMLT-M9-PORT",
            "NMLT-M9-ACTION-GRADE",
            "NMLT-M9-SELECTED-UPDATE",
            "NMLT-M9-RESOURCE-PROPERTY",
            "NMLT-M9-HIDING",
        ]
    );
}

#[test]
fn m9_module_layout_rejections_are_explicit() {
    let mixed = project_untyped(&parse_cst("module A {} system Outside {}\n"));
    assert!(
        mixed
            .m9_surface_issues()
            .iter()
            .all(|issue| issue.code == "NMLT-M9-MODULE-LAYOUT")
    );

    let nested = project_untyped(&parse_cst("module A { module B { system S {} } }\n"));
    assert!(
        nested
            .m9_surface_issues()
            .iter()
            .any(|issue| issue.code == "NMLT-M9-NESTED-MODULE")
    );
}

#[test]
fn parser_depth_cap_bounds_all_recursive_projection_walks() {
    let at_limit = nested_modules(nmlt_core::syntax::MAX_MODULE_NESTING_DEPTH);
    let projection = project_untyped(&parse_cst(&at_limit));
    assert!(projection.is_structurally_complete());
    assert!(projection.coverage.is_exact());

    let above_limit = nested_modules(nmlt_core::syntax::MAX_MODULE_NESTING_DEPTH + 1);
    let projection = project_untyped(&parse_cst(&above_limit));
    assert!(!projection.is_structurally_complete());
    assert!(projection.coverage.is_exact());
    assert!(projection.issues.iter().any(|issue| matches!(
        issue.kind,
        nmlt_core::ProjectionIssueKind::SyntaxDiagnostic { code: "NMLT2014" }
    )));
}

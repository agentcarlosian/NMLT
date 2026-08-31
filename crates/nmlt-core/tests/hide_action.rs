use nmlt_core::{
    HideSort, ObservationKind, SyntaxKind, UntypedDeclaration, UntypedMember, hidden_action_names,
    hidden_wired_actions, parse_cst, project_untyped, surface_connections, surface_endpoint_wires,
    surface_endpoint_wires_for_left, surface_wired_action_pairs,
    surface_wired_action_pairs_for_left, surface_wires_in_compose,
};

fn hidden_boundary_source() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/refinement/hidden_connected_action.nmlt"
    ))
    .expect("hidden-boundary fixture")
}

#[test]
fn hide_action_ping_is_classified_as_action_hiding() {
    let source = hidden_boundary_source();
    let parsed = parse_cst(&source);
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

    let concrete = projection
        .file
        .declarations
        .iter()
        .find_map(|decl| match decl {
            UntypedDeclaration::System(system)
                if system.name.as_ref().map(|name| name.text.as_str())
                    == Some("ConcreteSender") =>
            {
                Some(system)
            }
            _ => None,
        })
        .expect("ConcreteSender");

    let hide = concrete
        .members
        .iter()
        .find_map(|member| match member {
            UntypedMember::Observation(observation)
                if observation.kind == ObservationKind::Hide =>
            {
                Some(observation)
            }
            _ => None,
        })
        .expect("hide declaration");

    assert_eq!(hide.hide_sort, Some(HideSort::Actions));
    assert_eq!(
        hide.names
            .iter()
            .map(|name| name.text.as_str())
            .collect::<Vec<_>>(),
        ["ping"]
    );
    assert!(hide.hides_actions());
    let m9 = projection.m9_surface_issues();
    assert!(m9.iter().any(|issue| issue.code == "NMLT-M9-HIDE-ACTION"));
    assert!(
        m9.iter()
            .any(|issue| issue.code == "NMLT-M9-ACTION-POLARITY")
    );
}

#[test]
fn state_field_hide_remains_state_fields() {
    let source = concat!(
        "system Bounded {\n",
        "  state input: Nat = 0\n",
        "  state channel: Nat = 0\n",
        "  observe channel\n",
        "  hide input, channel\n",
        "}\n",
    );
    let parsed = parse_cst(source);
    assert!(parsed.diagnostics().is_empty());
    let projection = project_untyped(&parsed);
    assert!(projection.is_structurally_complete());
    let UntypedDeclaration::System(system) = &projection.file.declarations[0] else {
        panic!("expected system");
    };
    let hide = system
        .members
        .iter()
        .find_map(|member| match member {
            UntypedMember::Observation(observation)
                if observation.kind == ObservationKind::Hide =>
            {
                Some(observation)
            }
            _ => None,
        })
        .expect("hide");
    assert_eq!(hide.hide_sort, Some(HideSort::StateFields));
    assert_eq!(
        hide.names
            .iter()
            .map(|name| name.text.as_str())
            .collect::<Vec<_>>(),
        ["input", "channel"]
    );
    assert!(!hide.hides_actions());
    assert!(
        projection
            .m9_surface_issues()
            .iter()
            .any(|issue| issue.code == "NMLT-M9-HIDING")
    );
}

#[test]
fn lone_hide_action_identifier_is_a_state_field_named_action() {
    let source = "system S {\n  hide action\n}\n";
    let parsed = parse_cst(source);
    assert!(parsed.diagnostics().is_empty());
    let projection = project_untyped(&parsed);
    let UntypedDeclaration::System(system) = &projection.file.declarations[0] else {
        panic!("expected system");
    };
    let hide = system
        .members
        .iter()
        .find_map(|member| match member {
            UntypedMember::Observation(observation)
                if observation.kind == ObservationKind::Hide =>
            {
                Some(observation)
            }
            _ => None,
        })
        .expect("hide");
    assert_eq!(hide.hide_sort, Some(HideSort::StateFields));
    assert_eq!(hide.names[0].text, "action");
}

#[test]
fn hidden_wired_actions_flags_hidden_ping_wire() {
    let source = hidden_boundary_source();
    let projection = project_untyped(&parse_cst(&source));
    let concrete = projection
        .file
        .system_named("ConcreteSender")
        .expect("ConcreteSender");
    assert_eq!(hidden_action_names(concrete), vec!["ping"]);
    assert_eq!(
        hidden_wired_actions(concrete, [("ping", "receive")]),
        vec!["ping"]
    );

    let visible = projection
        .file
        .system_named("VisibleAbstractSender")
        .expect("VisibleAbstractSender");
    assert!(hidden_action_names(visible).is_empty());
    assert!(hidden_wired_actions(visible, [("ping", "receive")]).is_empty());

    let abstract_sender = projection
        .file
        .system_named("AbstractSender")
        .expect("AbstractSender");
    assert!(hidden_action_names(abstract_sender).is_empty());
}

#[test]
fn hidden_boundary_compose_connect_projects_wiring() {
    let source = hidden_boundary_source();
    let parsed = parse_cst(&source);
    assert!(
        parsed.diagnostics().is_empty(),
        "{:?}",
        parsed.diagnostics()
    );
    assert_eq!(parsed.root().descendants(SyntaxKind::ComposeDecl).len(), 2);
    assert_eq!(parsed.root().descendants(SyntaxKind::ConnectDecl).len(), 2);

    let projection = project_untyped(&parsed);
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    assert_eq!(projection.coverage.expected, projection.coverage.projected);

    let connections = surface_connections(&projection.file);
    assert_eq!(connections.len(), 2);

    assert!(projection.file.compose_named("InvalidHiddenPing").is_some());
    assert!(projection.file.compose_named("VisibleSync").is_some());
    assert_eq!(
        surface_endpoint_wires(&projection.file),
        vec![
            ("ConcreteSender", "ping", "Receiver", "receive"),
            ("VisibleAbstractSender", "ping", "Receiver", "receive"),
        ]
    );
    assert_eq!(
        surface_wires_in_compose(&projection.file, "InvalidHiddenPing")
            .iter()
            .filter_map(|wire| wire.endpoints())
            .collect::<Vec<_>>(),
        vec![("ConcreteSender", "ping", "Receiver", "receive")]
    );
    assert_eq!(
        surface_wires_in_compose(&projection.file, "VisibleSync")
            .iter()
            .filter_map(|wire| wire.endpoints())
            .collect::<Vec<_>>(),
        vec![("VisibleAbstractSender", "ping", "Receiver", "receive")]
    );
    assert_eq!(
        surface_endpoint_wires_for_left(&projection.file, "ConcreteSender"),
        vec![("ConcreteSender", "ping", "Receiver", "receive")]
    );

    let pairs = surface_wired_action_pairs(&projection.file);
    assert_eq!(pairs, vec![("ping", "receive"), ("ping", "receive")]);

    let concrete = projection
        .file
        .system_named("ConcreteSender")
        .expect("ConcreteSender");
    let visible = projection
        .file
        .system_named("VisibleAbstractSender")
        .expect("VisibleAbstractSender");

    // NHB from real surface wires: InvalidHiddenPing flags ping; VisibleSync does not.
    let invalid_pairs = projection.wired_action_pairs_for_left("ConcreteSender");
    let visible_pairs =
        surface_wired_action_pairs_for_left(&projection.file, "VisibleAbstractSender");
    assert_eq!(invalid_pairs, vec![("ping", "receive")]);
    assert_eq!(hidden_wired_actions(concrete, invalid_pairs), vec!["ping"]);
    assert!(hidden_wired_actions(visible, visible_pairs).is_empty());

    let m9 = projection.m9_surface_issues();
    assert!(m9.iter().any(|issue| issue.code == "NMLT-M9-COMPOSE"));
    assert!(m9.iter().any(|issue| issue.code == "NMLT-M9-CONNECT"));
    assert!(m9.iter().any(|issue| issue.code == "NMLT-M9-HIDE-ACTION"));
}

#[test]
fn top_level_connect_projects_without_compose_wrapper() {
    let source = concat!(
        "system A {\n",
        "  state x: Bool = false\n",
        "  action ping { set x = x }\n",
        "  hide action ping\n",
        "}\n",
        "system B {\n",
        "  state y: Bool = false\n",
        "  action receive { set y = true }\n",
        "}\n",
        "connect A.ping -> B.receive\n",
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
    assert_eq!(
        surface_wired_action_pairs(&projection.file),
        vec![("ping", "receive")]
    );
    let a = projection
        .file
        .declarations
        .iter()
        .find_map(|decl| match decl {
            UntypedDeclaration::System(system)
                if system.name.as_ref().map(|n| n.text.as_str()) == Some("A") =>
            {
                Some(system)
            }
            _ => None,
        })
        .expect("A");
    assert_eq!(
        hidden_wired_actions(a, surface_wired_action_pairs(&projection.file)),
        vec!["ping"]
    );
    assert!(
        projection
            .m9_surface_issues()
            .iter()
            .any(|issue| issue.code == "NMLT-M9-CONNECT")
    );
}

#[test]
fn polarized_actions_do_not_change_hide_classification() {
    let projection = project_untyped(&parse_cst(&hidden_boundary_source()));
    let concrete = projection
        .file
        .system_named("ConcreteSender")
        .expect("ConcreteSender");
    assert_eq!(hidden_action_names(concrete), vec!["ping"]);
    let ping = concrete
        .members
        .iter()
        .find_map(|member| match member {
            UntypedMember::Action(action)
                if action.name.as_ref().map(|n| n.text.as_str()) == Some("ping") =>
            {
                Some(action)
            }
            _ => None,
        })
        .expect("ping");
    assert_eq!(ping.polarity, Some(nmlt_core::SurfacePolarity::Output));
}

//! Surface complementary-polarity check for `connect Left.action -> Right.action`.
//!
//! Polarities come from caller-supplied maps, same-named `port input` /
//! `port output` members, or optional `action input|output name`. Port wins
//! when both exist. Bare `action ping` stays unpolarized. This is not
//! composition elaboration; M9 fail-closes compose and action polarity.

use nmlt_core::{
    SurfacePolarity, UntypedAction, UntypedMember, action_polarity, declared_polarity,
    non_complementary_declared_wires, non_complementary_port_wires,
    non_complementary_surface_wires, parse_cst, port_polarity, project_untyped,
    surface_action_polarities, surface_declared_polarities, surface_port_polarities, surface_wires,
};

fn hidden_boundary_source() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/refinement/hidden_connected_action.nmlt"
    ))
    .expect("hidden-boundary fixture")
}

fn action_named<'a>(system: &'a nmlt_core::UntypedSystem, name: &str) -> &'a UntypedAction {
    system
        .members
        .iter()
        .find_map(|member| match member {
            UntypedMember::Action(action)
                if action.name.as_ref().map(|n| n.text.as_str()) == Some(name) =>
            {
                Some(action)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("action {name}"))
}

#[test]
fn direction_text_parses_only_input_and_output() {
    assert_eq!(
        SurfacePolarity::from_direction_text("input"),
        Some(SurfacePolarity::Input)
    );
    assert_eq!(
        SurfacePolarity::from_direction_text("output"),
        Some(SurfacePolarity::Output)
    );
    assert_eq!(SurfacePolarity::from_direction_text("in"), None);
    assert_eq!(SurfacePolarity::from_direction_text("out"), None);
    assert_eq!(SurfacePolarity::from_direction_text("internal"), None);
}

#[test]
fn complementary_rejects_same_polarity() {
    assert!(SurfacePolarity::Output.is_complementary_to(SurfacePolarity::Input));
    assert!(SurfacePolarity::Input.is_complementary_to(SurfacePolarity::Output));
    assert!(!SurfacePolarity::Input.is_complementary_to(SurfacePolarity::Input));
    assert!(!SurfacePolarity::Output.is_complementary_to(SurfacePolarity::Output));
}

#[test]
fn action_output_name_projects_output_polarity() {
    let source = concat!(
        "system S {\n",
        "  action output ping { set unit = unit }\n",
        "  state unit: Bool = false\n",
        "}\n",
    );
    let projection = project_untyped(&parse_cst(source));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let system = projection.file.system_named("S").expect("S");
    let ping = action_named(system, "ping");
    assert_eq!(ping.polarity, Some(SurfacePolarity::Output));
    assert_eq!(
        action_polarity(system, "ping"),
        Some(SurfacePolarity::Output)
    );
    assert!(
        projection
            .m9_surface_issues()
            .iter()
            .any(|issue| issue.code == "NMLT-M9-ACTION-POLARITY")
    );
    assert!(
        !projection
            .m9_surface_issues()
            .iter()
            .any(|issue| issue.code == "NMLT-M9-COMPOSE")
    );
}

#[test]
fn action_input_name_projects_input_polarity() {
    let source = concat!(
        "system S {\n",
        "  action input receive { set bit = true }\n",
        "  state bit: Bool = false\n",
        "}\n",
    );
    let projection = project_untyped(&parse_cst(source));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let system = projection.file.system_named("S").expect("S");
    assert_eq!(
        action_named(system, "receive").polarity,
        Some(SurfacePolarity::Input)
    );
}

#[test]
fn action_name_without_polarity_stays_none() {
    let source = concat!(
        "system S {\n",
        "  action ping { set unit = unit }\n",
        "  state unit: Bool = false\n",
        "}\n",
    );
    let projection = project_untyped(&parse_cst(source));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let system = projection.file.system_named("S").expect("S");
    assert_eq!(action_named(system, "ping").polarity, None);
    assert!(action_polarity(system, "ping").is_none());
    assert!(
        !projection
            .m9_surface_issues()
            .iter()
            .any(|issue| issue.code == "NMLT-M9-ACTION-POLARITY")
    );
}

#[test]
fn action_input_brace_is_named_input_with_no_polarity() {
    let source = concat!(
        "system S {\n",
        "  action input { set unit = unit }\n",
        "  state unit: Bool = false\n",
        "}\n",
    );
    let projection = project_untyped(&parse_cst(source));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let system = projection.file.system_named("S").expect("S");
    let action = action_named(system, "input");
    assert_eq!(action.polarity, None);
    assert!(
        !projection
            .m9_surface_issues()
            .iter()
            .any(|issue| issue.code == "NMLT-M9-ACTION-POLARITY")
    );
}

#[test]
fn action_output_brace_is_named_output_with_no_polarity() {
    let source = concat!(
        "system S {\n",
        "  action output { set unit = unit }\n",
        "  state unit: Bool = false\n",
        "}\n",
    );
    let projection = project_untyped(&parse_cst(source));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let system = projection.file.system_named("S").expect("S");
    assert_eq!(action_named(system, "output").polarity, None);
}

#[test]
fn hidden_boundary_fixture_has_no_ports_and_action_polarities_from_source() {
    let projection = project_untyped(&parse_cst(&hidden_boundary_source()));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );

    for name in ["ConcreteSender", "Receiver", "VisibleAbstractSender"] {
        let system = projection.file.system_named(name).expect(name);
        assert!(
            system
                .members
                .iter()
                .all(|member| !matches!(member, UntypedMember::Port(_))),
            "{name} should have no port members"
        );
        assert!(port_polarity(system, "ping").is_none());
        assert!(port_polarity(system, "receive").is_none());
    }

    let concrete = projection
        .file
        .system_named("ConcreteSender")
        .expect("ConcreteSender");
    assert_eq!(
        action_named(concrete, "ping").polarity,
        Some(SurfacePolarity::Output)
    );
    let receiver = projection.file.system_named("Receiver").expect("Receiver");
    assert_eq!(
        action_named(receiver, "receive").polarity,
        Some(SurfacePolarity::Input)
    );
    let visible = projection
        .file
        .system_named("VisibleAbstractSender")
        .expect("VisibleAbstractSender");
    assert_eq!(
        action_named(visible, "ping").polarity,
        Some(SurfacePolarity::Output)
    );
    let abstract_sender = projection
        .file
        .system_named("AbstractSender")
        .expect("AbstractSender");
    assert!(
        abstract_sender
            .members
            .iter()
            .all(|member| { !matches!(member, UntypedMember::Action(_)) })
    );

    assert!(surface_port_polarities(&projection.file).is_empty());
    assert_eq!(
        surface_action_polarities(&projection.file),
        [
            (
                ("ConcreteSender".into(), "ping".into()),
                SurfacePolarity::Output
            ),
            (
                ("Receiver".into(), "receive".into()),
                SurfacePolarity::Input
            ),
            (
                ("VisibleAbstractSender".into(), "ping".into()),
                SurfacePolarity::Output
            ),
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        surface_declared_polarities(&projection.file),
        surface_action_polarities(&projection.file)
    );
    assert!(
        non_complementary_port_wires(&projection.file).is_empty(),
        "no ports: port-only helper does not invent polarity"
    );
    assert!(
        non_complementary_declared_wires(&projection.file).is_empty(),
        "InvalidHiddenPing / VisibleSync are output->input: {wires:?}",
        wires = non_complementary_declared_wires(&projection.file)
    );

    let m9 = projection.m9_surface_issues();
    assert!(m9.iter().any(|issue| issue.code == "NMLT-M9-COMPOSE"));
    assert!(m9.iter().any(|issue| issue.code == "NMLT-M9-CONNECT"));
    assert!(
        m9.iter()
            .any(|issue| issue.code == "NMLT-M9-ACTION-POLARITY")
    );
    assert!(m9.iter().any(|issue| issue.code == "NMLT-M9-HIDE-ACTION"));
}

#[test]
fn hidden_boundary_explicit_output_input_is_complementary() {
    let projection = project_untyped(&parse_cst(&hidden_boundary_source()));
    let mismatches =
        non_complementary_surface_wires(surface_wires(&projection.file), |system, action| {
            match (system, action) {
                ("ConcreteSender" | "VisibleAbstractSender", "ping") => {
                    Some(SurfacePolarity::Output)
                }
                ("Receiver", "receive") => Some(SurfacePolarity::Input),
                _ => None,
            }
        });
    assert!(
        mismatches.is_empty(),
        "explicit Output->Input should be complementary: {mismatches:?}"
    );
}

#[test]
fn hidden_boundary_polarities_flag_only_synthetic_output_output() {
    let projection = project_untyped(&parse_cst(&hidden_boundary_source()));
    let complementary = non_complementary_declared_wires(&projection.file);
    assert!(
        complementary.is_empty(),
        "fixture wires are complementary from source polarities: {complementary:?}"
    );

    let mismatches =
        non_complementary_surface_wires(surface_wires(&projection.file), |system, action| {
            match (system, action) {
                ("ConcreteSender" | "VisibleAbstractSender", "ping") => {
                    Some(SurfacePolarity::Output)
                }
                ("Receiver", "receive") => Some(SurfacePolarity::Output),
                _ => None,
            }
        });
    let endpoints: Vec<_> = mismatches
        .iter()
        .filter_map(|wire| wire.endpoints())
        .collect();
    assert_eq!(
        endpoints,
        vec![
            ("ConcreteSender", "ping", "Receiver", "receive"),
            ("VisibleAbstractSender", "ping", "Receiver", "receive"),
        ]
    );
}

#[test]
fn hidden_boundary_explicit_output_output_is_rejected() {
    let projection = project_untyped(&parse_cst(&hidden_boundary_source()));
    let mismatches =
        non_complementary_surface_wires(surface_wires(&projection.file), |system, action| {
            match (system, action) {
                ("ConcreteSender" | "VisibleAbstractSender", "ping") => {
                    Some(SurfacePolarity::Output)
                }
                ("Receiver", "receive") => Some(SurfacePolarity::Output),
                _ => None,
            }
        });
    let endpoints: Vec<_> = mismatches
        .iter()
        .filter_map(|wire| wire.endpoints())
        .collect();
    assert_eq!(
        endpoints,
        vec![
            ("ConcreteSender", "ping", "Receiver", "receive"),
            ("VisibleAbstractSender", "ping", "Receiver", "receive"),
        ]
    );
}

#[test]
fn hidden_boundary_explicit_input_input_is_rejected() {
    let projection = project_untyped(&parse_cst(&hidden_boundary_source()));
    let mismatches = non_complementary_surface_wires(surface_wires(&projection.file), |_, _| {
        Some(SurfacePolarity::Input)
    });
    assert_eq!(mismatches.len(), 2);
}

#[test]
fn unknown_polarity_is_not_invented_or_flagged() {
    let projection = project_untyped(&parse_cst(&hidden_boundary_source()));
    let mismatches =
        non_complementary_surface_wires(surface_wires(&projection.file), |system, action| {
            match (system, action) {
                ("Receiver", "receive") => Some(SurfacePolarity::Input),
                _ => None,
            }
        });
    assert!(
        mismatches.is_empty(),
        "left polarity unknown: skip, do not invent: {mismatches:?}"
    );
}

#[test]
fn synthetic_output_output_connect_rejected_from_action_polarity() {
    let source = concat!(
        "system Left {\n",
        "  action output ping { set unit = unit }\n",
        "  state unit: Bool = false\n",
        "}\n",
        "system Right {\n",
        "  action output receive { set bit = true }\n",
        "  state bit: Bool = false\n",
        "}\n",
        "connect Left.ping -> Right.receive\n",
    );
    let projection = project_untyped(&parse_cst(source));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let mismatches = non_complementary_declared_wires(&projection.file);
    assert_eq!(
        mismatches
            .iter()
            .filter_map(|wire| wire.endpoints())
            .collect::<Vec<_>>(),
        vec![("Left", "ping", "Right", "receive")]
    );
    assert!(mismatches[0].has_non_complementary_declared_polarities(&projection.file));
    assert!(!mismatches[0].has_non_complementary_port_polarities(&projection.file));
    let m9 = projection.m9_surface_issues();
    assert!(
        m9.iter()
            .any(|issue| issue.code == "NMLT-M9-ACTION-POLARITY")
    );
    assert!(m9.iter().any(|issue| issue.code == "NMLT-M9-CONNECT"));
    assert!(!m9.iter().any(|issue| issue.code == "NMLT-M9-COMPOSE"));
}

#[test]
fn port_decls_supply_polarity_and_flag_mismatches() {
    let complementary = concat!(
        "system Left {\n",
        "  port output ping: Unit\n",
        "  action ping { set unit = unit }\n",
        "  state unit: Bool = false\n",
        "}\n",
        "system Right {\n",
        "  port input receive: Unit\n",
        "  action receive { set bit = true }\n",
        "  state bit: Bool = false\n",
        "}\n",
        "compose Sync {\n",
        "  connect Left.ping -> Right.receive\n",
        "}\n",
    );
    let projection = project_untyped(&parse_cst(complementary));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let left = projection.file.system_named("Left").expect("Left");
    let right = projection.file.system_named("Right").expect("Right");
    assert_eq!(port_polarity(left, "ping"), Some(SurfacePolarity::Output));
    assert_eq!(
        port_polarity(right, "receive"),
        Some(SurfacePolarity::Input)
    );
    assert_eq!(action_named(left, "ping").polarity, None);
    assert_eq!(
        surface_port_polarities(&projection.file),
        [
            (("Left".into(), "ping".into()), SurfacePolarity::Output),
            (("Right".into(), "receive".into()), SurfacePolarity::Input),
        ]
        .into_iter()
        .collect()
    );

    let wires = surface_wires(&projection.file);
    assert_eq!(wires.len(), 1);
    assert_eq!(
        wires[0].port_polarities(&projection.file),
        (Some(SurfacePolarity::Output), Some(SurfacePolarity::Input))
    );
    assert!(!wires[0].has_non_complementary_port_polarities(&projection.file));
    assert!(non_complementary_port_wires(&projection.file).is_empty());
    assert!(
        projection
            .m9_surface_issues()
            .iter()
            .any(|issue| issue.code == "NMLT-M9-COMPOSE")
    );
    assert!(
        projection
            .m9_surface_issues()
            .iter()
            .any(|issue| issue.code == "NMLT-M9-PORT")
    );

    let same_polarity = concat!(
        "system Left {\n",
        "  port output ping: Unit\n",
        "  action ping { set unit = unit }\n",
        "  state unit: Bool = false\n",
        "}\n",
        "system Right {\n",
        "  port output receive: Unit\n",
        "  action receive { set bit = true }\n",
        "  state bit: Bool = false\n",
        "}\n",
        "connect Left.ping -> Right.receive\n",
    );
    let projection = project_untyped(&parse_cst(same_polarity));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let mismatches = non_complementary_port_wires(&projection.file);
    assert_eq!(
        mismatches
            .iter()
            .filter_map(|wire| wire.endpoints())
            .collect::<Vec<_>>(),
        vec![("Left", "ping", "Right", "receive")]
    );
    assert!(mismatches[0].has_non_complementary_port_polarities(&projection.file));
}

#[test]
fn port_wins_over_conflicting_action_polarity() {
    let source = concat!(
        "system Left {\n",
        "  port output ping: Unit\n",
        "  action input ping { set unit = unit }\n",
        "  state unit: Bool = false\n",
        "}\n",
        "system Right {\n",
        "  port input receive: Unit\n",
        "  action output receive { set bit = true }\n",
        "  state bit: Bool = false\n",
        "}\n",
        "connect Left.ping -> Right.receive\n",
    );
    let projection = project_untyped(&parse_cst(source));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let left = projection.file.system_named("Left").expect("Left");
    let right = projection.file.system_named("Right").expect("Right");
    assert_eq!(port_polarity(left, "ping"), Some(SurfacePolarity::Output));
    assert_eq!(action_polarity(left, "ping"), Some(SurfacePolarity::Input));
    assert_eq!(
        declared_polarity(left, "ping"),
        Some(SurfacePolarity::Output)
    );
    assert_eq!(
        declared_polarity(right, "receive"),
        Some(SurfacePolarity::Input)
    );
    assert!(non_complementary_declared_wires(&projection.file).is_empty());
    // Action-only lookup would be input->output complementary too; port-only is output->input.
    assert!(non_complementary_port_wires(&projection.file).is_empty());
}

#[test]
fn port_lookup_does_not_use_action_names_without_ports() {
    let source = concat!(
        "system Left {\n",
        "  action ping { set unit = unit }\n",
        "  state unit: Bool = false\n",
        "}\n",
        "system Right {\n",
        "  port input receive: Unit\n",
        "  action receive { set bit = true }\n",
        "  state bit: Bool = false\n",
        "}\n",
        "connect Left.ping -> Right.receive\n",
    );
    let projection = project_untyped(&parse_cst(source));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let wires = surface_wires(&projection.file);
    assert_eq!(
        wires[0].port_polarities(&projection.file),
        (None, Some(SurfacePolarity::Input))
    );
    assert_eq!(
        wires[0].declared_polarities(&projection.file),
        (None, Some(SurfacePolarity::Input))
    );
    assert!(!wires[0].has_non_complementary_port_polarities(&projection.file));
    assert!(!wires[0].has_non_complementary_declared_polarities(&projection.file));
    assert!(non_complementary_port_wires(&projection.file).is_empty());
    assert!(non_complementary_declared_wires(&projection.file).is_empty());
}

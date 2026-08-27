//! Surface complementary-polarity check for `connect Left.action -> Right.action`.
//!
//! Honest scope: [`UntypedAction`] has no polarity field. Polarities come from
//! caller-supplied maps or same-named `port input` / `port output` members.
//! Paper 1 ping/receive has no ports. This is not composition elaboration; M9
//! still fail-closes compose.

use nmlt_core::{
    SurfacePolarity, UntypedMember, non_complementary_port_wires, non_complementary_surface_wires,
    parse_cst, port_polarity, project_untyped, surface_port_polarities, surface_wires,
};

fn paper1_source() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/paper1/hidden_ping_receive.nmlt"
    ))
    .expect("paper1 fixture")
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
fn paper1_actions_have_no_polarity_and_no_ports() {
    let projection = project_untyped(&parse_cst(&paper1_source()));
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

    assert!(surface_port_polarities(&projection.file).is_empty());
    assert!(
        non_complementary_port_wires(&projection.file).is_empty(),
        "no ports: do not invent polarity or flag Paper 1 wires"
    );

    let m9 = projection.m9_surface_issues();
    assert!(m9.iter().any(|issue| issue.code == "NMLT-M9-COMPOSE"));
    assert!(m9.iter().any(|issue| issue.code == "NMLT-M9-CONNECT"));
}

#[test]
fn paper1_explicit_output_input_is_complementary() {
    let projection = project_untyped(&parse_cst(&paper1_source()));
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
fn paper1_explicit_output_output_is_rejected() {
    let projection = project_untyped(&parse_cst(&paper1_source()));
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
fn paper1_explicit_input_input_is_rejected() {
    let projection = project_untyped(&parse_cst(&paper1_source()));
    let mismatches = non_complementary_surface_wires(surface_wires(&projection.file), |_, _| {
        Some(SurfacePolarity::Input)
    });
    assert_eq!(mismatches.len(), 2);
}

#[test]
fn unknown_polarity_is_not_invented_or_flagged() {
    let projection = project_untyped(&parse_cst(&paper1_source()));
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
    assert!(!wires[0].has_non_complementary_port_polarities(&projection.file));
    assert!(non_complementary_port_wires(&projection.file).is_empty());
}

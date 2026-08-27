//! Parse the Paper 1 fixture, sketch systems, adapt to OpenSystem, and feed
//! the existing finite `HiddenConnectedAction` path.
//!
//! Claim: sketch fragment + finite checker. Not a verified compiler, not
//! source-to-LTS in general. `nmlt-temporal` still does not depend on
//! `nmlt-core`.

use nmlt_core::{UntypedFile, parse_cst, project_untyped, surface_wires_in_compose};
use nmlt_paper1_sketch::open_system_from_untyped;
use nmlt_temporal::{
    ActionHiding, CompositionSpec, CongruenceIssue, CongruenceSpec, ObservationMap,
    OpenRefinementCongruenceChecker, OpenSystem, RefinementSpec,
    congruence_inputs_from_surface_names, hidden_connected_left_actions,
};

fn paper1_source() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/paper1/hidden_ping_receive.nmlt"
    ))
    .expect("paper1 fixture")
}

fn paper1_file() -> UntypedFile {
    let projection = project_untyped(&parse_cst(&paper1_source()));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    projection.file
}

fn sketched(file: &UntypedFile, name: &str) -> (nmlt_core::BooleanSketch, OpenSystem) {
    let system = file.system_named(name).unwrap_or_else(|| panic!("{name}"));
    open_system_from_untyped(system).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn compose_wires<'a>(
    file: &'a UntypedFile,
    compose_name: &str,
) -> Vec<(&'a str, &'a str, &'a str)> {
    surface_wires_in_compose(file, compose_name)
        .into_iter()
        .filter_map(|wire| Some((wire.left_action?, wire.right_action?, wire.compose_name?)))
        .collect()
}

fn identity_state_map(system: &OpenSystem) -> Vec<usize> {
    (0..system.graph().states().len()).collect()
}

fn observed(sketch: &nmlt_core::BooleanSketch) -> ObservationMap {
    ObservationMap::identity(sketch.observed_fields.iter().map(String::as_str))
}

#[test]
fn invalid_hidden_ping_sketch_reports_hidden_connected_action() {
    let file = paper1_file();
    let (concrete_sketch, concrete) = sketched(&file, "ConcreteSender");
    let (abstract_sketch, abstract_sender) = sketched(&file, "AbstractSender");
    let (receiver_sketch, receiver) = sketched(&file, "Receiver");

    assert_eq!(concrete_sketch.action_names, ["ping"]);
    assert_eq!(concrete_sketch.hidden_actions, ["ping"]);
    assert!(abstract_sketch.action_names.is_empty());
    assert_eq!(receiver_sketch.action_names, ["receive"]);

    let wires = compose_wires(&file, "InvalidHiddenPing");
    assert_eq!(wires, [("ping", "receive", "InvalidHiddenPing")]);

    let (hiding, concrete_composition) = congruence_inputs_from_surface_names(
        concrete_sketch.hidden_actions.iter().cloned(),
        [] as [(String, String); 0],
        wires,
    );
    assert_eq!(
        hiding.get("ping"),
        Some(None),
        "hide action ping lowers to ActionHiding::from_hide_actions"
    );
    assert_eq!(
        hidden_connected_left_actions(&hiding, &concrete_composition.connections),
        vec!["ping".to_owned()]
    );
    assert_eq!(
        hiding,
        ActionHiding::from_hide_actions(["ping"], [] as [(&str, &str); 0])
    );
    assert_eq!(
        concrete_composition,
        CompositionSpec::from_left_right_wires([("ping", "receive", "InvalidHiddenPing")])
    );

    let report = OpenRefinementCongruenceChecker::check(
        &concrete,
        &abstract_sender,
        &receiver,
        &CongruenceSpec {
            local_refinement: RefinementSpec {
                state_map: identity_state_map(&concrete),
                concrete_observation: observed(&concrete_sketch),
                abstract_observation: observed(&abstract_sketch),
                actions: hiding,
            },
            concrete_composition,
            abstract_composition: CompositionSpec::default(),
            peer_observation: observed(&receiver_sketch),
        },
    );
    assert!(
        report
            .issues
            .contains(&CongruenceIssue::HiddenConnectedAction("ping".to_owned())),
        "{:#?}",
        report.issues
    );
    assert!(!report.accepted);
}

#[test]
fn visible_sync_sketch_does_not_flag_hidden_connected_ping() {
    let file = paper1_file();
    let (visible_sketch, visible) = sketched(&file, "VisibleAbstractSender");
    let (receiver_sketch, receiver) = sketched(&file, "Receiver");

    assert!(visible_sketch.hidden_actions.is_empty());
    assert_eq!(visible_sketch.action_names, ["ping"]);

    let wires = compose_wires(&file, "VisibleSync");
    assert_eq!(wires, [("ping", "receive", "VisibleSync")]);

    let visible_pairs = visible_sketch
        .action_names
        .iter()
        .map(|name| (name.clone(), name.clone()))
        .collect::<Vec<_>>();
    let (hiding, concrete_composition) = congruence_inputs_from_surface_names(
        visible_sketch.hidden_actions.iter().cloned(),
        visible_pairs,
        wires,
    );
    assert_eq!(hiding.get("ping"), Some(Some("ping")));
    assert!(hidden_connected_left_actions(&hiding, &concrete_composition.connections).is_empty());
    assert_eq!(
        concrete_composition,
        CompositionSpec::from_left_right_wires([("ping", "receive", "VisibleSync")])
    );

    let report = OpenRefinementCongruenceChecker::check(
        &visible,
        &visible,
        &receiver,
        &CongruenceSpec {
            local_refinement: RefinementSpec {
                state_map: identity_state_map(&visible),
                concrete_observation: observed(&visible_sketch),
                abstract_observation: observed(&visible_sketch),
                actions: hiding,
            },
            abstract_composition: CompositionSpec::from_left_right_wires([(
                "ping",
                "receive",
                "VisibleSync",
            )]),
            concrete_composition,
            peer_observation: observed(&receiver_sketch),
        },
    );
    assert!(
        !report.issues.iter().any(
            |issue| matches!(issue, CongruenceIssue::HiddenConnectedAction(name) if name == "ping")
        ),
        "VisibleSync must not flag ping: {:#?}",
        report.issues
    );
}

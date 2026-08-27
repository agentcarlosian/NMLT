//! Parse the Paper 1 fixture, sketch systems, adapt to OpenSystem, and feed
//! the existing finite `HiddenConnectedAction` / VisibleSync paths.
//!
//! Claim: sketch fragment + finite checker. Not a verified compiler, not
//! source-to-LTS in general. `nmlt-temporal` still does not depend on
//! `nmlt-core`. VisibleSync product refinement mirrors Lean
//! `VisibleSync.visibleSync_productRefinement` at the finite-instance level
//! only (one-wire sketch product, not OpenSystem `compose`).
//!
//! Two receivers: Lean `Receiver` (not receptive at bit=true) stays rejected
//! by OpenRefinementCongruenceChecker; `ReceptiveReceiver` is the OpenSystem
//! dual, not the paper small model.

use nmlt_core::{UntypedFile, parse_cst, project_untyped, surface_wires_in_compose};
use nmlt_paper1_sketch::{
    local_refinement_spec, observation_map_from_sketch, open_system_from_untyped,
    paper1_sync_product_graph, product_observation_map,
};
use nmlt_temporal::{
    ActionHiding, CompatibilityIssue, CompositionSpec, CongruenceIssue, CongruenceSpec,
    ObservationMap, OpenRefinementChecker, OpenRefinementCongruenceChecker, OpenSystem,
    RefinementChecker, RefinementSpec, Side, congruence_inputs_from_surface_names,
    hidden_connected_left_actions, identity_refinement_spec,
};

fn paper1_source() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/paper1/hidden_ping_receive.nmlt"
    ))
    .expect("paper1 fixture")
}

fn receptive_source() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/paper1/receptive_receiver.nmlt"
    ))
    .expect("receptive dual fixture")
}

fn receptive_file() -> UntypedFile {
    let projection = project_untyped(&parse_cst(&receptive_source()));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    projection.file
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
    assert_eq!(concrete_sketch.observed_fields, ["unit"]);
    assert_eq!(abstract_sketch.observed_fields, ["unit"]);
    assert_eq!(receiver_sketch.observed_fields, ["bit"]);

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
            local_refinement: local_refinement_spec(
                &concrete,
                &concrete_sketch,
                &abstract_sketch,
                hiding,
            ),
            concrete_composition,
            abstract_composition: CompositionSpec::default(),
            peer_observation: observation_map_from_sketch(&receiver_sketch),
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
    assert_eq!(visible_sketch.observed_fields, ["unit"]);
    assert_eq!(receiver_sketch.observed_fields, ["bit"]);

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
            local_refinement: local_refinement_spec(
                &visible,
                &visible_sketch,
                &visible_sketch,
                hiding,
            ),
            abstract_composition: CompositionSpec::from_left_right_wires([(
                "ping",
                "receive",
                "VisibleSync",
            )]),
            concrete_composition,
            peer_observation: observation_map_from_sketch(&receiver_sketch),
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

#[test]
fn visible_sync_local_and_product_refinement_accepted() {
    // Finite-instance mirror of Lean VisibleSync.visibleSync_productRefinement:
    // VisibleAbstractSender (visible ping, no hide) wired to Lean Receiver.
    // Local identity refinement accepts. The one-wire sketch product accepts
    // with peer observation `bit` (false→true on both products).
    // OpenRefinementCongruenceChecker still cannot accept: sketched `receive`
    // is not enabled after the bit flips, and CompatibilityChecker requires
    // inputs in every local state. That is OpenSystem receptiveness, not a
    // VisibleSync / AbstractSender mismatch, and not M9 / general LTS / C1.
    // The OpenSystem-receptive dual is a different fixture (not this Receiver).
    let file = paper1_file();
    let (visible_sketch, visible) = sketched(&file, "VisibleAbstractSender");
    let (receiver_sketch, receiver) = sketched(&file, "Receiver");

    assert_eq!(visible_sketch.observed_fields, ["unit"]);
    assert_eq!(receiver_sketch.observed_fields, ["bit"]);
    assert_eq!(
        observation_map_from_sketch(&visible_sketch),
        ObservationMap::identity(["unit"])
    );
    assert_eq!(
        observation_map_from_sketch(&receiver_sketch),
        ObservationMap::identity(["bit"])
    );

    let local_open = OpenRefinementChecker::check(
        &visible,
        &visible,
        &identity_refinement_spec(&visible, observation_map_from_sketch(&visible_sketch)),
    );
    assert!(
        local_open.accepted,
        "VisibleAbstractSender identity open refinement: {:#?}",
        local_open.issues
    );

    let (hiding, _) = congruence_inputs_from_surface_names(
        visible_sketch.hidden_actions.iter().cloned(),
        [("ping".to_owned(), "ping".to_owned())],
        compose_wires(&file, "VisibleSync"),
    );
    let local = RefinementChecker::check(
        visible.graph(),
        visible.graph(),
        &local_refinement_spec(&visible, &visible_sketch, &visible_sketch, hiding),
    );
    assert!(
        local.accepted,
        "VisibleAbstractSender identity behavior: {:#?}",
        local.mismatches
    );

    let product = paper1_sync_product_graph(
        visible.graph(),
        receiver.graph(),
        "ping",
        "receive",
        "VisibleSync",
    )
    .expect("VisibleSync sketch product");
    assert_eq!(product.states().len(), 2);
    assert_eq!(
        product.states()[0].get("bit"),
        Some(&nmlt_temporal::Value::Bool(false))
    );
    assert_eq!(
        product.states()[1].get("bit"),
        Some(&nmlt_temporal::Value::Bool(true))
    );
    assert_eq!(product.transitions().len(), 1);
    assert_eq!(product.transitions()[0].from, 0);
    assert_eq!(product.transitions()[0].to, 1);
    assert_eq!(product.transitions()[0].kind.action(), Some("VisibleSync"));

    let product_spec = RefinementSpec {
        state_map: vec![0, 1],
        concrete_observation: product_observation_map(&visible_sketch, &receiver_sketch),
        abstract_observation: product_observation_map(&visible_sketch, &receiver_sketch),
        actions: ActionHiding::new([("VisibleSync", Some("VisibleSync"))]),
    };
    let product_report = RefinementChecker::check(&product, &product, &product_spec);
    assert!(
        product_report.accepted,
        "VisibleSync product identity: {:#?}",
        product_report.mismatches
    );

    let wires = compose_wires(&file, "VisibleSync");
    let (hiding, concrete_composition) = congruence_inputs_from_surface_names(
        visible_sketch.hidden_actions.iter().cloned(),
        [("ping".to_owned(), "ping".to_owned())],
        wires,
    );
    let report = OpenRefinementCongruenceChecker::check(
        &visible,
        &visible,
        &receiver,
        &CongruenceSpec {
            local_refinement: local_refinement_spec(
                &visible,
                &visible_sketch,
                &visible_sketch,
                hiding,
            ),
            abstract_composition: CompositionSpec::from_left_right_wires([(
                "ping",
                "receive",
                "VisibleSync",
            )]),
            concrete_composition,
            peer_observation: observation_map_from_sketch(&receiver_sketch),
        },
    );
    assert!(
        !report.issues.iter().any(
            |issue| matches!(issue, CongruenceIssue::HiddenConnectedAction(name) if name == "ping")
        ),
        "{:#?}",
        report.issues
    );
    assert!(
        !report.accepted,
        "OpenSystem congruence must not silently accept a non-receptive receive"
    );
    assert!(
        report
            .issues
            .contains(&CongruenceIssue::ConcreteCompositionIncompatible)
    );
    assert!(
        report.concrete_compatibility.issues.iter().any(|issue| {
            matches!(
                issue,
                CompatibilityIssue::InputNotReceptive {
                    side: Side::Right,
                    action,
                    state: 1
                } if action == "receive"
            )
        }),
        "expected InputNotReceptive receive @ bit=true: {:#?}",
        report.concrete_compatibility.issues
    );
}

#[test]
fn visible_sync_receptive_dual_open_congruence_accepted() {
    // OpenSystem-faithful peer, not the Lean small-model Receiver.
    // Unguarded `set bit = true` enables receive at false and true.
    let file = receptive_file();
    let (visible_sketch, visible) = sketched(&file, "VisibleAbstractSender");
    let (receiver_sketch, receiver) = sketched(&file, "ReceptiveReceiver");

    assert!(visible_sketch.hidden_actions.is_empty());
    assert_eq!(visible_sketch.action_names, ["ping"]);
    assert_eq!(visible_sketch.observed_fields, ["unit"]);
    assert_eq!(receiver_sketch.action_names, ["receive"]);
    assert_eq!(receiver_sketch.observed_fields, ["bit"]);
    assert_eq!(receiver_sketch.states.len(), 2);
    assert_eq!(
        receiver_sketch.transitions.len(),
        2,
        "receive must be enabled at bit=false and bit=true: {:#?}",
        receiver_sketch.transitions
    );
    assert!(
        receiver_sketch
            .transitions
            .iter()
            .any(|t| t.action == "receive" && t.from == 0 && t.to == 1)
    );
    assert!(
        receiver_sketch
            .transitions
            .iter()
            .any(|t| t.action == "receive" && t.from == 1 && t.to == 1)
    );

    let wires = compose_wires(&file, "VisibleSyncReceptive");
    assert_eq!(wires, [("ping", "receive", "VisibleSyncReceptive")]);

    let (hiding, concrete_composition) = congruence_inputs_from_surface_names(
        visible_sketch.hidden_actions.iter().cloned(),
        [("ping".to_owned(), "ping".to_owned())],
        wires,
    );
    assert_eq!(hiding.get("ping"), Some(Some("ping")));
    assert!(hidden_connected_left_actions(&hiding, &concrete_composition.connections).is_empty());

    let report = OpenRefinementCongruenceChecker::check(
        &visible,
        &visible,
        &receiver,
        &CongruenceSpec {
            local_refinement: local_refinement_spec(
                &visible,
                &visible_sketch,
                &visible_sketch,
                hiding,
            ),
            abstract_composition: CompositionSpec::from_left_right_wires([(
                "ping",
                "receive",
                "VisibleSyncReceptive",
            )]),
            concrete_composition,
            peer_observation: observation_map_from_sketch(&receiver_sketch),
        },
    );
    assert!(
        report.accepted,
        "receptive dual must pass OpenSystem VisibleSync congruence: {:#?}",
        report.issues
    );
    assert!(!report.issues.iter().any(
        |issue| matches!(issue, CongruenceIssue::HiddenConnectedAction(name) if name == "ping")
    ));
    assert!(
        !report.concrete_compatibility.issues.iter().any(|issue| {
            matches!(issue, CompatibilityIssue::InputNotReceptive { action, .. } if action == "receive")
        }),
        "receptive dual must not report InputNotReceptive: {:#?}",
        report.concrete_compatibility.issues
    );
}

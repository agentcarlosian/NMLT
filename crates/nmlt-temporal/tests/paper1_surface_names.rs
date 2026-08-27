//! Paper 1 name pipeline into the existing finite congruence checker.
//!
//! This test duplicates names from `examples/paper1/hidden_ping_receive.nmlt`
//! (`hide action ping`, `compose InvalidHiddenPing` / `VisibleSync`). It does
//! **not** parse surface source or elaborate an LTS: `nmlt-temporal` stays
//! independent of `nmlt-core`. Claim: surface connect names can be fed to
//! `HiddenConnectedAction` via `congruence_inputs_from_surface_names`.

use std::collections::BTreeMap;

use nmlt_temporal::{
    ActionHiding, ActionPolarity, ActionSignature, CompositionSpec, CongruenceIssue,
    CongruenceSpec, FiniteContract, FiniteGraph, ModelState, ObservationMap, OpenInterface,
    OpenRefinementCongruenceChecker, OpenSystem, PayloadPredicate, PayloadType, RefinementSpec,
    Transition, Value, congruence_inputs_from_surface_names, hidden_connected_left_actions,
};

fn state(field: &str, value: bool) -> ModelState {
    BTreeMap::from([(field.to_owned(), Value::Bool(value))])
}

fn unit() -> PayloadType {
    PayloadType::unit()
}

fn system(
    graph: FiniteGraph,
    actions: impl IntoIterator<Item = (&'static str, ActionSignature)>,
) -> OpenSystem {
    let actions = actions.into_iter().collect::<Vec<_>>();
    let assumptions = actions
        .iter()
        .filter(|(_, signature)| signature.polarity == ActionPolarity::Input)
        .map(|(action, signature)| {
            (
                *action,
                PayloadPredicate::all(
                    signature
                        .payload_type
                        .as_ref()
                        .expect("input helper actions have payload types"),
                ),
            )
        })
        .collect::<Vec<_>>();
    let guarantees = actions
        .iter()
        .filter(|(_, signature)| signature.polarity == ActionPolarity::Output)
        .map(|(action, signature)| {
            (
                *action,
                PayloadPredicate::all(
                    signature
                        .payload_type
                        .as_ref()
                        .expect("output helper actions have payload types"),
                ),
            )
        })
        .collect::<Vec<_>>();
    OpenSystem::new(
        graph,
        OpenInterface::new(actions).unwrap(),
        FiniteContract::new(assumptions, guarantees).unwrap(),
    )
    .unwrap()
}

fn concrete_sender_hiding_ping() -> OpenSystem {
    system(
        FiniteGraph::new(
            vec![state("visible", false)],
            vec![0],
            vec![Transition::action(0, "ping", 0)],
        )
        .unwrap(),
        [("ping", ActionSignature::output("bus", unit()))],
    )
}

fn abstract_sender_without_ping() -> OpenSystem {
    system(
        FiniteGraph::new(vec![state("visible", false)], vec![0], vec![]).unwrap(),
        [],
    )
}

fn visible_sender() -> OpenSystem {
    system(
        FiniteGraph::new(
            vec![state("visible", false), state("visible", true)],
            vec![0],
            vec![Transition::action(0, "ping", 1)],
        )
        .unwrap(),
        [("ping", ActionSignature::output("bus", unit()))],
    )
}

fn receiver() -> OpenSystem {
    system(
        FiniteGraph::new(
            vec![state("peer", false)],
            vec![0],
            vec![Transition::action(0, "receive", 0)],
        )
        .unwrap(),
        [("receive", ActionSignature::input("bus", unit()))],
    )
}

#[test]
fn invalid_hidden_ping_surface_names_reach_hidden_connected_action() {
    // Names from compose InvalidHiddenPing { connect ConcreteSender.ping -> Receiver.receive }
    // plus ConcreteSender's `hide action ping`. Sync label is the compose name.
    let (hiding, concrete_composition) = congruence_inputs_from_surface_names(
        ["ping"],
        [] as [(&str, &str); 0],
        [("ping", "receive", "InvalidHiddenPing")],
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

    let report = OpenRefinementCongruenceChecker::check(
        &concrete_sender_hiding_ping(),
        &abstract_sender_without_ping(),
        &receiver(),
        &CongruenceSpec {
            local_refinement: RefinementSpec {
                state_map: vec![0],
                concrete_observation: ObservationMap::identity(["visible"]),
                abstract_observation: ObservationMap::identity(["visible"]),
                actions: hiding,
            },
            concrete_composition,
            abstract_composition: CompositionSpec::default(),
            peer_observation: ObservationMap::identity(["peer"]),
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
fn visible_sync_surface_names_do_not_flag_hidden_connected_ping() {
    // Names from compose VisibleSync { connect VisibleAbstractSender.ping -> Receiver.receive }.
    // ping is visible, not hidden.
    let (hiding, concrete_composition) = congruence_inputs_from_surface_names(
        [] as [&str; 0],
        [("ping", "ping")],
        [("ping", "receive", "VisibleSync")],
    );
    assert_eq!(hiding.get("ping"), Some(Some("ping")));
    assert!(hidden_connected_left_actions(&hiding, &concrete_composition.connections).is_empty());

    let left = visible_sender();
    let report = OpenRefinementCongruenceChecker::check(
        &left,
        &visible_sender(),
        &receiver(),
        &CongruenceSpec {
            local_refinement: RefinementSpec {
                state_map: vec![0, 1],
                concrete_observation: ObservationMap::identity(["visible"]),
                abstract_observation: ObservationMap::identity(["visible"]),
                actions: hiding,
            },
            abstract_composition: CompositionSpec::from_left_right_wires([(
                "ping",
                "receive",
                "VisibleSync",
            )]),
            concrete_composition,
            peer_observation: ObservationMap::identity(["peer"]),
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
fn from_hide_actions_is_the_documented_adapter_with_from_left_right_wires() {
    let hiding = ActionHiding::from_hide_actions(["ping"], [] as [(&str, &str); 0]);
    let spec = CompositionSpec::from_left_right_wires([("ping", "receive", "InvalidHiddenPing")]);
    assert_eq!(
        hidden_connected_left_actions(&hiding, &spec.connections),
        vec!["ping".to_owned()]
    );
}

use nmlt_compile::{compile_behavior_single, compile_behavior_v2};
use nmlt_eval::{ExploreConfig, execution_path, explore};
use nmlt_ir::BehaviorCoreProgram;

const SOURCE: &str = include_str!("../../../examples/pivot/affine_continuation.nmlt");

fn program() -> BehaviorCoreProgram {
    compile_behavior_v2("examples/pivot/affine_continuation.nmlt", SOURCE).unwrap()
}

#[test]
fn continuation_requires_explicit_v2_and_does_not_grant_initial_ownership() {
    assert!(compile_behavior_single("fixture.nmlt", SOURCE).is_err());
    let p = program();
    assert_eq!(p.known_capabilities["Receiver"]["permit"], "Once<Unit>");
    assert!(p.systems["Receiver"].capabilities.is_empty());
    assert_eq!(
        p.initial_authority["Network"]["permit"].as_deref(),
        Some("Sender")
    );
    assert_eq!(p.initial_authority["Receiver"]["permit"], None);
    let json = p.to_json_pretty();
    assert!(BehaviorCoreProgram::from_canonical_json(&json).is_err());
    assert_eq!(
        BehaviorCoreProgram::from_canonical_json_v2(&json).unwrap(),
        p
    );
}

#[test]
fn received_authority_can_be_consumed_or_returned_once() {
    let p = program();
    let graph = explore(&p, "Network", ExploreConfig { max_states: 32 }).unwrap();
    assert_eq!(
        (graph.states.len(), graph.transitions.len(), graph.truncated),
        (8, 12, false)
    );
    assert!(
        !graph
            .transitions
            .iter()
            .any(|e| e.from == 0 && e.label == "Receiver.use")
    );
    for labels in [
        vec![
            "Receiver.receive|Sender.send",
            "Receiver.monitor",
            "Receiver.use",
        ],
        vec![
            "Receiver.receive|Sender.send",
            "Receiver.giveback|Sender.returned",
            "Sender.finish",
        ],
    ] {
        let labels: Vec<String> = labels.into_iter().map(str::to_owned).collect();
        let path = execution_path(&p, &graph, "0".repeat(64), &labels).unwrap();
        assert_eq!(path.states.last().unwrap().authority["permit"], None);
        let mut repeated = labels;
        repeated.push(repeated.last().unwrap().clone());
        assert!(execution_path(&p, &graph, "0".repeat(64), &repeated).is_err());
    }
    for edge in &graph.transitions {
        if edge.label == "Receiver.use" {
            assert_eq!(
                graph.states[edge.from]
                    .authority
                    .get("permit")
                    .map(String::as_str),
                Some("Receiver")
            );
            assert!(!graph.states[edge.to].authority.contains_key("permit"));
        }
    }
}

#[test]
fn unmatched_ports_cannot_create_or_remove_authority_in_v2() {
    let p = program();
    let sender = explore(&p, "Sender", ExploreConfig::default()).unwrap();
    assert_eq!((sender.states.len(), sender.transitions.len()), (1, 0));
    let receiver = explore(&p, "Receiver", ExploreConfig::default()).unwrap();
    assert_eq!((receiver.states.len(), receiver.transitions.len()), (2, 1));
    assert!(receiver.states.iter().all(|s| s.authority.is_empty()));
}

#[test]
fn claimed_execution_maps_cannot_override_declared_types_or_owners() {
    let original = program();
    let mut altered = original.clone();
    altered
        .initial_authority
        .get_mut("Network")
        .unwrap()
        .insert("permit".into(), Some("Receiver".into()));
    assert!(BehaviorCoreProgram::from_canonical_json_v2(&altered.to_json_pretty()).is_err());
    altered = original;
    altered
        .known_capabilities
        .get_mut("Receiver")
        .unwrap()
        .insert("permit".into(), "Once<Bool>".into());
    assert!(BehaviorCoreProgram::from_canonical_json_v2(&altered.to_json_pretty()).is_err());
}

#[test]
fn conflicting_received_nominal_types_are_rejected() {
    let source = SOURCE.replace(
        "action input returned(permit: Once<Unit>)",
        "action input returned(permit: Once<Bool>)",
    );
    let error = compile_behavior_v2("fixture.nmlt", source).unwrap_err();
    assert!(error.to_string().contains("inconsistent capability type"));
}

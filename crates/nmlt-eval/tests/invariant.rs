use nmlt_compile::compile_safety;
use nmlt_eval::safety_claim;
use nmlt_ir::SafetyClaim;

const SOURCE: &str = include_str!("../../../examples/pivot/safety_invariant.nmlt");

#[test]
fn invariant_and_initialized_counterexamples_use_the_common_step_graph() {
    let program = compile_safety("safety.nmlt", SOURCE.as_bytes()).unwrap();
    let property = |name| program.properties.iter().find(|p| p.name == name).unwrap();
    let claim = safety_claim(
        &program.core,
        "Network",
        property("UseAfterReceive"),
        "a".repeat(64),
        32,
    )
    .unwrap();
    let SafetyClaim::Invariant { states } = claim else {
        panic!("expected closure candidate")
    };
    assert_eq!(states.len(), 3);
    let claim = safety_claim(
        &program.core,
        "Network",
        property("NeverUsed"),
        "a".repeat(64),
        32,
    )
    .unwrap();
    let SafetyClaim::Counterexample { path } = claim else {
        panic!("expected reachable counterexample")
    };
    assert_eq!(path.actions.len(), 2);
    let claim = safety_claim(
        &program.core,
        "Network",
        property("AlreadyReceived"),
        "a".repeat(64),
        32,
    )
    .unwrap();
    let SafetyClaim::Counterexample { path } = claim else {
        panic!("expected initial counterexample")
    };
    assert!(path.actions.is_empty());
    assert!(
        safety_claim(
            &program.core,
            "Network",
            property("UseAfterReceive"),
            "a".repeat(64),
            1
        )
        .is_err()
    );
}

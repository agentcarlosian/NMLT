use std::fs;
use std::path::PathBuf;

use nmlt_compile::compile_behavior_single;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("crate lives below repository root")
        .to_owned()
}

#[test]
fn positive_slice_has_an_exact_canonical_artifact() {
    let root = repository_root();
    let source_path = root.join("examples/pivot/visible_resource_sync.nmlt");
    let expected_path = root.join("examples/pivot/visible_resource_sync.behavior-core-v1.json");
    let source = fs::read(source_path).expect("read source fixture");
    let expected = fs::read_to_string(expected_path).expect("read artifact fixture");

    let first =
        compile_behavior_single("examples/pivot/visible_resource_sync.nmlt", source.clone())
            .expect("positive fixture elaborates")
            .to_json_pretty();
    let second = compile_behavior_single("examples/pivot/visible_resource_sync.nmlt", source)
        .expect("positive fixture elaborates twice")
        .to_json_pretty();

    assert_eq!(first, expected);
    assert_eq!(first, second);
}

#[test]
fn canonical_terms_and_bidirectional_wiring_match_the_lean_fixture() {
    let root = repository_root();
    let path = "examples/pivot/canonical_terms_and_wiring.nmlt";
    let source = fs::read_to_string(root.join(path)).unwrap();
    let program = compile_behavior_single(path, source.as_bytes()).unwrap();
    let expected = fs::read_to_string(
        root.join("examples/pivot/canonical_terms_and_wiring.behavior-core-v1.json"),
    )
    .unwrap();
    assert_eq!(program.to_json_pretty(), expected);

    let conventional = source
        .replace("! false", "!false")
        .replace("! ready", "!ready")
        .replace("! done", "!done")
        .replace("==", " == ")
        .replace("Phase.Idle", "Idle")
        .replace("Phase.Ready", "Ready")
        .replace("()", "unit")
        .replace("/* phase */", "")
        .replace("/* reply */", "");
    let conventional = compile_behavior_single(path, conventional.as_bytes()).unwrap();
    assert_eq!(program.systems, conventional.systems);
    assert_eq!(program.compositions, conventional.compositions);
    assert_ne!(program.source_sha256, conventional.source_sha256);

    assert_eq!(
        program.systems["Sender"].observations,
        ["ready", "phase", "marker"]
    );
    assert!(program.systems["Receiver"].actions["idle"].hidden);
    assert!(!program.systems["Receiver"].actions["reply"].hidden);
    let network = &program.compositions["Network"];
    assert_eq!(network.left_system, "Receiver");
    assert_eq!(network.right_system, "Sender");
    let endpoints = network
        .connections
        .iter()
        .map(|connection| {
            assert_eq!(connection.left_system, network.left_system);
            assert_eq!(connection.right_system, network.right_system);
            (
                connection.left_action.as_str(),
                connection.right_action.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(endpoints, [("receive", "send"), ("reply", "ack")]);
}

#[test]
fn negative_slice_reaches_distinct_typed_boundaries() {
    let root = repository_root();
    let cases = [
        ("hidden_connected.nmlt", "NMLT-BHV-HIDDEN-BOUNDARY"),
        ("shared_capability.nmlt", "NMLT-BHV-CAPABILITY-PARTITION"),
        ("unmatched_transfer.nmlt", "NMLT-BHV-TRANSFER-MISMATCH"),
        ("hidden_consume.nmlt", "NMLT-BHV-HIDDEN-CONSUME"),
        ("hidden_grade.nmlt", "NMLT-BHV-HIDDEN-GRADE"),
        ("hidden_rely.nmlt", "NMLT-BHV-HIDDEN-RELY"),
        ("hidden_state_change.nmlt", "NMLT-BHV-HIDDEN-STATE"),
        ("incompatible_port.nmlt", "NMLT-BHV-CONNECT-DIRECTION"),
        ("undischarged_rely.nmlt", "NMLT-BHV-RELY-UNDISCHARGED"),
        ("incomplete_state_map.nmlt", "NMLT-BHV-STATE-MAP-INCOMPLETE"),
    ];

    for (fixture, expected_code) in cases {
        let source = fs::read(root.join("examples/pivot/negative").join(fixture))
            .expect("read negative fixture");
        let error = compile_behavior_single(format!("examples/pivot/negative/{fixture}"), source)
            .expect_err("negative fixture must fail");
        assert_eq!(error.code(), expected_code, "fixture: {fixture}");
    }
}

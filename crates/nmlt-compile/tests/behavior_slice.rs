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

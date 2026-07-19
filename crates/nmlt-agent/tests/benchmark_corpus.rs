use std::fs;
use std::path::PathBuf;

use nmlt_agent::digest::sha256_hex;
use nmlt_agent::evaluate_held_out_suite;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn checked_in_evaluation_is_exactly_reproducible() {
    let expected =
        fs::read_to_string(repository_root().join("benchmarks/agentic/evaluation.json")).unwrap();
    let actual = format!("{}\n", evaluate_held_out_suite().unwrap().to_json());
    assert_eq!(actual, expected);
}

#[test]
fn suite_binds_every_candidate_and_trusted_file_digest() {
    let root = repository_root();
    let suite = fs::read_to_string(root.join("benchmarks/agentic/suite.json")).unwrap();
    let evaluation = fs::read_to_string(root.join("benchmarks/agentic/evaluation.json")).unwrap();
    for relative in [
        "benchmarks/agentic/candidates/held-out-syntax.nmlt",
        "benchmarks/agentic/candidates/held-out-type.nmlt",
        "benchmarks/agentic/candidates/held-out-semantic.nmlt",
        "benchmarks/agentic/trusted/held-out-syntax-terminator.intent.txt",
        "benchmarks/agentic/trusted/held-out-syntax-terminator.property.txt",
        "benchmarks/agentic/trusted/held-out-syntax-terminator.oracle.txt",
        "benchmarks/agentic/trusted/held-out-type-boolean.intent.txt",
        "benchmarks/agentic/trusted/held-out-type-boolean.property.txt",
        "benchmarks/agentic/trusted/held-out-type-boolean.oracle.txt",
        "benchmarks/agentic/trusted/held-out-semantic-authority.intent.txt",
        "benchmarks/agentic/trusted/held-out-semantic-authority.property.txt",
        "benchmarks/agentic/trusted/held-out-semantic-authority.oracle.txt",
    ] {
        let bytes = fs::read(root.join(relative)).unwrap();
        let digest = format!("sha256:{}", sha256_hex(&bytes));
        assert!(suite.contains(relative), "suite omitted {relative}");
        assert!(
            suite.contains(&digest),
            "suite has stale digest for {relative}"
        );
        assert!(
            evaluation.contains(&digest),
            "evaluation graph has stale digest for {relative}"
        );
    }
}

#[test]
fn suite_contains_no_expected_patch_payload() {
    let suite =
        fs::read_to_string(repository_root().join("benchmarks/agentic/suite.json")).unwrap();
    assert!(!suite.contains("\"expected_patch\":"));
    assert!(!suite.contains("\"gold_patch\":"));
    assert!(suite.contains("\"expected_patch\"")); // explicitly listed as withheld
}

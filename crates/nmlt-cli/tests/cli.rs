use std::path::PathBuf;
use std::process::Command;

fn example_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/technicus/provider_attempt.nmlt")
}

#[test]
fn checks_the_provider_attempt_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .arg("check")
        .arg(example_path())
        .output()
        .expect("run nmlt");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("1 system declaration"));
    assert!(stdout.contains("no semantic verification ran"));
}

#[test]
fn evidence_is_unknown() {
    let output = Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .arg("evidence")
        .arg(example_path())
        .output()
        .expect("run nmlt");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"result\": \"unknown\""));
    assert!(stdout.contains("Only structural parsing ran."));
}

#[test]
fn prints_lossless_tokens_including_trivia() {
    let output = Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .arg("tokens")
        .arg(example_path())
        .output()
        .expect("run nmlt");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("LineComment"));
    assert!(stdout.contains("Whitespace"));
    assert!(stdout.contains("Identifier\tsystem"));
}

#[test]
fn emits_structured_model_check_results() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../benchmarks/seeded-defects/provider-attempt/dispatch-before-authorize.nmlt");
    let output = Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .args(["model-check", "--json"])
        .arg(path)
        .output()
        .expect("run nmlt");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"result\": \"refuted\""));
    assert!(stdout.contains("\"action\": \"dispatch\""));
}

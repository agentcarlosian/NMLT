use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("crate lives below repository root")
        .to_owned()
}

fn legacy_example_path() -> PathBuf {
    repository_root().join("examples/technicus/provider_attempt.nmlt")
}

fn pivot_source_path() -> PathBuf {
    repository_root().join("examples/pivot/visible_resource_sync.nmlt")
}

#[test]
fn check_is_explicitly_structural() {
    let output = Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .arg("check")
        .arg(legacy_example_path())
        .output()
        .expect("run nmlt");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("1 system declaration"));
    assert!(stdout.contains("structural parsing only"));
}

#[test]
fn removed_verifier_commands_fail() {
    for command in ["model-check", "evidence"] {
        let output = Command::new(env!("CARGO_BIN_EXE_nmlt"))
            .arg(command)
            .arg(legacy_example_path())
            .output()
            .expect("run nmlt");
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("unknown command"));
    }
}

#[test]
fn prints_lossless_tokens_including_trivia() {
    let output = Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .arg("tokens")
        .arg(legacy_example_path())
        .output()
        .expect("run nmlt");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("LineComment"));
    assert!(stdout.contains("Whitespace"));
    assert!(stdout.contains("Identifier\tsystem"));
}

#[test]
fn elaborates_then_explores_without_a_proof_claim() {
    let root = repository_root();
    let artifact_dir = root.join("target/test-artifacts");
    fs::create_dir_all(&artifact_dir).expect("create test artifact directory");
    let artifact = artifact_dir.join(format!("behavior-{}.json", std::process::id()));

    let elaborated = Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .current_dir(&root)
        .arg("elaborate")
        .arg(pivot_source_path())
        .arg("--emit-core")
        .arg(&artifact)
        .output()
        .expect("elaborate fixture");
    assert!(
        elaborated.status.success(),
        "{}",
        String::from_utf8_lossy(&elaborated.stderr)
    );

    let explored = Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .current_dir(&root)
        .args([
            "explore",
            "--behavior",
            "ConcreteNetwork",
            "--max-states",
            "8",
        ])
        .arg(&artifact)
        .output()
        .expect("explore artifact");
    fs::remove_file(&artifact).expect("remove test artifact");

    assert!(
        explored.status.success(),
        "{}",
        String::from_utf8_lossy(&explored.stderr)
    );
    let stdout = String::from_utf8(explored.stdout).unwrap();
    assert!(stdout.contains("assurance: none (reference exploration only)"));
    assert!(stdout.contains("Receiver.bit=true"));
    assert!(stdout.contains("grade=[work=3]"));
    assert!(stdout.contains("permit: ConcreteSender -> Receiver"));
}

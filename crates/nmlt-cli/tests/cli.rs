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
fn emits_reproducible_v2_paths_and_rejects_early_use() {
    let root = repository_root();
    let directory = root.join("target/test-artifacts");
    fs::create_dir_all(&directory).unwrap();
    let artifact = directory.join(format!("execution-core-{}.json", std::process::id()));
    let witness = directory.join(format!("execution-path-{}.json", std::process::id()));
    let cli = env!("CARGO_BIN_EXE_nmlt");
    let result = Command::new(cli)
        .current_dir(&root)
        .args([
            "elaborate",
            "examples/pivot/affine_continuation.nmlt",
            "--core-version",
            "v2",
            "--emit-core",
        ])
        .arg(&artifact)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(&artifact).unwrap(),
        fs::read(root.join("examples/pivot/affine_continuation.behavior-core-v2.json")).unwrap()
    );
    let run_trace = |actions: &str| {
        Command::new(cli)
            .current_dir(&root)
            .args([
                "trace",
                "--behavior",
                "Network",
                "--actions",
                actions,
                "--emit-path",
            ])
            .arg(&witness)
            .args(["--max-states", "32"])
            .arg(&artifact)
            .output()
            .unwrap()
    };
    let accepted = run_trace("Receiver.receive|Sender.send,Receiver.monitor,Receiver.use");
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    assert!(String::from_utf8_lossy(&accepted.stdout).contains("assurance: none"));
    let before = fs::read(&witness).unwrap();
    assert_eq!(
        before,
        fs::read(root.join("examples/pivot/receive_consume.behavior-execution-v1.json")).unwrap()
    );
    let rejected = run_trace("Receiver.use");
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("expected one enabled transition"));
    assert_eq!(fs::read(&witness).unwrap(), before);
    fs::remove_file(artifact).unwrap();
    fs::remove_file(witness).unwrap();
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

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../../examples/pivot/finite_retry.nmlt");

fn directory(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/r2-cli-tests");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = root.join(format!("{name}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("main.nmlt"), SOURCE).unwrap();
    directory.canonicalize().unwrap()
}

fn cli(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap()
}

fn run(directory: &Path) -> Value {
    let output = cli(
        directory,
        &[
            "run",
            "main.nmlt",
            "--behavior",
            "Main",
            "--max-steps",
            "8",
            "--emit-run",
            "run.json",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(fs::read(directory.join("run.json")).unwrap(), output.stdout);
    value
}

#[test]
fn source_run_is_deterministic_and_replays_after_moving_the_source() {
    let directory = directory("roundtrip");
    let record = run(&directory);
    assert_eq!(record["schema"], "nmlt-finite-run-v1");
    assert_eq!(record["assurance"], "none");
    assert_eq!(record["trace"]["outcome"]["kind"], "quiescent");
    assert_eq!(record["trace"]["steps"].as_array().unwrap().len(), 5);
    assert_eq!(record["trace"]["model_grade"]["work"], 2);
    let repeat = cli(
        &directory,
        &[
            "run",
            "main.nmlt",
            "--behavior",
            "Main",
            "--max-steps",
            "8",
            "--emit-run",
            "repeat.json",
        ],
    );
    assert!(repeat.status.success());
    assert_eq!(fs::read(directory.join("run.json")).unwrap(), repeat.stdout);
    fs::rename(directory.join("main.nmlt"), directory.join("moved.nmlt")).unwrap();
    let replay = cli(
        &directory,
        &["replay", "run.json", "--source", "moved.nmlt"],
    );
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    let result: Value = serde_json::from_slice(&replay.stdout).unwrap();
    assert_eq!(result["matched"], true);
    assert_eq!(result["assurance"], "none");
}

#[test]
fn changed_input_changes_execution_and_cannot_replay_old_acceptance() {
    let directory = directory("input");
    run(&directory);
    fs::write(
        directory.join("main.nmlt"),
        SOURCE.replace("reject_first: Bool = true", "reject_first: Bool = false"),
    )
    .unwrap();
    let replay = cli(&directory, &["replay", "run.json", "--source", "main.nmlt"]);
    assert!(!replay.status.success());
    assert!(String::from_utf8_lossy(&replay.stderr).contains("source mismatch"));
    let changed = cli(
        &directory,
        &[
            "run",
            "main.nmlt",
            "--behavior",
            "Main",
            "--max-steps",
            "8",
            "--emit-run",
            "changed.json",
        ],
    );
    assert!(changed.status.success());
    let record: Value = serde_json::from_slice(&changed.stdout).unwrap();
    assert_eq!(record["trace"]["steps"].as_array().unwrap().len(), 3);
    assert_eq!(record["trace"]["model_grade"]["work"], 1);
}

#[test]
fn replay_rejects_inconsistent_versions_identities_states_and_outcomes() {
    let directory = directory("tampering");
    let original = run(&directory);
    for (pointer, replacement) in [
        ("/schema", json!("nmlt-finite-run-v9")),
        ("/profile", json!("host-effects")),
        ("/assurance", json!("verified")),
        ("/implementation_sha256", json!("0".repeat(64))),
        ("/source_sha256", json!("0".repeat(64))),
        ("/source_path", json!("other.nmlt")),
        ("/artifact_sha256", json!("0".repeat(64))),
        ("/behavior", json!("Missing")),
        ("/config/max_steps", json!(0)),
        ("/config/schedule/policy", json!("unknown")),
        ("/trace/initial/authority/permit", json!("Worker")),
        ("/trace/steps/0/after/authority/permit", json!("Sender")),
        ("/trace/steps/0/label", json!("Worker.finish")),
        (
            "/trace/steps/4/after/values/Worker.copied_result/value",
            json!(false),
        ),
        ("/trace/steps/1/model_grade/work", json!(0)),
        ("/trace/model_grade/work", json!(0)),
        ("/trace/outcome/kind", json!("schedule_complete")),
    ] {
        let mut tampered = original.clone();
        *tampered
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("missing {pointer}")) = replacement;
        fs::write(
            directory.join("tampered.json"),
            serde_json::to_vec(&tampered).unwrap(),
        )
        .unwrap();
        let output = cli(
            &directory,
            &["replay", "tampered.json", "--source", "main.nmlt"],
        );
        assert!(!output.status.success(), "accepted mutation {pointer}");
    }
    for pointer in [
        "",
        "/config",
        "/config/schedule",
        "/trace",
        "/trace/steps/0",
        "/trace/steps/0/after",
        "/trace/outcome",
    ] {
        let mut tampered = original.clone();
        tampered
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(true));
        fs::write(
            directory.join("tampered.json"),
            serde_json::to_vec(&tampered).unwrap(),
        )
        .unwrap();
        assert!(
            !cli(
                &directory,
                &["replay", "tampered.json", "--source", "main.nmlt"]
            )
            .status
            .success(),
            "accepted unknown field at {pointer}"
        );
    }
}

#[test]
fn incomplete_runs_preserve_their_prefix_and_can_be_replayed_as_incomplete() {
    let directory = directory("partial");
    for (name, extra, expected_steps, expected_outcome) in [
        ("limit.json", vec!["--max-steps", "1"], 1, "step_limit"),
        (
            "unavailable.json",
            vec![
                "--max-steps",
                "8",
                "--actions",
                "Sender.send|Worker.accept,Worker.finish",
            ],
            1,
            "action_unavailable",
        ),
    ] {
        let mut args = vec!["run", "main.nmlt", "--behavior", "Main", "--emit-run", name];
        args.extend(extra);
        let output = cli(&directory, &args);
        assert!(!output.status.success());
        let record: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(record["trace"]["outcome"]["kind"], expected_outcome);
        assert_eq!(
            record["trace"]["steps"].as_array().unwrap().len(),
            expected_steps
        );
        let replay = cli(&directory, &["replay", name, "--source", "main.nmlt"]);
        assert!(replay.status.success());
        let result: Value = serde_json::from_slice(&replay.stdout).unwrap();
        assert_eq!(result["outcome"]["kind"], expected_outcome);
    }
}

#[test]
fn run_never_overwrites_sources_or_existing_records() {
    let directory = directory("overwrite");
    run(&directory);
    for path in ["main.nmlt", "run.json"] {
        let before = fs::read(directory.join(path)).unwrap();
        let output = cli(
            &directory,
            &[
                "run",
                "main.nmlt",
                "--behavior",
                "Main",
                "--max-steps",
                "8",
                "--emit-run",
                path,
            ],
        );
        assert!(!output.status.success());
        assert_eq!(before, fs::read(directory.join(path)).unwrap());
    }
}

#[test]
fn invalid_options_bounds_and_sources_do_not_create_records() {
    let directory = directory("invalid");
    for extra in [
        vec!["--max-steps", "0"],
        vec!["--max-steps", "10001"],
        vec!["--max-steps", "2", "--max-steps", "3"],
        vec![
            "--max-steps",
            "8",
            "--actions",
            "Sender.send|Worker.accept,,Worker.fail",
        ],
        vec!["--max-steps", "8", "--unknown", "value"],
        vec!["--max-steps"],
    ] {
        let mut args = vec![
            "run",
            "main.nmlt",
            "--behavior",
            "Main",
            "--emit-run",
            "invalid.json",
        ];
        args.extend(extra);
        assert!(!cli(&directory, &args).status.success());
        assert!(!directory.join("invalid.json").exists());
    }
    fs::write(
        directory.join("main.nmlt"),
        SOURCE.replace(
            "set copied_result = result",
            "set copied_result = Phase.Complete",
        ),
    )
    .unwrap();
    assert!(
        !cli(
            &directory,
            &[
                "run",
                "main.nmlt",
                "--behavior",
                "Main",
                "--max-steps",
                "8",
                "--emit-run",
                "invalid.json"
            ]
        )
        .status
        .success()
    );
    assert!(!directory.join("invalid.json").exists());
}

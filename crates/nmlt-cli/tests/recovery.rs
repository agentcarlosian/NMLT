//! Coherent durable-prefix fixtures exercise every write ordering boundary.
//! Real process death and parent cleanup are separately tested by the runtime.
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const SOURCE: &str = "fn main(input: Int) -> Outcome<Int> { let job = job_start_square(input); let result = job_collect(job); match result { Ok(value) => Ok(value + value), Err(message) => { let backup = job_start_square(5); job_collect(backup) } } }";
fn cli(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}
fn assert_ok(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn original(label: &str, attempts: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/r2-recovery-tests")
        .join(format!("{label}-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("main.nmlt"), SOURCE).unwrap();
    let output = cli(
        &dir,
        &[
            "run",
            "main.nmlt",
            "--entry",
            "main",
            "--arg",
            "input=3",
            "--max-steps",
            "100",
            "--emit-run",
            "original.json",
            "--jobs-dir",
            "original",
            "--job-slots",
            "1",
            "--max-jobs",
            attempts,
            "--job-timeout-ms",
            "10000",
        ],
    );
    assert_ok(&output);
    dir
}
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let destination = to.join(entry.file_name());
        let metadata = entry.file_type().unwrap();
        assert!(!metadata.is_symlink());
        if metadata.is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}
fn prefix(path: &Path, rows: usize) {
    let original = fs::read(path).unwrap();
    let bytes: Vec<u8> = original
        .split_inclusive(|b| *b == b'\n')
        .take(rows + 1)
        .flatten()
        .copied()
        .collect();
    fs::write(path, bytes).unwrap();
}
fn fixture(
    dir: &Path,
    label: &str,
    commands: usize,
    source_rows: usize,
    saved_observation: bool,
) -> PathBuf {
    let destination = dir.join(label);
    copy_tree(&dir.join("original"), &destination);
    prefix(&destination.join("journal.jsonl"), commands);
    prefix(&destination.join("source-journal.jsonl"), source_rows);
    for entry in fs::read_dir(&destination).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("evidence-") {
            let value: Value = serde_json::from_slice(&fs::read(entry.path()).unwrap()).unwrap();
            let before = value["before"].as_u64().unwrap() as usize;
            if before >= commands && !(saved_observation && before == commands) {
                fs::remove_file(entry.path()).unwrap();
            }
        } else if commands < 2 && (name.starts_with("dispatch-") || name.starts_with("input-")) {
            fs::remove_file(entry.path()).unwrap();
        }
    }
    destination
}
fn commands(path: &Path, kind: &str) -> usize {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .skip(1)
        .filter(|line| serde_json::from_str::<Value>(line).unwrap()["command"]["kind"] == kind)
        .count()
}
fn resume(dir: &Path, label: &str, output: &str, ack: bool) -> Output {
    let mut args = vec!["jobs-resume", label, "--emit-run", output];
    if ack {
        args.extend([
            "--acknowledge-uncertain-effects",
            "fixture operator has reconciled the unknown local attempt",
        ]);
    }
    cli(dir, &args)
}

#[derive(Serialize)]
struct RuntimeRow {
    sequence: u32,
    previous_sha256: String,
    command: nmlt_runtime::Command,
    receipt: nmlt_runtime::Receipt,
    state_sha256: String,
}
#[derive(Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum Captured {
    Observation(nmlt_runtime::session::Observation),
}
#[derive(Serialize)]
struct Saved {
    before: u32,
    command: nmlt_runtime::Command,
    evidence: Captured,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SourceItem {
    Reply {
        result: Result<(), nmlt_workflow::JobError>,
        through: u32,
    },
}
#[derive(Serialize)]
struct SourceRow {
    sequence: usize,
    previous_sha256: String,
    item: SourceItem,
}
fn tail_hash(bytes: &[u8]) -> String {
    nmlt_runtime::sha256(
        bytes[..bytes.len() - 1]
            .rsplit(|b| *b == b'\n')
            .next()
            .unwrap(),
    )
}

#[test]
fn captured_timeout_reply_is_retried_only_after_explicit_reconciliation() {
    use nmlt_runtime::{self as rt, process};
    let dir = original("timeout-reply", "2");
    let jobs = fixture(&dir, "timeout", 2, 3, false);
    let manifest: rt::session::Manifest =
        serde_json::from_slice(&fs::read(jobs.join("manifest.json")).unwrap()).unwrap();
    let path = jobs.join("journal.jsonl");
    let mut bytes = fs::read(&path).unwrap();
    let mut state = rt::Lifecycle::new(manifest.spec).unwrap();
    for line in bytes
        .split(|b| *b == b'\n')
        .skip(1)
        .filter(|line| !line.is_empty())
    {
        let value: Value = serde_json::from_slice(line).unwrap();
        let command = serde_json::from_value(value["command"].clone()).unwrap();
        state = state.step(&command).unwrap().0;
    }
    let command = rt::Command::Timeout {
        control: state.attempts()[0].control.clone(),
    };
    let (next, receipt) = state.step(&command).unwrap();
    let row = RuntimeRow {
        sequence: 3,
        previous_sha256: tail_hash(&bytes),
        command: command.clone(),
        receipt,
        state_sha256: rt::sha256(&serde_json::to_vec(&next).unwrap()),
    };
    bytes.extend(serde_json::to_vec(&row).unwrap());
    bytes.push(b'\n');
    fs::write(path, bytes).unwrap();
    let original: Value =
        serde_json::from_slice(&fs::read(dir.join("original.json")).unwrap()).unwrap();
    let mut observation: rt::session::Observation =
        serde_json::from_value(original["snapshot"]["observations"][0].clone()).unwrap();
    observation.completion = Err(process::Failure {
        kind: process::FailureKind::Timeout,
        child_reaped: true,
        detail: None,
    });
    fs::write(
        jobs.join("evidence-2.json"),
        serde_json::to_vec(&Saved {
            before: 2,
            command,
            evidence: Captured::Observation(observation),
        })
        .unwrap(),
    )
    .unwrap();
    let path = jobs.join("source-journal.jsonl");
    let mut bytes = fs::read(&path).unwrap();
    let row = SourceRow {
        sequence: 3,
        previous_sha256: tail_hash(&bytes),
        item: SourceItem::Reply {
            result: Err(nmlt_workflow::JobError::HostFailure),
            through: 3,
        },
    };
    bytes.extend(serde_json::to_vec(&row).unwrap());
    bytes.push(b'\n');
    fs::write(path, bytes).unwrap();
    assert!(
        !resume(&dir, "timeout", "blocked.json", false)
            .status
            .success()
    );
    let run = resume(&dir, "timeout", "resumed.json", true);
    assert_ok(&run);
    let record: Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(
        record["execution"]["stop"]["value"],
        json!({"kind":"ok","value":{"kind":"int","value":25}})
    );
    assert!(
        record["trace"]
            .as_str()
            .unwrap()
            .contains("\"kind\":\"retry\"")
    );
    assert_eq!(commands(&jobs.join("journal.jsonl"), "dispatch"), 2);
    assert_ok(&cli(
        &dir,
        &[
            "replay",
            "resumed.json",
            "--source",
            "timeout/sources/main.nmlt",
        ],
    ));
}

#[test]
fn explicit_tail_repair_quarantines_only_incomplete_appends_and_never_reexecutes() {
    let dir = original("tail-repair", "2");
    let jobs = fixture(&dir, "torn", 4, 3, true);
    let runtime_path = jobs.join("journal.jsonl");
    let source_path = jobs.join("source-journal.jsonl");
    let mut runtime_bytes = fs::read(&runtime_path).unwrap();
    runtime_bytes.extend(b"{\"sequence\":5,\"previous_sha256\":");
    let mut source_bytes = fs::read(&source_path).unwrap();
    source_bytes.extend(b"{\"sequence\":3,\"previous_sha256\":");
    fs::write(&runtime_path, &runtime_bytes).unwrap();
    fs::write(&source_path, &source_bytes).unwrap();
    assert!(!resume(&dir, "torn", "blocked.json", false).status.success());
    assert!(!cli(&dir, &["jobs-repair", "torn"]).status.success());
    let repaired = cli(
        &dir,
        &[
            "jobs-repair",
            "torn",
            "--acknowledge-incomplete-tail",
            "operator inspected interrupted final writes",
        ],
    );
    assert_ok(&repaired);
    let value: Value = serde_json::from_slice(&repaired.stdout).unwrap();
    for (key, original) in [
        ("runtime_tail", runtime_bytes),
        ("source_tail", source_bytes),
    ] {
        let archive: Value =
            serde_json::from_slice(&fs::read(dir.join(value[key].as_str().unwrap())).unwrap())
                .unwrap();
        assert_eq!(archive["original_sha256"], nmlt_runtime::sha256(&original));
        assert!(!archive["discarded_bytes"].as_array().unwrap().is_empty());
    }
    assert_ok(&resume(&dir, "torn", "resumed.json", false));
    assert_eq!(commands(&runtime_path, "dispatch"), 1);
    assert_eq!(commands(&runtime_path, "collect"), 1);
    let corrupt = fixture(&dir, "corrupt-complete", 2, 3, false);
    let path = corrupt.join("source-journal.jsonl");
    let mut bytes = fs::read(&path).unwrap();
    bytes.extend(b"{\"invalid\":true}\n{\"partial\":");
    fs::write(&path, &bytes).unwrap();
    assert!(
        !cli(
            &dir,
            &[
                "jobs-repair",
                "corrupt-complete",
                "--acknowledge-incomplete-tail",
                "operator inspected tail"
            ]
        )
        .status
        .success()
    );
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn durable_boundaries_resume_saved_source_without_redispatch_or_duplicate_collection() {
    let dir = original("boundaries", "2");
    // Deliberately change the working file. Every resume uses its retained closure.
    fs::write(dir.join("main.nmlt"), "unfinished working edit").unwrap();
    for (label, journal, source, saved, unknown) in [
        ("intent", 0, 1, false, false),
        ("reserved", 1, 1, false, false),
        ("dispatched", 2, 1, false, true),
        ("start_reply", 2, 2, false, true),
        ("collect_intent", 2, 3, false, true),
        ("saved_observation", 2, 3, true, false),
        ("delivered", 3, 3, true, false),
        ("collected", 4, 3, true, false),
        ("collect_reply", 4, 4, true, false),
    ] {
        let jobs = fixture(&dir, label, journal, source, saved);
        if unknown {
            let blocked = resume(&dir, label, &format!("{label}-blocked.json"), false);
            assert!(!blocked.status.success());
            assert!(
                String::from_utf8_lossy(&blocked.stderr).contains("unresolved external effects")
            );
            assert_eq!(commands(&jobs.join("journal.jsonl"), "dispatch"), 1);
        }
        if label == "saved_observation" {
            assert_ok(&cli(&dir, &["jobs-recover", label]));
        }
        let output = format!("{label}-resumed.json");
        let run = resume(&dir, label, &output, unknown);
        assert_ok(&run);
        let record: Value = serde_json::from_slice(&run.stdout).unwrap();
        let value = if unknown { 25 } else { 18 };
        assert_eq!(
            record["execution"]["stop"]["value"],
            json!({"kind":"ok","value":{"kind":"int","value":value}}),
            "{label}"
        );
        assert_eq!(
            commands(&jobs.join("journal.jsonl"), "dispatch"),
            if unknown { 2 } else { 1 },
            "{label}"
        );
        assert_eq!(
            commands(&jobs.join("journal.jsonl"), "collect"),
            if unknown { 2 } else { 1 },
            "{label}"
        );
        assert_eq!(
            commands(&jobs.join("journal.jsonl"), "reconcile"),
            usize::from(unknown),
            "{label}"
        );
        let before = fs::read(jobs.join("journal.jsonl")).unwrap();
        let replay = cli(
            &dir,
            &[
                "replay",
                &output,
                "--source",
                &format!("{label}/sources/main.nmlt"),
            ],
        );
        assert_ok(&replay);
        assert_eq!(fs::read(jobs.join("journal.jsonl")).unwrap(), before);
        // A fully collected resume is idempotent even after a previous resume.
        assert_ok(&resume(&dir, label, &format!("{label}-again.json"), false));
        assert_eq!(fs::read(jobs.join("journal.jsonl")).unwrap(), before);
    }
}

#[test]
fn uncertainty_retains_spend_and_cannot_reset_the_attempt_budget() {
    let dir = original("budget", "1");
    let jobs = fixture(&dir, "unknown", 2, 3, false);
    let run = resume(&dir, "unknown", "resumed.json", true);
    assert!(!run.status.success());
    let record: Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(record["execution"]["stop"]["reason"], "limit");
    assert_eq!(commands(&jobs.join("journal.jsonl"), "dispatch"), 1);
    assert_eq!(commands(&jobs.join("journal.jsonl"), "reserve"), 1);
    assert_eq!(commands(&jobs.join("journal.jsonl"), "reconcile"), 1);
    assert_ok(&cli(
        &dir,
        &[
            "replay",
            "resumed.json",
            "--source",
            "unknown/sources/main.nmlt",
        ],
    ));
}

#[test]
fn modified_sources_missing_evidence_and_incomplete_decisions_fail_closed() {
    let dir = original("rejections", "2");
    let modified = fixture(&dir, "modified", 2, 3, false);
    fs::write(
        modified.join("sources/main.nmlt"),
        SOURCE.replace("input)", "5)"),
    )
    .unwrap();
    let before = fs::read(modified.join("journal.jsonl")).unwrap();
    assert!(
        !resume(&dir, "modified", "modified.json", true)
            .status
            .success()
    );
    assert_eq!(fs::read(modified.join("journal.jsonl")).unwrap(), before);
    let changed_input = fixture(&dir, "changed-input", 4, 4, true);
    fs::write(
        changed_input.join("input-0.json"),
        b"changed retained dispatch",
    )
    .unwrap();
    let before = fs::read(changed_input.join("journal.jsonl")).unwrap();
    assert!(
        !resume(&dir, "changed-input", "changed-input.json", false)
            .status
            .success()
    );
    assert_eq!(
        fs::read(changed_input.join("journal.jsonl")).unwrap(),
        before
    );
    let missing = fixture(&dir, "missing", 3, 3, false);
    fs::remove_file(missing.join("evidence-2.json")).unwrap();
    assert!(
        !resume(&dir, "missing", "missing.json", true)
            .status
            .success()
    );
    let incomplete = fixture(&dir, "incomplete", 2, 3, false);
    let path = incomplete.join("source-journal.jsonl");
    let mut bytes = fs::read(&path).unwrap();
    bytes.pop();
    fs::write(path, bytes).unwrap();
    let output = resume(&dir, "incomplete", "incomplete.json", true);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("incomplete"));
    assert_eq!(commands(&incomplete.join("journal.jsonl"), "dispatch"), 1);
    assert_eq!(commands(&incomplete.join("journal.jsonl"), "reconcile"), 0);
}

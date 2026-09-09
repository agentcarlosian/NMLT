use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[test]
fn source_components_transfer_real_handles_and_replay_without_new_attempts() {
    let dir = directory("transfer");
    fs::write(
        dir.join("main.nmlt"),
        include_str!("../../../examples/pivot/transfer_jobs.nmlt"),
    )
    .unwrap();
    let out = run(&dir, "main", "2", "2", "200");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value = record(&out);
    assert_eq!(
        value["execution"]["stop"]["value"],
        json!({"kind":"ok","value":{"kind":"int","value":50}})
    );
    assert_eq!(
        value["snapshot"]["observations"].as_array().unwrap().len(),
        2
    );
    let journal = fs::read(dir.join("jobs/journal.jsonl")).unwrap();
    assert!(replay(&dir, "run.json").status.success());
    assert_eq!(fs::read(dir.join("jobs/journal.jsonl")).unwrap(), journal);
    let external = cli(
        &dir,
        &[
            "run",
            "main.nmlt",
            "--entry",
            "launch",
            "--arg",
            "input=3",
            "--max-steps",
            "30",
            "--emit-run",
            "forged.json",
            "--jobs-dir",
            "forged",
            "--max-jobs",
            "1",
            "--job-slots",
            "1",
            "--job-timeout-ms",
            "1000",
        ],
    );
    assert!(!external.status.success());
    assert!(!dir.join("forged").exists());
    assert!(!dir.join("forged.json").exists());
}

fn directory(name: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/r2-source-async-tests")
        .join(format!("{name}-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("main.nmlt"),
        include_str!("../../../examples/pivot/async_fallback.nmlt"),
    )
    .unwrap();
    dir.canonicalize().unwrap()
}
fn cli(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}
fn run(dir: &Path, entry: &str, slots: &str, attempts: &str, steps: &str) -> Output {
    let mut args = vec![
        "run",
        "main.nmlt",
        "--entry",
        entry,
        "--max-steps",
        steps,
        "--emit-run",
        "run.json",
        "--jobs-dir",
        "jobs",
        "--max-jobs",
        attempts,
        "--job-slots",
        slots,
        "--job-timeout-ms",
        "10000",
    ];
    if entry != "lean" {
        args.extend(["--arg", "input=-3"]);
    }
    if entry == "main" {
        args.extend(["--arg", "fallback=5"]);
    }
    cli(dir, &args)
}
fn record(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stderr)))
}
fn replay(dir: &Path, file: &str) -> Output {
    cli(dir, &["replay", file, "--source", "main.nmlt"])
}

#[test]
fn concurrent_dispatch_fallback_reuse_and_replay_without_live_state() {
    let dir = directory("concurrent");
    let out = run(&dir, "main", "2", "2", "100");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value = record(&out);
    assert_eq!(value["context"]["assurance"], "none");
    assert_eq!(
        value["execution"]["stop"]["value"],
        json!({"kind":"ok", "value":{"kind":"int", "value":50}})
    );
    assert_eq!(value["events"][0]["through"], 2);
    assert_eq!(value["events"][1]["through"], 4);
    assert_eq!(
        value["snapshot"]["observations"].as_array().unwrap().len(),
        2
    );
    let before = fs::read(dir.join("jobs/journal.jsonl")).unwrap();
    assert!(cli(&dir, &["jobs-recover", "jobs"]).status.success());
    assert_eq!(fs::read(dir.join("jobs/journal.jsonl")).unwrap(), before);
    fs::rename(dir.join("jobs"), dir.join("saved-jobs")).unwrap();
    assert!(replay(&dir, "run.json").status.success());
    assert!(!dir.join("jobs").exists());
    assert_eq!(
        fs::read(dir.join("saved-jobs/journal.jsonl")).unwrap(),
        before
    );
}

#[test]
fn cancellation_race_is_recorded_and_collected_once() {
    let dir = directory("cancel");
    let out = run(&dir, "cancel", "1", "2", "100");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value = record(&out);
    let cancelled = value["events"][1]["result"]["Ok"]["value"]["value"]
        .as_bool()
        .unwrap();
    assert_eq!(
        value["execution"]["stop"]["value"]["value"],
        if cancelled {
            "job cancelled"
        } else {
            "negative input"
        }
    );
    assert!(replay(&dir, "run.json").status.success());
}

#[test]
fn slot_exhaustion_and_early_stop_preserve_uncollected_work_for_recovery() {
    for (slots, steps, kind, charged) in [
        ("1", "100", "job_stopped", 1),
        ("2", "1", "step_limit", 0),
        ("2", "4", "step_limit", 1),
    ] {
        let dir = directory("limits");
        let out = run(&dir, "main", slots, "2", steps);
        assert!(!out.status.success());
        let value = record(&out);
        assert_eq!(value["execution"]["stop"]["kind"], kind);
        assert_eq!(
            value["snapshot"]["journal"]
                .as_str()
                .unwrap()
                .matches("\"kind\":\"dispatch\"")
                .count(),
            charged * 2
        );
        assert!(replay(&dir, "run.json").status.success());
        let recovered = cli(&dir, &["jobs-recover", "jobs"]);
        assert!(
            recovered.status.success(),
            "{}",
            String::from_utf8_lossy(&recovered.stderr)
        );
        let state = record(&recovered);
        assert_eq!(
            state["state"]["attempts"].as_array().unwrap().len(),
            charged
        );
        if charged > 0 {
            assert_eq!(state["state"]["attempts"][0]["phase"]["kind"], "uncertain");
        }
    }
}

#[test]
fn async_preflight_requires_all_bounds_and_lean_tool_before_creating_evidence() {
    for (entry, slots, attempts) in [
        ("main", "0", "2"),
        ("main", "3", "2"),
        ("main", "2", "17"),
        ("lean", "2", "2"),
    ] {
        let dir = directory("preflight");
        let out = run(&dir, entry, slots, attempts, "100");
        assert!(!out.status.success());
        assert!(!dir.join("jobs").exists());
        assert!(!dir.join("run.json").exists());
    }
    let dir = directory("missing");
    let out = cli(
        &dir,
        &[
            "run",
            "main.nmlt",
            "--entry",
            "cancel",
            "--arg",
            "input=3",
            "--max-steps",
            "100",
            "--emit-run",
            "run.json",
        ],
    );
    assert!(!out.status.success());
    assert!(!dir.join("run.json").exists());
}

#[test]
fn collected_slots_are_reused_by_synchronous_calls_and_iteration_local_jobs() {
    let dir = directory("reuse");
    fs::write(
        dir.join("main.nmlt"),
        r#"
fn main(input: Int, fallback: Int) -> Outcome<Int> {
    let first = job_start_square(input);
    let rejected = job_collect(first);
    let backup = job_square(fallback);
    let total = fold([1,2], 0, acc, item => {
        let h = job_start_square(item);
        match job_collect(h) { Ok(v) => acc + v, Err(e) => acc }
    });
    match backup { Ok(v) => Ok(v + total), Err(e) => Err(e) }
}
"#,
    )
    .unwrap();
    let out = run(&dir, "main", "1", "4", "200");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value = record(&out);
    assert_eq!(
        value["execution"]["stop"]["value"],
        json!({"kind":"ok", "value":{"kind":"int", "value":30}})
    );
    let observations = value["snapshot"]["observations"].as_array().unwrap();
    assert_eq!(observations.len(), 4);
    for (index, observation) in observations.iter().enumerate() {
        let attempt = &observation["dispatch"]["binding"]["attempt"];
        assert_eq!(attempt["slot"], 0);
        assert_eq!(attempt["generation"], index + 1);
    }
    assert!(replay(&dir, "run.json").status.success());
}

#[test]
fn source_operations_results_boundaries_and_snapshot_tampering_fail_replay() {
    let dir = directory("tamper");
    let out = run(&dir, "main", "2", "2", "100");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let original = record(&out);
    for (pointer, replacement) in [
        ("/context/assurance", json!("verified")),
        ("/context/inputs/input/value", json!(4)),
        ("/context/program_sha256", json!("0".repeat(64))),
        ("/events/0/call/request/input", json!(4)),
        ("/events/0/through", json!(4)),
        ("/events/0/result/Ok/job", json!(1)),
        ("/events/2/call/job", json!(1)),
        ("/events/2/at/source", json!(1)),
        (
            "/events/3/result/Ok/value",
            json!({"kind":"ok", "value":{"kind":"int", "value":99}}),
        ),
        ("/execution/steps", json!(1)),
        ("/snapshot/observations", json!([])),
    ] {
        let mut mutant = original.clone();
        *mutant
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("missing {pointer}")) = replacement;
        fs::write(
            dir.join("tampered.json"),
            serde_json::to_vec(&mutant).unwrap(),
        )
        .unwrap();
        assert!(
            !replay(&dir, "tampered.json").status.success(),
            "accepted {pointer}"
        );
    }
    for mode in 0..4 {
        let mut mutant = original.clone();
        let events = mutant["events"].as_array_mut().unwrap();
        match mode {
            0 => {
                events.remove(2);
            }
            1 => {
                events.swap(0, 1);
            }
            2 => {
                events.push(events[2].clone());
            }
            _ => {
                events[2]["result"]["Ok"]["value"]["value"] = json!(
                    !events[2]["result"]["Ok"]["value"]["value"]
                        .as_bool()
                        .unwrap()
                );
            }
        }
        fs::write(
            dir.join("tampered.json"),
            serde_json::to_vec(&mutant).unwrap(),
        )
        .unwrap();
        assert!(
            !replay(&dir, "tampered.json").status.success(),
            "accepted event mutation {mode}"
        );
    }
    fs::write(dir.join("main.nmlt"), "fn main() -> Int { 0 }").unwrap();
    assert!(!replay(&dir, "run.json").status.success());
}

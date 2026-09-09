use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn directory(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/r2-source-job-tests")
        .join(format!(
            "{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("main.nmlt"),
        include_str!("../../../examples/pivot/job_fallback.nmlt"),
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
fn run(dir: &Path, input: &str, fallback: &str, max_jobs: &str, steps: &str) -> Output {
    cli(
        dir,
        &[
            "run",
            "main.nmlt",
            "--entry",
            "main",
            "--arg",
            input,
            "--arg",
            fallback,
            "--max-steps",
            steps,
            "--emit-run",
            "run.json",
            "--jobs-dir",
            "jobs",
            "--max-jobs",
            max_jobs,
            "--job-timeout-ms",
            "10000",
        ],
    )
}
fn record(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&output.stderr)))
}
fn replay(dir: &Path, path: &str) -> Output {
    cli(dir, &["replay", path, "--source", "main.nmlt"])
}

#[test]
fn subprocess_outcomes_fallback_collection_and_value_reuse_replay() {
    for (input, fallback, jobs, expected) in [
        (
            "input=4",
            "fallback=5",
            1,
            json!({"kind":"ok","value":{"kind":"int","value":32}}),
        ),
        (
            "input=-3",
            "fallback=5",
            2,
            json!({"kind":"ok","value":{"kind":"int","value":50}}),
        ),
        (
            "input=-3",
            "fallback=-5",
            2,
            json!({"kind":"err","value":"negative input"}),
        ),
        (
            "input=9223372036854775807",
            "fallback=3",
            2,
            json!({"kind":"ok","value":{"kind":"int","value":18}}),
        ),
    ] {
        let dir = directory("outcomes");
        let output = run(&dir, input, fallback, "2", "100");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value = record(&output);
        assert_eq!(value["assurance"], "none");
        assert_eq!(value["execution"]["stop"]["value"], expected);
        assert_eq!(value["events"].as_array().unwrap().len(), jobs);
        assert_eq!(value["accounting"]["charged_work"], jobs);
        assert_eq!(value["accounting"]["reserved_work"], 0);
        assert_eq!(
            fs::read_to_string(dir.join("jobs/journal.jsonl")).unwrap(),
            value["journal"]
        );
        // Replay is self-contained evidence: removing its live journal cannot
        // cause a new run or require recovery authority.
        fs::rename(dir.join("jobs"), dir.join("saved-jobs")).unwrap();
        assert!(replay(&dir, "run.json").status.success());
        assert!(!dir.join("jobs").exists());
    }
}

#[test]
fn job_and_step_limits_stop_before_new_dispatch() {
    for (max_jobs, steps, expected, jobs) in
        [("1", "100", "job_stopped", 1), ("2", "1", "step_limit", 0)]
    {
        let dir = directory("limits");
        let output = run(&dir, "input=-3", "fallback=5", max_jobs, steps);
        assert!(!output.status.success());
        let value = record(&output);
        assert_eq!(value["execution"]["stop"]["kind"], expected);
        assert_eq!(value["events"].as_array().unwrap().len(), jobs);
        assert_eq!(value["accounting"]["dispatched_attempts"], jobs);
        if jobs == 1 {
            assert_eq!(value["execution"]["stop"]["reason"], "limit");
        }
        assert!(replay(&dir, "run.json").status.success());
    }
}

#[test]
fn pure_route_and_invalid_preflight_never_allocate_a_job_directory() {
    let dir = directory("preflight");
    let output = cli(
        &dir,
        &[
            "run",
            "main.nmlt",
            "--entry",
            "main",
            "--arg",
            "input=3",
            "--arg",
            "fallback=5",
            "--max-steps",
            "100",
            "--emit-run",
            "run.json",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires local jobs"));
    assert!(!dir.join("run.json").exists());
    for (input, jobs, steps) in [
        ("input=true", "2", "100"),
        ("input=3", "0", "100"),
        ("input=3", "17", "100"),
        ("input=3", "2", "0"),
    ] {
        assert!(!run(&dir, input, "fallback=5", jobs, steps).status.success());
        assert!(!dir.join("jobs").exists());
        assert!(!dir.join("run.json").exists());
    }
    fs::write(dir.join("run.json"), "keep").unwrap();
    assert!(
        !run(&dir, "input=3", "fallback=5", "2", "100")
            .status
            .success()
    );
    assert!(!dir.join("jobs").exists());
    assert_eq!(fs::read_to_string(dir.join("run.json")).unwrap(), "keep");
}

#[test]
fn transcript_context_and_journal_tampering_fail() {
    let dir = directory("tamper");
    let output = run(&dir, "input=-3", "fallback=5", "2", "100");
    assert!(output.status.success());
    let original = record(&output);
    for (pointer, replacement) in [
        ("/assurance", json!("verified")),
        (
            "/manifest/context/implementation_sha256",
            json!("0".repeat(64)),
        ),
        ("/manifest/context/inputs/input/value", json!(9)),
        ("/manifest/context/max_jobs", json!(3)),
        ("/manifest/context/program_sha256", json!("0".repeat(64))),
        ("/events/0/at/source", json!(1)),
        ("/events/0/input", json!(3)),
        (
            "/events/1/result/value/binding/attempt/generation",
            json!(1),
        ),
        ("/events/1/result/value/outcome/value/value", json!(99)),
        ("/accounting/charged_work", json!(0)),
        ("/execution/steps", json!(0)),
        (
            "/journal",
            json!(original["journal"].as_str().unwrap().trim_end()),
        ),
    ] {
        let mut altered = original.clone();
        *altered
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("missing pointer {pointer}")) = replacement;
        fs::write(dir.join("bad.json"), serde_json::to_vec(&altered).unwrap()).unwrap();
        assert!(!replay(&dir, "bad.json").status.success(), "{pointer}");
    }
    for duplicate in [true, false] {
        let mut altered = original.clone();
        if duplicate {
            altered["events"]
                .as_array_mut()
                .unwrap()
                .push(original["events"][0].clone());
        } else {
            altered["events"].as_array_mut().unwrap().remove(0);
        }
        fs::write(dir.join("bad.json"), serde_json::to_vec(&altered).unwrap()).unwrap();
        assert!(!replay(&dir, "bad.json").status.success());
    }
    let bytes = serde_json::to_string(&original).unwrap().replacen(
        "\"assurance\":\"none\"",
        "\"assurance\":\"none\",\"assurance\":\"none\"",
        1,
    );
    fs::write(dir.join("bad.json"), bytes).unwrap();
    assert!(!replay(&dir, "bad.json").status.success());
    let mut altered = original;
    altered["events"][0]["result"]["value"]["extra"] = json!(true);
    fs::write(dir.join("bad.json"), serde_json::to_vec(&altered).unwrap()).unwrap();
    assert!(!replay(&dir, "bad.json").status.success());
}

#[test]
fn recovery_classifies_an_unfinished_prefix_without_redispatch() {
    let dir = directory("recover");
    assert!(
        run(&dir, "input=4", "fallback=5", "2", "100")
            .status
            .success()
    );
    let path = dir.join("jobs/journal.jsonl");
    let full = fs::read_to_string(&path).unwrap();
    // A complete durable prefix ending at dispatch, as left by an interruption.
    let prefix = format!("{}\n", full.lines().take(3).collect::<Vec<_>>().join("\n"));
    fs::write(&path, &prefix).unwrap();
    let output = cli(&dir, &["jobs-recover", "jobs"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = record(&output);
    assert_eq!(value["state"]["attempts"][0]["phase"]["kind"], "uncertain");
    let recovered = fs::read_to_string(&path).unwrap();
    assert!(recovered.starts_with(&prefix));
    assert_eq!(recovered.lines().count(), 4);
    assert!(recovered.lines().last().unwrap().contains("recover"));
    assert!(replay(&dir, "run.json").status.success());
    fs::write(&path, recovered.trim_end()).unwrap();
    assert!(!cli(&dir, &["jobs-recover", "jobs"]).status.success());
}

#[test]
fn imported_worker_effects_bind_every_loaded_source() {
    let dir = directory("imports");
    fs::write(
        dir.join("main.nmlt"),
        "import Worker\nfn main(input: Int, fallback: Int) -> Outcome<Int> { Worker.run(input) }",
    )
    .unwrap();
    fs::write(
        dir.join("Worker.nmlt"),
        "fn run(input: Int) -> Outcome<Int> { job_square(input) }",
    )
    .unwrap();
    let output = run(&dir, "input=4", "fallback=5", "2", "100");
    assert!(output.status.success());
    assert_eq!(record(&output)["events"][0]["at"]["source"], 1);
    let checked = record(&cli(
        &dir,
        &["typecheck", "main.nmlt", "--profile", "workflow"],
    ));
    assert!(
        checked["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["requires_jobs"] == true)
    );
    assert!(replay(&dir, "run.json").status.success());
    fs::write(
        dir.join("Worker.nmlt"),
        "fn run(input: Int) -> Outcome<Int> { job_square(input + 1) }",
    )
    .unwrap();
    assert!(!replay(&dir, "run.json").status.success());
}

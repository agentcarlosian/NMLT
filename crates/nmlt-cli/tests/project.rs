use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn directory(name: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/r2-project-tests");
    fs::create_dir_all(&root).unwrap();
    root.canonicalize()
        .unwrap()
        .join(format!("{name}-{}-{stamp}", std::process::id()))
}
fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .args(args)
        .output()
        .unwrap()
}
fn ok(args: &[&str]) -> Value {
    let out = cli(args);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn convenience_worker_calls_use_the_resumable_project_host() {
    let root = directory("convenience-resume");
    ok(&["init", path(&root)]);
    fs::write(root.join("main.nmlt"), "fn main(input: Int, fallback: Int) -> Outcome<Int> { match job_square(input) { Ok(value) => Ok(value + value), Err(message) => match job_square(fallback) { Ok(value) => Ok(value + value), Err(problem) => Err(problem) } } }").unwrap();
    let completed = ok(&["run", path(&root)]);
    assert_eq!(
        completed["value"],
        json!({"kind":"ok","value":{"kind":"int","value":50}})
    );
    let record = PathBuf::from(completed["record"].as_str().unwrap());
    let run = record.parent().unwrap();
    let source: Value =
        serde_json::from_slice(&fs::read(run.join("source.json")).unwrap()).unwrap();
    assert_eq!(source["schema"], "nmlt-async-source-run-v2");
    let before = fs::read(run.join("jobs/journal.jsonl")).unwrap();
    assert_eq!(
        ok(&["resume", path(run), "--project", path(&root)])["value"],
        completed["value"]
    );
    assert_eq!(fs::read(run.join("jobs/journal.jsonl")).unwrap(), before);
}

#[test]
fn project_resume_uses_saved_invocation_and_preserves_the_original_job_budget() {
    let root = directory("resume");
    ok(&["init", path(&root)]);
    let completed = ok(&["run", path(&root)]);
    let original_record = PathBuf::from(completed["record"].as_str().unwrap());
    let run = original_record.parent().unwrap().to_owned();
    let jobs = run.join("jobs");
    // A coherent interrupted prefix: one dispatched attempt has returned its
    // source handle, while its completion and later source work are unknown.
    for (name, entries) in [("journal.jsonl", 2), ("source-journal.jsonl", 2)] {
        let bytes = fs::read(jobs.join(name)).unwrap();
        let prefix: Vec<u8> = bytes
            .split_inclusive(|b| *b == b'\n')
            .take(entries + 1)
            .flatten()
            .copied()
            .collect();
        fs::write(jobs.join(name), prefix).unwrap();
    }
    for name in [
        "evidence-4.json",
        "evidence-6.json",
        "dispatch-1.json",
        "input-1.json",
    ] {
        fs::remove_file(jobs.join(name)).unwrap();
    }
    fs::write(run.join("source.json"), b"").unwrap();
    fs::remove_file(&original_record).unwrap();
    fs::write(root.join("main.nmlt"), "unfinished source edit").unwrap();
    fs::write(root.join("nmlt.toml"), "unfinished manifest edit").unwrap();
    let blocked = cli(&["resume", path(&run), "--project", path(&root)]);
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("unresolved external effects"));
    let resumed = ok(&[
        "resume",
        path(&run),
        "--project",
        path(&root),
        "--acknowledge-uncertain-effects",
        "local operator reconciled the previous attempt",
    ]);
    assert_eq!(
        resumed["value"],
        json!({"kind":"ok","value":{"kind":"int","value":50}})
    );
    assert!(fs::read(run.join("source.json")).unwrap().is_empty());
    let record = PathBuf::from(resumed["record"].as_str().unwrap());
    assert_ne!(record.parent(), Some(run.as_path()));
    assert_eq!(
        ok(&["replay", path(&record), "--project", path(&root)])["matched"],
        true
    );
    let before = fs::read(jobs.join("journal.jsonl")).unwrap();
    let dispatched = String::from_utf8(before.clone())
        .unwrap()
        .lines()
        .skip(1)
        .filter(|line| {
            serde_json::from_str::<Value>(line).unwrap()["command"]["kind"] == "dispatch"
        })
        .count();
    assert_eq!(dispatched, 2);
    let again = ok(&[
        "resume",
        path(record.parent().unwrap()),
        "--project",
        path(&root),
    ]);
    assert_eq!(again["value"], resumed["value"]);
    assert_eq!(fs::read(jobs.join("journal.jsonl")).unwrap(), before);
    let invocation_path = run.join("invocation.json");
    let mut invocation: Value =
        serde_json::from_slice(&fs::read(&invocation_path).unwrap()).unwrap();
    invocation["inputs"]["fallback"] = json!(99);
    fs::write(
        invocation_path,
        serde_json::to_vec_pretty(&invocation).unwrap(),
    )
    .unwrap();
    assert!(
        !cli(&["resume", path(&run), "--project", path(&root)])
            .status
            .success()
    );
    assert_eq!(fs::read(jobs.join("journal.jsonl")).unwrap(), before);
}

#[test]
fn initialized_project_changes_input_handles_failure_reuses_tests_and_replays() {
    let root = directory("acceptance");
    ok(&["init", path(&root)]);
    ok(&["check-project", path(&root)]);
    let fallback = ok(&["run", path(&root)]);
    assert_eq!(
        fallback["value"],
        json!({"kind":"ok","value":{"kind":"int","value":50}})
    );
    let changed = ok(&["run", path(&root), "--arg", "input=4"]);
    assert_eq!(
        changed["value"],
        json!({"kind":"ok","value":{"kind":"int","value":32}})
    );
    let record = PathBuf::from(fallback["record"].as_str().unwrap());
    let jobs = record.parent().unwrap().join("jobs");
    fs::rename(&jobs, record.parent().unwrap().join("saved-jobs")).unwrap();
    assert_eq!(
        ok(&["replay", path(&record), "--project", path(&root)])["matched"],
        true
    );
    assert!(!jobs.exists());
    let main = fs::read(root.join("main.nmlt")).unwrap();
    fs::write(root.join("main.nmlt"), "an unfinished edit").unwrap();
    assert_eq!(
        ok(&["replay", path(&record), "--project", path(&root)])["matched"],
        true
    );
    fs::write(root.join("main.nmlt"), main).unwrap();
    assert_eq!(ok(&["test", path(&root)])["passed"], true);
    let original = fs::read(root.join("main.nmlt")).unwrap();
    assert!(!cli(&["init", path(&root)]).status.success());
    assert_eq!(fs::read(root.join("main.nmlt")).unwrap(), original);
}

#[test]
fn project_tests_assert_results_and_preflight_invalid_expectations() {
    let root = directory("assertions");
    ok(&["init", path(&root)]);
    let manifest = root.join("nmlt.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(&manifest, text.replace("Ok = 50", "Ok = 51")).unwrap();
    let out = cli(&["test", path(&root)]);
    assert!(!out.status.success());
    let result: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["tests"][0]["passed"], false);
    assert_eq!(result["tests"][1]["passed"], true);
    fs::write(&manifest, text.replace("Ok = 50", "Ok = \"bad\"")).unwrap();
    let before = fs::read_dir(root.join(".nmlt")).unwrap().count();
    let out = cli(&["--json", "test", path(&root)]);
    assert!(!out.status.success());
    let error: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(error["code"], "NMLT-TEST-EXPECT");
    assert_eq!(fs::read_dir(root.join(".nmlt")).unwrap().count(), before);
}

#[test]
fn imported_dependencies_require_explicit_lock_and_main_remains_editable() {
    let root = directory("dependencies");
    ok(&["init", path(&root)]);
    fs::write(root.join("main.nmlt"), "import helper\nfn main(input: Int, fallback: Int) -> Outcome<Int> { Ok(helper.square(input)) }\n").unwrap();
    fs::write(
        root.join("helper.nmlt"),
        "fn square(x: Int) -> Int { x * x }\n",
    )
    .unwrap();
    assert!(!cli(&["check-project", path(&root)]).status.success());
    ok(&["lock", path(&root)]);
    ok(&["check-project", path(&root)]);
    fs::write(
        root.join("helper.nmlt"),
        "fn square(x: Int) -> Int { x + x }\n",
    )
    .unwrap();
    assert!(!cli(&["run", path(&root)]).status.success());
    ok(&["lock", path(&root)]);
    fs::write(root.join("main.nmlt"), "import helper\nfn main(input: Int, fallback: Int) -> Outcome<Int> { Ok(helper.square(fallback)) }\n").unwrap();
    assert_eq!(ok(&["run", path(&root)])["value"]["value"]["value"], 10);
    let locked = fs::read_to_string(root.join("nmlt.lock")).unwrap();
    let mut lock: Value = serde_json::from_str(&locked).unwrap();
    lock["implementation_sha256"] = json!("0".repeat(64));
    fs::write(root.join("nmlt.lock"), serde_json::to_vec(&lock).unwrap()).unwrap();
    assert!(!cli(&["run", path(&root)]).status.success());
}

#[test]
fn project_replay_rejects_changed_outer_invocation_and_inner_result() {
    let root = directory("mutations");
    ok(&["init", path(&root)]);
    let run = ok(&["run", path(&root)]);
    let record = PathBuf::from(run["record"].as_str().unwrap());
    let original = fs::read(&record).unwrap();
    let mut value: Value = serde_json::from_slice(&original).unwrap();
    value["manifest"] = json!(
        value["manifest"]
            .as_str()
            .unwrap()
            .replace("fallback and reuse", "renamed test")
    );
    value["manifest_sha256"] = json!(nmlt_runtime::sha256(
        value["manifest"].as_str().unwrap().as_bytes()
    ));
    fs::write(&record, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        !cli(&["replay", path(&record), "--project", path(&root)])
            .status
            .success()
    );
    let mut value: Value = serde_json::from_slice(&original).unwrap();
    value["inputs"]["input"] = json!(4);
    fs::write(&record, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        !cli(&["replay", path(&record), "--project", path(&root)])
            .status
            .success()
    );
    fs::write(&record, original).unwrap();
    let source = record.parent().unwrap().join("source.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
    value["execution"]["stop"]["value"]["value"]["value"] = json!(99);
    fs::write(&source, serde_json::to_vec(&value).unwrap()).unwrap();
    let mut outer: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    outer["execution_sha256"] = json!(nmlt_runtime::sha256(&fs::read(&source).unwrap()));
    fs::write(&record, serde_json::to_vec(&outer).unwrap()).unwrap();
    assert!(
        !cli(&["replay", path(&record), "--project", path(&root)])
            .status
            .success()
    );
}

#[test]
fn formatter_preflights_the_import_closure_and_json_errors_keep_source_locations() {
    let root = directory("format");
    ok(&["init", path(&root)]);
    fs::write(
        root.join("main.nmlt"),
        "import helper\nfn main(input:Int,fallback:Int)->Outcome<Int>{Ok(helper.square(input))}",
    )
    .unwrap();
    let invalid = "fn square(x: Int) -> Int {\n  true\n}\n";
    fs::write(root.join("helper.nmlt"), invalid).unwrap();
    let before = fs::read(root.join("main.nmlt")).unwrap();
    let out = cli(&["--json", "fmt", path(&root)]);
    assert!(!out.status.success());
    let error: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert!(
        error["location"]["path"]
            .as_str()
            .unwrap()
            .ends_with("helper.nmlt")
    );
    assert_eq!(error["location"]["line"], 2);
    assert_eq!(error["expected"]["kind"], "int");
    assert_eq!(error["actual"]["kind"], "bool");
    assert_eq!(error["related"][0]["location"]["line"], 1);
    assert_eq!(fs::read(root.join("main.nmlt")).unwrap(), before);
    fs::write(
        root.join("helper.nmlt"),
        "// exact λ\nfn square(x:Int)->Int{x*x}",
    )
    .unwrap();
    assert!(!cli(&["fmt", path(&root), "--check"]).status.success());
    assert_eq!(fs::read(root.join("main.nmlt")).unwrap(), before);
    ok(&["fmt", path(&root)]);
    assert_eq!(ok(&["fmt", path(&root), "--check"])["changed"], json!([]));
    assert!(
        fs::read_to_string(root.join("helper.nmlt"))
            .unwrap()
            .contains("// exact λ")
    );
    ok(&["lock", path(&root)]);
    ok(&["check-project", path(&root)]);
}

use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const SOURCE: &str = include_str!("../../../examples/pivot/pure_fallback.nmlt");
fn directory(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/r2-workflow-tests");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = root.join(format!("{name}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("main.nmlt"), SOURCE).unwrap();
    dir.canonicalize().unwrap()
}
fn cli(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}
fn run(dir: &Path, input: &str, fallback: &str, limit: &str, output: &str) -> Output {
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
            limit,
            "--emit-run",
            output,
        ],
    )
}
fn replay(dir: &Path, path: &str, source: &str) -> Output {
    cli(dir, &["replay", path, "--source", source])
}
fn json_output(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn structured_batch_inputs_and_nominal_results_round_trip() {
    let dir = directory("batch");
    fs::write(
        dir.join("main.nmlt"),
        include_str!("../../../examples/pivot/batch_summary.nmlt"),
    )
    .unwrap();
    let checked = cli(&dir, &["typecheck", "main.nmlt", "--profile", "workflow"]);
    assert!(checked.status.success());
    assert!(json_output(&checked)["records"]["Work.Item"].is_array());
    let output = cli(
        &dir,
        &[
            "run",
            "main.nmlt",
            "--entry",
            "main",
            "--arg",
            r#"items=[{"input":-3,"fallback":5},{"input":4,"fallback":9},{"input":-1,"fallback":-2}]"#,
            "--max-steps",
            "1000",
            "--emit-run",
            "batch.json",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let original = json_output(&output);
    assert_eq!(
        original["execution"]["stop"]["value"]["value"]["fields"],
        json!({
            "accepted": {"kind":"int","value":2}, "rejected": {"kind":"int","value":1}, "total": {"kind":"int","value":41}
        })
    );
    assert!(replay(&dir, "batch.json", "main.nmlt").status.success());
    for (pointer, replacement) in [
        ("/inputs/items/value/0/value/name", json!("Other.Item")),
        ("/inputs/items/value/0/value/fields/extra", json!(0)),
        ("/execution/stop/value/value/name", json!("OtherSummary")),
        ("/execution/stop/value/value/fields/total/value", json!(42)),
        ("/schema", json!("nmlt-pure-run-v1")),
        ("/profile", json!("pure-workflow-v1")),
    ] {
        let mut changed = original.clone();
        if pointer.ends_with("/extra") {
            changed["inputs"]["items"]["value"][0]["value"]["fields"]["extra"] = replacement;
        } else {
            *changed.pointer_mut(pointer).unwrap() = replacement;
        }
        fs::write(
            dir.join("changed.json"),
            serde_json::to_vec(&changed).unwrap(),
        )
        .unwrap();
        assert!(
            !replay(&dir, "changed.json", "main.nmlt").status.success(),
            "{pointer}"
        );
    }
    // Nested duplicate keys cannot be hidden inside an otherwise valid input record.
    let duplicate = String::from_utf8(output.stdout).unwrap().replacen(
        "\"name\": \"Work.Item\"",
        "\"name\": \"Other\", \"name\": \"Work.Item\"",
        1,
    );
    fs::write(dir.join("duplicate.json"), duplicate).unwrap();
    assert!(!replay(&dir, "duplicate.json", "main.nmlt").status.success());
}

#[test]
fn invalid_structured_cli_inputs_create_no_record() {
    let dir = directory("bad-batch");
    fs::write(
        dir.join("main.nmlt"),
        include_str!("../../../examples/pivot/batch_summary.nmlt"),
    )
    .unwrap();
    for input in [
        r#"items=[{"input":1}]"#,
        r#"items=[{"input":1,"fallback":2,"extra":3}]"#,
        r#"items=[{"input":true,"fallback":2}]"#,
        r#"items=[{"input":1,"input":2,"fallback":3}]"#,
        "items=null",
        "items={}",
    ] {
        let output = cli(
            &dir,
            &[
                "run",
                "main.nmlt",
                "--entry",
                "main",
                "--arg",
                input,
                "--max-steps",
                "1000",
                "--emit-run",
                "absent.json",
            ],
        );
        assert!(!output.status.success(), "{input}");
        assert!(!dir.join("absent.json").exists());
    }
}

#[test]
fn aggregate_runtime_stops_are_recorded_and_replay_as_incomplete() {
    let dir = directory("value-limits");
    let input = format!("input={}", json!("x".repeat(4096)));
    for (source, kind) in [
        (
            format!(
                "fn main(input: Text, fallback: Int) -> List<Text> {{ [{}] }}",
                vec!["input"; 17].join(",")
            ),
            "value_limit",
        ),
        (
            format!(
                "fn main(input: Text, fallback: Int) -> Text {{ fold([{}], input, a, x => a) }}",
                vec!["0"; 256].join(",")
            ),
            "value_work_limit",
        ),
    ] {
        fs::write(dir.join("main.nmlt"), source).unwrap();
        let name = format!("{kind}.json");
        let output = run(&dir, &input, "fallback=0", "100000", &name);
        assert!(!output.status.success());
        assert_eq!(json_output(&output)["execution"]["stop"]["kind"], kind);
        let replayed = replay(&dir, &name, "main.nmlt");
        assert!(
            replayed.status.success(),
            "{}",
            String::from_utf8_lossy(&replayed.stderr)
        );
        assert_eq!(json_output(&replayed)["execution"]["stop"]["kind"], kind);
    }
}

#[test]
fn source_entry_runs_handles_failure_reuses_output_and_replays_after_move() {
    let dir = directory("roundtrip");
    let checked = cli(&dir, &["typecheck", "main.nmlt", "--profile", "workflow"]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert_eq!(
        json_output(&checked)["entries"].as_array().unwrap().len(),
        2
    );
    let output = run(&dir, "input=-3", "fallback=5", "1000", "run.json");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(dir.join("run.json")).unwrap(), output.stdout);
    let value = json_output(&output);
    assert_eq!(value["schema"], "nmlt-pure-run-v3");
    assert_eq!(value["assurance"], "none");
    assert_eq!(value["execution"]["steps"], 26);
    assert_eq!(
        value["execution"]["stop"]["value"],
        json!({"kind":"ok", "value":{"kind":"int", "value":50}})
    );
    assert_eq!(
        run(&dir, "input=-3", "fallback=5", "1000", "repeat.json").stdout,
        output.stdout
    );
    fs::rename(dir.join("main.nmlt"), dir.join("moved.nmlt")).unwrap();
    let output = replay(&dir, "run.json", "moved.nmlt");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(json_output(&output)["matched"], true);
}

#[test]
fn changed_inputs_domain_errors_and_incomplete_evaluation_keep_distinct_results() {
    let dir = directory("outcomes");
    let direct = run(&dir, "input=4", "fallback=5", "1000", "direct.json");
    assert!(direct.status.success());
    assert_eq!(
        json_output(&direct)["execution"]["stop"]["value"]["value"]["value"],
        32
    );
    let failed = run(&dir, "input=-3", "fallback=-5", "1000", "failed.json");
    assert!(failed.status.success()); // Returned Err is a domain value, not a successful job.
    assert_eq!(
        json_output(&failed)["execution"]["stop"]["value"],
        json!({"kind":"err","value":"negative input"})
    );
    let limited = run(&dir, "input=4", "fallback=5", "1", "limited.json");
    assert!(!limited.status.success());
    assert_eq!(
        json_output(&limited)["execution"]["stop"]["kind"],
        "step_limit"
    );
    let output = replay(&dir, "limited.json", "main.nmlt");
    assert!(output.status.success());
    assert_eq!(
        json_output(&output)["execution"]["stop"]["kind"],
        "step_limit"
    );
    let overflow = run(
        &dir,
        "input=9223372036854775807",
        "fallback=5",
        "1000",
        "overflow.json",
    );
    assert!(!overflow.status.success());
    assert_eq!(
        json_output(&overflow)["execution"]["stop"]["kind"],
        "integer_overflow"
    );
    assert!(replay(&dir, "overflow.json", "main.nmlt").status.success());
}

#[test]
fn identity_result_and_unknown_field_changes_are_rejected() {
    let dir = directory("mutations");
    let original = json_output(&run(
        &dir,
        "input=-3",
        "fallback=5",
        "1000",
        "original.json",
    ));
    let changes = [
        ("/schema", json!("nmlt-pure-run-v99")),
        ("/profile", json!("future")),
        ("/assurance", json!("proved")),
        ("/implementation_sha256", json!("00")),
        ("/source_sha256", json!("00")),
        ("/program_sha256", json!("00")),
        ("/entry", json!("Math.square_nonnegative")),
        ("/inputs/input/value", json!(4)),
        ("/max_steps", json!(1)),
        ("/execution/steps", json!(0)),
        ("/execution/stop/kind", json!("step_limit")),
        ("/execution/stop/value/value/value", json!(51)),
    ];
    for (i, (pointer, replacement)) in changes.into_iter().enumerate() {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        let name = format!("changed-{i}.json");
        fs::write(dir.join(&name), serde_json::to_vec(&changed).unwrap()).unwrap();
        assert!(
            !replay(&dir, &name, "main.nmlt").status.success(),
            "accepted {pointer}"
        );
    }
    for (i, pointer) in [
        "",
        "/execution",
        "/execution/stop",
        "/execution/stop/value",
        "/execution/stop/value/value",
        "/inputs/input",
    ]
    .into_iter()
    .enumerate()
    {
        let mut changed = original.clone();
        changed
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(true));
        let name = format!("extra-{i}.json");
        fs::write(dir.join(&name), serde_json::to_vec(&changed).unwrap()).unwrap();
        assert!(
            !replay(&dir, &name, "main.nmlt").status.success(),
            "accepted unknown field at {pointer}"
        );
    }
    fs::write(
        dir.join("main.nmlt"),
        SOURCE.replace("value + value", "value + 1"),
    )
    .unwrap();
    assert!(!replay(&dir, "original.json", "main.nmlt").status.success());
}

#[test]
fn bad_source_options_inputs_and_overwrites_leave_no_new_record() {
    let dir = directory("rejection");
    for (input, fallback, limit) in [
        ("input=true", "fallback=5", "100"),
        ("x=1", "fallback=5", "100"),
        ("input=1", "input=2", "100"),
        ("input=1", "fallback=5", "0"),
        ("input=1", "fallback=5", "100001"),
        ("input={}", "fallback=5", "100"),
    ] {
        assert!(
            !run(&dir, input, fallback, limit, "absent.json")
                .status
                .success()
        );
        assert!(!dir.join("absent.json").exists());
    }
    assert!(
        !cli(
            &dir,
            &[
                "run",
                "main.nmlt",
                "--entry",
                "main",
                "--behavior",
                "Main",
                "--max-steps",
                "100",
                "--emit-run",
                "absent.json"
            ]
        )
        .status
        .success()
    );
    assert!(
        !run(&dir, "input=1", "fallback=5", "100", "main.nmlt")
            .status
            .success()
    );
    assert_eq!(fs::read_to_string(dir.join("main.nmlt")).unwrap(), SOURCE);
    assert!(
        run(&dir, "input=1", "fallback=5", "100", "existing.json")
            .status
            .success()
    );
    let before = fs::read(dir.join("existing.json")).unwrap();
    assert!(
        !run(&dir, "input=2", "fallback=5", "100", "existing.json")
            .status
            .success()
    );
    assert_eq!(fs::read(dir.join("existing.json")).unwrap(), before);
    fs::write(dir.join("main.nmlt"), "fn main() -> Int { false }\n").unwrap();
    let bad = run(&dir, "input=1", "fallback=5", "100", "absent.json");
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("at 1:"));
    assert!(!dir.join("absent.json").exists());
}

#[test]
fn explicit_source_route_never_promotes_workflows_to_behavior_evidence() {
    let dir = directory("routing");
    for args in [
        vec!["check", "main.nmlt"],
        vec!["typecheck", "main.nmlt"],
        vec!["elaborate", "main.nmlt", "--emit-core", "absent.json"],
        vec![
            "run",
            "main.nmlt",
            "--behavior",
            "Main",
            "--max-steps",
            "100",
            "--emit-run",
            "absent.json",
        ],
    ] {
        assert!(!cli(&dir, &args).status.success());
        assert!(!dir.join("absent.json").exists());
    }
    let model = "system Hidden { state x: Bool = false }\n";
    fs::write(dir.join("main.nmlt"), model).unwrap();
    assert!(
        !cli(&dir, &["typecheck", "main.nmlt", "--profile", "workflow"])
            .status
            .success()
    );
}

#[test]
fn replay_accepts_formatting_but_rejects_duplicate_keys_and_oversized_records() {
    let dir = directory("encoding");
    let output = run(&dir, "input=-3", "fallback=5", "1000", "original.json");
    let encoded = serde_json::to_string(&json_output(&output)).unwrap();
    fs::write(dir.join("compact.json"), &encoded).unwrap();
    assert!(replay(&dir, "compact.json", "main.nmlt").status.success());
    for (i, (before, after)) in [
        (
            "\"inputs\":{",
            "\"inputs\":{\"input\":{\"kind\":\"int\",\"value\":-3},",
        ),
        ("\"steps\":26", "\"steps\":26,\"steps\":26"),
        ("\"value\":-3", "\"value\":-3,\"value\":-3"),
    ]
    .into_iter()
    .enumerate()
    {
        let mutated = encoded.replacen(before, after, 1);
        assert_ne!(mutated, encoded);
        let path = format!("duplicate-{i}.json");
        fs::write(dir.join(&path), mutated).unwrap();
        let output = replay(&dir, &path, "main.nmlt");
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate"));
    }
    let large = format!(
        "{{\"schema\":\"nmlt-pure-run-v3\",\"padding\":\"{}\"}}",
        "x".repeat(4 * 1024 * 1024)
    );
    fs::write(dir.join("large.json"), large).unwrap();
    assert!(!replay(&dir, "large.json", "main.nmlt").status.success());
}

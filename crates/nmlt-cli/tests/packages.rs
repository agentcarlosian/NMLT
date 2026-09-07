use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

fn directory() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/r2-package-tests/{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    for (name, source) in [
        (
            "main.nmlt",
            include_str!("../../../examples/pivot/package_batch/main.nmlt"),
        ),
        (
            "Arithmetic.nmlt",
            include_str!("../../../examples/pivot/package_batch/Arithmetic.nmlt"),
        ),
        (
            "Work.nmlt",
            include_str!("../../../examples/pivot/package_batch/Work.nmlt"),
        ),
        (
            "Reports.nmlt",
            include_str!("../../../examples/pivot/package_batch/Reports.nmlt"),
        ),
    ] {
        fs::write(path.join(name), source).unwrap();
    }
    path.canonicalize().unwrap()
}
fn cli(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nmlt"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}
fn run(dir: &Path, output: &str) -> Output {
    cli(
        dir,
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
            output,
        ],
    )
}
fn decoded(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)))
}
fn replay(dir: &Path, record: &str, source: &str) -> Output {
    cli(dir, &["replay", record, "--source", source])
}

#[test]
fn package_run_and_replay_survive_directory_move_and_entry_rename() {
    let dir = directory();
    let checked = cli(&dir, &["typecheck", "main.nmlt", "--profile", "workflow"]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert_eq!(decoded(&checked)["sources"].as_array().unwrap().len(), 4);
    let result = run(&dir, "run.json");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let record = decoded(&result);
    assert_eq!(record["schema"], "nmlt-pure-run-v3");
    assert_eq!(
        record["execution"]["stop"]["value"]["value"]["name"],
        "Reports.Summary"
    );
    assert_eq!(
        record["execution"]["stop"]["value"]["value"]["fields"]["total"]["value"],
        41
    );
    assert_eq!(fs::read(dir.join("run.json")).unwrap(), result.stdout);
    assert_eq!(run(&dir, "repeat.json").stdout, result.stdout);
    let moved = dir.with_extension("moved");
    fs::rename(&dir, &moved).unwrap();
    fs::rename(moved.join("main.nmlt"), moved.join("renamed.nmlt")).unwrap();
    let result = replay(&moved, "run.json", "renamed.nmlt");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(decoded(&result)["matched"], true);
    assert_eq!(decoded(&result)["sources"], record["sources"]);
}

#[test]
fn dependency_edits_missing_files_and_unused_errors_invalidate_replay() {
    let dir = directory();
    assert!(run(&dir, "run.json").status.success());
    let arithmetic = fs::read(dir.join("Arithmetic.nmlt")).unwrap();
    let mut changed = arithmetic.clone();
    changed.extend_from_slice(b"\n// changed dependency\n");
    fs::write(dir.join("Arithmetic.nmlt"), &changed).unwrap();
    let failed = replay(&dir, "run.json", "main.nmlt");
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("package source mismatch"));
    changed.extend_from_slice(b"fn unused() -> Int { false }\n");
    fs::write(dir.join("Arithmetic.nmlt"), &changed).unwrap();
    let failed = run(&dir, "absent.json");
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("Arithmetic.nmlt"));
    assert!(!dir.join("absent.json").exists());
    fs::rename(dir.join("Arithmetic.nmlt"), dir.join("removed.nmlt")).unwrap();
    let failed = replay(&dir, "run.json", "main.nmlt");
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("Work.nmlt"));
    fs::write(dir.join("Arithmetic.nmlt"), arithmetic).unwrap();
    assert!(replay(&dir, "run.json", "main.nmlt").status.success());
    fs::write(dir.join("Unrelated.nmlt"), "invalid unrelated source").unwrap();
    assert!(replay(&dir, "run.json", "main.nmlt").status.success());
}

#[test]
fn manifest_versions_and_nested_metadata_mutations_fail() {
    let dir = directory();
    let original = decoded(&run(&dir, "run.json"));
    for (pointer, value) in [
        ("/sources/1/path", json!("Unknown.nmlt")),
        ("/sources/2/source_sha256", json!("00")),
        ("/sources/3/bytes", json!(0)),
        ("/sources/0/path", json!("../main.nmlt")),
        ("/sources", json!([])),
        ("/schema", json!("nmlt-pure-run-v2")),
        ("/profile", json!("pure-workflow-v2")),
    ] {
        let mut value_record = original.clone();
        *value_record.pointer_mut(pointer).unwrap() = value;
        fs::write(
            dir.join("changed.json"),
            serde_json::to_vec(&value_record).unwrap(),
        )
        .unwrap();
        assert!(
            !replay(&dir, "changed.json", "main.nmlt").status.success(),
            "{pointer}"
        );
    }
    let mut changed = original.clone();
    changed["sources"][1]["extra"] = json!(true);
    fs::write(
        dir.join("changed.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    assert!(!replay(&dir, "changed.json", "main.nmlt").status.success());
    let duplicate = serde_json::to_string(&original).unwrap().replacen(
        "\"path\":\"Reports.nmlt\"",
        "\"path\":\"Other.nmlt\",\"path\":\"Reports.nmlt\"",
        1,
    );
    fs::write(dir.join("changed.json"), duplicate).unwrap();
    assert!(!replay(&dir, "changed.json", "main.nmlt").status.success());
}

#[test]
fn imported_runtime_stops_have_file_indices_and_replay_without_promotion() {
    let dir = directory();
    fs::write(
        dir.join("main.nmlt"),
        "import Lib\nfn main() -> Int { Lib.work() }",
    )
    .unwrap();
    fs::write(
        dir.join("Lib.nmlt"),
        "fn work() -> Int { 9223372036854775807 + 1 }",
    )
    .unwrap();
    let result = cli(
        &dir,
        &[
            "run",
            "main.nmlt",
            "--entry",
            "main",
            "--max-steps",
            "100",
            "--emit-run",
            "stop.json",
        ],
    );
    assert!(!result.status.success());
    let record = decoded(&result);
    let at = &record["execution"]["stop"]["at"];
    assert_eq!(record["execution"]["stop"]["kind"], "integer_overflow");
    assert_eq!(
        record["sources"][at["source"].as_u64().unwrap() as usize]["path"],
        "Lib.nmlt"
    );
    assert!(replay(&dir, "stop.json", "main.nmlt").status.success());
    let mut changed = record;
    changed["execution"]["stop"]["at"]["source"] = json!(0);
    fs::write(
        dir.join("changed.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    assert!(!replay(&dir, "changed.json", "main.nmlt").status.success());
}

#[test]
fn loader_requires_exact_case_regular_utf8_files_and_bounded_source() {
    let dir = directory();
    fs::write(
        dir.join("main.nmlt"),
        "import Lower\nfn main() -> Int { 0 }",
    )
    .unwrap();
    fs::write(dir.join("lower.nmlt"), "record R {}").unwrap();
    let failed = cli(&dir, &["typecheck", "main.nmlt", "--profile", "workflow"]);
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("exact-case"));
    fs::write(dir.join("main.nmlt"), "import Bad\nfn main() -> Int { 0 }").unwrap();
    fs::create_dir(dir.join("Bad.nmlt")).unwrap();
    assert!(
        !cli(&dir, &["typecheck", "main.nmlt", "--profile", "workflow"])
            .status
            .success()
    );
    fs::write(
        dir.join("main.nmlt"),
        "import Bytes\nfn main() -> Int { 0 }",
    )
    .unwrap();
    fs::write(dir.join("Bytes.nmlt"), [0xff, 0xfe]).unwrap();
    assert!(
        !cli(&dir, &["typecheck", "main.nmlt", "--profile", "workflow"])
            .status
            .success()
    );
    fs::write(dir.join("Bytes.nmlt"), vec![b' '; 131073]).unwrap();
    assert!(
        !cli(&dir, &["typecheck", "main.nmlt", "--profile", "workflow"])
            .status
            .success()
    );
}

#[cfg(unix)]
#[test]
fn symbolic_source_links_are_rejected_for_entries_and_dependencies() {
    use std::os::unix::fs::symlink;
    let dir = directory();
    symlink(dir.join("main.nmlt"), dir.join("entry.nmlt")).unwrap();
    assert!(
        !cli(&dir, &["typecheck", "entry.nmlt", "--profile", "workflow"])
            .status
            .success()
    );
    let package = dir.join("package");
    fs::create_dir(&package).unwrap();
    fs::write(
        package.join("main.nmlt"),
        "import Reports\nfn main() -> Int { 0 }",
    )
    .unwrap();
    symlink(dir.join("Reports.nmlt"), package.join("Reports.nmlt")).unwrap();
    assert!(
        !cli(
            &package,
            &["typecheck", "main.nmlt", "--profile", "workflow"]
        )
        .status
        .success()
    );
}

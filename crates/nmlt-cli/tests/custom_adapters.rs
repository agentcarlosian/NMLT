use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn nmlt_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nmlt"))
}

fn compile_test_adapter(dir: &Path) -> PathBuf {
    let src = dir.join("test_adapter.rs");
    let bin = dir.join(if cfg!(windows) {
        "test_adapter.exe"
    } else {
        "test_adapter"
    });
    // Minimal zero-dependency Rust executable speaking `nmlt-job-response-v1`.
    // Echoes `binding` verbatim and transforms the request `input` string.
    let code = r##"
use std::io::{self, Read};

fn find_json_field<'a>(s: &'a str, key: &str) -> &'a str {
    let needle = format!("\"{key}\":");
    let start = s.find(&needle).expect("missing field") + needle.len();
    let rest = &s[start..];
    let first = rest.as_bytes()[0];
    if first == b'{' {
        let mut depth = 0usize;
        let mut in_str = false;
        let mut esc = false;
        for (idx, b) in rest.bytes().enumerate() {
            if in_str {
                if esc { esc = false; }
                else if b == b'\\' { esc = true; }
                else if b == b'"' { in_str = false; }
            } else if b == b'"' { in_str = true; }
            else if b == b'{' { depth += 1; }
            else if b == b'}' {
                depth -= 1;
                if depth == 0 {
                    return &rest[..=idx];
                }
            }
        }
        panic!("unterminated object");
    } else if first == b'"' {
        let mut esc = false;
        for (idx, b) in rest[1..].bytes().enumerate() {
            if esc { esc = false; }
            else if b == b'\\' { esc = true; }
            else if b == b'"' {
                return &rest[1..1 + idx];
            }
        }
        panic!("unterminated string");
    } else {
        panic!("unexpected token");
    }
}

fn main() {
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw).unwrap();
    let binding = find_json_field(&raw, "binding");
    // Unescape the nested JSON text in `input.value` enough to read `"input":"..."`.
    let inner_escaped = find_json_field(&raw, "value");
    let inner = inner_escaped.replace("\\\"", "\"").replace("\\\\", "\\");
    let payload = find_json_field(&inner, "input");
    let outcome = if let Some(msg) = payload.strip_prefix("fail:") {
        format!(r#"{{"kind":"failed","message":"{msg}"}}"#)
    } else {
        format!(r#"{{"kind":"completed","value":{{"type":"text","value":"ADAPTER:{}"}}}}"#, payload.to_uppercase())
    };
    println!(
        r#"{{"schema":"nmlt-job-response-v1","binding":{binding},"outcome":{outcome},"observed_work":1}}"#
    );
}
"##;
    fs::write(&src, code).unwrap();
    let status = Command::new("rustc")
        .arg(&src)
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("rustc must be available in cargo test environment");
    assert!(status.success(), "failed to compile test custom adapter");
    bin
}

#[test]
fn custom_adapter_end_to_end_project_lock_run_replay_and_tamper_rejection() {
    let root = std::env::temp_dir().join(format!("nmlt-custom-adapter-e2e-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let adapter_bin = compile_test_adapter(&root);
    let bin_name = adapter_bin.file_name().unwrap().to_str().unwrap();
    let pinned_config = root.join("adapter_policy.txt");
    fs::write(&pinned_config, "strict-policy-v1\n").unwrap();

    let workflow = r#"
fn main(first_in: Text, backup_in: Text) -> Outcome<Text> {
  let a = job_start_adapter("echo", first_in);
  let b = job_start_adapter("echo", backup_in);
  let ra = job_collect(a);
  let rb = job_collect(b);
  match ra {
    Ok(v) => Ok(v),
    Err(msg) => match rb {
      Ok(v) => Ok(v),
      Err(other) => Err(other)
    }
  }
}
"#;
    fs::write(root.join("main.nmlt"), workflow).unwrap();

    let manifest = format!(
        r#"schema = "nmlt-project-v1"
source = "main.nmlt"
entry = "main"
max_steps = 1000

[inputs]
first_in = "fail:primary_down"
backup_in = "hello"

[jobs]
slots = 2
max_attempts = 4
timeout_ms = 10000

[[adapters]]
alias = "echo"
executable = "{bin_name}"
args = []
timeout_ms = 5000
inputs = ["adapter_policy.txt"]

[[tests]]
name = "fallback succeeds via custom adapter"
entry = "main"
inputs = {{ first_in = "fail:primary_down", backup_in = "hello" }}
expect = {{ Ok = "ADAPTER:HELLO" }}

[[tests]]
name = "primary succeeds directly"
entry = "main"
inputs = {{ first_in = "direct", backup_in = "unused" }}
expect = {{ Ok = "ADAPTER:DIRECT" }}
"#
    );
    fs::write(root.join("nmlt.toml"), manifest).unwrap();

    // 1. Lock pins adapter executable SHA-256 into nmlt.lock.
    let lock_out = Command::new(nmlt_bin())
        .args(["lock"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        lock_out.status.success(),
        "lock failed: {}",
        String::from_utf8_lossy(&lock_out.stderr)
    );
    let lock_text = fs::read_to_string(root.join("nmlt.lock")).unwrap();
    assert!(lock_text.contains("nmlt-custom-adapters-v1"));
    assert!(lock_text.contains("\"alias\": \"echo\""));
    assert!(lock_text.contains("adapter_policy.txt"));

    // 2. `nmlt test` runs both real subprocess adapter tests and verifies outcomes.
    let test_out = Command::new(nmlt_bin())
        .args(["test"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        test_out.status.success(),
        "test failed: {}\n{}",
        String::from_utf8_lossy(&test_out.stdout),
        String::from_utf8_lossy(&test_out.stderr)
    );

    // 3. `nmlt run` executes and produces a replayable record.
    let run_out = Command::new(nmlt_bin())
        .args(["run"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        run_out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run_out.stderr)
    );
    let run_json: serde_json::Value = serde_json::from_slice(&run_out.stdout).unwrap();
    assert_eq!(
        run_json["value"],
        serde_json::json!({"kind": "ok", "value": {"kind": "text", "value": "ADAPTER:HELLO"}})
    );
    let record_path = run_json["record"].as_str().unwrap();

    // 4. `nmlt replay` verifies the retained custom-adapter record offline.
    let replay_out = Command::new(nmlt_bin())
        .args(["replay", record_path, "--project"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        replay_out.status.success(),
        "replay failed: {}",
        String::from_utf8_lossy(&replay_out.stderr)
    );

    // 5a. Negative control: tampering with a pinned `inputs` file must be rejected!
    fs::write(&pinned_config, "tampered-policy\n").unwrap();
    let tampered_input_out = Command::new(nmlt_bin())
        .args(["run"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        !tampered_input_out.status.success(),
        "tampered pinned adapter input file must fail lock verification"
    );
    fs::write(&pinned_config, "strict-policy-v1\n").unwrap();

    // 5b. Negative control: tampering with the pinned adapter binary must be rejected!
    let mut bytes = fs::read(&adapter_bin).unwrap();
    bytes.push(0);
    fs::write(&adapter_bin, bytes).unwrap();
    let tampered_out = Command::new(nmlt_bin())
        .args(["run"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        !tampered_out.status.success(),
        "tampered adapter binary must fail lock verification"
    );

    // 6. Negative control: affine handle checker rejects dropped or double-collected adapter jobs.
    let leaked = root.join("leaked.nmlt");
    fs::write(
        &leaked,
        "fn main(x: Text) -> Int {\n  let h = job_start_adapter(\"echo\", x);\n  1\n}\n",
    )
    .unwrap();
    let leak_check = Command::new(nmlt_bin())
        .args(["typecheck"])
        .arg(&leaked)
        .args(["--profile", "workflow"])
        .output()
        .unwrap();
    assert!(
        !leak_check.status.success(),
        "leaked adapter handle must fail typecheck"
    );

    let double_collect = root.join("double.nmlt");
    fs::write(
        &double_collect,
        "fn main(x: Text) -> Outcome<Text> {\n  let h = job_start_adapter(\"echo\", x);\n  let a = job_collect(h);\n  job_collect(h)\n}\n",
    )
    .unwrap();
    let dbl_check = Command::new(nmlt_bin())
        .args(["typecheck"])
        .arg(&double_collect)
        .args(["--profile", "workflow"])
        .output()
        .unwrap();
    assert!(
        !dbl_check.status.success(),
        "double-collected adapter handle must fail typecheck"
    );

    let _ = fs::remove_dir_all(&root);
}

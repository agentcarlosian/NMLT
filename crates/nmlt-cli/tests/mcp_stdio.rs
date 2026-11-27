use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn nmlt_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nmlt"))
}

fn compile_mock_lean(root: &Path) -> PathBuf {
    let bin_dir = root.join("mock-lean").join("bin");
    let lib_dir = root.join("mock-lean").join("lib");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::create_dir_all(&lib_dir).unwrap();
    fs::write(lib_dir.join("marker.txt"), b"mock-lean-lib").unwrap();

    let src = root.join("mock_lean.rs");
    let bin = bin_dir.join(if cfg!(windows) { "lean.exe" } else { "lean" });
    let version = nmlt_runtime::lean::version();
    let code = format!(
        r##"
use std::env;

fn main() {{
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--version") {{
        println!("Lean (version {version}, mock-mcp-stdio)");
        return;
    }}
    let last = args.last().map(String::as_str).unwrap_or("");
    if last.ends_with("NMLTTask.lean") {{
        println!("{{}}", r#"NMLT_TASK={{"declaration":"Example.goal","module":"Example","type_repr":"True","type_lean":"True","level_params":[],"type_references":[]}}"#);
    }} else if last.ends_with("NMLTTaskInspect.lean") {{
        println!("{{}}", r#"NMLT_INSPECT={{"prefix":"Example","limit":5,"matched":1,"has_more":false,"declarations":[{{"name":"Example.goal","module":"Example","kind":"theorem","type_lean":"True","level_params":[],"axioms":[],"axioms_within_policy":true}}]}}"#);
    }}
}}
"##
    );
    fs::write(&src, code).unwrap();
    let status = Command::new("rustc")
        .arg(&src)
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("rustc must be available in cargo test environment");
    assert!(status.success(), "failed to compile mock lean");
    bin
}

#[test]
fn mcp_stdio_process_emits_pure_json_rpc_lines_including_successful_inspect() {
    let root = std::env::temp_dir().join(format!("nmlt-mcp-stdio-e2e-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let mock_lean = compile_mock_lean(&root);
    let project = root.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("nmlt-lean.json"),
        serde_json::to_string_pretty(&json!({
            "schema": "nmlt-lean-project-v1",
            "modules": ["Example"],
            "target_module": "Example",
            "target": "Example.goal",
            "permitted_axioms": []
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        project.join("Example.lean"),
        "theorem Example.goal : True := True.intro\n",
    )
    .unwrap();

    let work_dir = root.join("mcp-work");
    let bind_out = work_dir.join("bound-task");
    let inspect_out = work_dir.join("inspect-out");
    let escaped_out = root.join("escaped-outside-workdir");

    let requests = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "nmlt_lean_bind",
                "arguments": {
                    "project": project.display().to_string(),
                    "lean_bin": mock_lean.display().to_string(),
                    "output": escaped_out.display().to_string()
                }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "nmlt_lean_bind",
                "arguments": {
                    "project": project.display().to_string(),
                    "lean_bin": mock_lean.display().to_string(),
                    "output": bind_out.display().to_string()
                }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "nmlt_lean_inspect",
                "arguments": {
                    "task": bind_out.display().to_string(),
                    "prefix": "Example",
                    "limit": 5,
                    "lean_bin": mock_lean.display().to_string(),
                    "output": inspect_out.display().to_string()
                }
            }
        }),
    ];

    let mut child = Command::new(nmlt_bin())
        .arg("mcp")
        .arg("--work-dir")
        .arg(&work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn nmlt mcp");

    {
        let mut stdin = child.stdin.take().unwrap();
        for req in &requests {
            writeln!(stdin, "{}", serde_json::to_string(req).unwrap()).unwrap();
        }
    }

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "nmlt mcp failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    // Notification produces no reply -> exactly 5 JSON-RPC response lines.
    assert_eq!(
        lines.len(),
        5,
        "expected exactly 5 pure JSON-RPC lines on stdout, got {}:\n{stdout}",
        lines.len()
    );

    let responses: Vec<Value> = lines
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            serde_json::from_str::<Value>(line)
                .unwrap_or_else(|e| panic!("stdout line {idx} is not valid JSON ({e}): {line:?}"))
        })
        .collect();

    // Verify output path escaping --work-dir is rejected by default and not created.
    assert_eq!(responses[2]["id"], 3);
    assert_eq!(responses[2]["result"]["isError"], true);
    assert!(!escaped_out.exists());

    // Verify bind inside --work-dir succeeded cleanly.
    assert_eq!(responses[3]["id"], 4);
    assert_eq!(responses[3]["result"]["isError"], false);
    let bind_payload: Value = serde_json::from_str(
        responses[3]["result"]["content"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(bind_payload["target_declaration"], "Example.goal");
    assert_eq!(bind_payload["task_sha256"].as_str().unwrap().len(), 64);

    // Verify successful inspection produced zero non-JSON stdout leakage and valid payload.
    assert_eq!(responses[4]["id"], 5);
    assert_eq!(
        responses[4]["result"]["isError"], false,
        "inspect failed: {}",
        responses[4]
    );
    let inspect_payload: Value = serde_json::from_str(
        responses[4]["result"]["content"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        inspect_payload["assurance"],
        "none (context inspection only)"
    );
    assert_eq!(
        inspect_payload["inspection"]["schema"],
        "nmlt-lean-inspection-v1"
    );
    assert_eq!(inspect_payload["inspection"]["status"], "context_only");
    assert_eq!(inspect_payload["inspection"]["context"]["matched"], 1);
    assert_eq!(
        inspect_payload["inspection"]["context"]["declarations"][0]["name"],
        "Example.goal"
    );

    let _ = fs::remove_dir_all(&root);
}

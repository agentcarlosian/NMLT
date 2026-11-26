//! Stdio JSON-RPC 2.0 / MCP tamper-proof Lean verification oracle (RFC 0041).
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const PROTOCOL_VERSION: &str = "2024-11-05";
const MAX_LINE_BYTES: usize = 256 * 1024;

const HELP: &str = "\
Stdio MCP tamper-proof Lean verification oracle (RFC 0041):
  nmlt mcp [--work-dir DIR]

Speaks newline-delimited JSON-RPC 2.0 over stdin/stdout and exposes:
  nmlt_tools_status  Inspect resolved Lean, lean4export, and nanoda paths and digests
  nmlt_lean_bind     Bind a Lean project into an immutable task directory + task_sha256
  nmlt_lean_inspect  Query bound Lean declarations by bounded prefix
  nmlt_lean_prove    Independently verify a Lean proof (v1/v2/v3) with Lean + lean4export + nanoda
  nmlt_lean_recheck  Independently recheck a retained acceptance record
  nmlt_lean_revise   Explicitly revise a bound task with an audit reason
";

pub fn command(args: &[OsString]) -> Result<(), String> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{HELP}");
        return Ok(());
    }
    let mut work_dir: Option<PathBuf> = None;
    let mut rest = args;
    while !rest.is_empty() {
        match rest {
            [flag, value, tail @ ..] if flag == "--work-dir" => {
                if work_dir.is_some() {
                    return Err("duplicate --work-dir option".into());
                }
                work_dir = Some(PathBuf::from(value));
                rest = tail;
            }
            _ => return Err(format!("invalid mcp option\n\n{HELP}")),
        }
    }
    let work_dir = match work_dir {
        Some(dir) => dir,
        None => {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            std::env::temp_dir().join(format!("nmlt-mcp-{}-{stamp}", std::process::id()))
        }
    };
    let stdin = io::stdin();
    let stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());
    let writer = BufWriter::new(stdout.lock());
    let mut server = Server::new(work_dir);
    server.serve_io(reader, writer)
}

struct Server {
    work_dir: PathBuf,
    counter: u64,
}

impl Server {
    fn new(work_dir: PathBuf) -> Self {
        Self {
            work_dir,
            counter: 0,
        }
    }

    fn next_output_dir(&mut self, prefix: &str, explicit: Option<&str>) -> Result<PathBuf, String> {
        if let Some(path) = explicit {
            return Ok(PathBuf::from(path));
        }
        fs::create_dir_all(&self.work_dir).map_err(|e| {
            format!(
                "failed to create MCP work directory '{}': {e}",
                self.work_dir.display()
            )
        })?;
        loop {
            self.counter += 1;
            let candidate = self.work_dir.join(format!("{prefix}-{:04}", self.counter));
            if !candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    fn serve_io<R: BufRead, W: Write>(
        &mut self,
        mut reader: R,
        mut writer: W,
    ) -> Result<(), String> {
        let mut buf = Vec::new();
        loop {
            buf.clear();
            let read = read_bounded_line(&mut reader, &mut buf, MAX_LINE_BYTES)?;
            if read == 0 {
                break;
            }
            if read > MAX_LINE_BYTES {
                let err = jsonrpc_error(
                    Value::Null,
                    -32600,
                    "JSON-RPC request exceeds 256 KiB line limit",
                );
                write_message(&mut writer, &err)?;
                continue;
            }
            let text = match std::str::from_utf8(&buf) {
                Ok(s) => s.trim(),
                Err(_) => {
                    let err = jsonrpc_error(Value::Null, -32700, "request is not valid UTF-8");
                    write_message(&mut writer, &err)?;
                    continue;
                }
            };
            if text.is_empty() {
                continue;
            }
            let msg: Value = match serde_json::from_str(text) {
                Ok(v) => v,
                Err(e) => {
                    let err = jsonrpc_error(Value::Null, -32700, &format!("parse error: {e}"));
                    write_message(&mut writer, &err)?;
                    continue;
                }
            };
            if let Some(response) = self.handle_message(&msg) {
                write_message(&mut writer, &response)?;
            }
        }
        Ok(())
    }

    fn handle_message(&mut self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let Some(method) = msg.get("method").and_then(Value::as_str) else {
            return id.map(|id| jsonrpc_error(id, -32600, "missing JSON-RPC method"));
        };
        if msg.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return id.map(|id| jsonrpc_error(id, -32600, "jsonrpc field must be \"2.0\""));
        }
        // Notifications have no `id` and expect no reply.
        let id = id?;
        let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
        match method {
            "initialize" => Some(jsonrpc_ok(
                id,
                json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "nmlt-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            )),
            "ping" => Some(jsonrpc_ok(id, json!({}))),
            "tools/list" => Some(jsonrpc_ok(id, json!({ "tools": tool_definitions() }))),
            "tools/call" => Some(self.handle_tool_call(id, &params)),
            _ => Some(jsonrpc_error(
                id,
                -32601,
                &format!("method not found: {method}"),
            )),
        }
    }

    fn handle_tool_call(&mut self, id: Value, params: &Value) -> Value {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return jsonrpc_error(id, -32602, "tools/call requires string `name`");
        };
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if !args.is_object() {
            return jsonrpc_error(id, -32602, "tools/call `arguments` must be an object");
        }
        let outcome = match name {
            "nmlt_tools_status" => self.call_tools_status(&args),
            "nmlt_lean_bind" => self.call_lean_bind(&args),
            "nmlt_lean_inspect" => self.call_lean_inspect(&args),
            "nmlt_lean_prove" => self.call_lean_prove(&args),
            "nmlt_lean_recheck" => self.call_lean_recheck(&args),
            "nmlt_lean_revise" => self.call_lean_revise(&args),
            _ => return jsonrpc_error(id, -32602, &format!("unknown tool: {name}")),
        };
        match outcome {
            Ok(payload) => {
                let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into());
                jsonrpc_ok(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }),
                )
            }
            Err(message) => {
                let payload = json!({
                    "status": "rejected",
                    "assurance": "none",
                    "error": message
                });
                let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into());
                jsonrpc_ok(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": true
                    }),
                )
            }
        }
    }

    fn call_tools_status(&self, args: &Value) -> Result<Value, String> {
        let dir = opt_str(args, "dir")?.map(PathBuf::from);
        let report = crate::tools::status(dir.as_deref())?;
        serde_json::to_value(report).map_err(|e| e.to_string())
    }

    fn call_lean_bind(&mut self, args: &Value) -> Result<Value, String> {
        let project = req_str(args, "project")?;
        let lean_bin = opt_str(args, "lean_bin")?.map(PathBuf::from);
        let output = self.next_output_dir("bind", opt_str(args, "output")?)?;
        let bound = crate::lean_task::bind_api(Path::new(project), lean_bin.as_deref(), &output)?;
        Ok(json!({
            "assurance": "none (binding only; not proof acceptance)",
            "task_sha256": bound.task_sha256,
            "task_dir": output.display().to_string(),
            "task_path": bound.task_path.display().to_string(),
            "target_declaration": bound.target_declaration,
            "target_module": bound.target_module,
            "target_type": bound.target_type,
            "permitted_axioms": bound.permitted_axioms,
        }))
    }

    fn call_lean_inspect(&mut self, args: &Value) -> Result<Value, String> {
        let task = req_str(args, "task")?;
        let digest = opt_str(args, "task_sha256")?;
        let prefix = req_str(args, "prefix")?;
        let limit = match args.get("limit") {
            None | Some(Value::Null) => "10".to_owned(),
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::String(s)) => s.clone(),
            _ => return Err("`limit` must be an integer or string in 1..16".into()),
        };
        let lean_bin = opt_str(args, "lean_bin")?.map(PathBuf::from);
        let output = self.next_output_dir("inspect", opt_str(args, "output")?)?;
        let inspection = crate::lean_task::inspect_api(
            Path::new(task),
            digest,
            prefix,
            &limit,
            lean_bin.as_deref(),
            &output,
        )?;
        Ok(json!({
            "assurance": "none (context inspection only)",
            "output_dir": output.display().to_string(),
            "inspection": inspection
        }))
    }

    fn call_lean_prove(&mut self, args: &Value) -> Result<Value, String> {
        let task = req_str(args, "task")?;
        let digest = opt_str(args, "task_sha256")?;
        let proof = req_str(args, "proof")?;
        let lean_bin = opt_str(args, "lean_bin")?.map(PathBuf::from);
        let exporter = opt_str(args, "exporter")?.map(PathBuf::from);
        let nanoda = opt_str(args, "nanoda")?.map(PathBuf::from);
        let output = self.next_output_dir("prove", opt_str(args, "output")?)?;
        let summary = crate::lean_task::prove_api(
            Path::new(task),
            digest,
            proof,
            lean_bin.as_deref(),
            exporter.as_deref(),
            nanoda.as_deref(),
            &output,
        )?;
        serde_json::to_value(summary).map_err(|e| e.to_string())
    }

    fn call_lean_recheck(&mut self, args: &Value) -> Result<Value, String> {
        let record = req_str(args, "record")?;
        let digest = opt_str(args, "task_sha256")?;
        let lean_bin = opt_str(args, "lean_bin")?.map(PathBuf::from);
        let exporter = opt_str(args, "exporter")?.map(PathBuf::from);
        let nanoda = opt_str(args, "nanoda")?.map(PathBuf::from);
        let output = self.next_output_dir("recheck", opt_str(args, "output")?)?;
        let summary = crate::lean_task::recheck_api(
            Path::new(record),
            digest,
            lean_bin.as_deref(),
            exporter.as_deref(),
            nanoda.as_deref(),
            &output,
        )?;
        serde_json::to_value(summary).map_err(|e| e.to_string())
    }

    fn call_lean_revise(&mut self, args: &Value) -> Result<Value, String> {
        let task = req_str(args, "task")?;
        let digest = opt_str(args, "task_sha256")?;
        let project = req_str(args, "project")?;
        let reason = req_str(args, "reason")?;
        let lean_bin = opt_str(args, "lean_bin")?.map(PathBuf::from);
        let output = self.next_output_dir("revise", opt_str(args, "output")?)?;
        let bound = crate::lean_task::revise_api(
            Path::new(task),
            digest,
            Path::new(project),
            reason,
            lean_bin.as_deref(),
            &output,
        )?;
        Ok(json!({
            "assurance": "none (task revision only; prior acceptance invalidated for revised task)",
            "task_sha256": bound.task_sha256,
            "task_dir": output.display().to_string(),
            "task_path": bound.task_path.display().to_string(),
            "target_declaration": bound.target_declaration,
            "target_module": bound.target_module,
            "target_type": bound.target_type,
            "permitted_axioms": bound.permitted_axioms,
        }))
    }
}

fn req_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("missing or empty string argument `{key}`"))
}

fn opt_str<'a>(args: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.as_str())),
        _ => Err(format!(
            "optional argument `{key}` must be a non-empty string"
        )),
    }
}

fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    out: &mut Vec<u8>,
    limit: usize,
) -> Result<usize, String> {
    let mut total = 0usize;
    loop {
        let available = reader.fill_buf().map_err(|e| e.to_string())?;
        if available.is_empty() {
            return Ok(total);
        }
        if let Some(pos) = available.iter().position(|&b| b == b'\n') {
            let chunk = &available[..=pos];
            total += chunk.len();
            if total <= limit {
                out.extend_from_slice(chunk);
            }
            reader.consume(pos + 1);
            return Ok(total);
        }
        let len = available.len();
        total += len;
        if total <= limit {
            out.extend_from_slice(available);
        }
        reader.consume(len);
    }
}

fn write_message<W: Write>(writer: &mut W, value: &Value) -> Result<(), String> {
    let line = serde_json::to_string(value).map_err(|e| e.to_string())?;
    writer
        .write_all(line.as_bytes())
        .and_then(|()| writer.write_all(b"\n"))
        .and_then(|()| writer.flush())
        .map_err(|e| e.to_string())
}

fn jsonrpc_ok(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn jsonrpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "nmlt_tools_status",
            "description": "Inspect resolved Lean, lean4export, and nanoda executables, their SHA-256 digests, and whether independent proof checking is ready.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "dir": { "type": "string", "description": "Optional tool cache directory override." }
                }
            }
        },
        {
            "name": "nmlt_lean_bind",
            "description": "Bind a Lean project containing nmlt-lean.json into an immutable task directory and task_sha256 digest. Binding is not proof acceptance.",
            "inputSchema": {
                "type": "object",
                "required": ["project"],
                "properties": {
                    "project": { "type": "string", "description": "Path to Lean project directory containing nmlt-lean.json." },
                    "lean_bin": { "type": "string", "description": "Optional direct Lean executable override." },
                    "output": { "type": "string", "description": "Optional new output directory path." }
                }
            }
        },
        {
            "name": "nmlt_lean_inspect",
            "description": "Query declarations by bounded prefix in a bound Lean task. Returns context inspection only (not proof acceptance).",
            "inputSchema": {
                "type": "object",
                "required": ["task", "prefix"],
                "properties": {
                    "task": { "type": "string", "description": "Path to bound task.json or task directory." },
                    "task_sha256": { "type": "string", "description": "Optional 64-hex task digest (auto-read from task.sha256 if omitted)." },
                    "prefix": { "type": "string", "description": "Lean declaration prefix to query." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 16, "description": "Maximum declarations to return (default 10)." },
                    "lean_bin": { "type": "string", "description": "Optional direct Lean executable override." },
                    "output": { "type": "string", "description": "Optional new output directory path." }
                }
            }
        },
        {
            "name": "nmlt_lean_prove",
            "description": "Validate a Lean 4 proof candidate (term, bounded automation, or RFC 0039 guarded tactic script) and independently verify it with Lean, lean4export, and nanoda under the task's immutable target and axiom policy.",
            "inputSchema": {
                "type": "object",
                "required": ["task", "proof"],
                "properties": {
                    "task": { "type": "string", "description": "Path to bound task.json or task directory." },
                    "task_sha256": { "type": "string", "description": "Optional 64-hex task digest (auto-read from task.sha256 if omitted)." },
                    "proof": { "type": "string", "description": "Lean 4 proof term or `by ...` tactic script." },
                    "lean_bin": { "type": "string", "description": "Optional direct Lean executable override." },
                    "exporter": { "type": "string", "description": "Optional lean4export executable override." },
                    "nanoda": { "type": "string", "description": "Optional nanoda_bin executable override." },
                    "output": { "type": "string", "description": "Optional new output directory path." }
                }
            }
        },
        {
            "name": "nmlt_lean_recheck",
            "description": "Independently recheck a retained nmlt-lean-result-v5 acceptance record with Lean, lean4export, and nanoda.",
            "inputSchema": {
                "type": "object",
                "required": ["record"],
                "properties": {
                    "record": { "type": "string", "description": "Path to result.json or acceptance output directory." },
                    "task_sha256": { "type": "string", "description": "Optional 64-hex task digest (auto-read from task.sha256 if omitted)." },
                    "lean_bin": { "type": "string", "description": "Optional direct Lean executable override." },
                    "exporter": { "type": "string", "description": "Optional lean4export executable override." },
                    "nanoda": { "type": "string", "description": "Optional nanoda_bin executable override." },
                    "output": { "type": "string", "description": "Optional new output directory path." }
                }
            }
        },
        {
            "name": "nmlt_lean_revise",
            "description": "Explicitly revise a bound Lean task with a mandatory audit reason, generating a new task_sha256 and invalidating prior acceptance for the revised task.",
            "inputSchema": {
                "type": "object",
                "required": ["task", "project", "reason"],
                "properties": {
                    "task": { "type": "string", "description": "Path to parent task.json or task directory." },
                    "task_sha256": { "type": "string", "description": "Optional 64-hex parent task digest." },
                    "project": { "type": "string", "description": "Path to revised Lean project directory." },
                    "reason": { "type": "string", "description": "Explicit non-empty audit reason for revising the task." },
                    "lean_bin": { "type": "string", "description": "Optional direct Lean executable override." },
                    "output": { "type": "string", "description": "Optional new output directory path." }
                }
            }
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_session(lines: &[&str]) -> Vec<Value> {
        let base = std::env::temp_dir().join(format!("nmlt-mcp-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let input = format!("{}\n", lines.join("\n"));
        let mut output = Vec::new();
        let mut server = Server::new(base.clone());
        server
            .serve_io(io::Cursor::new(input.as_bytes()), &mut output)
            .unwrap();
        let _ = fs::remove_dir_all(&base);
        String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    #[test]
    fn mcp_handshake_tools_list_and_status_call_succeed() {
        let responses = run_session(&[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"nmlt_tools_status","arguments":{}}}"#,
        ]);
        // Notification produces no reply -> 4 responses total.
        assert_eq!(responses.len(), 4);
        assert_eq!(responses[0]["id"], 1);
        assert_eq!(responses[0]["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(responses[0]["result"]["serverInfo"]["name"], "nmlt-mcp");
        assert_eq!(responses[1]["id"], 2);
        assert_eq!(responses[1]["result"], json!({}));
        let tools = responses[2]["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 6);
        let names: Vec<_> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(
            names,
            vec![
                "nmlt_tools_status",
                "nmlt_lean_bind",
                "nmlt_lean_inspect",
                "nmlt_lean_prove",
                "nmlt_lean_recheck",
                "nmlt_lean_revise",
            ]
        );
        assert_eq!(responses[3]["id"], 4);
        assert_eq!(responses[3]["result"]["isError"], false);
        let payload: Value = serde_json::from_str(
            responses[3]["result"]["content"][0]["text"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(payload["schema"], "nmlt-tools-status-v1");
    }

    #[test]
    fn mcp_rejects_malformed_unknown_oversized_and_bad_proof_calls() {
        let huge = "a".repeat(MAX_LINE_BYTES + 64);
        let responses = run_session(&[
            "not json at all",
            r#"{"jsonrpc":"2.0","id":1,"method":"unknown/method"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nonexistent","arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nmlt_lean_prove","arguments":{"task":"/nonexistent","proof":"by\n  sorry\n"}}}"#,
            &huge,
        ]);
        assert_eq!(responses.len(), 5);
        assert_eq!(responses[0]["error"]["code"], -32700);
        assert_eq!(responses[1]["error"]["code"], -32601);
        assert_eq!(responses[2]["error"]["code"], -32602);
        assert_eq!(responses[3]["id"], 3);
        assert_eq!(responses[3]["result"]["isError"], true);
        assert_eq!(responses[4]["error"]["code"], -32600);
    }
}

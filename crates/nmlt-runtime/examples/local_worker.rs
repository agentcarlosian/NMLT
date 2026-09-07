//! A real local subprocess adapter demonstration, not .nmlt workflow syntax.
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command as Process, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use nmlt_runtime::{
    Command, Dispatch, Error, Journal, Limits, Outcome, Receipt, Request, Response, RunSpec, Value,
    sha256, worker,
};

fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|s| s == "--worker") {
        let mut input = String::new();
        std::io::stdin().take(65_537).read_to_string(&mut input)?;
        if input.len() > 65_536 {
            return Err(Error("worker request too large".into()));
        }
        let dispatch: Dispatch = serde_json::from_str(&input)?;
        println!("{}", serde_json::to_string(&worker::evaluate(&dispatch)?)?);
        return Ok(());
    }
    if !(2..=17).contains(&args.len()) {
        return Err(Error(
            "usage: local_worker <new-journal.jsonl> <integer> [fallback integers; at most 16]"
                .into(),
        ));
    }
    let inputs = args[1..]
        .iter()
        .map(|s| {
            s.to_str()
                .ok_or_else(|| Error("input must be UTF-8".into()))?
                .parse::<i64>()
                .map_err(|e| Error(e.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let context = sha256(&serde_json::to_vec(&(worker::adapter(), &inputs))?);
    let instance = format!(
        "{}:{}:{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error(e.to_string()))?
            .as_nanos(),
        context
    );
    let spec = RunSpec {
        run_id: sha256(instance.as_bytes()),
        context_sha256: context.clone(),
        limits: Limits {
            slots: 1,
            generations_per_slot: inputs.len() as u32,
            max_attempts: inputs.len() as u32,
            max_events: inputs.len() as u32 * 16,
            work_budget: inputs.len() as u64,
        },
    };
    let mut journal = Journal::create(Path::new(&args[0]), spec.clone())?;
    let mut outcomes = Vec::new();
    let mut result = None;
    for input in inputs {
        let Receipt::Control { control } = journal.apply(Command::Reserve {
            task: "square".into(),
            owner: "coordinator".into(),
            request: Request {
                adapter: worker::adapter(),
                context_sha256: context.clone(),
                input: Value::Int(input),
                reserved_work: 1,
            },
        })?
        else {
            unreachable!()
        };
        let Receipt::Control { control } = journal.apply(Command::Transfer {
            control,
            to: "worker".into(),
        })?
        else {
            unreachable!()
        };
        let Receipt::Dispatch { dispatch, control } =
            journal.apply(Command::Dispatch { control })?
        else {
            unreachable!()
        };
        // The journal's dispatch intent is synced before a process can start.
        let response = call_worker(&dispatch);
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                journal.apply(Command::Timeout { control })?;
                return Err(Error(format!(
                    "worker transport failed; journal records uncertain work: {error}"
                )));
            }
        };
        let Receipt::Control { control } = journal.apply(Command::Deliver { response })? else {
            unreachable!()
        };
        let Receipt::Collected { outcome, .. } = journal.apply(Command::Collect { control })?
        else {
            unreachable!()
        };
        if let Outcome::Completed { value } = &outcome {
            result = Some(value.clone());
        }
        outcomes.push(outcome);
        if result.is_some() {
            break;
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "nmlt-local-worker-demo-v1", "assurance": "none", "run": spec,
            "outcomes": outcomes, "reused_values": [result.clone(), result],
            "accounting": journal.state().accounting(), "journal": args[0].to_string_lossy(),
            "scope": "local subprocess prototype; no .nmlt effect syntax or Lean acceptance"
        }))?
    );
    if result.is_none() {
        return Err(Error("all bounded worker attempts failed".into()));
    }
    Ok(())
}

fn call_worker(dispatch: &Dispatch) -> Result<Response, Error> {
    let mut child = Process::new(std::env::current_exe()?)
        .arg("--worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let send = child
        .stdin
        .take()
        .ok_or_else(|| Error("missing worker stdin".into()))?
        .write_all(&serde_json::to_vec(dispatch)?);
    if let Err(error) = send {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error.into());
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(Error("worker exited without a typed result".into()));
    }
    let response: Response = serde_json::from_slice(&output.stdout)?;
    worker::validate(dispatch, &response)?;
    Ok(response)
}

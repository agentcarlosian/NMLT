//! Real subprocess exercise for the initial asynchronous host API.
use nmlt_runtime::{
    self as rt, Outcome, Value,
    lean::{self, Strategy},
    session::{self, Bounds, Handle, Session, SquareTool, Status},
};
use std::io::{Read, Write};
use std::path::Path;

fn write_json(
    path: &Path,
    value: &impl serde::Serialize,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = serde_json::to_vec_pretty(value)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("snapshot exceeds 16 MiB".into());
    }
    let mut file = std::fs::File::create_new(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}
fn collect(
    session: &mut Session,
    handle: &mut Handle,
) -> Result<Outcome, Box<dyn std::error::Error>> {
    let status = session.wait(handle)?;
    if status != Status::Ready {
        return Err(format!(
            "job did not settle ({status:?}); observation: {}",
            serde_json::to_string(&session.observations().last())?
        )
        .into());
    }
    session
        .collect(handle)?
        .ok_or_else(|| "settled worker could not be collected".into())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "__square-worker" {
        let mut bytes = vec![];
        std::io::stdin().take(65_537).read_to_end(&mut bytes)?;
        if bytes.len() > 65_536 {
            return Err("request too large".into());
        }
        let dispatch: rt::Dispatch = serde_json::from_slice(&bytes)?;
        println!(
            "{}",
            serde_json::to_string(&rt::worker::evaluate(&dispatch)?)?
        );
        return Ok(());
    }
    if let [flag, path] = args.as_slice() {
        if flag == "--replay" {
            let mut bytes = vec![];
            std::fs::File::open(path)?
                .take(16 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 16 * 1024 * 1024 {
                return Err("snapshot too large".into());
            }
            let snapshot: session::Snapshot = serde_json::from_slice(&bytes)?;
            if serde_json::to_vec_pretty(&snapshot)? != bytes {
                return Err("snapshot encoding is not exact".into());
            }
            let state = session::verify_snapshot(&snapshot)?;
            println!(
                "{}",
                serde_json::json!({"matched":true,"assurance":"none","accounting":state.accounting(),"meaning":"record consistency only; no processes launched"})
            );
            return Ok(());
        }
        if flag == "--recover" {
            println!(
                "{}",
                serde_json::to_string(&session::recover(Path::new(path))?)?
            );
            return Ok(());
        }
    }
    let [directory, lean_executable] = args.as_slice() else {
        return Err("usage: async_jobs <new-directory> <lean-executable|--without-lean> | --replay <snapshot.json> | --recover <directory>".into());
    };
    let directory = Path::new(directory);
    let toolchain = if lean_executable == "--without-lean" {
        None
    } else {
        Some(lean::Toolchain::open(Path::new(lean_executable))?)
    };
    let with_lean = toolchain.is_some();
    let square = SquareTool::open(&std::env::current_exe()?)?;
    let run_id = format!(
        "async-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    let mut session = Session::create(
        directory,
        run_id,
        rt::sha256(b"async demo v1: two square jobs, cancellation, and three Lean strategies"),
        Bounds {
            slots: 2,
            max_attempts: 6,
            timeout_ms: 30000,
        },
        Some(square),
        toolchain,
    )?;
    let mut wrong = if with_lean {
        session.start_lean("wrong-proof", Strategy::WrongTerm)?
    } else {
        session.start_square("negative-input", -3)?
    };
    let mut good = session.start_square("square", 5)?;
    // Two attempts have been durably dispatched before any wait or collection.
    assert_eq!(session.state().accounting().dispatched_attempts, 2);
    let before_poll = session.state().event_count();
    let first_poll = session.poll(&wrong)?;
    let failure = collect(&mut session, &mut wrong)?;
    assert!(matches!(failure, Outcome::Failed { .. }));
    let square = collect(&mut session, &mut good)?;
    assert_eq!(
        square,
        Outcome::Completed {
            value: Value::Int(25)
        }
    );
    assert!(session.collect(&mut good).is_err());
    let mut cancelled = if with_lean {
        session.start_lean("cancel-proof", Strategy::Induction)?
    } else {
        session.start_square("cancel-square", 9)?
    };
    let cancellation_won = session.cancel(&mut cancelled)?;
    let cancellation_outcome = collect(&mut session, &mut cancelled)?;
    if cancellation_won {
        assert_eq!(cancellation_outcome, Outcome::Cancelled);
    }
    let mut checks = vec![];
    if with_lean {
        let mut admitted = session.start_lean("admitted-proof", Strategy::Admitted)?;
        let mut lemma = session.start_lean("existing-lemma", Strategy::ExistingLemma)?;
        let rejected = collect(&mut session, &mut admitted)?;
        assert!(matches!(&rejected, Outcome::Failed {message} if message.contains("axiom")));
        checks.push(rejected);
        let accepted = collect(&mut session, &mut lemma)?;
        assert_eq!(
            accepted,
            Outcome::Completed {
                value: Value::Text(rt::sha256(lean::source(Strategy::ExistingLemma).as_bytes()))
            }
        );
        checks.push(accepted);
        let mut induction = session.start_lean("induction", Strategy::Induction)?;
        let accepted = collect(&mut session, &mut induction)?;
        assert_eq!(
            accepted,
            Outcome::Completed {
                value: Value::Text(rt::sha256(lean::source(Strategy::Induction).as_bytes()))
            }
        );
        checks.push(accepted);
    }
    let snapshot = session.snapshot()?;
    assert_eq!(&session::verify_snapshot(&snapshot)?, session.state());
    let report = serde_json::json!({"schema":"nmlt-async-example-v1","assurance":"none", "with_lean":with_lean,
        "first_poll":first_poll,"events_before_poll":before_poll,"failure":failure,"square":square,"reused_square":[25,25],
        "cancellation_won":cancellation_won,"cancellation_outcome":cancellation_outcome,"lean_checks":checks,
        "accounting":session.state().accounting(),"manifest":session.manifest(),
        "meaning":"local adapter observations and journal consistency; no verified host execution or independent Lean proof check"});
    write_json(&directory.join("snapshot.json"), &snapshot)?;
    write_json(&directory.join("report.json"), &report)?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}

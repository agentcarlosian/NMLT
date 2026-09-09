use nmlt_runtime::process::{FailureKind, PIPE_BYTES, Process};
use std::io::{Read, Write};
use std::process::Command;
use std::time::{Duration, Instant};

fn probe(mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "process_probe", "--nocapture"])
        .env("NMLT_ASYNC_PROBE", mode);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
}
#[test]
// These fixtures deliberately leave descendants for the supervisor to reap.
#[allow(clippy::zombie_processes)]
fn process_probe() {
    let Ok(mode) = std::env::var("NMLT_ASYNC_PROBE") else {
        return;
    };
    match mode.as_str() {
        "sleep" => std::thread::sleep(Duration::from_secs(5)),
        "stdout" => {
            let _ = std::io::stdout().write_all(&vec![b'x'; PIPE_BYTES * 2]);
        }
        "stderr" => {
            let _ = std::io::stderr().write_all(&vec![b'x'; PIPE_BYTES * 2]);
        }
        "exit" => {
            eprint!("error: rejected");
            std::process::exit(1);
        }
        "input" => {
            let mut input = vec![];
            std::io::stdin().read_to_end(&mut input).unwrap();
            assert_eq!(input, b"hello");
            print!("received");
        }
        "late-marker" => {
            std::thread::sleep(Duration::from_millis(800));
            std::fs::write(
                std::env::var_os("NMLT_PROCESS_MARKER").unwrap(),
                b"survived",
            )
            .unwrap();
        }
        "descendant" | "orphan" => {
            let marker = std::path::PathBuf::from(std::env::var_os("NMLT_PROCESS_MARKER").unwrap());
            let _child = probe("late-marker")
                .env("NMLT_PROCESS_MARKER", &marker)
                .spawn()
                .unwrap();
            std::fs::write(marker.with_extension("ready"), b"ready").unwrap();
            if mode == "orphan" {
                std::process::exit(0);
            }
            std::thread::sleep(Duration::from_secs(5));
        }
        #[cfg(windows)]
        "supervisor" => {
            let marker = std::path::PathBuf::from(std::env::var_os("NMLT_PROCESS_MARKER").unwrap());
            let mut command = probe("late-marker");
            command.env("NMLT_PROCESS_MARKER", &marker);
            let _child = Process::start(command, vec![], Duration::from_secs(5)).unwrap();
            std::fs::write(marker.with_extension("ready"), b"ready").unwrap();
            std::thread::sleep(Duration::from_secs(5));
        }
        #[cfg(windows)]
        "memory-limit" => {
            let mut data = Vec::<u8>::new();
            assert!(data.try_reserve_exact(2 * 1024 * 1024 * 1024).is_err());
            print!("memory-ceiling-enforced");
        }
        #[cfg(windows)]
        "process-limit" => {
            let mut children = vec![];
            for _ in 0..16 {
                match probe("sleep")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Ok(child) => children.push(child),
                    Err(_) => break,
                }
            }
            let count = children.len();
            for child in &mut children {
                let _ = child.kill();
                let _ = child.wait();
            }
            assert_eq!(
                count, 15,
                "root plus children must fit sixteen process slots"
            );
            print!("process-ceiling-enforced");
        }
        _ => panic!("unknown mode"),
    }
}

fn marker(name: &str) -> std::path::PathBuf {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/process-tree-tests");
    std::fs::create_dir_all(&root).unwrap();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    root.join(format!("{name}-{}-{stamp}", std::process::id()))
}
fn await_ready(marker: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !marker.with_extension("ready").exists() {
        assert!(Instant::now() < deadline, "descendant was not started");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn timeout_cancel_drop_and_root_exit_do_not_leave_descendants() {
    for mode in ["timeout", "cancel", "drop", "orphan"] {
        let marker = marker(mode);
        let mut command = probe(if mode == "orphan" {
            "orphan"
        } else {
            "descendant"
        });
        command.env("NMLT_PROCESS_MARKER", &marker);
        let mut child = Process::start(
            command,
            vec![],
            if mode == "timeout" {
                Duration::from_millis(400)
            } else {
                Duration::from_secs(5)
            },
        )
        .unwrap();
        child.policy().validate().unwrap();
        await_ready(&marker);
        if mode == "cancel" {
            assert!(child.cancel().as_ref().unwrap_err().child_reaped);
        } else if mode != "drop" {
            assert!(child.wait().as_ref().unwrap_err().child_reaped);
        }
        drop(child);
        std::thread::sleep(Duration::from_millis(950));
        assert!(!marker.exists(), "{mode} left a descendant alive");
    }
}

#[cfg(windows)]
#[test]
fn windows_job_limits_and_abrupt_supervisor_death_are_enforced() {
    for mode in ["memory-limit", "process-limit"] {
        let mut child = Process::start(probe(mode), vec![], Duration::from_secs(5)).unwrap();
        assert_eq!(child.policy().mechanism, "job_object");
        assert_eq!(child.policy().max_processes, Some(16));
        let output = child
            .wait()
            .as_ref()
            .unwrap_or_else(|error| panic!("{mode}: {error:?}"));
        assert_eq!(
            output.exit_code,
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let marker = marker("parent-death");
    let mut supervisor = probe("supervisor")
        .env("NMLT_PROCESS_MARKER", &marker)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    await_ready(&marker);
    supervisor.kill().unwrap();
    supervisor.wait().unwrap();
    std::thread::sleep(Duration::from_millis(1100));
    assert!(
        !marker.exists(),
        "kernel job must close when supervisor is killed"
    );
}
#[test]
fn completed_child_does_not_timeout_when_polled_late() {
    let mut child =
        Process::start(probe("input"), b"hello".to_vec(), Duration::from_secs(1)).unwrap();
    std::thread::sleep(Duration::from_millis(1300));
    let output = child.poll().unwrap().as_ref().unwrap();
    assert_eq!(output.exit_code, Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("received"));
}
#[test]
fn deadline_fires_without_polling_and_blocked_stdin_is_reaped() {
    let mut child = Process::start(
        probe("sleep"),
        vec![0; PIPE_BYTES],
        Duration::from_millis(200),
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(400));
    let error = child.wait().as_ref().unwrap_err();
    assert_eq!(error.kind, FailureKind::Timeout);
    assert!(error.child_reaped);
}
#[test]
fn cancellation_is_prompt_and_reports_direct_child_cleanup() {
    let mut child = Process::start(probe("sleep"), vec![], Duration::from_secs(5)).unwrap();
    let start = Instant::now();
    let error = child.cancel().as_ref().unwrap_err();
    assert_eq!(error.kind, FailureKind::Cancelled);
    assert!(error.child_reaped);
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(
        child.wait().as_ref().unwrap_err().kind,
        FailureKind::Cancelled
    );
}
#[test]
fn both_pipes_are_bounded_while_checker_rejections_remain_observable() {
    for mode in ["stdout", "stderr"] {
        let mut child = Process::start(probe(mode), vec![], Duration::from_secs(5)).unwrap();
        let error = child.wait().as_ref().unwrap_err();
        assert_eq!(error.kind, FailureKind::OutputLimit);
        assert!(error.child_reaped);
    }
    let mut child = Process::start(probe("exit"), vec![], Duration::from_secs(5)).unwrap();
    let output = child.wait().as_ref().unwrap();
    assert_eq!(output.exit_code, Some(1));
    assert_eq!(output.stderr, b"error: rejected");
}
#[test]
fn invalid_bounds_fail_before_spawn() {
    for duration in [Duration::ZERO, Duration::from_secs(31)] {
        assert!(Process::start(probe("sleep"), vec![], duration).is_err());
    }
    assert!(
        Process::start(
            probe("sleep"),
            vec![0; PIPE_BYTES + 1],
            Duration::from_secs(1)
        )
        .is_err()
    );
}

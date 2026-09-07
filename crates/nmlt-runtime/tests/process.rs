use nmlt_runtime::process::{FailureKind, PIPE_BYTES, Process};
use std::io::{Read, Write};
use std::process::Command;
use std::time::{Duration, Instant};

fn probe(mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "process_probe", "--nocapture"])
        .env("NMLT_ASYNC_PROBE", mode);
    command
}
#[test]
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
        _ => panic!("unknown mode"),
    }
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

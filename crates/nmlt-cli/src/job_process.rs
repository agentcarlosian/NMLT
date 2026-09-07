//! Supervisor for the fixed, same-executable square worker. No shell or source
//! command selection. Pipe memory and elapsed time are bounded; this is not a
//! CPU/memory quota or a process-tree sandbox.
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const PIPE_BYTES: usize = 65_536;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum FailureKind {
    Spawn,
    Io,
    Timeout,
    OutputLimit,
    Exit,
    Protocol,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Failure {
    pub kind: FailureKind,
    /// No child was started, or try_wait confirmed that the direct child exited.
    pub child_reaped: bool,
}

enum Event {
    Input(bool),
    Output(Result<Vec<u8>, FailureKind>),
    Error(Result<Vec<u8>, FailureKind>),
}

fn read_pipe(mut pipe: impl Read) -> Result<Vec<u8>, FailureKind> {
    let mut bytes = Vec::new();
    pipe.by_ref()
        .take(PIPE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| FailureKind::Io)?;
    if bytes.len() > PIPE_BYTES {
        Err(FailureKind::OutputLimit)
    } else {
        Ok(bytes)
    }
}

pub(super) fn run(
    mut command: Command,
    input: Vec<u8>,
    timeout: Duration,
) -> Result<Vec<u8>, Failure> {
    if input.len() > PIPE_BYTES {
        return Err(Failure {
            kind: FailureKind::Io,
            child_reaped: true,
        });
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let start = Instant::now();
    let mut child = command.spawn().map_err(|_| Failure {
        kind: FailureKind::Spawn,
        child_reaped: true,
    })?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let (send, receive) = mpsc::channel();
    let output_send = send.clone();
    let error_send = send.clone();
    std::thread::spawn(move || {
        let ok = stdin.write_all(&input).is_ok();
        drop(stdin);
        let _ = send.send(Event::Input(ok));
    });
    std::thread::spawn(move || {
        let _ = output_send.send(Event::Output(read_pipe(stdout)));
    });
    std::thread::spawn(move || {
        let _ = error_send.send(Event::Error(read_pipe(stderr)));
    });
    let (mut written, mut output, mut errors, mut status) = (false, None, false, None);
    let failure = loop {
        if start.elapsed() >= timeout {
            break FailureKind::Timeout;
        }
        match child.try_wait() {
            Ok(Some(exit)) => {
                status = Some(exit);
                if !exit.success() {
                    break FailureKind::Exit;
                }
            }
            Ok(None) => {}
            Err(_) => break FailureKind::Io,
        }
        if status.is_some()
            && written
            && errors
            && let Some(output) = output
        {
            return Ok(output);
        }
        match receive.recv_timeout(Duration::from_millis(2)) {
            Ok(Event::Input(true)) => written = true,
            Ok(Event::Input(false)) => break FailureKind::Io,
            Ok(Event::Output(Ok(bytes))) => output = Some(bytes),
            Ok(Event::Error(Ok(bytes))) if bytes.is_empty() => errors = true,
            Ok(Event::Error(Ok(_))) => break FailureKind::Protocol,
            Ok(Event::Output(Err(kind)) | Event::Error(Err(kind))) => break kind,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // All pipe tasks finished; try_wait may still lag pipe closure.
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    };
    let mut reaped = status.is_some();
    if !reaped {
        let _ = child.kill();
        let cleanup = Instant::now();
        while cleanup.elapsed() < Duration::from_secs(1) {
            if matches!(child.try_wait(), Ok(Some(_))) {
                reaped = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    // Pipe tasks hold only bounded buffers and OS handles. Do not join a task
    // indefinitely if cleanup failed. A failure never becomes a domain Err.
    Err(Failure {
        kind: failure,
        child_reaped: reaped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn probe(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "job_process::tests::process_probe",
                "--nocapture",
            ])
            .env("NMLT_JOB_PROCESS_PROBE", mode);
        command
    }
    #[test]
    fn process_probe() {
        let Ok(mode) = std::env::var("NMLT_JOB_PROCESS_PROBE") else {
            return;
        };
        match mode.as_str() {
            "sleep" => std::thread::sleep(Duration::from_secs(5)),
            "flood" => {
                let _ = std::io::stdout().write_all(&vec![b'x'; PIPE_BYTES * 4]);
            }
            "errflood" => {
                let _ = std::io::stderr().write_all(&vec![b'x'; PIPE_BYTES * 4]);
            }
            "stderr" => eprintln!("unexpected diagnostic"),
            "exit" => std::process::exit(3),
            "ok" => {
                let mut input = Vec::new();
                std::io::stdin().read_to_end(&mut input).unwrap();
                assert_eq!(input, b"request");
                print!("worker-result");
            }
            _ => panic!("unknown probe"),
        }
    }
    #[test]
    fn pipe_input_and_output_finish_before_success() {
        let bytes = run(probe("ok"), b"request".to_vec(), Duration::from_secs(5)).unwrap();
        assert!(String::from_utf8(bytes).unwrap().contains("worker-result"));
    }
    #[test]
    fn timeout_reaps_a_child_that_does_not_read_stdin() {
        let start = Instant::now();
        let failure = run(
            probe("sleep"),
            vec![b'x'; PIPE_BYTES],
            Duration::from_millis(200),
        )
        .unwrap_err();
        assert_eq!(failure.kind, FailureKind::Timeout);
        assert!(failure.child_reaped);
        assert!(start.elapsed() < Duration::from_secs(3));
    }
    #[test]
    fn both_output_pipes_are_bounded_and_rejected_with_cleanup() {
        for mode in ["flood", "errflood", "stderr", "exit"] {
            let failure = run(probe(mode), vec![], Duration::from_secs(5)).unwrap_err();
            assert!(failure.child_reaped, "{mode}: {failure:?}");
            assert_ne!(failure.kind, FailureKind::Timeout, "{mode}");
        }
    }
    #[test]
    fn launch_and_input_failures_do_not_claim_a_started_child() {
        let failure = run(
            Command::new("nmlt-nonexistent-worker-7e0c96"),
            vec![],
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert_eq!(
            failure,
            Failure {
                kind: FailureKind::Spawn,
                child_reaped: true
            }
        );
        let failure = run(
            probe("sleep"),
            vec![b'x'; PIPE_BYTES + 1],
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert_eq!(
            failure,
            Failure {
                kind: FailureKind::Io,
                child_reaped: true
            }
        );
    }
}

//! Synchronous fixed-worker view over the shared contained process supervisor.
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::io::{Read, Write};
use std::process::Command;
use std::time::Duration;
#[cfg(test)]
use std::time::Instant;
#[cfg(test)]
const PIPE_BYTES: usize = nmlt_runtime::process::PIPE_BYTES;

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
    pub child_reaped: bool,
}

pub(super) fn run(command: Command, input: Vec<u8>, timeout: Duration) -> Result<Vec<u8>, Failure> {
    use nmlt_runtime::process::{FailureKind as Kind, Process};
    let convert = |error: nmlt_runtime::process::Failure| Failure {
        kind: match error.kind {
            Kind::Spawn => FailureKind::Spawn,
            Kind::Timeout => FailureKind::Timeout,
            Kind::OutputLimit => FailureKind::OutputLimit,
            Kind::Io | Kind::Cancelled => FailureKind::Io,
        },
        child_reaped: error.child_reaped,
    };
    let mut process = Process::start(command, input, timeout).map_err(convert)?;
    let output = process.wait().clone().map_err(convert)?;
    if output.exit_code != Some(0) {
        return Err(Failure {
            kind: FailureKind::Exit,
            child_reaped: true,
        });
    }
    if !output.stderr.is_empty() {
        return Err(Failure {
            kind: FailureKind::Protocol,
            child_reaped: true,
        });
    }
    Ok(output.stdout)
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

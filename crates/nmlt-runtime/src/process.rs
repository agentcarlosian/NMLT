//! Bounded asynchronous direct-child supervision for trusted local adapters.
//! No shell selection, process-tree containment, or OS memory/CPU quota.
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

pub const PIPE_BYTES: usize = 65_536;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Spawn,
    Io,
    Timeout,
    OutputLimit,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub kind: FailureKind,
    pub child_reaped: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
pub type Result = std::result::Result<Output, Failure>;

/// The supervisor owns the child. Dropping this handle requests termination;
/// callers that need evidence of cleanup must call cancel and inspect its result.
pub struct Process {
    cancel: Sender<()>,
    receive: Receiver<Result>,
    result: Option<Result>,
    deadline: Instant,
}
impl Process {
    /// Spawn only after the adapter's durable dispatch receipt. The timeout
    /// starts here and runs independently of polling or another job's wait.
    pub fn start(
        mut command: Command,
        input: Vec<u8>,
        timeout: Duration,
    ) -> std::result::Result<Self, Failure> {
        if input.len() > PIPE_BYTES || timeout.is_zero() || timeout > Duration::from_secs(30) {
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
            command.creation_flags(0x0800_0000);
        }
        let start = Instant::now();
        let child = command.spawn().map_err(|_| Failure {
            kind: FailureKind::Spawn,
            child_reaped: true,
        })?;
        let (cancel, cancelled) = mpsc::channel();
        let (send, receive) = mpsc::channel();
        let child = ChildGuard(child, None);
        // A failed thread spawn drops ChildGuard and terminates the child.
        std::thread::Builder::new()
            .name("nmlt-job-supervisor".into())
            .spawn(move || {
                let result = supervise(child, input, start, timeout, cancelled);
                let _ = send.send(result);
            })
            .map_err(|_| Failure {
                kind: FailureKind::Io,
                child_reaped: false,
            })?;
        Ok(Self {
            cancel,
            receive,
            result: None,
            deadline: start + timeout + Duration::from_secs(2),
        })
    }
    pub fn poll(&mut self) -> Option<&Result> {
        if self.result.is_none() {
            match self.receive.try_recv() {
                Ok(result) => self.result = Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.result = Some(Err(Failure {
                        kind: FailureKind::Io,
                        child_reaped: false,
                    }))
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        self.result.as_ref()
    }
    pub fn wait(&mut self) -> &Result {
        if self.poll().is_none() {
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            self.result = Some(self.receive.recv_timeout(remaining).unwrap_or_else(|_| {
                let _ = self.cancel.send(());
                Err(Failure {
                    kind: FailureKind::Io,
                    child_reaped: false,
                })
            }));
        }
        self.result.as_ref().expect("completed wait")
    }
    pub fn cancel(&mut self) -> &Result {
        let _ = self.cancel.send(());
        self.deadline = self.deadline.min(Instant::now() + Duration::from_secs(2));
        self.wait()
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.cancel.send(());
    }
}

struct ChildGuard(Child, Option<bool>);
impl ChildGuard {
    fn terminate(&mut self) -> bool {
        if let Some(reaped) = self.1 {
            return reaped;
        }
        if matches!(self.0.try_wait(), Ok(Some(_))) {
            self.1 = Some(true);
            return true;
        }
        let _ = self.0.kill();
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            if matches!(self.0.try_wait(), Ok(Some(_))) {
                self.1 = Some(true);
                return true;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        self.1 = Some(false);
        false
    }
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.terminate();
    }
}

enum Event {
    Input(bool),
    Stdout(std::result::Result<Vec<u8>, FailureKind>),
    Stderr(std::result::Result<Vec<u8>, FailureKind>),
}
fn read_pipe(pipe: impl Read) -> std::result::Result<Vec<u8>, FailureKind> {
    let mut bytes = Vec::new();
    pipe.take(PIPE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| FailureKind::Io)?;
    if bytes.len() > PIPE_BYTES {
        Err(FailureKind::OutputLimit)
    } else {
        Ok(bytes)
    }
}
fn supervise(
    mut child: ChildGuard,
    input: Vec<u8>,
    start: Instant,
    timeout: Duration,
    cancelled: Receiver<()>,
) -> Result {
    let mut stdin = child.0.stdin.take().expect("piped stdin");
    let stdout = child.0.stdout.take().expect("piped stdout");
    let stderr = child.0.stderr.take().expect("piped stderr");
    let (send, receive) = mpsc::channel();
    let out = send.clone();
    let err = send.clone();
    std::thread::spawn(move || {
        let ok = stdin.write_all(&input).is_ok();
        drop(stdin);
        let _ = send.send(Event::Input(ok));
    });
    std::thread::spawn(move || {
        let _ = out.send(Event::Stdout(read_pipe(stdout)));
    });
    std::thread::spawn(move || {
        let _ = err.send(Event::Stderr(read_pipe(stderr)));
    });
    let (mut written, mut output, mut errors, mut status) =
        (None, None::<Vec<u8>>, None::<Vec<u8>>, None);
    let kind = loop {
        if !matches!(cancelled.try_recv(), Err(mpsc::TryRecvError::Empty)) {
            break FailureKind::Cancelled;
        }
        if start.elapsed() >= timeout {
            break FailureKind::Timeout;
        }
        match child.0.try_wait() {
            Ok(Some(exit)) => status = Some(exit),
            Ok(None) => {}
            Err(_) => break FailureKind::Io,
        }
        if let (Some(written), Some(output), Some(errors), Some(status)) =
            (&written, &output, &errors, status)
        {
            if !written {
                break FailureKind::Io;
            }
            return Ok(Output {
                exit_code: status.code(),
                stdout: output.clone(),
                stderr: errors.clone(),
            });
        }
        match receive.recv_timeout(Duration::from_millis(2)) {
            Ok(Event::Input(ok)) => written = Some(ok),
            Ok(Event::Stdout(Ok(bytes))) => output = Some(bytes),
            Ok(Event::Stderr(Ok(bytes))) => errors = Some(bytes),
            Ok(Event::Stdout(Err(kind)) | Event::Stderr(Err(kind))) => break kind,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                std::thread::sleep(Duration::from_millis(2))
            }
        }
    };
    Err(Failure {
        kind,
        child_reaped: child.terminate(),
    })
}

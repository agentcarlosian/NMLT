//! Bounded process-tree supervision. Process isolation is not a filesystem sandbox.
use serde::{Deserialize, Serialize};
use std::io;
use std::pin::Pin;
use std::process::Command;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::io::AsyncWrite;
use tokio_util::sync::CancellationToken;

mod file_output;
mod profile;
use file_output::FileCapture;
pub use file_output::{
    FILE_BYTES, FILE_CONTRACT, FileOutput, FilePolicy, FileProcess, FileReceipt, PROJECT_FILE_BYTES,
};
use profile::Profile;
pub use profile::{PROJECT_CONTRACT, PROJECT_FILE_CONTRACT, PROJECT_WORKER_CONTRACT};

pub const PIPE_BYTES: usize = 65_536;
pub const CONTRACT: &str = "nmlt-contained-process-v1;processkit-3.3.4;raw-pipes-65536;tree-kill-before-pipe-drain;explicit-environment;no-filesystem-or-network-sandbox";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub contract: String,
    pub mechanism: String,
    pub memory_bytes: u64,
    pub memory_scope: String,
    pub max_processes: Option<u32>,
    pub parent_death: String,
    pub cpu_limit: String,
    pub max_file_bytes: Option<u64>,
    pub filesystem_sandbox: bool,
    pub network_sandbox: bool,
}

impl Policy {
    pub fn validate(&self) -> std::result::Result<(), crate::Error> {
        self.validate_profile(Profile::Ordinary, false)
    }
    /// A separately selected resource policy for bound project builds/jobs.
    /// Ordinary R2 sessions continue to require `validate` and cannot adopt it.
    pub fn validate_project(&self) -> std::result::Result<(), crate::Error> {
        self.validate_profile(Profile::Project, false)
    }
    pub fn validate_project_worker(&self) -> std::result::Result<(), crate::Error> {
        self.validate_profile(Profile::ProjectWorker, false)
    }
    fn validate_profile(
        &self,
        profile: Profile,
        file: bool,
    ) -> std::result::Result<(), crate::Error> {
        let windows = self.mechanism == "job_object";
        let known = ["job_object", "cgroup_v2", "process_group", "process_reaper"]
            .contains(&self.mechanism.as_str());
        if !known
            || self.contract != profile.contract(file)
            || self.filesystem_sandbox
            || self.network_sandbox
            || self.memory_bytes != profile.memory(windows)
            || self.memory_scope
                != if windows {
                    "whole_job_commit"
                } else {
                    "per_process_data_segment"
                }
            || self.max_processes != if windows { Some(16) } else { None }
            || self.cpu_limit
                != if windows {
                    profile.windows_cpu_limit().into()
                } else {
                    format!("{}_seconds_per_process", profile.cpu_seconds())
                }
            || self.max_file_bytes
                != if windows {
                    None
                } else {
                    Some(profile.file_bytes())
                }
            || if windows {
                self.parent_death != "whole_job"
            } else {
                !["direct_child", "not_guaranteed"].contains(&self.parent_death.as_str())
            }
        {
            return Err(crate::Error(
                "unsupported process containment policy".into(),
            ));
        }
        Ok(())
    }
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
pub type Result = std::result::Result<Output, Failure>;

pub struct Process {
    cancel: Sender<()>,
    receive: Receiver<Result>,
    result: Option<Result>,
    deadline: Instant,
    policy: Policy,
}

impl Process {
    /// The durable adapter dispatch must precede this call. Only explicitly set
    /// environment variables are forwarded. Job timeout continues without polls.
    pub fn start(
        command: Command,
        input: Vec<u8>,
        timeout: Duration,
    ) -> std::result::Result<Self, Failure> {
        Self::start_capture(command, input, timeout, None, Profile::Ordinary)
    }

    /// Explicit project policy: up to 30 minutes and 8 GiB, with the same
    /// bounded pipes, cancellation and process-tree cleanup mechanisms.
    pub fn start_project(
        command: Command,
        input: Vec<u8>,
        timeout: Duration,
    ) -> std::result::Result<Self, Failure> {
        Self::start_capture(command, input, timeout, None, Profile::Project)
    }
    /// Coordinator for a fixed worker that contains every proof stage. Windows
    /// nested CPU quotas multiply, so the coordinator adds no second rate cap.
    /// All other project limits and complete-tree cleanup remain in force.
    pub fn start_project_worker(
        command: Command,
        input: Vec<u8>,
        timeout: Duration,
    ) -> std::result::Result<Self, Failure> {
        Self::start_capture(command, input, timeout, None, Profile::ProjectWorker)
    }

    fn start_capture(
        command: Command,
        input: Vec<u8>,
        timeout: Duration,
        file: Option<FileCapture>,
        profile: Profile,
    ) -> std::result::Result<Self, Failure> {
        if input.len() > PIPE_BYTES || timeout.is_zero() || timeout > profile.timeout() {
            return Err(failure(FailureKind::Io, true));
        }
        let mut spec = processkit::Command::new(command.get_program())
            .args(command.get_args())
            .env_clear()
            .stdin(processkit::Stdin::from_bytes(input))
            .timeout(timeout)
            .timeout_grace(Duration::ZERO)
            .cancel_grace(Duration::ZERO)
            .output_buffer(
                processkit::OutputBufferPolicy::unbounded()
                    .with_max_bytes(PIPE_BYTES)
                    .with_overflow(processkit::OverflowMode::Error),
            );
        if let Some(directory) = command.get_current_dir() {
            spec = spec.current_dir(directory);
        }
        for (name, value) in command.get_envs() {
            spec = match value {
                Some(value) => spec.env(name, value),
                None => spec.env_remove(name),
            };
        }
        #[cfg(windows)]
        {
            spec = spec.create_no_window();
        }
        #[cfg(target_os = "linux")]
        {
            spec = spec.kill_on_parent_death();
        }
        #[cfg(unix)]
        {
            use processkit::RlimitResource as R;
            spec = spec
                .rlimit(R::Data, profile.memory(false), profile.memory(false))
                .rlimit(R::Cpu, profile.cpu_seconds(), profile.cpu_seconds())
                .rlimit(R::Core, 0, 0)
                .rlimit(R::FileSize, profile.file_bytes(), profile.file_bytes())
                .rlimit(R::NoFile, profile.open_files(), profile.open_files());
        }
        let (cancel, cancelled) = mpsc::channel();
        let (send, receive) = mpsc::channel();
        let (ready, started) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("nmlt-contained-supervisor".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(_) => {
                        let _ = ready.send(Err(failure(FailureKind::Io, true)));
                        return;
                    }
                };
                let result =
                    runtime.block_on(supervise(spec, timeout, cancelled, ready, file, profile));
                let _ = send.send(result);
            })
            .map_err(|_| failure(FailureKind::Io, true))?;
        let policy = match started.recv_timeout(Duration::from_secs(5)) {
            Ok(result) => result?,
            Err(_) => {
                let _ = cancel.send(());
                return Err(failure(FailureKind::Io, false));
            }
        };
        Ok(Self {
            cancel,
            receive,
            result: None,
            deadline: Instant::now() + timeout + Duration::from_secs(3),
            policy,
        })
    }
    pub fn policy(&self) -> &Policy {
        &self.policy
    }
    pub fn poll(&mut self) -> Option<&Result> {
        if self.result.is_none() {
            match self.receive.try_recv() {
                Ok(result) => self.result = Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.result = Some(Err(failure(FailureKind::Io, false)))
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        self.result.as_ref()
    }
    pub fn wait(&mut self) -> &Result {
        if self.poll().is_none() {
            self.result = Some(
                self.receive
                    .recv_timeout(self.deadline.saturating_duration_since(Instant::now()))
                    .unwrap_or_else(|_| {
                        let _ = self.cancel.send(());
                        Err(failure(FailureKind::Io, false))
                    }),
            );
        }
        self.result.as_ref().expect("completed wait")
    }
    pub fn cancel(&mut self) -> &Result {
        let _ = self.cancel.send(());
        self.deadline = self.deadline.min(Instant::now() + Duration::from_secs(3));
        self.wait()
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.cancel.send(());
    }
}
fn failure(kind: FailureKind, child_reaped: bool) -> Failure {
    Failure {
        kind,
        child_reaped,
        detail: None,
    }
}
fn described_failure(
    kind: FailureKind,
    child_reaped: bool,
    profile: Profile,
    detail: impl std::fmt::Display,
) -> Failure {
    let mut value = failure(kind, child_reaped);
    if profile != Profile::Ordinary {
        value.detail = Some(detail.to_string().chars().take(1000).collect());
    }
    value
}

#[derive(Clone)]
struct Capture {
    bytes: Arc<Mutex<Vec<u8>>>,
    overflow: Arc<AtomicBool>,
    cancel: CancellationToken,
}
impl AsyncWrite for Capture {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut bytes = self.bytes.lock().expect("capture lock");
        if bytes.len() + data.len() > PIPE_BYTES {
            self.overflow.store(true, Ordering::Release);
            self.cancel.cancel();
            return Poll::Ready(Err(io::Error::other("raw pipe bound exceeded")));
        }
        bytes.extend_from_slice(data);
        Poll::Ready(Ok(data.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

async fn supervise(
    spec: processkit::Command,
    timeout: Duration,
    cancelled: Receiver<()>,
    ready: mpsc::SyncSender<std::result::Result<Policy, Failure>>,
    file: Option<FileCapture>,
    profile: Profile,
) -> Result {
    let options = processkit::ProcessGroupOptions::default().shutdown_timeout(Duration::ZERO);
    #[cfg(windows)]
    let options = options.max_memory(profile.memory(true)).max_processes(16);
    #[cfg(windows)]
    let options = if profile == Profile::ProjectWorker {
        options
    } else {
        options.cpu_quota(1.0)
    };
    let group = match processkit::ProcessGroup::with_options(options) {
        Ok(group) => group,
        Err(cause) => {
            let error = described_failure(
                FailureKind::Io,
                true,
                profile,
                format!("creating process group: {cause}"),
            );
            let _ = ready.send(Err(error.clone()));
            return Err(error);
        }
    };
    let policy = Policy {
        contract: profile.contract(file.is_some()).into(),
        mechanism: group.mechanism().name().into(),
        memory_bytes: profile.memory(cfg!(windows)),
        memory_scope: if cfg!(windows) {
            "whole_job_commit"
        } else {
            "per_process_data_segment"
        }
        .into(),
        max_processes: if cfg!(windows) { Some(16) } else { None },
        parent_death: if cfg!(windows) {
            "whole_job"
        } else if cfg!(target_os = "linux") {
            "direct_child"
        } else {
            "not_guaranteed"
        }
        .into(),
        cpu_limit: if cfg!(windows) {
            profile.windows_cpu_limit().into()
        } else {
            format!("{}_seconds_per_process", profile.cpu_seconds())
        },
        max_file_bytes: if cfg!(windows) {
            None
        } else {
            Some(profile.file_bytes())
        },
        filesystem_sandbox: false,
        network_sandbox: false,
    };
    let token = file
        .as_ref()
        .map(|file| file.cancel.clone())
        .unwrap_or_default();
    let stderr = Capture {
        bytes: Arc::new(Mutex::new(vec![])),
        overflow: Arc::new(AtomicBool::new(false)),
        cancel: token.clone(),
    };
    let spec = spec.cancel_on(token.clone()).stderr_raw_tee(stderr.clone());
    let spec = if let Some(file) = &file {
        // The raw tee runs on the line pump. Drain it with a small line-assembly
        // bound while the file sink independently limits all raw stdout bytes.
        spec.stdout_raw_tee(file.clone())
            .output_buffer(processkit::OutputBufferPolicy::bounded(0).with_max_bytes(PIPE_BYTES))
    } else {
        spec
    };
    let running = match group.start(&spec).await {
        Ok(running) => running,
        Err(error) => {
            let known_not_started = matches!(error.kind(), processkit::ErrorKind::NotFound);
            let failure = described_failure(
                if known_not_started {
                    FailureKind::Spawn
                } else {
                    FailureKind::Io
                },
                known_not_started,
                profile,
                format!("starting contained process: {error}"),
            );
            let _ = ready.send(Err(failure.clone()));
            return Err(failure);
        }
    };
    let Some(root_pid) = running.pid() else {
        let _ = group.kill_all();
        let error = failure(FailureKind::Io, false);
        let _ = ready.send(Err(error.clone()));
        return Err(error);
    };
    let deadline = Instant::now() + timeout.saturating_sub(running.elapsed());
    let _ = ready.send(Ok(policy));
    struct Completed {
        code: Option<i32>,
        timed_out: bool,
        truncated: bool,
        stdout: Vec<u8>,
    }
    let file_mode = file.is_some();
    let output = async {
        if file_mode {
            running.drain().await.map(|outcome| Completed {
                code: outcome.code(),
                timed_out: outcome.timed_out(),
                truncated: false,
                stdout: vec![],
            })
        } else {
            running.output_bytes().await.map(|result| Completed {
                code: result.code(),
                timed_out: result.timed_out(),
                truncated: result.truncated(),
                stdout: result.into_stdout(),
            })
        }
    };
    tokio::pin!(output);
    let mut detached = false;
    let mut expired = false;
    let mut root_gone = None;
    let result = loop {
        tokio::select! {
            biased;
            result = &mut output => break result,
            _ = tokio::time::sleep(Duration::from_millis(10)) => {
                if !matches!(cancelled.try_recv(), Err(mpsc::TryRecvError::Empty)) { token.cancel(); }
                if !token.is_cancelled() && Instant::now() >= deadline {
                    expired = true;
                    token.cancel();
                }
                // A shared-group RunningProcess timeout/cancel reaches the root.
                // Kill the group here before awaiting inherited pipe EOF.
                if token.is_cancelled() { let _ = group.kill_all(); }
                if let Ok(members) = group.members() {
                    // POSIX fallback membership lists group leaders even after
                    // a leader exits while descendants retain its group. On
                    // Linux/macOS, enriched membership omits a vanished leader.
                    let root_present = if group.mechanism().name() == "process_group"
                        && cfg!(any(target_os = "linux", target_vendor = "apple"))
                    {
                        match group.members_info() {
                            Ok(info) => info.iter().any(|member| member.pid() == root_pid),
                            Err(_) => {
                                detached = true;
                                let _ = group.kill_all();
                                false
                            }
                        }
                    } else { members.contains(&root_pid) };
                    if !root_present && !members.is_empty() {
                        // A job can briefly retain a just-terminated member in
                        // its kernel list. Allow that cleanup to settle before
                        // classifying a persistent descendant as detached work.
                        let since = root_gone.get_or_insert_with(Instant::now);
                        if since.elapsed() >= Duration::from_millis(100) {
                            detached = true;
                            let _ = group.kill_all();
                        }
                    } else {
                        root_gone = None;
                    }
                }
            }
        }
    };
    if result.as_ref().is_ok_and(|r| !r.timed_out) {
        let settle = Instant::now() + Duration::from_millis(100);
        loop {
            match group.members() {
                Ok(members) if members.iter().all(|pid| *pid == root_pid) => break,
                _ if Instant::now() >= settle => {
                    detached = true;
                    break;
                }
                _ => tokio::time::sleep(Duration::from_millis(2)).await,
            }
        }
    }
    // Never leave descendants behind on root completion or failure.
    let killed = group.kill_all().is_ok();
    let until = Instant::now() + Duration::from_secs(1);
    let clean = loop {
        match group.members() {
            Ok(members) if members.is_empty() => break true,
            _ if Instant::now() >= until => break false,
            _ => tokio::time::sleep(Duration::from_millis(2)).await,
        }
    };
    if !clean {
        return Err(described_failure(
            FailureKind::Io,
            false,
            profile,
            "process group did not become empty after termination",
        ));
    }
    if stderr.overflow.load(Ordering::Acquire) {
        return Err(failure(FailureKind::OutputLimit, clean));
    }
    if let Some(kind) = file.as_ref().and_then(FileCapture::failure) {
        return Err(failure(kind, clean));
    }
    if expired {
        return Err(failure(FailureKind::Timeout, clean));
    }
    if detached {
        return Err(described_failure(
            FailureKind::Io,
            clean,
            profile,
            "descendants remained after the root process exited",
        ));
    }
    match result {
        Ok(result) => {
            if result.timed_out {
                return Err(failure(FailureKind::Timeout, clean));
            }
            if result.truncated {
                return Err(failure(FailureKind::OutputLimit, clean));
            }
            let exit_code = result.code;
            let stdout = result.stdout;
            let stderr = std::mem::take(&mut *stderr.bytes.lock().expect("capture lock"));
            if stdout.len() > PIPE_BYTES {
                return Err(failure(FailureKind::OutputLimit, clean));
            }
            Ok(Output {
                exit_code,
                stdout,
                stderr,
            })
        }
        Err(error) => {
            let kind = match error.reason() {
                processkit::ErrorReason::OutputTooLarge { .. } => FailureKind::OutputLimit,
                _ => match error.kind() {
                    processkit::ErrorKind::Timeout => FailureKind::Timeout,
                    processkit::ErrorKind::Cancelled => FailureKind::Cancelled,
                    _ => FailureKind::Io,
                },
            };
            Err(described_failure(
                kind,
                clean && (killed || !error.is_teardown()),
                profile,
                error,
            ))
        }
    }
}

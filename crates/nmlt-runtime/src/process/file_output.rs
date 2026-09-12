//! Bounded byte-exact file capture with the same process-tree supervisor.
use super::{Failure, FailureKind, Output, PIPE_BYTES, Policy, Process, Profile, failure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::AsyncWrite;
use tokio_util::sync::CancellationToken;

pub const FILE_BYTES: u64 = 16 * 1024 * 1024;
pub const PROJECT_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const FILE_CONTRACT: &str = "nmlt-contained-file-process-v1;processkit-3.3.4;raw-stdout-file-16777216;raw-stderr-65536;line-assembly-65536;tree-kill-before-pipe-drain;explicit-environment;no-filesystem-or-network-sandbox";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilePolicy {
    pub process: Policy,
    pub max_stdout_bytes: u64,
    pub max_stderr_bytes: usize,
}
impl FilePolicy {
    pub fn validate(&self) -> std::result::Result<(), crate::Error> {
        self.validate_profile(Profile::Ordinary)
    }
    pub fn validate_project(&self) -> std::result::Result<(), crate::Error> {
        self.validate_profile(Profile::Project)
    }
    fn validate_profile(&self, profile: Profile) -> std::result::Result<(), crate::Error> {
        self.process.validate_profile(profile, true)?;
        if self.max_stdout_bytes != profile.stdout_file_bytes()
            || self.max_stderr_bytes != PIPE_BYTES
        {
            return Err(crate::Error("unsupported file capture limits".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileReceipt {
    pub bytes: u64,
    pub sha256: String,
}
impl FileReceipt {
    pub fn validate(&self) -> std::result::Result<(), crate::Error> {
        self.validate_limit(FILE_BYTES)
    }
    pub fn validate_project(&self) -> std::result::Result<(), crate::Error> {
        self.validate_limit(PROJECT_FILE_BYTES)
    }
    fn validate_limit(&self, limit: u64) -> std::result::Result<(), crate::Error> {
        if self.bytes > limit
            || self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(crate::Error("invalid captured file receipt".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileOutput {
    /// Stdout is retained in the selected file, so this output's stdout is empty.
    pub output: Output,
    pub file: FileReceipt,
}

pub type FileResult = std::result::Result<FileOutput, Failure>;

pub struct FileProcess {
    process: Process,
    capture: FileCapture,
    policy: FilePolicy,
    result: Option<FileResult>,
}
impl FileProcess {
    /// Create a new output file and capture at most 16 MiB of raw stdout.
    /// Existing destinations are never overwritten. Interrupted/capture-failed
    /// runs can leave a bounded partial file, but do not return a receipt.
    /// A complete capture preserves the child's exit code; callers still decide
    /// whether that exit code constitutes command success.
    pub fn start(
        command: Command,
        input: Vec<u8>,
        timeout: Duration,
        path: &Path,
    ) -> std::result::Result<Self, Failure> {
        Self::start_profile(command, input, timeout, path, Profile::Ordinary)
    }

    /// Project-file policy: at most 64 MiB of stdout, with the explicitly
    /// selected project process bounds. Ordinary file capture remains 16 MiB.
    pub fn start_project(
        command: Command,
        input: Vec<u8>,
        timeout: Duration,
        path: &Path,
    ) -> std::result::Result<Self, Failure> {
        Self::start_profile(command, input, timeout, path, Profile::Project)
    }

    fn start_profile(
        command: Command,
        input: Vec<u8>,
        timeout: Duration,
        path: &Path,
        profile: Profile,
    ) -> std::result::Result<Self, Failure> {
        if input.len() > PIPE_BYTES || timeout.is_zero() || timeout > profile.timeout() {
            return Err(failure(FailureKind::Io, true));
        }
        let mut capture = FileCapture::new(path).map_err(|_| failure(FailureKind::Io, true))?;
        capture.limit = profile.stdout_file_bytes();
        let process =
            Process::start_capture(command, input, timeout, Some(capture.clone()), profile)?;
        let policy = FilePolicy {
            process: process.policy().clone(),
            max_stdout_bytes: profile.stdout_file_bytes(),
            max_stderr_bytes: PIPE_BYTES,
        };
        Ok(Self {
            process,
            capture,
            policy,
            result: None,
        })
    }

    pub fn policy(&self) -> &FilePolicy {
        &self.policy
    }

    fn finish(&mut self, result: super::Result) {
        self.result = Some(result.and_then(|output| {
            if !output.stdout.is_empty() {
                return Err(failure(FailureKind::Io, true));
            }
            self.capture
                .receipt()
                .map(|file| FileOutput { output, file })
        }));
    }

    pub fn poll(&mut self) -> Option<&FileResult> {
        if self.result.is_none()
            && let Some(result) = self.process.poll().cloned()
        {
            self.finish(result);
        }
        self.result.as_ref()
    }

    pub fn wait(&mut self) -> &FileResult {
        if self.result.is_none() {
            let result = self.process.wait().clone();
            self.finish(result);
        }
        self.result.as_ref().expect("completed file wait")
    }

    pub fn cancel(&mut self) -> &FileResult {
        self.process.cancel();
        self.wait()
    }
}

struct State {
    file: tokio::fs::File,
    sync: File,
    digest: Sha256,
    bytes: u64,
    flushed: bool,
    io_failed: bool,
    overflow: bool,
}

#[derive(Clone)]
pub(super) struct FileCapture {
    state: Arc<Mutex<State>>,
    limit: u64,
    pub(super) cancel: CancellationToken,
}
impl FileCapture {
    fn new(path: &Path) -> io::Result<Self> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let sync = options.open(path)?;
        let file = tokio::fs::File::from_std(sync.try_clone()?);
        Ok(Self {
            limit: FILE_BYTES,
            state: Arc::new(Mutex::new(State {
                file,
                sync,
                digest: Sha256::new(),
                bytes: 0,
                flushed: false,
                io_failed: false,
                overflow: false,
            })),
            cancel: CancellationToken::new(),
        })
    }

    pub(super) fn failure(&self) -> Option<FailureKind> {
        let state = self.state.lock().expect("file capture lock");
        if state.overflow {
            Some(FailureKind::OutputLimit)
        } else if state.io_failed {
            Some(FailureKind::Io)
        } else {
            None
        }
    }

    fn receipt(&self) -> std::result::Result<FileReceipt, Failure> {
        if let Some(kind) = self.failure() {
            return Err(failure(kind, true));
        }
        let state = self.state.lock().expect("file capture lock");
        // The raw tee can be silently disabled by processkit after a sink error.
        // Require our explicit error/EOF-flush evidence, not just child exit zero.
        if !state.flushed
            || state.sync.sync_all().is_err()
            || !state
                .sync
                .metadata()
                .is_ok_and(|m| m.is_file() && m.len() == state.bytes)
        {
            return Err(failure(FailureKind::Io, true));
        }
        Ok(FileReceipt {
            bytes: state.bytes,
            sha256: format!("{:x}", state.digest.clone().finalize()),
        })
    }
}

impl AsyncWrite for FileCapture {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut state = self.state.lock().expect("file capture lock");
        if state.bytes.saturating_add(bytes.len() as u64) > self.limit {
            state.overflow = true;
            self.cancel.cancel();
            return Poll::Ready(Err(io::Error::other("raw stdout file bound exceeded")));
        }
        match Pin::new(&mut state.file).poll_write(cx, bytes) {
            Poll::Ready(Ok(0)) if !bytes.is_empty() => {
                state.io_failed = true;
                self.cancel.cancel();
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "stdout file write made no progress",
                )))
            }
            Poll::Ready(Ok(n)) => {
                state.digest.update(&bytes[..n]);
                state.bytes += n as u64;
                state.flushed = false;
                Poll::Ready(Ok(n))
            }
            Poll::Ready(Err(error)) => {
                state.io_failed = true;
                self.cancel.cancel();
                Poll::Ready(Err(error))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut state = self.state.lock().expect("file capture lock");
        match Pin::new(&mut state.file).poll_flush(cx) {
            Poll::Ready(Ok(())) => {
                state.flushed = true;
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => {
                state.io_failed = true;
                self.cancel.cancel();
                Poll::Ready(Err(error))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    fn path(label: &str) -> std::path::PathBuf {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/file-capture-tests");
        std::fs::create_dir_all(&root).unwrap();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        root.join(format!("{label}-{}-{stamp}", std::process::id()))
    }

    #[tokio::test]
    async fn a_missing_flush_never_produces_a_receipt() {
        let mut capture = FileCapture::new(&path("flush")).unwrap();
        capture.write_all(b"captured").await.unwrap();
        assert_eq!(capture.receipt().unwrap_err().kind, FailureKind::Io);
        capture.flush().await.unwrap();
        let receipt = capture.receipt().unwrap();
        assert_eq!(receipt.bytes, 8);
        assert_eq!(receipt.sha256, crate::sha256(b"captured"));
    }

    #[tokio::test]
    async fn write_or_flush_errors_cancel_and_invalidate_the_capture() {
        let path = path("read-only");
        std::fs::write(&path, b"preserve").unwrap();
        let sync = File::open(&path).unwrap();
        // A real read-only descriptor gives a portable failed disk write,
        // including implementations that report it only during flush.
        let mut capture = FileCapture {
            limit: FILE_BYTES,
            state: Arc::new(Mutex::new(State {
                file: tokio::fs::File::from_std(sync.try_clone().unwrap()),
                sync,
                digest: Sha256::new(),
                bytes: 0,
                flushed: false,
                io_failed: false,
                overflow: false,
            })),
            cancel: CancellationToken::new(),
        };
        let wrote = capture.write_all(b"replace").await;
        let flushed = capture.flush().await;
        assert!(wrote.is_err() || flushed.is_err());
        assert!(capture.cancel.is_cancelled());
        assert_eq!(capture.failure(), Some(FailureKind::Io));
        assert_eq!(capture.receipt().unwrap_err().kind, FailureKind::Io);
        assert_eq!(std::fs::read(path).unwrap(), b"preserve");
    }
}

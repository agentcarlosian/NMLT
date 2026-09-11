use super::{CONTRACT, FILE_CONTRACT};
use std::time::Duration;

pub const PROJECT_CONTRACT: &str = "nmlt-contained-project-process-v1;processkit-3.3.4;raw-pipes-65536;deadline-at-most-1800s;memory-8589934592;tree-kill-before-pipe-drain;explicit-environment;no-filesystem-or-network-sandbox";
pub const PROJECT_FILE_CONTRACT: &str = "nmlt-contained-project-file-process-v1;processkit-3.3.4;raw-stdout-file-67108864;raw-stderr-65536;line-assembly-65536;deadline-at-most-1800s;memory-8589934592;tree-kill-before-pipe-drain;explicit-environment;no-filesystem-or-network-sandbox";
pub const PROJECT_WORKER_CONTRACT: &str = "nmlt-contained-project-worker-v1;processkit-3.3.4;raw-pipes-65536;deadline-at-most-1800s;memory-8589934592;windows-no-additional-cpu-rate-cap;proof-stages-use-contained-processes;tree-kill-before-pipe-drain;explicit-environment;no-filesystem-or-network-sandbox";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Profile {
    Ordinary,
    Project,
    ProjectWorker,
}

impl Profile {
    pub fn contract(self, file: bool) -> &'static str {
        match (self, file) {
            (Self::Ordinary, false) => CONTRACT,
            (Self::Ordinary, true) => FILE_CONTRACT,
            (Self::Project, false) => PROJECT_CONTRACT,
            (Self::Project, true) => PROJECT_FILE_CONTRACT,
            (Self::ProjectWorker, _) => PROJECT_WORKER_CONTRACT,
        }
    }

    pub fn timeout(self) -> Duration {
        Duration::from_secs(match self {
            Self::Ordinary => 30,
            Self::Project | Self::ProjectWorker => 1800,
        })
    }

    pub fn memory(self, windows: bool) -> u64 {
        match self {
            Self::Ordinary => (if windows { 1 } else { 2 }) * 1024 * 1024 * 1024,
            Self::Project | Self::ProjectWorker => 8 * 1024 * 1024 * 1024,
        }
    }

    pub fn cpu_seconds(self) -> u64 {
        match self {
            Self::Ordinary => 32,
            Self::Project | Self::ProjectWorker => 1802,
        }
    }

    pub fn file_bytes(self) -> u64 {
        match self {
            Self::Ordinary => 64 * 1024 * 1024,
            Self::Project | Self::ProjectWorker => 512 * 1024 * 1024,
        }
    }

    #[cfg(unix)]
    pub fn open_files(self) -> u64 {
        match self {
            Self::Ordinary => 256,
            Self::Project | Self::ProjectWorker => 512,
        }
    }

    pub fn stdout_file_bytes(self) -> u64 {
        match self {
            Self::Ordinary => super::FILE_BYTES,
            Self::Project | Self::ProjectWorker => super::PROJECT_FILE_BYTES,
        }
    }
    pub fn windows_cpu_limit(self) -> &'static str {
        if self == Self::ProjectWorker {
            "no_additional_cpu_rate_cap"
        } else {
            "one_core_whole_job"
        }
    }
}

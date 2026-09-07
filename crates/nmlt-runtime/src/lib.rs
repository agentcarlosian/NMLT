//! Executable-only local job control. No Lean or host-execution theorem.
//!
//! The pure lifecycle supports testing. Host adapters must receive dispatches
//! only through the locked journal, after their intent has been flushed.

#![forbid(unsafe_code)]

mod journal;
pub mod lean;
mod lifecycle;
pub mod process;
mod protocol;
pub mod session;
pub mod worker;

pub use journal::{Journal, replay_journal};
pub use lifecycle::*;
pub use protocol::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self(value.to_string())
    }
}

#[must_use]
pub fn sha256(bytes: &[u8]) -> String {
    nmlt_hir::sha256_bytes(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.:/".contains(&c))
}

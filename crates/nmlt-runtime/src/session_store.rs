//! Immutable per-transition evidence, durable before releasing a settlement.
use crate::{
    Command, Error,
    session::{Acknowledgement, Observation},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

const MAX_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum Evidence {
    Observation(Observation),
    Acknowledgement(Acknowledgement),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Stored {
    pub before: u32,
    pub command: Command,
    pub evidence: Evidence,
}
pub(crate) fn read<T: DeserializeOwned + Serialize>(path: &Path, limit: u64) -> Result<T, Error> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > limit {
        return Err(Error("evidence requires a bounded regular file".into()));
    }
    let mut bytes = vec![];
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error("evidence grew beyond bound".into()));
    }
    let result: T = serde_json::from_slice(&bytes)?;
    if serde_json::to_vec(&result)? != bytes {
        return Err(Error("evidence is not canonical".into()));
    }
    Ok(result)
}
pub(crate) fn write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    // The final name is published only after its complete contents are flushed.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error(e.to_string()))?
        .as_nanos();
    let temporary = path.with_extension(format!("{}-{stamp}.tmp", std::process::id()));
    let mut file = File::create_new(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    if path.exists() {
        return Err(Error("evidence artifact already exists".into()));
    }
    fs::rename(&temporary, path)?;
    // Directory fsync is supported on Unix. Windows durability is limited to
    // flushed file contents plus the local filesystem's rename guarantees.
    #[cfg(unix)]
    File::open(
        path.parent()
            .ok_or_else(|| Error("evidence parent missing".into()))?,
    )?
    .sync_all()?;
    Ok(())
}
pub(crate) fn save(directory: &Path, stored: &Stored) -> Result<(), Error> {
    let bytes = serde_json::to_vec(stored)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(Error("transition evidence exceeds bound".into()));
    }
    write(
        &directory.join(format!("evidence-{}.json", stored.before)),
        &bytes,
    )
}
pub(crate) fn load(directory: &Path, max_events: u32) -> Result<Vec<Stored>, Error> {
    let mut result = vec![];
    for before in 0..max_events {
        let path = directory.join(format!("evidence-{before}.json"));
        if path.exists() {
            let stored: Stored = read(&path, MAX_BYTES)?;
            if stored.before != before {
                return Err(Error("evidence sequence mismatch".into()));
            }
            result.push(stored);
        }
    }
    Ok(result)
}

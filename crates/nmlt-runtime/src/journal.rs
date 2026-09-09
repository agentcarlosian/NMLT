use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{Command, Error, Lifecycle, Phase, Receipt, RunSpec, sha256};

const SCHEMA: &str = "nmlt-job-journal-v1";
const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ENTRY_BYTES: usize = 65_536;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    schema: String,
    implementation_sha256: String,
    spec: RunSpec,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    sequence: u32,
    previous_sha256: String,
    command: Command,
    receipt: Receipt,
    state_sha256: String,
}

fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    Ok(serde_json::to_vec(value)?)
}

fn decode<T: Serialize + DeserializeOwned>(line: &[u8]) -> Result<T, Error> {
    if line.len() > MAX_ENTRY_BYTES {
        return Err(Error("journal entry too large".into()));
    }
    let value: T = serde_json::from_slice(line)?;
    if canonical(&value)? != line {
        return Err(Error(
            "journal entry is not canonical (unknown/duplicate fields or changed encoding)".into(),
        ));
    }
    Ok(value)
}

fn implementation() -> Result<String, Error> {
    crate::identity::file(&std::env::current_exe()?, 256 * 1024 * 1024).map(|(_, digest)| digest)
}

/// Check an immutable journal without locking, recovering, or issuing dispatch
/// authority. The returned lifecycle is evidence only, just like Lifecycle::step.
pub fn replay_journal(
    bytes: &[u8],
    expected: &RunSpec,
) -> Result<(Lifecycle, Vec<Command>), Error> {
    let (state, _, commands) = decode_log(bytes, expected)?;
    Ok((state, commands))
}

fn decode_log(
    bytes: &[u8],
    expected: &RunSpec,
) -> Result<(Lifecycle, String, Vec<Command>), Error> {
    if bytes.len() as u64 > MAX_BYTES || bytes.last() != Some(&b'\n') {
        return Err(Error(
            "oversized or incomplete journal; reconciliation required".into(),
        ));
    }
    let mut lines = bytes[..bytes.len() - 1].split(|b| *b == b'\n');
    let first = lines
        .next()
        .ok_or_else(|| Error("missing journal header".into()))?;
    let header: Header = decode(first)?;
    if header.schema != SCHEMA
        || header.spec != *expected
        || header.implementation_sha256 != implementation()?
    {
        return Err(Error(
            "journal schema, run context, or executable identity mismatch".into(),
        ));
    }
    let mut lifecycle = Lifecycle::new(header.spec)?;
    let mut tail = sha256(first);
    let mut commands = Vec::new();
    for line in lines {
        let entry: Entry = decode(line)?;
        if entry.sequence != lifecycle.event_count() + 1 || entry.previous_sha256 != tail {
            return Err(Error("journal sequence or hash-chain mismatch".into()));
        }
        let (next, receipt) = lifecycle.step(&entry.command)?;
        if receipt != entry.receipt || sha256(&canonical(&next)?) != entry.state_sha256 {
            return Err(Error(
                "journal receipt or reconstructed state mismatch".into(),
            ));
        }
        commands.push(entry.command);
        lifecycle = next;
        tail = sha256(line);
    }
    Ok((lifecycle, tail, commands))
}

// Some mounted filesystems (including the tested WSL/Windows mount) do not
// coordinate flock across hardlink aliases despite reporting the same inode.
// Reject that configuration on Unix instead of trusting an ineffective lock.
fn check_links(_file: &File) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if _file.metadata()?.nlink() != 1 {
            return Err(std::io::Error::other(
                "Unix job journals require exactly one filesystem link",
            ));
        }
    }
    Ok(())
}

/// Single writer over one cooperating local filesystem. Never Clone; never
/// replace/unlink the locked file. A copied or rolled-back journal is not fenced.
pub struct Journal {
    file: File,
    lifecycle: Lifecycle,
    tail_sha256: String,
    length: u64,
    poisoned: bool,
}

impl Journal {
    pub fn create(path: &Path, spec: RunSpec) -> Result<Self, Error> {
        let lifecycle = Lifecycle::new(spec.clone())?;
        let header = Header {
            schema: SCHEMA.into(),
            implementation_sha256: implementation()?,
            spec,
        };
        let bytes = canonical(&header)?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)?;
        file.try_lock()
            .map_err(|e| Error(format!("journal lock unavailable: {e}")))?;
        check_links(&file)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(Self {
            file,
            lifecycle,
            tail_sha256: sha256(&bytes),
            length: bytes.len() as u64 + 1,
            poisoned: false,
        })
    }

    /// Validate the complete log under an exact caller-supplied run context,
    /// then durably classify unfinished work before exposing live control.
    pub fn open(path: &Path, expected: &RunSpec) -> Result<Self, Error> {
        let mut result = Self::open_existing(path, expected)?;
        if result
            .lifecycle
            .attempts()
            .iter()
            .any(|a| !matches!(a.phase, Phase::Collected { .. }))
        {
            result.apply(Command::Recover)?;
        }
        Ok(result)
    }

    /// Restore the locked log before recovery so a saved observation whose
    /// settlement append was interrupted can first be completed and validated.
    pub(crate) fn open_existing(path: &Path, expected: &RunSpec) -> Result<Self, Error> {
        Self::open_inner(path, expected, None).map(|(journal, _)| journal)
    }

    /// Explicitly quarantine only a final incomplete append. Complete entries,
    /// even malformed ones, are never discarded. Keep this lock while repairing
    /// any source decision log that shares the same session.
    pub fn open_repair(
        path: &Path,
        expected: &RunSpec,
        reason: &str,
    ) -> Result<(Self, Option<std::path::PathBuf>), Error> {
        if reason.trim().is_empty() || reason.len() > 1024 {
            return Err(Error(
                "tail repair requires an explicit 1..1024-byte reason".into(),
            ));
        }
        Self::open_inner(path, expected, Some(reason))
    }
    fn open_inner(
        path: &Path,
        expected: &RunSpec,
        repair: Option<&str>,
    ) -> Result<(Self, Option<std::path::PathBuf>), Error> {
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        check_links(&file)?;
        file.try_lock()
            .map_err(|e| Error(format!("journal lock unavailable: {e}")))?;
        check_links(&file)?;
        if file.metadata()?.len() > MAX_BYTES {
            return Err(Error("journal exceeds byte bound".into()));
        }
        let mut bytes = Vec::new();
        (&mut file).take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
        let mut quarantine = None;
        if bytes.last() != Some(&b'\n')
            && let Some(reason) = repair
        {
            let boundary = bytes.iter().rposition(|b| *b == b'\n').ok_or_else(|| {
                Error("no complete journal header; cannot repair this file".into())
            })? + 1;
            if bytes.len() - boundary > MAX_ENTRY_BYTES {
                return Err(Error("incomplete journal tail exceeds entry bound".into()));
            }
            decode_log(&bytes[..boundary], expected)?;
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| Error(e.to_string()))?
                .as_nanos();
            let destination = path.with_extension(format!("incomplete-{stamp}.json"));
            let report = serde_json::json!({"schema":"nmlt-incomplete-tail-v1","reason":reason,"original_sha256":sha256(&bytes),"prefix_bytes":boundary,"prefix_sha256":sha256(&bytes[..boundary]),"discarded_bytes":&bytes[boundary..]});
            crate::session_store::write(&destination, &serde_json::to_vec(&report)?)?;
            file.set_len(boundary as u64)?;
            file.sync_all()?;
            bytes.truncate(boundary);
            quarantine = Some(destination);
        }
        let (lifecycle, tail_sha256, _) = decode_log(&bytes, expected)?;
        let result = Self {
            file,
            lifecycle,
            tail_sha256,
            length: bytes.len() as u64,
            poisoned: false,
        };
        Ok((result, quarantine))
    }

    #[must_use]
    pub fn state(&self) -> &Lifecycle {
        &self.lifecycle
    }

    /// Capture the exact durable bytes while retaining the writer lock. Never
    /// use a pathname re-open, which could capture a replaced file instead.
    pub fn snapshot(&mut self) -> Result<String, Error> {
        if self.poisoned {
            return Err(Error("journal handle poisoned by an I/O failure".into()));
        }
        check_links(&self.file)?;
        self.file.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        (&mut self.file)
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        let (state, tail, _) = decode_log(&bytes, self.lifecycle.spec())?;
        if state != self.lifecycle || tail != self.tail_sha256 || bytes.len() as u64 != self.length
        {
            return Err(Error("journal changed outside its owner".into()));
        }
        String::from_utf8(bytes).map_err(|e| Error(e.to_string()))
    }

    /// Dispatch receipts are released only after write_all + sync_all succeed.
    /// Any uncertain I/O failure poisons this handle until a fresh validated open.
    pub fn apply(&mut self, command: Command) -> Result<Receipt, Error> {
        self.commit_with(command, |file, bytes| {
            file.write_all(bytes)?;
            file.sync_all()
        })
    }

    #[cfg(test)]
    pub(crate) fn inject_failed_append(&mut self, command: Command) -> Result<Receipt, Error> {
        self.commit_with(command, |file, bytes| {
            file.write_all(&bytes[..bytes.len() / 2])?;
            Err(std::io::Error::other("injected partial journal append"))
        })
    }

    fn commit_with(
        &mut self,
        command: Command,
        persist: impl FnOnce(&mut File, &[u8]) -> std::io::Result<()>,
    ) -> Result<Receipt, Error> {
        if self.poisoned {
            return Err(Error("journal handle poisoned by an I/O failure".into()));
        }
        let (next, receipt) = self.lifecycle.step(&command)?;
        let entry = Entry {
            sequence: next.event_count(),
            previous_sha256: self.tail_sha256.clone(),
            command,
            receipt: receipt.clone(),
            state_sha256: sha256(&canonical(&next)?),
        };
        let mut bytes = canonical(&entry)?;
        if bytes.len() > MAX_ENTRY_BYTES || self.length + bytes.len() as u64 + 1 > MAX_BYTES {
            return Err(Error("journal entry or byte limit exhausted".into()));
        }
        let digest = sha256(&bytes);
        bytes.push(b'\n');
        let persisted = (|| {
            check_links(&self.file)?;
            if self.file.metadata()?.len() != self.length {
                return Err(std::io::Error::other(
                    "journal length changed outside its owner",
                ));
            }
            self.file.seek(SeekFrom::End(0))?;
            persist(&mut self.file, &bytes)
        })();
        if let Err(error) = persisted {
            self.poisoned = true;
            return Err(error.into());
        }
        self.length += bytes.len() as u64;
        self.tail_sha256 = digest;
        self.lifecycle = next;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, Request, Value, worker};
    use std::fs;

    fn fixture() -> (std::path::PathBuf, RunSpec) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/job-journal-faults");
        fs::create_dir_all(&dir).unwrap();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let spec = RunSpec {
            run_id: "fault-test".into(),
            context_sha256: sha256(b"context"),
            limits: Limits {
                slots: 1,
                generations_per_slot: 2,
                max_attempts: 2,
                max_events: 30,
                work_budget: 3,
            },
        };
        (
            dir.join(format!("{}-{unique}.jsonl", std::process::id())),
            spec,
        )
    }

    #[test]
    fn failed_writes_release_no_dispatch_and_poison_the_handle() {
        for full in [false, true] {
            let (path, spec) = fixture();
            let mut journal = Journal::create(&path, spec.clone()).unwrap();
            let Receipt::Control { control } = journal
                .apply(Command::Reserve {
                    task: "task".into(),
                    owner: "worker".into(),
                    request: Request {
                        adapter: worker::adapter(),
                        context_sha256: spec.context_sha256.clone(),
                        input: Value::Int(3),
                        reserved_work: 1,
                    },
                })
                .unwrap()
            else {
                panic!()
            };
            let before = journal.state().clone();
            let command = Command::Dispatch { control };
            let error = journal
                .commit_with(command.clone(), |file, bytes| {
                    file.write_all(if full {
                        bytes
                    } else {
                        &bytes[..bytes.len() / 2]
                    })?;
                    Err(std::io::Error::other("injected write/sync failure"))
                })
                .unwrap_err();
            assert!(error.0.contains("injected"));
            assert_eq!(journal.state(), &before);
            assert!(journal.apply(command).unwrap_err().0.contains("poisoned"));
            drop(journal);
            if full {
                let recovered = Journal::open(&path, &spec).unwrap();
                assert!(matches!(
                    recovered.state().attempts()[0].phase,
                    Phase::Uncertain { .. }
                ));
                assert_eq!(recovered.state().accounting().charged_work, 1);
            } else {
                assert!(Journal::open(&path, &spec).is_err());
            }
        }
    }
}

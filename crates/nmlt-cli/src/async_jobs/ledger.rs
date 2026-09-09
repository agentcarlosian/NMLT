//! Flush source intents before effects and replies before source decisions.
use super::{Call, Event, JobError, Location, Reply};
use nmlt_runtime::sha256;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

pub(super) const LIMIT: u64 = 16 * 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Item {
    Intent {
        at: Location,
        call: Call,
        before: u32,
    },
    Reply {
        result: Result<Reply, JobError>,
        through: u32,
    },
    Retry,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    schema: String,
    context_sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    sequence: usize,
    previous_sha256: String,
    item: Item,
}

#[derive(Clone, Debug)]
pub(super) struct Intent {
    pub at: Location,
    pub call: Call,
    pub before: u32,
}
#[derive(Clone)]
pub(super) struct Decoded {
    pub events: Vec<Event>,
    pub pending: Option<Intent>,
    pub replies: Vec<Event>,
    pub items: Vec<Item>,
    tail: String,
    latest: u32,
}
impl Decoded {
    fn check(&self, item: &Item) -> Result<(), String> {
        match item {
            Item::Intent { before, .. } if self.pending.is_some() || self.latest > *before => {
                Err("overlapping source operation intents".into())
            }
            Item::Reply { through, .. } => {
                let intent = self.pending.as_ref().ok_or("source reply without intent")?;
                if *through < intent.before || *through < self.latest {
                    Err("source reply precedes intent or earlier reply".into())
                } else {
                    Ok(())
                }
            }
            Item::Retry => {
                let event = self
                    .events
                    .last()
                    .ok_or("retry without suspended source operation")?;
                if self.pending.is_some()
                    || event.result != Err(JobError::HostFailure)
                    || !matches!(event.call, Call::Control { .. })
                {
                    Err("only a suspended control failure can resume".into())
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
    fn apply(&mut self, item: &Item) -> Result<(), String> {
        self.check(item)?;
        match item {
            Item::Intent { at, call, before } => {
                if self.pending.is_some() || self.latest > *before {
                    return Err("overlapping source operation intents".into());
                }
                self.pending = Some(Intent {
                    at: *at,
                    call: call.clone(),
                    before: *before,
                });
            }
            Item::Reply { result, through } => {
                let intent = self.pending.take().ok_or("source reply without intent")?;
                if *through < intent.before || *through < self.latest {
                    return Err("source reply precedes intent or earlier reply".into());
                }
                self.latest = *through;
                let event = Event {
                    at: intent.at,
                    call: intent.call,
                    before: intent.before,
                    result: result.clone(),
                    through: *through,
                };
                self.replies.push(event.clone());
                self.events.push(event);
            }
            Item::Retry => {
                if self.pending.is_some() {
                    return Err("retry with pending source operation".into());
                }
                let event = self
                    .events
                    .pop()
                    .ok_or("retry without suspended source operation")?;
                if event.result != Err(JobError::HostFailure)
                    || !matches!(event.call, Call::Control { .. })
                {
                    return Err("only a suspended control failure can resume".into());
                }
                self.pending = Some(Intent {
                    at: event.at,
                    call: event.call,
                    before: event.before,
                });
            }
        }
        self.items.push(item.clone());
        Ok(())
    }
}

pub(super) fn decode(bytes: &[u8], context: &str) -> Result<Decoded, String> {
    if bytes.len() as u64 > LIMIT || bytes.last() != Some(&b'\n') {
        return Err("incomplete or oversized source journal; no source decision can be inferred from its tail".into());
    }
    let mut lines = bytes[..bytes.len() - 1].split(|b| *b == b'\n');
    let header = lines.next().ok_or("missing source journal header")?;
    let value: Header = serde_json::from_slice(header).map_err(|e| e.to_string())?;
    if value.schema != "nmlt-source-journal-v1"
        || value.context_sha256 != context
        || serde_json::to_vec(&value).map_err(|e| e.to_string())? != header
    {
        return Err("source journal context or canonical header mismatch".into());
    }
    let mut result = Decoded {
        events: vec![],
        pending: None,
        replies: vec![],
        items: vec![],
        tail: sha256(header),
        latest: 0,
    };
    for line in lines {
        let row: Row = serde_json::from_slice(line).map_err(|e| e.to_string())?;
        if row.sequence != result.items.len()
            || row.previous_sha256 != result.tail
            || serde_json::to_vec(&row).map_err(|e| e.to_string())? != line
        {
            return Err(
                "source journal sequence, hash chain or canonical encoding mismatch".into(),
            );
        }
        result.apply(&row.item)?;
        result.tail = sha256(line);
    }
    Ok(result)
}

pub(super) struct Ledger {
    file: File,
    pub state: Decoded,
    bytes: Vec<u8>,
}
impl Ledger {
    /// Requires the runtime journal's exclusive lock. Preserve the incomplete
    /// suffix and its hashes before trimming; reject corrupt complete rows.
    pub fn repair(
        path: &Path,
        context: &str,
        reason: &str,
    ) -> Result<Option<std::path::PathBuf>, String> {
        let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > LIMIT {
            return Err("source journal must be a bounded regular file".into());
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        let mut bytes = vec![];
        (&mut file)
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > LIMIT {
            return Err("source journal grew beyond its bound".into());
        }
        if bytes.last() == Some(&b'\n') {
            decode(&bytes, context)?;
            return Ok(None);
        }
        let boundary = bytes
            .iter()
            .rposition(|b| *b == b'\n')
            .ok_or("no complete source journal header; cannot repair this file")?
            + 1;
        if bytes.len() - boundary > 65_536 {
            return Err("incomplete source row exceeds bound".into());
        }
        decode(&bytes[..boundary], context)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let destination = path.with_extension(format!("incomplete-{stamp}.json"));
        let report = serde_json::json!({"schema":"nmlt-incomplete-tail-v1","reason":reason,"original_sha256":sha256(&bytes),"prefix_bytes":boundary,"prefix_sha256":sha256(&bytes[..boundary]),"discarded_bytes":&bytes[boundary..]});
        let mut archive = File::create_new(&destination).map_err(|e| e.to_string())?;
        archive
            .write_all(&serde_json::to_vec(&report).map_err(|e| e.to_string())?)
            .and_then(|()| archive.sync_all())
            .map_err(|e| e.to_string())?;
        file.set_len(boundary as u64)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(Some(destination))
    }
    pub fn create(path: &Path, context: &str) -> Result<Self, String> {
        let mut bytes = serde_json::to_vec(&Header {
            schema: "nmlt-source-journal-v1".into(),
            context_sha256: context.into(),
        })
        .map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        let mut file = File::create_new(path).map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        let state = decode(&bytes, context)?;
        Ok(Self { file, state, bytes })
    }
    /// The Session journal lock must already be held throughout this lifetime.
    pub fn open(path: &Path, context: &str) -> Result<Self, String> {
        let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > LIMIT {
            return Err("source journal must be a bounded regular file".into());
        }
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        let mut bytes = vec![];
        (&mut file)
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        let state = decode(&bytes, context)?;
        Ok(Self { file, state, bytes })
    }
    pub fn append(&mut self, item: Item) -> Result<(), String> {
        self.state.check(&item)?;
        let mut encoded = serde_json::to_vec(&Row {
            sequence: self.state.items.len(),
            previous_sha256: self.state.tail.clone(),
            item: item.clone(),
        })
        .map_err(|e| e.to_string())?;
        if (self.bytes.len() + encoded.len() + 1) as u64 > LIMIT {
            return Err("source journal byte budget exhausted".into());
        }
        let tail = sha256(&encoded);
        encoded.push(b'\n');
        self.file
            .write_all(&encoded)
            .and_then(|()| self.file.sync_all())
            .map_err(|e| e.to_string())?;
        self.state.apply(&item)?;
        self.state.tail = tail;
        self.bytes.extend(encoded);
        Ok(())
    }
    pub fn text(&self) -> String {
        String::from_utf8(self.bytes.clone()).expect("JSON UTF-8")
    }
}

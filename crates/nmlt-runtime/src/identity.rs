//! Bounded, relocatable file identities for local tool/dependency locks.
use crate::Error;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileIdentity {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub link_target: Option<String>,
}

pub fn file(path: &Path, max_bytes: u64) -> Result<(u64, String), Error> {
    if !fs::metadata(path)?.is_file() {
        return Err(Error("identity requires a regular file".into()));
    }
    let mut input = fs::File::open(path)?;
    let metadata = input.metadata()?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return Err(Error("identity requires a bounded regular file".into()));
    }
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65_536];
    let mut bytes = 0u64;
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > max_bytes {
            return Err(Error("file grew beyond identity bound".into()));
        }
        digest.update(&buffer[..count]);
    }
    if bytes != metadata.len() {
        return Err(Error("file changed size during identity capture".into()));
    }
    Ok((bytes, format!("{:x}", digest.finalize())))
}

/// Capture every file in the selected installation subdirectories. Names and
/// link targets are relative to root; directory links cannot introduce cycles.
pub fn tree(root: &Path, directories: &[&str]) -> Result<Vec<FileIdentity>, Error> {
    let root = root.canonicalize()?;
    let mut files = vec![];
    let mut names = BTreeSet::new();
    let mut total = 0u64;
    let mut nodes = 0usize;
    for directory in directories {
        if directory.is_empty()
            || !directory
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(Error("invalid dependency root directory".into()));
        }
        visit(
            &root,
            &root.join(directory),
            0,
            &mut files,
            &mut names,
            &mut total,
            &mut nodes,
        )?;
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn visit(
    root: &Path,
    path: &Path,
    depth: usize,
    files: &mut Vec<FileIdentity>,
    names: &mut BTreeSet<String>,
    total: &mut u64,
    nodes: &mut usize,
) -> Result<(), Error> {
    *nodes += 1;
    if depth > 32 || *nodes > 65_536 {
        return Err(Error("dependency tree exceeds depth/entry bound".into()));
    }
    let metadata = fs::symlink_metadata(path)?;
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root) {
        return Err(Error("dependency escapes installation root".into()));
    }
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        let mut entries = fs::read_dir(path)?
            .take(65_537)
            .collect::<Result<Vec<_>, _>>()?;
        if entries.len() + *nodes > 65_536 {
            return Err(Error("dependency tree exceeds entry bound".into()));
        }
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            visit(root, &entry.path(), depth + 1, files, names, total, nodes)?;
        }
        return Ok(());
    }
    if !fs::metadata(&canonical)?.is_file() {
        return Err(Error(
            "unsupported dependency file or directory link".into(),
        ));
    }
    let relative = path
        .strip_prefix(root)
        .map_err(|e| Error(e.to_string()))?
        .to_str()
        .ok_or_else(|| Error("dependency path is not UTF-8".into()))?
        .replace('\\', "/");
    if relative.len() > 4096 {
        return Err(Error("dependency path exceeds 4096 bytes".into()));
    }
    if !names.insert(relative.to_lowercase()) {
        return Err(Error(
            "duplicate or case-conflicting dependency path".into(),
        ));
    }
    let target = if metadata.file_type().is_symlink() {
        let raw = fs::read_link(path)?;
        Some(if raw.is_absolute() {
            format!(
                "@root/{}",
                canonical
                    .strip_prefix(root)
                    .map_err(|e| Error(e.to_string()))?
                    .to_str()
                    .ok_or_else(|| Error("dependency link target is not UTF-8".into()))?
                    .replace('\\', "/")
            )
        } else {
            raw.to_str()
                .ok_or_else(|| Error("dependency link is not UTF-8".into()))?
                .replace('\\', "/")
        })
    } else {
        None
    };
    let (bytes, sha256) = file(path, 1024 * 1024 * 1024)?;
    *total += bytes;
    if *total > 16 * 1024 * 1024 * 1024 {
        return Err(Error("dependency tree exceeds 16 GiB".into()));
    }
    files.push(FileIdentity {
        path: relative,
        bytes,
        sha256,
        link_target: target,
    });
    Ok(())
}

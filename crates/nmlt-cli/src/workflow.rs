//! Explicit pure workflow command route. Existing finite behavior mode remains separate.
use super::runtime::{digest, implementation_digest};
use nmlt_workflow::{Execution, Inputs, Program, SourceIdentity, compile_package, execute};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

const SCHEMA: &str = "nmlt-pure-run-v3";
const PROFILE: &str = "pure-workflow-v3";
const USAGE: &str = "usage: nmlt run <source.nmlt> --entry <name> --max-steps <1..100000> --emit-run <new.json> [--arg name=value]";
const MAX_RECORD_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: String,
    profile: String,
    assurance: String,
    implementation_sha256: String,
    source_path: String,
    source_sha256: String,
    sources: Vec<SourceIdentity>,
    program_sha256: String,
    entry: String,
    inputs: Inputs,
    max_steps: u32,
    execution: Execution,
}

pub(super) fn bounded_read(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("could not read '{}': {e}", path.display()))?;
    let mut bytes = vec![];
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err(format!("'{}' exceeds its {max}-byte limit", path.display()));
    }
    Ok(bytes)
}
pub(super) fn load(path: &Path, logical_entry: Option<&str>) -> Result<Program, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("workflow sources must be regular files, not symbolic links".into());
    }
    let physical = path.canonicalize().map_err(|e| e.to_string())?;
    let root = physical
        .parent()
        .ok_or("entry requires a parent directory")?;
    let physical_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("entry filename must be UTF-8")?;
    let entry = logical_entry.unwrap_or(physical_name);
    let mut loaded = std::collections::BTreeMap::new();
    compile_package(entry, |name| {
        let target = if name == entry {
            physical.clone()
        } else {
            root.join(name)
        };
        let exact_name = if name == entry { physical_name } else { name };
        let mut exact = false;
        for item in std::fs::read_dir(root).map_err(|e| e.to_string())? {
            if item.map_err(|e| e.to_string())?.file_name() == std::ffi::OsStr::new(exact_name) {
                exact = true;
                break;
            }
        }
        if !exact {
            return Err(format!("missing exact-case source `{exact_name}`"));
        }
        let metadata = std::fs::symlink_metadata(&target).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("workflow sources must be regular files, not symbolic links".into());
        }
        let canonical = target.canonicalize().map_err(|e| e.to_string())?;
        if canonical.parent() != Some(root) {
            return Err("source resolves outside package directory".into());
        }
        let bytes = bounded_read(&canonical, nmlt_workflow::MAX_SOURCE_BYTES as u64)?;
        let source = String::from_utf8(bytes).map_err(|_| "workflow source must be UTF-8")?;
        loaded.insert(name.to_owned(), source.clone());
        Ok(source)
    })
    .map_err(|e| {
        let rendered = loaded
            .get(&e.path)
            .map(|source| {
                nmlt_core::render_diagnostic_snapshot(source, std::slice::from_ref(&e.diagnostic))
            })
            .unwrap_or_else(|| e.diagnostic.message.clone());
        format!("{}: {rendered}", root.join(&e.path).display())
    })
}
pub(super) fn typecheck(path: &Path) -> Result<(), String> {
    let program = load(path, None)?;
    println!(
        "{}",
        serde_json::json!({ "schema": "nmlt-workflow-typecheck-v4", "profile": "workflow-v4", "assurance": "none", "program_sha256": program.identity(), "records": program.records(), "sources": program.sources(),
        "entries": program.entries().map(|(name, parameters, result)| serde_json::json!({"name":name, "parameters": parameters, "result":result, "requires_jobs":program.requires_jobs(name).expect("entry")})).collect::<Vec<_>>() })
    );
    Ok(())
}
fn record(
    program: &Program,
    source_path: String,
    entry: String,
    inputs: Inputs,
    max_steps: u32,
) -> Result<Record, String> {
    let execution = execute(program, &entry, &inputs, max_steps)?;
    Ok(Record {
        schema: SCHEMA.into(),
        profile: PROFILE.into(),
        assurance: "none".into(),
        implementation_sha256: implementation_digest()?,
        source_path,
        source_sha256: program.sources()[0].source_sha256.clone(),
        sources: program.sources().to_vec(),
        program_sha256: program.identity(),
        entry,
        inputs,
        max_steps,
        execution,
    })
}
pub(super) fn run(args: &[OsString]) -> Result<(), String> {
    let Some(source) = args.first().map(PathBuf::from) else {
        return Err(USAGE.into());
    };
    let (mut entry, mut limit, mut output) = (None, None, None);
    let (mut jobs_dir, mut max_jobs, mut job_timeout_ms) = (None, None, None);
    let mut raw_inputs = std::collections::BTreeMap::new();
    let mut pairs = args[1..].chunks_exact(2);
    for pair in &mut pairs {
        match pair[0].to_str() {
            Some("--entry") if entry.is_none() => {
                entry = Some(pair[1].to_str().ok_or("entry must be UTF-8")?.to_owned())
            }
            Some("--max-steps") if limit.is_none() => {
                limit = Some(
                    pair[1]
                        .to_str()
                        .ok_or("max-steps must be UTF-8")?
                        .parse::<u32>()
                        .map_err(|_| "invalid max-steps")?,
                )
            }
            Some("--emit-run") if output.is_none() => output = Some(PathBuf::from(&pair[1])),
            Some("--jobs-dir") if jobs_dir.is_none() => jobs_dir = Some(PathBuf::from(&pair[1])),
            Some("--max-jobs") if max_jobs.is_none() => {
                max_jobs = Some(
                    pair[1]
                        .to_str()
                        .ok_or("max-jobs must be UTF-8")?
                        .parse::<u32>()
                        .map_err(|_| "invalid max-jobs")?,
                )
            }
            Some("--job-timeout-ms") if job_timeout_ms.is_none() => {
                job_timeout_ms = Some(
                    pair[1]
                        .to_str()
                        .ok_or("job-timeout-ms must be UTF-8")?
                        .parse::<u64>()
                        .map_err(|_| "invalid job-timeout-ms")?,
                )
            }
            Some("--arg") => {
                let text = pair[1].to_str().ok_or("arg must be UTF-8")?;
                if text.len() > 32768 || raw_inputs.len() == 16 {
                    return Err("input count/encoding bound exceeded".into());
                }
                let (name, value) = text.split_once('=').ok_or("arg requires name=value")?;
                let value = serde_json::from_str::<super::strict_json::Unique>(value)
                    .map_err(|e| format!("arg requires exact JSON: {e}"))?
                    .0;
                if raw_inputs.insert(name.to_owned(), value).is_some() {
                    return Err(format!("duplicate input `{name}`"));
                }
            }
            _ => {
                return Err(format!(
                    "unknown, conflicting, or duplicate workflow option\n{USAGE}"
                ));
            }
        }
    }
    if !pairs.remainder().is_empty() {
        return Err(USAGE.into());
    }
    let output = output.ok_or(USAGE)?;
    let entry = entry.ok_or(USAGE)?;
    let program = load(&source, None)?;
    let (_, parameters, _) = program
        .entries()
        .find(|(name, _, _)| *name == entry)
        .ok_or_else(|| format!("unknown workflow entry `{entry}`"))?;
    if raw_inputs.len() != parameters.len() {
        return Err("entry requires exactly its declared named inputs".into());
    }
    let mut inputs = Inputs::new();
    for p in parameters {
        let json = raw_inputs
            .get(&p.name)
            .ok_or_else(|| format!("missing input `{}`", p.name))?;
        inputs.insert(p.name.clone(), program.input_value(&p.ty, json)?);
    }
    let max_steps = limit.ok_or(USAGE)?;
    if jobs_dir.is_some() || max_jobs.is_some() || job_timeout_ms.is_some() {
        let options = super::jobs::Options {
            directory: jobs_dir
                .ok_or("jobs require --jobs-dir, --max-jobs, and --job-timeout-ms")?,
            max_jobs: max_jobs.ok_or("jobs require --max-jobs")?,
            timeout_ms: job_timeout_ms.ok_or("jobs require --job-timeout-ms")?,
        };
        return super::jobs::run(
            &program,
            super::repository_path(&source)?,
            entry,
            inputs,
            max_steps,
            &output,
            options,
        );
    }
    let record = record(
        &program,
        super::repository_path(&source)?,
        entry,
        inputs,
        max_steps,
    )?;
    let json = format!(
        "{}\n",
        serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?
    );
    if json.len() as u64 > MAX_RECORD_BYTES {
        return Err("run record exceeds byte bound".into());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(|e| {
            format!(
                "could not create new run record '{}': {e}",
                output.display()
            )
        })?;
    file.write_all(json.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    print!("{json}");
    if record.execution.stop.returned() {
        Ok(())
    } else {
        Err("pure execution stopped; bounded outcome recorded".into())
    }
}

pub(super) fn replay_or_finite(args: &[OsString]) -> Result<(), String> {
    let [path, flag, source] = args else {
        return Err("usage: nmlt replay <record.json> --source <source.nmlt>".into());
    };
    if flag != "--source" {
        return Err("replay requires an explicit --source path".into());
    }
    // Dispatch by explicit schema. The finite record format and checker retain
    // their earlier contract; only the new format uses this bounded reader.
    let file = File::open(path).map_err(|e| e.to_string())?;
    #[derive(Deserialize)]
    struct Format {
        schema: String,
    }
    // Stream past unrelated fields instead of materializing an unbounded
    // generic JSON tree just to select the format-specific validator.
    let format: Format = serde_json::from_reader(BufReader::new(file))
        .map_err(|e| format!("invalid run record: {e}"))?;
    if format.schema == super::jobs::SCHEMA {
        return super::jobs::replay(
            &bounded_read(Path::new(path), MAX_RECORD_BYTES)?,
            Path::new(source),
        );
    }
    if format.schema != SCHEMA {
        if format.schema.starts_with("nmlt-pure-run-") {
            return Err("unsupported pure run version; retain the original executable to replay older profiles".into());
        }
        return super::runtime::replay(args);
    }
    let bytes = bounded_read(Path::new(path), MAX_RECORD_BYTES)?;
    let recorded: Record =
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid pure run record: {e}"))?;
    // This catches surplus fields in internally tagged unit/value variants.
    let raw = serde_json::from_slice::<super::strict_json::Unique>(&bytes)
        .map_err(|e| e.to_string())?
        .0;
    if raw != serde_json::to_value(&recorded).map_err(|e| e.to_string())? {
        return Err("invalid pure record: unknown fields or non-exact values".into());
    }
    if recorded.profile != PROFILE || recorded.assurance != "none" {
        return Err("unsupported pure profile or assurance claim".into());
    }
    if recorded.implementation_sha256 != implementation_digest()? {
        return Err("replay implementation mismatch: exact recorded executable required".into());
    }
    let source = Path::new(source);
    if digest(&bounded_read(
        source,
        nmlt_workflow::MAX_SOURCE_BYTES as u64,
    )?) != recorded.source_sha256
    {
        return Err("replay source mismatch".into());
    }
    let logical_entry = recorded
        .sources
        .first()
        .ok_or("pure record has no source manifest")?
        .path
        .as_str();
    let program = load(source, Some(logical_entry))?;
    if program.sources() != recorded.sources {
        return Err("replay package source mismatch".into());
    }
    let rebuilt = record(
        &program,
        recorded.source_path.clone(),
        recorded.entry.clone(),
        recorded.inputs.clone(),
        recorded.max_steps,
    )?;
    if rebuilt != recorded {
        return Err("replay mismatch: typed program, inputs, steps, or outcome differ".into());
    }
    println!(
        "{}",
        serde_json::json!({"schema":"nmlt-pure-replay-v3", "assurance":"none", "matched":true, "execution":recorded.execution, "sources":recorded.sources,
        "meaning":"same-executable pure evaluation consistency; no Lean acceptance or host effects"})
    );
    Ok(())
}

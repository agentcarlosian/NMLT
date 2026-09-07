//! Source-driven finite execution and exact-record replay. No host effects.

use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use nmlt_compile::compile_behavior_v2;
use nmlt_eval::{RunConfig, RunTrace, Schedule, execute};
use serde::{Deserialize, Serialize};

const SCHEMA: &str = "nmlt-finite-run-v1";
const PROFILE: &str = "finite-v2-local-v1";
const RUN_USAGE: &str = "usage: nmlt run <source.nmlt> --behavior <name> --max-steps <1..10000> --emit-run <new-record.json> [--actions <comma-separated labels>]";

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Assurance {
    None,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunRecord {
    schema: String,
    profile: String,
    assurance: Assurance,
    implementation_sha256: String,
    source_path: String,
    source_sha256: String,
    artifact_sha256: String,
    behavior: String,
    config: RunConfig,
    trace: RunTrace,
}

pub(super) fn digest(bytes: &[u8]) -> String {
    nmlt_hir::sha256_bytes(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn implementation_digest() -> Result<String, String> {
    let executable = env::current_exe().map_err(|e| e.to_string())?;
    fs::read(executable)
        .map(|bytes| digest(&bytes))
        .map_err(|e| e.to_string())
}

fn make_record(
    source: &Path,
    source_path: String,
    behavior: String,
    config: RunConfig,
) -> Result<RunRecord, String> {
    let bytes =
        fs::read(source).map_err(|e| format!("could not read '{}': {e}", source.display()))?;
    let artifact = compile_behavior_v2(&source_path, bytes).map_err(|e| e.to_string())?;
    let trace = execute(&artifact, &behavior, &config).map_err(|e| e.to_string())?;
    Ok(RunRecord {
        schema: SCHEMA.into(),
        profile: PROFILE.into(),
        assurance: Assurance::None,
        implementation_sha256: implementation_digest()?,
        source_path,
        source_sha256: artifact.source_sha256.clone(),
        artifact_sha256: digest(artifact.to_json_pretty().as_bytes()),
        behavior,
        config,
        trace,
    })
}

pub(super) fn run(arguments: &[OsString]) -> Result<(), String> {
    let Some(source) = arguments.first().map(PathBuf::from) else {
        return Err(RUN_USAGE.into());
    };
    let mut behavior = None;
    let mut max_steps = None;
    let mut output = None;
    let mut schedule = None;
    let mut options = arguments[1..].chunks_exact(2);
    for option in &mut options {
        let [flag, value] = option else {
            unreachable!()
        };
        match flag.to_str() {
            Some("--behavior") if behavior.is_none() => {
                behavior = Some(value.to_str().ok_or("behavior is not UTF-8")?.to_owned());
            }
            Some("--max-steps") if max_steps.is_none() => {
                max_steps = Some(
                    value
                        .to_str()
                        .ok_or("max-steps is not UTF-8")?
                        .parse::<usize>()
                        .map_err(|_| "invalid max-steps")?,
                );
            }
            Some("--emit-run") if output.is_none() => output = Some(PathBuf::from(value)),
            Some("--actions") if schedule.is_none() => {
                let text = value.to_str().ok_or("actions are not UTF-8")?;
                let labels = if text.is_empty() {
                    vec![]
                } else {
                    text.split(',').map(str::to_owned).collect()
                };
                schedule = Some(Schedule::Actions { labels });
            }
            _ => return Err(format!("unknown or duplicate run option\n{RUN_USAGE}")),
        }
    }
    if !options.remainder().is_empty() {
        return Err(RUN_USAGE.into());
    }
    let record = make_record(
        &source,
        super::repository_path(&source)?,
        behavior.ok_or(RUN_USAGE)?,
        RunConfig {
            max_steps: max_steps.ok_or(RUN_USAGE)?,
            schedule: schedule.unwrap_or(Schedule::FirstEnabled),
        },
    )?;
    let output = output.ok_or(RUN_USAGE)?;
    let json = format!(
        "{}\n",
        serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?
    );
    // New files only: never overwrite a source, existing result, or link alias.
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
        .map_err(|e| format!("could not finish run record '{}': {e}", output.display()))?;
    print!("{json}");
    if record.trace.outcome.completed() {
        Ok(())
    } else {
        Err(format!(
            "execution stopped: {:?}; partial trace recorded",
            record.trace.outcome
        ))
    }
}

pub(super) fn replay(arguments: &[OsString]) -> Result<(), String> {
    let [record_path, source_flag, source] = arguments else {
        return Err("usage: nmlt replay <record.json> --source <source.nmlt>".into());
    };
    if source_flag != "--source" {
        return Err("replay requires an explicit --source path".into());
    }
    let bytes = fs::read(record_path).map_err(|e| e.to_string())?;
    let record: RunRecord =
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid run record: {e}"))?;
    // Serde's internally tagged unit variants can ignore surplus fields even
    // with deny_unknown_fields. Compare the complete decoded shape as well.
    let raw: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if raw != serde_json::to_value(&record).map_err(|e| e.to_string())? {
        return Err("invalid run record: unrecognized fields or non-exact values".into());
    }
    if record.schema != SCHEMA || record.profile != PROFILE {
        return Err("unsupported run schema or execution profile".into());
    }
    if record.implementation_sha256 != implementation_digest()? {
        return Err(
            "replay implementation mismatch: use the exact recorded nmlt executable".into(),
        );
    }
    let source = Path::new(source);
    let source_bytes = fs::read(source).map_err(|e| e.to_string())?;
    if digest(&source_bytes) != record.source_sha256 {
        return Err("replay source mismatch: exact recorded source bytes are required".into());
    }
    let rebuilt = make_record(
        source,
        record.source_path.clone(),
        record.behavior.clone(),
        record.config.clone(),
    )?;
    if record != rebuilt {
        return Err(
            "replay mismatch: artifact, states, authority, steps, grades, or outcome differ".into(),
        );
    }
    println!(
        "{}",
        serde_json::json!({
            "schema": "nmlt-finite-replay-v1",
            "assurance": "none",
            "matched": true,
            "steps": record.trace.steps.len(),
            "outcome": record.trace.outcome,
            "meaning": "same-implementation record consistency; no Lean check or host effects"
        })
    );
    Ok(())
}

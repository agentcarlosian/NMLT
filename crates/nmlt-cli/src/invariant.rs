//! Source safety checking with explicit, retained Lean checker evidence.
use super::{diagnostics::Error, runtime, strict_json::Unique, workflow::bounded_read};
use nmlt_ir::{SafetyArtifact, SafetyClaim, SafetyWitness};
use nmlt_runtime::{identity, lean, process};
use serde_json::json;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const USAGE: &str = "usage: nmlt check-invariant <source.nmlt> --behavior <binary-name> --property <System.Name> --max-states <1..256> --checker <nmlt-invariant-check> --emit-evidence <new-directory>";

fn persist(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("safety evidence file exceeds 4 MiB".into());
    }
    let mut file = File::create_new(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub(super) fn command(arguments: &[OsString]) -> Result<(), Error> {
    let Some(source_path) = arguments.first().map(PathBuf::from) else {
        return Err(USAGE.into());
    };
    let mut options = BTreeMap::new();
    let mut pairs = arguments[1..].chunks_exact(2);
    for pair in &mut pairs {
        let name = pair[0].to_str().ok_or(USAGE)?;
        if ![
            "--behavior",
            "--property",
            "--max-states",
            "--checker",
            "--emit-evidence",
        ]
        .contains(&name)
            || options.insert(name, pair[1].clone()).is_some()
        {
            return Err(USAGE.into());
        }
    }
    if !pairs.remainder().is_empty() || options.len() != 5 {
        return Err(USAGE.into());
    }
    let behavior = options["--behavior"].to_str().ok_or(USAGE)?;
    let property_name = options["--property"].to_str().ok_or(USAGE)?;
    let max_states: usize = options["--max-states"]
        .to_str()
        .ok_or(USAGE)?
        .parse()
        .map_err(|_| Error::from(USAGE))?;
    if !(1..=256).contains(&max_states) {
        return Err(USAGE.into());
    }
    let checker = PathBuf::from(&options["--checker"]).canonicalize()?;
    let checker_sha256 = identity::file(&checker, 256 * 1024 * 1024)?.1;
    let source = bounded_read(&source_path, 131_072)?;
    let source_text = std::str::from_utf8(&source).map_err(|_| "safety source must be UTF-8")?;
    let program =
        nmlt_compile::compile_safety(super::repository_path(&source_path)?, source.clone())
            .map_err(|e| {
                let mut error = Error::new(e.code(), e.message());
                if let Some(span) = e.span() {
                    error = error.located(&source_path, source_text, span);
                }
                error.expected = e.expected().map(|s| json!(s));
                error.actual = e.actual().map(|s| json!(s));
                error
            })?;
    let property = program
        .properties
        .iter()
        .find(|p| format!("{}.{}", p.system, p.name) == property_name)
        .ok_or_else(|| {
            Error::new(
                "NMLT-SAFETY-NAME",
                "selected safety declaration does not exist",
            )
        })?
        .clone();
    nmlt_eval::safety_universe(&program.core, behavior)
        .map_err(|e| Error::new("NMLT-SAFETY-BOUND", e.to_string()))?;
    let core = program.core.to_json_pretty();
    let core_sha256 = runtime::digest(core.as_bytes());
    let artifact = SafetyArtifact {
        schema: "behavior-invariant-v1".into(),
        core_sha256: core_sha256.clone(),
        source_sha256: runtime::digest(&source),
        behavior: behavior.into(),
        property,
    };
    let encoded_artifact = serde_json::to_vec_pretty(&artifact)?;
    let directory = PathBuf::from(&options["--emit-evidence"]);
    fs::create_dir(&directory)?;
    let directory = directory.canonicalize()?;
    persist(&directory.join("source.nmlt"), &source)?;
    persist(&directory.join("core.json"), core.as_bytes())?;
    persist(&directory.join("invariant.json"), &encoded_artifact)?;
    let claim = match nmlt_eval::safety_claim(
        &program.core,
        behavior,
        &artifact.property,
        core_sha256.clone(),
        max_states,
    ) {
        Ok(claim) => claim,
        Err(error) => {
            let report = json!({"schema":"nmlt-invariant-result-v1","assurance":"none","verdict":"incomplete","reason":error.to_string(),"max_states":max_states});
            persist(
                &directory.join("result.json"),
                &serde_json::to_vec_pretty(&report)?,
            )?;
            return Err(Error::new("NMLT-SAFETY-INCOMPLETE", error.to_string()));
        }
    };
    let expected = match &claim {
        SafetyClaim::Invariant { .. } => "invariant",
        SafetyClaim::Counterexample { .. } => "counterexample",
    };
    let artifact_sha256 = runtime::digest(&encoded_artifact);
    let witness = SafetyWitness {
        schema: "behavior-invariant-witness-v1".into(),
        core_sha256: core_sha256.clone(),
        invariant_sha256: artifact_sha256.clone(),
        claim,
    };
    let encoded_witness = serde_json::to_vec_pretty(&witness)?;
    let witness_sha256 = runtime::digest(&encoded_witness);
    persist(&directory.join("witness.json"), &encoded_witness)?;
    let mut command = Command::new(&checker);
    command
        .current_dir(&directory)
        .env_clear()
        .env("LEAN_STACK_SIZE_KB", lean::STACK_KIB)
        .env("MIMALLOC_ARENA_RESERVE", lean::ARENA_KIB);
    for name in ["PATH", "SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    for name in ["core.json", "source.nmlt", "invariant.json", "witness.json"] {
        command.arg(directory.join(name));
    }
    let mut child =
        process::Process::start(command, vec![], Duration::from_secs(30)).map_err(|e| {
            Error::new(
                "NMLT-SAFETY-CHECKER",
                format!("checker start failed: {e:?}"),
            )
        })?;
    let observation = child.wait().clone();
    persist(
        &directory.join("process-policy.json"),
        &serde_json::to_vec_pretty(child.policy())?,
    )?;
    persist(
        &directory.join("observation.json"),
        &serde_json::to_vec_pretty(&observation)?,
    )?;
    let output = observation.map_err(|e| {
        Error::new(
            "NMLT-SAFETY-CHECKER",
            format!("checker stopped without acceptance: {e:?}"),
        )
    })?;
    persist(&directory.join("checker.stdout.json"), &output.stdout)?;
    persist(&directory.join("checker.stderr.txt"), &output.stderr)?;
    if output.exit_code != Some(0) || !output.stderr.is_empty() {
        return Err(Error::new(
            "NMLT-SAFETY-REJECTED",
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    let checked = serde_json::from_slice::<Unique>(&output.stdout)?.0;
    if checked["schema"] != "nmlt-safety-check-v1"
        || checked["verdict"] != expected
        || checked["behavior"] != behavior
        || checked["core_sha256"] != core_sha256
        || checked["source_sha256"] != artifact.source_sha256
        || checked["invariant_sha256"] != artifact_sha256
        || checked["witness_sha256"] != witness_sha256
        || identity::file(&checker, 256 * 1024 * 1024)?.1 != checker_sha256
    {
        return Err(Error::new(
            "NMLT-SAFETY-CHECKER",
            "checker identity, result, or input bindings differ",
        ));
    }
    for (file, expected) in [
        ("core.json", &core_sha256),
        ("source.nmlt", &artifact.source_sha256),
        ("invariant.json", &artifact_sha256),
        ("witness.json", &witness_sha256),
    ] {
        if identity::file(&directory.join(file), 4 * 1024 * 1024)?.1 != *expected {
            return Err(Error::new(
                "NMLT-SAFETY-CHECKER",
                "input changed while checker was running",
            ));
        }
    }
    let report = json!({"schema":"nmlt-invariant-result-v1","assurance":"lean_checked_finite_model","verdict":expected,"property":property_name,"checker_sha256":checker_sha256,"implementation_sha256":runtime::implementation_digest()?,"max_states":max_states,"checker":checked,"evidence":directory});
    persist(
        &directory.join("result.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if expected == "counterexample" {
        return Err(Error::new(
            "NMLT-SAFETY-REFUTED",
            "the safety predicate has a Lean-checked reachable counterexample",
        ));
    }
    Ok(())
}

//! Operator-selected task registry and the fixed contained proof worker.
use super::*;
use nmlt_runtime::{self as rt, project_proof as protocol};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema: String,
    lean_bin: PathBuf,
    exporter: PathBuf,
    nanoda: PathBuf,
    timeout_ms: u64,
    tasks: Vec<Entry>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    alias: String,
    task: PathBuf,
    task_sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    schema: String,
    workspace_root: PathBuf,
    lean_bin: PathBuf,
    exporter: PathBuf,
    nanoda: PathBuf,
    lean: lean::Identity,
    tools: export::Identity,
    tasks: Vec<Entry>,
}

pub(crate) fn open(path: &Path) -> Result<protocol::Tool> {
    let path = path.canonicalize().map_err(err)?;
    let parent = path.parent().ok_or("registry requires a parent")?;
    let text = String::from_utf8(read_bounded(&path, 1024 * 1024)?).map_err(err)?;
    let registry: Registry = toml::from_str(&text).map_err(err)?;
    if registry.schema != "nmlt-lean-project-jobs-v1" || !(1..=16).contains(&registry.tasks.len()) {
        return Err("unsupported project job registry or task count".into());
    }
    let resolve = |p: &Path| parent.join(p).canonicalize().map_err(err);
    let lean_bin = resolve(&registry.lean_bin)?;
    let exporter = resolve(&registry.exporter)?;
    let nanoda = resolve(&registry.nanoda)?;
    let lean = lean::Toolchain::open(&lean_bin).map_err(err)?;
    let tools = export::Tools::open(&exporter, &nanoda)?;
    let mut inputs = vec![];
    let mut entries = vec![];
    let mut bindings = vec![];
    let mut seen = BTreeSet::new();
    for entry in registry.tasks {
        if !protocol::alias(&entry.alias) || !seen.insert(entry.alias.clone()) {
            return Err("invalid or duplicate project alias".into());
        }
        let source = resolve(&entry.task)?;
        let task: Task = read_json(&source, MAX_JSON)?;
        task.validate()?;
        if task.digest()? != entry.task_sha256 || task.lean != *lean.identity() {
            return Err(
                "registered task or Lean installation differs from the selected pin".into(),
            );
        }
        let relative = format!("tasks/{}/task.json", entry.alias);
        let (bytes, sha256) = identity::file(&source, MAX_JSON).map_err(err)?;
        inputs.push(protocol::Input {
            source: source.clone(),
            file: identity::FileIdentity {
                path: relative.clone(),
                bytes,
                sha256,
                link_target: None,
            },
        });
        if let Some(lake) = &task.lake {
            let root = source
                .parent()
                .ok_or("task parent missing")?
                .join("lake-sources");
            lake.verify_files(&root)?;
            for file in &lake.files {
                let mut retained = file.clone();
                retained.path = format!("tasks/{}/lake-sources/{}", entry.alias, file.path);
                inputs.push(protocol::Input {
                    source: root.join(&file.path),
                    file: retained,
                });
            }
        }
        entries.push(Entry {
            alias: entry.alias.clone(),
            task: relative.into(),
            task_sha256: entry.task_sha256.clone(),
        });
        bindings.push(protocol::Binding {
            alias: entry.alias,
            task_sha256: entry.task_sha256,
        });
    }
    let configuration = Configuration {
        schema: "nmlt-project-proof-configuration-v1".into(),
        workspace_root: native_workspace_root()?,
        lean_bin,
        exporter,
        nanoda,
        lean: lean.identity().clone(),
        tools: tools.identity,
        tasks: entries,
    };
    protocol::Tool::open(
        &std::env::current_exe().map_err(err)?,
        serde_json::to_vec(&configuration).map_err(err)?,
        inputs,
        bindings,
        registry.timeout_ms,
    )
    .map_err(err)
}

fn native_workspace_root() -> Result<PathBuf> {
    #[cfg(windows)]
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let path = PathBuf::from(local).join("Temp");
        if path.is_dir() {
            return path.canonicalize().map_err(err);
        }
    }
    std::env::temp_dir().canonicalize().map_err(err)
}

fn configuration(
    directory: &Path,
    identity: &protocol::Identity,
    verify_tools: bool,
) -> Result<Configuration> {
    let bytes = read_bounded(&directory.join("configuration.json"), 1024 * 1024)?;
    if sha256(&bytes) != identity.configuration_sha256 {
        return Err("project configuration changed".into());
    }
    let config: Configuration = decode_json(&bytes)?;
    if config.schema != "nmlt-project-proof-configuration-v1"
        || config.tasks.len() != identity.bindings.len()
        || config
            .tasks
            .iter()
            .zip(&identity.bindings)
            .any(|(entry, binding)| {
                entry.alias != binding.alias
                    || entry.task_sha256 != binding.task_sha256
                    || entry.task != Path::new(&format!("tasks/{}/task.json", entry.alias))
            })
    {
        return Err("project configuration and selected task bindings differ".into());
    }
    for entry in &config.tasks {
        let task: Task = read_json(&directory.join(&entry.task), MAX_JSON)?;
        task.validate()?;
        if task.digest()? != entry.task_sha256 || task.lean != config.lean {
            return Err("retained registered task changed".into());
        }
    }
    if verify_tools
        && (*lean::Toolchain::open(&config.lean_bin)
            .map_err(err)?
            .identity()
            != config.lean
            || export::Tools::open(&config.exporter, &config.nanoda)?.identity != config.tools)
    {
        return Err("project proof tool installation changed".into());
    }
    Ok(config)
}

pub(crate) fn restore(directory: &Path, identity: &protocol::Identity) -> Result<protocol::Tool> {
    let tool = protocol::Tool::restore(directory, identity).map_err(err)?;
    if identity.executable_sha256 != implementation()? {
        return Err("project proof executable differs from current CLI".into());
    }
    configuration(directory, identity, true)?;
    Ok(tool)
}

pub(crate) fn worker(args: &[OsString]) -> Result<()> {
    let [flag, path] = args else {
        return Err("project worker requires --bundle DIR".into());
    };
    if flag != "--bundle" {
        return Err("project worker requires --bundle DIR".into());
    }
    let directory = Path::new(path).canonicalize().map_err(err)?;
    let identity: protocol::Identity = read_json(&directory.join("identity.json"), 65_536)?;
    protocol::Tool::restore(&directory, &identity).map_err(err)?;
    if identity.executable_sha256 != implementation()? {
        return Err("project worker executable changed".into());
    }
    let mut bytes = vec![];
    std::io::stdin()
        .take(process::PIPE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    if bytes.len() > process::PIPE_BYTES {
        return Err("project dispatch exceeds input bound".into());
    }
    let dispatch: rt::Dispatch = decode_json(&bytes)?;
    if bytes != serde_json::to_vec(&dispatch).map_err(err)? {
        return Err("project dispatch is not canonical".into());
    }
    let request = protocol::request(&identity, &dispatch).map_err(err)?;
    let config = configuration(&directory, &identity, false)?;
    let entry = config
        .tasks
        .iter()
        .find(|e| e.alias == request.alias)
        .ok_or("project alias missing")?;
    let key = protocol::result_key(&dispatch).map_err(err)?;
    let results = directory.join("results");
    fs::create_dir_all(&results).map_err(err)?;
    let output = results.join(&key);
    // Lake currently normalizes away Windows extended-length path prefixes.
    // Keep native build paths short; the final bundle remains dispatch-bound.
    let temporary = if cfg!(windows) {
        let temp = config.workspace_root.canonicalize().map_err(err)?;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(err)?
            .as_nanos();
        let base = temp.join(format!("nmlt-p-{}-{nonce:x}", std::process::id()));
        fs::create_dir(&base).map_err(|e| {
            format!(
                "could not create native proof workspace {}: {e}",
                base.display()
            )
        })?;
        let base = base.canonicalize().map_err(err)?;
        if !base.starts_with(&temp) {
            return Err("native proof workspace escaped its temporary root".into());
        }
        Some(base)
    } else {
        None
    };
    let work = temporary
        .as_ref()
        .map(|base| base.join("run"))
        .unwrap_or_else(|| output.clone());
    write_json(
        &results.join(format!("{key}.work.json")),
        &serde_json::json!({"schema":"nmlt-project-proof-work-v1", "dispatch_sha256":key, "work_directory":work}),
    )?;
    let result = (|| {
        let task: Task = read_json(&directory.join(&entry.task), MAX_JSON)?;
        let candidate = Candidate::from_proof(&request.task_sha256, &request.proof)?;
        let tools = export::Tools::open(&config.exporter, &config.nanoda)?;
        if tools.identity != config.tools {
            return Err("project checker installation changed".into());
        }
        let checked = prove(
            task,
            candidate,
            &config.lean_bin,
            tools,
            &work,
            None,
            directory
                .join(&entry.task)
                .parent()
                .ok_or("task parent missing")?,
        );
        if let Some(base) = &temporary {
            if work.exists() {
                retain_work(base, &work, &results, &output)?;
            }
            // This removes only the now-empty directory after a successful move.
            let _ = fs::remove_dir(base);
        }
        checked?;
        let record_bytes = read_bounded(&output.join("result.json"), MAX_JSON)?;
        let record: ResultRecord = decode_json(&record_bytes)?;
        let receipt = protocol::ProofReceipt {
            schema: "nmlt-project-proof-receipt-v1".into(),
            status: "independently_checked".into(),
            alias: request.alias.clone(),
            task_sha256: request.task_sha256.clone(),
            dispatch_sha256: protocol::result_key(&dispatch).map_err(err)?,
            result_sha256: sha256(&record_bytes),
            export_sha256: record.export_sha256,
        };
        validate_receipt(&directory, &config, &dispatch, &receipt)?;
        Ok::<_, String>(receipt)
    })();
    let outcome = match result {
        Ok(receipt) => rt::ResponseOutcome::Completed {
            value: rt::Value::Text(serde_json::to_string(&receipt).map_err(err)?),
        },
        Err(message) => {
            // Preserve full diagnostics beside this exact dispatched attempt.
            let mut bounded = message.chars().take(1000).collect::<String>();
            if bounded.is_empty() {
                bounded = "project proof failed".into();
            }
            rt::ResponseOutcome::Failed { message: bounded }
        }
    };
    let response = rt::Response {
        schema: rt::RESPONSE_SCHEMA.into(),
        binding: dispatch.binding,
        outcome,
        observed_work: Some(1),
    };
    std::io::stdout()
        .write_all(&serde_json::to_vec(&response).map_err(err)?)
        .map_err(err)
}

fn retain_work(base: &Path, work: &Path, results: &Path, output: &Path) -> Result<()> {
    let work = work.canonicalize().map_err(err)?;
    let root = results.canonicalize().map_err(err)?;
    if !work.starts_with(base)
        || output
            .parent()
            .ok_or("result parent missing")?
            .canonicalize()
            .map_err(err)?
            != root
        || output.exists()
    {
        return Err("proof artifact move exceeds its owned workspace".into());
    }
    match fs::rename(&work, output) {
        Ok(()) => Ok(()),
        // Moving across volumes is not supported by rename. Copy a bounded
        // portable bundle, omitting only the rebuildable native project cache.
        Err(_) => {
            fs::create_dir(output).map_err(err)?;
            let mut pending = vec![(work.clone(), PathBuf::new(), 0usize)];
            let mut bytes = 0u64;
            let mut nodes = 0usize;
            while let Some((directory, relative, depth)) = pending.pop() {
                if depth > 32 {
                    return Err("proof artifact tree exceeds depth bound".into());
                }
                for entry in fs::read_dir(directory).map_err(err)? {
                    let entry = entry.map_err(err)?;
                    nodes += 1;
                    if nodes > 65_536 {
                        return Err("proof artifact tree exceeds count bound".into());
                    }
                    let path = relative.join(entry.file_name());
                    if path == Path::new("lake-project") || path == Path::new("result.json") {
                        continue;
                    }
                    let metadata = fs::symlink_metadata(entry.path()).map_err(err)?;
                    #[cfg(windows)]
                    {
                        use std::os::windows::fs::MetadataExt;
                        if metadata.file_attributes() & 0x400 != 0 {
                            return Err("linked proof artifact".into());
                        }
                    }
                    if metadata.file_type().is_symlink() {
                        return Err("linked proof artifact".into());
                    }
                    if metadata.is_dir() {
                        fs::create_dir(output.join(&path)).map_err(err)?;
                        pending.push((entry.path(), path, depth + 1));
                    } else if metadata.is_file() {
                        bytes += metadata.len();
                        if bytes > 2 * 1024 * 1024 * 1024 {
                            return Err("proof bundle exceeds 2 GiB".into());
                        }
                        let content = read_bounded(&entry.path(), 256 * 1024 * 1024)?;
                        write_new(&output.join(path), &content)?;
                    } else {
                        return Err("non-regular proof artifact".into());
                    }
                }
            }
            if work.join("result.json").exists() {
                write_new(
                    &output.join("result.json"),
                    &read_bounded(&work.join("result.json"), MAX_JSON)?,
                )?;
            }
            Ok(())
        }
    }
}

fn validate_receipt(
    directory: &Path,
    config: &Configuration,
    dispatch: &rt::Dispatch,
    receipt: &protocol::ProofReceipt,
) -> Result<()> {
    let identity: protocol::Identity = read_json(&directory.join("identity.json"), 65_536)?;
    let request = protocol::request(&identity, dispatch).map_err(err)?;
    let output = directory
        .join("results")
        .join(protocol::result_key(dispatch).map_err(err)?);
    let bytes = read_bounded(&output.join("result.json"), MAX_JSON)?;
    if sha256(&bytes) != receipt.result_sha256 {
        return Err("project result hash differs from worker receipt".into());
    }
    let record: ResultRecord = decode_json(&bytes)?;
    let entry = config
        .tasks
        .iter()
        .find(|e| e.alias == request.alias)
        .ok_or("unknown result alias")?;
    let task: Task = read_json(&directory.join(&entry.task), MAX_JSON)?;
    task.validate()?;
    let candidate = Candidate::from_proof(&request.task_sha256, &request.proof)?;
    if record.schema != "nmlt-lean-result-v5"
        || record.status != "independently_checked"
        || record.task != task
        || record.task_sha256 != request.task_sha256
        || record.task.digest()? != request.task_sha256
        || record.candidate != candidate
        || record.tools != config.tools
        || record.export_sha256 != receipt.export_sha256
        || record.proof.root != "NMLTChecked.result"
        || record
            .proof
            .axioms
            .iter()
            .any(|a| !task.manifest.permitted_axioms.contains(a))
        || record.proof_source
            != source::proof(&task.target, &candidate.validate(&request.task_sha256)?)?
    {
        return Err(
            "project acceptance differs from its registered task, proof, tools or policy".into(),
        );
    }
    if task.lake.is_some() {
        record.process_policy.validate_project().map_err(err)?;
    } else {
        record.process_policy.validate().map_err(err)?;
    }
    record.validate_export_capture()?;
    record.validate_dependencies()?;
    let export = read_bounded(
        &output.join(EXPORT_PATH),
        if task.lake.is_some() {
            process::PROJECT_FILE_BYTES
        } else {
            process::FILE_BYTES
        },
    )?;
    if sha256(&export) != record.export_sha256 || export.len() as u64 != record.export_bytes {
        return Err("project export identity changed".into());
    }
    let inspected = export::inspect_export(&export)?;
    if inspected.declarations != record.exported_declarations
        || inspected.dependencies != record.proof_dependencies
        || record.checked_declarations != record.exported_declarations.len() as u64
    {
        return Err("project export or dependency graph changed".into());
    }
    let mut checker_stages = record.stages.iter().filter(|s| s.name == "nanoda");
    let checker = checker_stages
        .next()
        .ok_or("project result has no independent checker stage")?;
    if checker_stages.next().is_some()
        || checker.output.exit_code != Some(0)
        || checker.stdout_file.is_some()
        || export::success_count(&checker.output.stdout, &checker.output.stderr)?
            != record.checked_declarations
    {
        return Err("project result lacks a complete independent check".into());
    }
    if let Some(lake) = &task.lake {
        lake.verify_files(&output.join("lake-sources"))?;
    }
    for (name, expected) in [
        ("build/NMLTProof.lean", record.proof_source.as_bytes()),
        (
            "proof-dependencies.json",
            &record.proof_dependencies.json()?,
        ),
    ] {
        if read_bounded(&output.join(name), MAX_JSON)? != expected {
            return Err(format!("project proof artifact changed: {name}"));
        }
    }
    Ok(())
}

pub(crate) fn validate_snapshot(
    directory: &Path,
    snapshot: &rt::session::Snapshot,
    verify_tools: bool,
) -> Result<()> {
    let Some(identity) = &snapshot.manifest.configuration.projects else {
        return Ok(());
    };
    let manifest: rt::session::Manifest = read_json(
        &directory
            .parent()
            .ok_or("project bundle parent missing")?
            .join("manifest.json"),
        65_536,
    )?;
    if manifest != snapshot.manifest {
        return Err("project artifact session differs from the recorded session".into());
    }
    protocol::Tool::restore(directory, identity).map_err(err)?;
    let config = configuration(directory, identity, verify_tools)?;
    for observation in &snapshot.observations {
        if observation.dispatch.binding.adapter != protocol::adapter() {
            continue;
        }
        let Ok(output) = &observation.completion else {
            continue;
        };
        let Ok(response) = protocol::response(identity, &observation.dispatch, output) else {
            continue;
        };
        if let rt::ResponseOutcome::Completed {
            value: rt::Value::Text(text),
        } = response.outcome
        {
            let receipt = decode_json(text.as_bytes())?;
            validate_receipt(directory, &config, &observation.dispatch, &receipt)?;
        }
    }
    Ok(())
}

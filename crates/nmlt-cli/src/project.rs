//! Local project orchestration over the existing source execution/replay routes.
use super::{diagnostics::Error, runtime, strict_json::Unique, workflow};
use nmlt_runtime::{identity, lean};
use nmlt_workflow::{Inputs, Program, SourceIdentity, Value};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value as Json, json};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const MANIFEST: &str = "nmlt.toml";
const LOCK: &str = "nmlt.lock";
const MAX_RECORD: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    schema: String,
    pub source: String,
    entry: String,
    max_steps: u32,
    #[serde(default)]
    inputs: BTreeMap<String, Json>,
    #[serde(default)]
    jobs: Option<Jobs>,
    #[serde(default)]
    tools: Tools,
    #[serde(default)]
    tests: Vec<Test>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Jobs {
    slots: u32,
    max_attempts: u32,
    timeout_ms: u64,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tools {
    #[serde(default)]
    lean: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lean_projects: Option<PathBuf>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Test {
    name: String,
    entry: String,
    inputs: BTreeMap<String, Json>,
    expect: Json,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LeanLock {
    identity: lean::Identity,
    files: Vec<identity::FileIdentity>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Lock {
    schema: String,
    implementation_sha256: String,
    dependencies: Vec<SourceIdentity>,
    lean: Option<LeanLock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    projects: Option<nmlt_runtime::project_proof::Identity>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: String,
    assurance: String,
    manifest: String,
    manifest_sha256: String,
    lock: Lock,
    entry: String,
    inputs: BTreeMap<String, Json>,
    execution_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Invocation {
    schema: String,
    manifest: String,
    lock: Lock,
    entry: String,
    inputs: BTreeMap<String, Json>,
    project_context_sha256: String,
    jobs_directory: Option<String>,
}

pub(super) struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
    text: String,
    pub program: Program,
}

fn regular(path: &Path, max: u64) -> Result<Vec<u8>, Error> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(Error::new(
            "NMLT-PROJECT-FILE",
            format!("{} must be a regular file", path.display()),
        ));
    }
    workflow::bounded_read(path, max).map_err(Into::into)
}
fn strict<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T, Error> {
    let raw = serde_json::from_slice::<Unique>(bytes)?.0;
    let result: T = serde_json::from_value(raw.clone())?;
    if serde_json::to_value(&result)? != raw {
        return Err(Error::new(
            "NMLT-PROJECT-RECORD",
            "record contains non-exact values or fields",
        ));
    }
    Ok(result)
}
fn parse_manifest(path: &Path, text: &str) -> Result<Manifest, Error> {
    let manifest: Manifest = toml::from_str(text).map_err(|e: toml::de::Error| {
        let error = Error::new("NMLT-MANIFEST", e.message());
        e.span().map_or(error.clone(), |s| {
            error.located(path, text, nmlt_core::Span::new(s.start, s.end))
        })
    })?;
    if manifest.schema != "nmlt-project-v1"
        || !(1..=100_000).contains(&manifest.max_steps)
        || manifest.tests.len() > 32
    {
        return Err(Error::new(
            "NMLT-MANIFEST",
            "unsupported project version or execution/test bounds",
        ));
    }
    // The workflow loader additionally enforces portable, exact-case names.
    if Path::new(&manifest.source).components().count() != 1 || !manifest.source.ends_with(".nmlt")
    {
        return Err(Error::new(
            "NMLT-MANIFEST",
            "source must be a sibling .nmlt filename",
        ));
    }
    if let Some(j) = &manifest.jobs
        && (!(1..=4).contains(&j.slots)
            || !(1..=16).contains(&j.max_attempts)
            || j.slots > j.max_attempts
            || !(1..=30_000).contains(&j.timeout_ms))
    {
        return Err(Error::new(
            "NMLT-MANIFEST",
            "invalid job slots, attempt limit, or timeout",
        ));
    }
    let mut names = BTreeSet::new();
    for test in &manifest.tests {
        if test.name.is_empty() || test.name.len() > 64 || !names.insert(&test.name) {
            return Err(Error::new(
                "NMLT-MANIFEST",
                "test names must be unique and contain 1..64 bytes",
            ));
        }
    }
    Ok(manifest)
}
impl Project {
    pub fn open(root: &Path) -> Result<Self, Error> {
        let root = root.canonicalize()?;
        let path = root.join(MANIFEST);
        let text = String::from_utf8(regular(&path, 131_072)?)
            .map_err(|_| Error::new("NMLT-MANIFEST", "manifest must be UTF-8"))?;
        let manifest = parse_manifest(&path, &text)?;
        let program = workflow::load_diagnostic(&root.join(&manifest.source), None)?;
        Ok(Self {
            root,
            manifest,
            text,
            program,
        })
    }
    fn lean_path(&self) -> Option<PathBuf> {
        self.manifest.tools.lean.as_ref().map(|p| self.root.join(p))
    }
    fn projects_path(&self) -> Option<PathBuf> {
        self.manifest
            .tools
            .lean_projects
            .as_ref()
            .map(|p| self.root.join(p))
    }
    fn capture_lock(&self) -> Result<Lock, Error> {
        let lean = self
            .lean_path()
            .map(|path| -> Result<LeanLock, Error> {
                let tool = lean::Toolchain::open(&path)?;
                Ok(LeanLock {
                    identity: tool.identity().clone(),
                    files: tool.files().to_vec(),
                })
            })
            .transpose()?;
        let lock = Lock {
            schema: "nmlt-lock-v1".into(),
            implementation_sha256: runtime::implementation_digest()?,
            dependencies: self.program.sources().iter().skip(1).cloned().collect(),
            lean,
            projects: self
                .projects_path()
                .as_deref()
                .map(super::lean_task::jobs::open)
                .transpose()?
                .map(|t| t.identity().clone()),
        };
        if serde_json::to_vec(&lock)?.len() > 24 * 1024 * 1024 {
            return Err("dependency lock exceeds 24 MiB".into());
        }
        Ok(lock)
    }
    fn verify_lock(&self) -> Result<Lock, Error> {
        let locked: Lock = strict(&regular(&self.root.join(LOCK), MAX_RECORD)?)?;
        let actual = self.capture_lock()?;
        if locked != actual {
            return Err(Error::new(
                "NMLT-LOCK-MISMATCH",
                "NMLT executable, imported source, or tool installation differs from nmlt.lock; review the changes and run `nmlt lock`",
            ));
        }
        Ok(locked)
    }
    fn inputs(&self, entry: &str, raw: &BTreeMap<String, Json>) -> Result<Inputs, Error> {
        let (_, parameters, _) = self
            .program
            .entries()
            .find(|(n, _, _)| *n == entry)
            .ok_or_else(|| Error::new("NMLT-ENTRY", format!("unknown entry `{entry}`")))?;
        let expected: BTreeSet<_> = parameters.iter().map(|p| &p.name).collect();
        if expected != raw.keys().collect() {
            return Err(Error::mismatch(
                "NMLT-INPUTS",
                "entry requires exactly its declared inputs",
                json!(expected),
                json!(raw.keys().collect::<Vec<_>>()),
            ));
        }
        let mut inputs = Inputs::new();
        for parameter in parameters {
            let value = self
                .program
                .input_value(&parameter.ty, &raw[&parameter.name])
                .map_err(|reason| {
                    Error::mismatch(
                        "NMLT-INPUT-TYPE",
                        format!("input `{}`: {reason}", parameter.name),
                        json!(parameter.ty),
                        raw[&parameter.name].clone(),
                    )
                })?;
            inputs.insert(parameter.name.clone(), value);
        }
        nmlt_workflow::validate_inputs(&self.program, entry, &inputs, self.manifest.max_steps)?;
        if self.program.requires_jobs(entry)? && self.manifest.jobs.is_none() {
            return Err(Error::new(
                "NMLT-JOBS-CONFIG",
                "job entry requires a [jobs] configuration",
            ));
        }
        if self.program.requires_lean_jobs(entry)? && self.lean_path().is_none() {
            return Err(Error::new(
                "NMLT-LEAN-CONFIG",
                "Lean entry requires tools.lean",
            ));
        }
        if self.program.requires_project_jobs(entry)? && self.projects_path().is_none() {
            return Err(Error::new(
                "NMLT-LEAN-CONFIG",
                "project proof entry requires tools.lean_projects",
            ));
        }
        Ok(inputs)
    }
    fn validate_tests(&self) -> Result<(), Error> {
        self.inputs(&self.manifest.entry, &self.manifest.inputs)?;
        for test in &self.manifest.tests {
            self.inputs(&test.entry, &test.inputs)?;
            self.expect(test)?;
        }
        Ok(())
    }
    fn expect(&self, test: &Test) -> Result<Value, Error> {
        let (_, _, result) = self
            .program
            .entries()
            .find(|(n, _, _)| *n == test.entry)
            .ok_or("unknown test entry")?;
        self.program
            .input_value(result, &test.expect)
            .map_err(|message| {
                Error::mismatch(
                    "NMLT-TEST-EXPECT",
                    format!("test `{}`: {message}", test.name),
                    json!(result),
                    test.expect.clone(),
                )
            })
    }
    fn execute(
        &self,
        locked: &Lock,
        entry: &str,
        raw: &BTreeMap<String, Json>,
    ) -> Result<(PathBuf, Json), Error> {
        self.inputs(entry, raw)?;
        let directory = new_run(&self.root)?;
        let sources = directory.join("sources");
        fs::create_dir(&sources)?;
        for source in self.program.sources() {
            let bytes = regular(
                &self.root.join(&source.path),
                nmlt_workflow::MAX_SOURCE_BYTES as u64,
            )?;
            if runtime::digest(&bytes) != source.source_sha256 {
                return Err(Error::new(
                    "NMLT-FILE-CHANGED",
                    "source changed before run snapshot",
                ));
            }
            write_new(&sources.join(&source.path), &bytes)?;
        }
        let executable = retained_executable(&self.root, &locked.implementation_sha256)?;
        let execution_file = directory.join("source.json");
        let project_context = invocation_digest(&self.text, locked, entry, raw)?;
        // Durable project intent survives a child/parent interruption before the
        // completed source record exists. Recovery must still classify the journal.
        write_new(
            &directory.join("invocation.json"),
            &serde_json::to_vec_pretty(&Invocation {
                schema: "nmlt-project-invocation-v2".into(),
                manifest: self.text.clone(),
                lock: locked.clone(),
                entry: entry.into(),
                inputs: raw.clone(),
                project_context_sha256: project_context.clone(),
                jobs_directory: None,
            })?,
        )?;
        let mut command = Command::new(executable);
        command
            .current_dir(&self.root)
            .arg("run")
            .arg(sources.join(&self.manifest.source))
            .args([
                "--entry",
                entry,
                "--max-steps",
                &self.manifest.max_steps.to_string(),
                "--emit-run",
            ])
            .arg(&execution_file);
        command.args(["--project-context", &project_context]);
        for (name, value) in raw {
            command
                .arg("--arg")
                .arg(format!("{name}={}", serde_json::to_string(value)?));
        }
        if self.program.requires_jobs(entry)? {
            let jobs = self.manifest.jobs.as_ref().ok_or("missing jobs")?;
            command.arg("--jobs-dir").arg(directory.join("jobs")).args([
                "--max-jobs",
                &jobs.max_attempts.to_string(),
                "--job-timeout-ms",
                &jobs.timeout_ms.to_string(),
            ]);
            command.args(["--job-slots", &jobs.slots.to_string()]);
            if self.program.requires_lean_jobs(entry)? {
                command
                    .arg("--lean-bin")
                    .arg(self.lean_path().ok_or("missing Lean")?);
            }
            if self.program.requires_project_jobs(entry)? {
                command.arg("--lean-projects").arg(
                    self.projects_path()
                        .ok_or("missing project proof registry")?,
                );
            }
        }
        let output = command.output()?;
        write_new(&directory.join("stderr.txt"), &output.stderr)?;
        if !execution_file.exists() {
            return Err(Error::new(
                "NMLT-PROJECT-RUN",
                format!(
                    "source run failed before a record was written: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }
        let bytes = regular(&execution_file, MAX_RECORD)?;
        let execution = serde_json::from_slice::<Unique>(&bytes)?.0;
        let record = Record {
            schema: "nmlt-project-run-v1".into(),
            assurance: "none".into(),
            manifest: self.text.clone(),
            manifest_sha256: runtime::digest(self.text.as_bytes()),
            lock: locked.clone(),
            entry: entry.into(),
            inputs: raw.clone(),
            execution_sha256: runtime::digest(&bytes),
        };
        bind(self, &record, &execution)?;
        write_new(
            &directory.join("project.json"),
            &serde_json::to_vec_pretty(&record)?,
        )?;
        if self.capture_lock()? != *locked {
            return Err(Error::new(
                "NMLT-LOCK-MISMATCH",
                "dependencies changed during execution; record retained without project acceptance",
            ));
        }
        if !output.status.success() {
            return Err(Error::new(
                "NMLT-PROJECT-STOP",
                format!(
                    "bounded source run stopped; evidence: {}",
                    directory.display()
                ),
            ));
        }
        Ok((directory, execution["execution"]["stop"]["value"].clone()))
    }
}

fn bind(project: &Project, record: &Record, execution: &Json) -> Result<(), Error> {
    if record.schema != "nmlt-project-run-v1"
        || record.assurance != "none"
        || record.lock.schema != "nmlt-lock-v1"
        || record.manifest_sha256 != runtime::digest(record.manifest.as_bytes())
        || record.lock.implementation_sha256 != runtime::implementation_digest()?
        || record.lock.dependencies
            != project
                .program
                .sources()
                .iter()
                .skip(1)
                .cloned()
                .collect::<Vec<_>>()
    {
        return Err(Error::new(
            "NMLT-PROJECT-REPLAY",
            "project version, source dependency, manifest, or executable mismatch",
        ));
    }
    let context = execution.get("context").unwrap_or(execution);
    if context["project_context_sha256"]
        != invocation_digest(
            &record.manifest,
            &record.lock,
            &record.entry,
            &record.inputs,
        )?
    {
        return Err(Error::new(
            "NMLT-PROJECT-REPLAY",
            "project manifest or lock is not bound to the source execution",
        ));
    }
    let inputs = project.inputs(&record.entry, &record.inputs)?;
    if context["entry"] != record.entry
        || context["inputs"] != json!(inputs)
        || context["max_steps"] != project.manifest.max_steps
        || context["program_sha256"] != project.program.identity()
        || context["sources"] != json!(project.program.sources())
        || context["implementation_sha256"] != record.lock.implementation_sha256
    {
        return Err(Error::new(
            "NMLT-PROJECT-REPLAY",
            "project invocation differs from underlying execution context",
        ));
    }
    if project.program.requires_jobs(&record.entry)? {
        let jobs = project.manifest.jobs.as_ref().ok_or("missing jobs")?;
        if context["bounds"]
            != json!({"slots":jobs.slots,"max_attempts":jobs.max_attempts,"timeout_ms":jobs.timeout_ms})
        {
            return Err(Error::new(
                "NMLT-PROJECT-REPLAY",
                "project job bounds mismatch",
            ));
        }
    }
    if project.program.requires_lean_jobs(&record.entry)? {
        let lean = record
            .lock
            .lean
            .as_ref()
            .ok_or("project record lacks Lean lock")?;
        if execution["snapshot"]["manifest"]["configuration"]["lean"] != json!(lean.identity) {
            return Err(Error::new(
                "NMLT-PROJECT-REPLAY",
                "project Lean identity mismatch",
            ));
        }
    }
    if project.program.requires_project_jobs(&record.entry)? {
        let projects = record
            .lock
            .projects
            .as_ref()
            .ok_or("project record lacks bound proof registry")?;
        if execution["snapshot"]["manifest"]["configuration"]["projects"] != json!(projects)
            || context["projects"] != json!(projects)
        {
            return Err(Error::new(
                "NMLT-PROJECT-REPLAY",
                "project proof registry identity mismatch",
            ));
        }
    }
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let mut output = File::create_new(path)?;
    output.write_all(bytes)?;
    output.sync_all()?;
    Ok(())
}

fn invocation_digest(
    manifest: &str,
    lock: &Lock,
    entry: &str,
    inputs: &BTreeMap<String, Json>,
) -> Result<String, Error> {
    Ok(runtime::digest(&serde_json::to_vec(&(
        "nmlt-project-invocation-v1",
        runtime::digest(manifest.as_bytes()),
        lock,
        entry,
        inputs,
    ))?))
}
pub(super) fn replace(path: &Path, original: Option<&[u8]>, bytes: &[u8]) -> Result<(), Error> {
    if let Some(original) = original {
        if regular(path, MAX_RECORD)? != original {
            return Err(Error::new(
                "NMLT-FILE-CHANGED",
                "file changed before replacement",
            ));
        }
    } else if fs::symlink_metadata(path).is_ok() {
        return Err("refusing to overwrite an existing file".into());
    }
    let temporary = path.with_file_name(format!(
        ".{}.nmlt-write-{}",
        path.file_name()
            .ok_or("missing filename")?
            .to_string_lossy(),
        std::process::id()
    ));
    write_new(&temporary, bytes)?;
    let result = (|| -> Result<(), Error> {
        if original.is_some() {
            fs::set_permissions(&temporary, fs::metadata(path)?.permissions())?;
        }
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
fn new_run(root: &Path) -> Result<PathBuf, Error> {
    let parent = root.join(".nmlt");
    if !parent.exists() {
        fs::create_dir(&parent)?;
    }
    let metadata = fs::symlink_metadata(&parent)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || parent.canonicalize()?.parent() != Some(root)
    {
        return Err(".nmlt must be a local project directory".into());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = parent.join(format!("run-{}-{stamp}", std::process::id()));
    fs::create_dir(&directory)?;
    Ok(directory)
}

fn retained_executable(root: &Path, digest: &str) -> Result<PathBuf, Error> {
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let path = root.join(".nmlt").join(format!("nmlt-{digest}{suffix}"));
    if fs::symlink_metadata(&path).is_err() {
        let source = std::env::current_exe()?;
        let bytes = regular(&source, 256 * 1024 * 1024)?;
        if runtime::digest(&bytes) != digest {
            return Err("executable changed before retention".into());
        }
        match write_new(&path, &bytes) {
            Ok(()) => fs::set_permissions(&path, fs::metadata(source)?.permissions())?,
            Err(error) if !path.exists() => return Err(error),
            Err(_) => {}
        }
    }
    if fs::symlink_metadata(&path)?.file_type().is_symlink()
        || identity::file(&path, 256 * 1024 * 1024)?.1 != digest
    {
        return Err("retained NMLT executable identity mismatch".into());
    }
    Ok(path)
}

pub(super) fn command(command: &str, arguments: &[OsString]) -> Result<(), Error> {
    let root = arguments
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if command == "init" {
        if arguments.len() != 1 {
            return Err("usage: nmlt init <new-directory>".into());
        }
        return init(&root);
    }
    let project = Project::open(&root)?;
    if command != "run" && arguments.len() > 1 {
        return Err(format!("usage: nmlt {command} [project-directory]").into());
    }
    if command == "lock" {
        project.validate_tests()?;
        let lock = project.capture_lock()?;
        let path = project.root.join(LOCK);
        let original = if path.exists() {
            Some(regular(&path, MAX_RECORD)?)
        } else {
            None
        };
        replace(
            &path,
            original.as_deref(),
            &serde_json::to_vec_pretty(&lock)?,
        )?;
        println!(
            "{}",
            json!({"schema":"nmlt-project-lock-v1","path":path,"dependencies":lock.dependencies.len(),"lean_files":lock.lean.as_ref().map(|l|l.files.len()).unwrap_or(0)})
        );
        return Ok(());
    }
    let lock = project.verify_lock()?;
    match command {
        "check-project" => {
            project.validate_tests()?;
            println!(
                "{}",
                json!({"schema":"nmlt-project-check-v1","assurance":"none","checked":true,"sources":project.program.sources(),"tests":project.manifest.tests.len()})
            );
        }
        "run" => {
            let mut inputs = project.manifest.inputs.clone();
            let mut supplied = BTreeSet::new();
            let mut pairs = arguments.get(1..).unwrap_or_default().chunks_exact(2);
            for pair in &mut pairs {
                if pair[0] != "--arg" {
                    return Err("project run supports --arg name=value overrides".into());
                }
                let text = pair[1].to_str().ok_or("input is not UTF-8")?;
                if text.len() > 32_768 {
                    return Err("input encoding exceeds 32 KiB".into());
                }
                let (name, value) = text.split_once('=').ok_or("input requires name=value")?;
                if !supplied.insert(name.to_owned()) {
                    return Err("duplicate input override".into());
                }
                inputs.insert(name.into(), serde_json::from_str::<Unique>(value)?.0);
            }
            if !pairs.remainder().is_empty() {
                return Err("input option requires a value".into());
            }
            let (directory, value) = project.execute(&lock, &project.manifest.entry, &inputs)?;
            println!(
                "{}",
                json!({"schema":"nmlt-project-result-v1","assurance":"none","record":directory.join("project.json"),"value":value})
            );
        }
        "test" => {
            project.validate_tests()?;
            if project.manifest.tests.is_empty() {
                return Err(Error::new("NMLT-TEST-EMPTY", "project declares no tests"));
            }
            let mut results = vec![];
            let mut passed = true;
            for test in &project.manifest.tests {
                let expected = json!(project.expect(test)?);
                match project.execute(&lock, &test.entry, &test.inputs) {
                    Ok((directory, value)) => {
                        let matched = value == expected;
                        passed &= matched;
                        results.push(json!({"name":test.name,"passed":matched,"expected":expected,"actual":value,"record":directory.join("project.json")}));
                    }
                    Err(error) => {
                        passed = false;
                        results.push(json!({"name":test.name,"passed":false,"diagnostic":error}));
                    }
                }
            }
            println!(
                "{}",
                json!({"schema":"nmlt-project-tests-v1","assurance":"none","passed":passed,"tests":results})
            );
            if !passed {
                return Err(Error::new(
                    "NMLT-TEST-FAILED",
                    "one or more project tests failed",
                ));
            }
        }
        _ => return Err("unsupported project command".into()),
    }
    Ok(())
}

pub(super) fn resume(arguments: &[OsString]) -> Result<(), Error> {
    let run = arguments.first().map(PathBuf::from).ok_or("usage: nmlt resume <saved-run-directory> --project <directory> [--acknowledge-uncertain-effects <reason>]")?.canonicalize()?;
    let mut project_root = None;
    let mut acknowledgement = None;
    let mut pairs = arguments[1..].chunks_exact(2);
    for pair in &mut pairs {
        match pair[0].to_str() {
            Some("--project") if project_root.is_none() => {
                project_root = Some(PathBuf::from(&pair[1]).canonicalize()?)
            }
            Some("--acknowledge-uncertain-effects") if acknowledgement.is_none() => {
                let reason = pair[1].to_str().ok_or("acknowledgement must be UTF-8")?;
                if reason.trim().is_empty() || reason.len() > 1024 {
                    return Err("acknowledgement requires 1..1024 bytes".into());
                }
                acknowledgement = Some(reason.to_owned());
            }
            _ => return Err("unknown or duplicate project resume option".into()),
        }
    }
    if !pairs.remainder().is_empty() {
        return Err("project resume options require values".into());
    }
    let project_root = project_root.ok_or("project resume requires --project")?;
    if !run.starts_with(project_root.join(".nmlt")) {
        return Err("saved run must be inside this project's .nmlt directory".into());
    }
    let mut invocation: Invocation = strict(&regular(&run.join("invocation.json"), MAX_RECORD)?)?;
    if invocation.schema != "nmlt-project-invocation-v2"
        || invocation.project_context_sha256
            != invocation_digest(
                &invocation.manifest,
                &invocation.lock,
                &invocation.entry,
                &invocation.inputs,
            )?
    {
        return Err("saved project invocation identity differs".into());
    }
    let manifest = parse_manifest(&project_root.join(MANIFEST), &invocation.manifest)?;
    let source_root = run.join("sources");
    let program = workflow::load_diagnostic(&source_root.join(&manifest.source), None)?;
    let project = Project {
        root: source_root,
        manifest,
        text: invocation.manifest.clone(),
        program,
    };
    if !project.program.requires_jobs(&invocation.entry)? {
        return Err("project resumption requires the durable asynchronous source profile".into());
    }
    let jobs = if let Some(relative) = &invocation.jobs_directory {
        let path = Path::new(relative);
        if path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("invalid saved jobs directory".into());
        }
        project_root.join(path).canonicalize()?
    } else {
        run.join("jobs").canonicalize()?
    };
    if !jobs.starts_with(project_root.join(".nmlt")) {
        return Err("saved jobs directory leaves the project".into());
    }
    let context: Json = serde_json::from_slice::<Unique>(&regular(
        &jobs.join("source-context.json"),
        2 * 1024 * 1024,
    )?)?
    .0;
    let session: Json =
        serde_json::from_slice::<Unique>(&regular(&jobs.join("manifest.json"), 65_536)?)?.0;
    let mut record = Record {
        schema: "nmlt-project-run-v1".into(),
        assurance: "none".into(),
        manifest: invocation.manifest.clone(),
        manifest_sha256: runtime::digest(invocation.manifest.as_bytes()),
        lock: invocation.lock.clone(),
        entry: invocation.entry.clone(),
        inputs: invocation.inputs.clone(),
        execution_sha256: String::new(),
    };
    // Check the saved invocation against the underlying immutable source/session
    // context before granting the child any resumption authority.
    bind(
        &project,
        &record,
        &json!({"context":context,"snapshot":{"manifest":session}}),
    )?;
    let executable = retained_executable(&project_root, &record.lock.implementation_sha256)?;
    let directory = new_run(&project_root)?;
    let sources = directory.join("sources");
    fs::create_dir(&sources)?;
    for source in project.program.sources() {
        let bytes = regular(
            &project.root.join(&source.path),
            nmlt_workflow::MAX_SOURCE_BYTES as u64,
        )?;
        if runtime::digest(&bytes) != source.source_sha256 {
            return Err("saved source changed during resumption capture".into());
        }
        write_new(&sources.join(&source.path), &bytes)?;
    }
    invocation.jobs_directory = Some(
        jobs.strip_prefix(&project_root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("jobs path is not UTF-8")?
            .replace('\\', "/"),
    );
    write_new(
        &directory.join("invocation.json"),
        &serde_json::to_vec_pretty(&invocation)?,
    )?;
    let execution_path = directory.join("source.json");
    let mut command = Command::new(executable);
    command
        .current_dir(&project_root)
        .arg("jobs-resume")
        .arg(&jobs)
        .arg("--emit-run")
        .arg(&execution_path);
    if project.program.requires_lean_jobs(&invocation.entry)? {
        command.arg("--lean-bin").arg(
            project_root.join(
                project
                    .manifest
                    .tools
                    .lean
                    .as_ref()
                    .ok_or("saved Lean configuration missing")?,
            ),
        );
    }
    if let Some(reason) = acknowledgement {
        command.args(["--acknowledge-uncertain-effects", &reason]);
    }
    let output = command.output()?;
    write_new(&directory.join("stderr.txt"), &output.stderr)?;
    if !execution_path.is_file() || fs::metadata(&execution_path)?.len() == 0 {
        return Err(Error::new(
            "NMLT-PROJECT-RESUME",
            format!(
                "resumption requires attention: {}; saved invocation: {}",
                String::from_utf8_lossy(&output.stderr).trim(),
                directory.join("invocation.json").display()
            ),
        ));
    }
    let bytes = regular(&execution_path, MAX_RECORD)?;
    let execution = serde_json::from_slice::<Unique>(&bytes)?.0;
    record.execution_sha256 = runtime::digest(&bytes);
    bind(&project, &record, &execution)?;
    write_new(
        &directory.join("project.json"),
        &serde_json::to_vec_pretty(&record)?,
    )?;
    println!(
        "{}",
        json!({"schema":"nmlt-project-result-v1","assurance":"none","record":directory.join("project.json"),"value":execution["execution"]["stop"]["value"]})
    );
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::new(
            "NMLT-PROJECT-RESUME",
            "resumed execution stopped; its bounded outcome was retained",
        ))
    }
}

pub(super) fn replay(arguments: &[OsString]) -> Result<(), Error> {
    let [record_path, flag, root] = arguments else {
        return Err("usage: nmlt replay <project.json> --project <directory>".into());
    };
    if flag != "--project" {
        return Err("project replay requires --project".into());
    }
    let path = Path::new(record_path).canonicalize()?;
    let record: Record = strict(&regular(&path, MAX_RECORD)?)?;
    let project_root = Path::new(root).canonicalize()?;
    if !path.starts_with(project_root.join(".nmlt")) {
        return Err("record must be in this project's .nmlt directory".into());
    }
    let root = path
        .parent()
        .ok_or("record has no directory")?
        .join("sources");
    let manifest = parse_manifest(&project_root.join(MANIFEST), &record.manifest)?;
    let program = workflow::load_diagnostic(&root.join(&manifest.source), None)?;
    let project = Project {
        root,
        manifest,
        text: record.manifest.clone(),
        program,
    };
    let execution_path = path
        .parent()
        .ok_or("record has no directory")?
        .join("source.json");
    let bytes = regular(&execution_path, MAX_RECORD)?;
    if runtime::digest(&bytes) != record.execution_sha256 {
        return Err(Error::new(
            "NMLT-PROJECT-REPLAY",
            "underlying execution record digest mismatch",
        ));
    }
    let execution = serde_json::from_slice::<Unique>(&bytes)?.0;
    bind(&project, &record, &execution)?;
    // The source replay path validates all captured events. It never dispatches.
    workflow::replay_or_finite(&[
        execution_path.into_os_string(),
        "--source".into(),
        project.root.join(&project.manifest.source).into_os_string(),
    ])?;
    Ok(())
}

fn init(root: &Path) -> Result<(), Error> {
    fs::create_dir(root)?;
    let source = "// A failed input falls back; the collected result is reusable.\nfn main(input: Int, fallback: Int) -> Outcome<Int> {\n  let first = job_start_square(input);\n  let backup = job_start_square(fallback);\n  let result = job_collect(first);\n  let alternative = job_collect(backup);\n  match result {\n    Ok(value) => Ok(value + value),\n    Err(message) => match alternative {\n      Ok(value) => Ok(value + value),\n      Err(problem) => Err(problem)\n    }\n  }\n}\n";
    let manifest = "schema = \"nmlt-project-v1\"\nsource = \"main.nmlt\"\nentry = \"main\"\nmax_steps = 1000\n\n[inputs]\ninput = -3\nfallback = 5\n\n[jobs]\nslots = 2\nmax_attempts = 4\ntimeout_ms = 10000\n\n[[tests]]\nname = \"fallback and reuse\"\nentry = \"main\"\ninputs = { input = -3, fallback = 5 }\nexpect = { Ok = 50 }\n\n[[tests]]\nname = \"changed real input\"\nentry = \"main\"\ninputs = { input = 4, fallback = 5 }\nexpect = { Ok = 32 }\n";
    write_new(&root.join("main.nmlt"), source.as_bytes())?;
    write_new(&root.join(MANIFEST), manifest.as_bytes())?;
    write_new(&root.join(".gitignore"), b".nmlt/\n")?;
    let project = Project::open(root)?;
    project.validate_tests()?;
    write_new(
        &root.join(LOCK),
        &serde_json::to_vec_pretty(&project.capture_lock()?)?,
    )?;
    println!(
        "{}",
        json!({"schema":"nmlt-project-init-v1","directory":project.root,"next":["check-project","run","test","fmt"]})
    );
    Ok(())
}

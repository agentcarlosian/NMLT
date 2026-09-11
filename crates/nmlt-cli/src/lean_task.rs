//! Executable-only, bound local Lean tasks. See RFC 0031 for the trusted-host boundary.
use nmlt_runtime::{identity, lean, process, sha256};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

mod dependencies;
mod diagnostics;
mod discovery;
mod export;
mod inspection;
mod source;

const MAX_JSON: u64 = 64 * 1024 * 1024;
const MAX_SOURCE: u64 = 1024 * 1024;
const EXPORT_PATH: &str = "build/environment.ndjson";
const HELP: &str = "\
Bound local Lean tasks (trusted project code; closed candidate proof terms):
  nmlt lean-task bind --project DIR --lean-bin FILE --output NEW_DIR
  nmlt lean-task inspect --task task.json --task-sha256 HASH --prefix NAME --limit 1..16 --lean-bin FILE --output NEW_DIR
  nmlt lean-task prove --task task.json --task-sha256 HASH --candidate candidate.json --lean-bin FILE --exporter FILE --nanoda FILE --output NEW_DIR
  nmlt lean-task recheck --record result.json --task-sha256 HASH --lean-bin FILE --exporter FILE --nanoda FILE --output NEW_DIR
Projects select explicit modules or source roots, target and axiom policy in nmlt-lean.json.
Binding is not human approval. Select the task hash outside the candidate channel.
";

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    modules: Vec<String>,
    target_module: String,
    target: String,
    permitted_axioms: Vec<String>,
}
impl Manifest {
    fn validate(&self) -> Result<()> {
        if self.schema != "nmlt-lean-project-v1"
            || self.modules.is_empty()
            || self.modules.len() > 64
            || !name(&self.target)
            || !self.modules.contains(&self.target_module)
        {
            return Err("invalid Lean project schema, module count or target".into());
        }
        let mut seen = BTreeSet::new();
        for module in &self.modules {
            if !name(module)
                || module.starts_with("NMLTTask")
                || module.starts_with("NMLTProof")
                || module.starts_with("NMLTExport")
                || ["Init", "Lean", "Std", "Lake", "Main"]
                    .contains(&module.split('.').next().unwrap_or(""))
                || !seen.insert(module.to_lowercase())
            {
                return Err("invalid, reserved or duplicate Lean module".into());
            }
        }
        let mut axes = BTreeSet::new();
        for axiom in &self.permitted_axioms {
            if !["propext", "Quot.sound", "Classical.choice"].contains(&axiom.as_str())
                || !axes.insert(axiom)
            {
                return Err("unsupported or duplicate permitted axiom".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    module: String,
    text: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    declaration: String,
    module: String,
    type_repr: String,
    type_lean: String,
    level_params: Vec<String>,
    type_references: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Task {
    schema: String,
    implementation_sha256: String,
    manifest: Manifest,
    sources: Vec<Source>,
    lean: lean::Identity,
    target: Target,
    discovery: Option<discovery::Closure>,
}
impl Task {
    fn validate(&self) -> Result<()> {
        self.manifest.validate()?;
        if self.schema != "nmlt-lean-task-v2"
            || self.implementation_sha256 != implementation()?
            || self.sources.len() != self.manifest.modules.len()
            || self.target.declaration != self.manifest.target
            || self.target.module != self.manifest.target_module
            || self.target.type_repr.is_empty()
            || self.target.type_repr.len() > 48 * 1024
            || self.target.level_params.len() > 16
            || self
                .target
                .level_params
                .iter()
                .any(|p| !name(p) || p.contains('.'))
        {
            return Err("task schema, implementation or target metadata mismatch".into());
        }
        let mut total = 0usize;
        for (module, source) in self.manifest.modules.iter().zip(&self.sources) {
            total += source.text.len();
            if source.module != *module
                || source.text.len() > MAX_SOURCE as usize
                || total > 16 * 1024 * 1024
            {
                return Err("task sources differ from the declared bounded module sequence".into());
            }
        }
        if let Some(closure) = &self.discovery {
            closure.validate(&self.manifest, &self.sources)?;
        }
        Ok(())
    }
    fn digest(&self) -> Result<String> {
        Ok(sha256(&serde_json::to_vec(self).map_err(err)?))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    schema: String,
    task_sha256: String,
    proof: String,
}
impl Candidate {
    fn validate(&self, digest: &str) -> Result<String> {
        if self.schema != "nmlt-lean-proof-candidate-v1" || self.task_sha256 != digest {
            return Err("candidate does not refer to the selected task hash".into());
        }
        lean::render_proof_term(&self.proof).map_err(err)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProofMetadata {
    root: String,
    axioms: Vec<String>,
    proof_references: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stage {
    name: String,
    output: process::Output,
    stdout_file: Option<FileArtifact>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileArtifact {
    path: String,
    receipt: process::FileReceipt,
    policy: process::FilePolicy,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultRecord {
    schema: String,
    status: String,
    task_sha256: String,
    task: Task,
    candidate: Candidate,
    proof_source: String,
    proof: ProofMetadata,
    tools: export::Identity,
    export_sha256: String,
    export_bytes: u64,
    exported_declarations: Vec<String>,
    checked_declarations: u64,
    proof_dependencies: dependencies::Graph,
    process_policy: process::Policy,
    stages: Vec<Stage>,
}

impl ResultRecord {
    fn validate_dependencies(&self) -> Result<()> {
        let graph = &self.proof_dependencies;
        graph.validate()?;
        if graph.export_sha256 != self.export_sha256
            || graph
                .nodes
                .iter()
                .map(|node| &node.name)
                .collect::<Vec<_>>()
                != self.exported_declarations.iter().collect::<Vec<_>>()
            || graph.node(&graph.root)?.type_references != [graph.target.clone()]
            || graph.node(&graph.root)?.value_references != self.proof.proof_references
            || graph.node(&graph.target)?.value_references != self.task.target.type_references
        {
            return Err(
                "proof dependency graph differs from the bound export or Lean references".into(),
            );
        }
        Ok(())
    }

    fn validate_export_capture(&self) -> Result<()> {
        let mut files = self
            .stages
            .iter()
            .filter(|stage| stage.stdout_file.is_some());
        let stage = files.next().ok_or("missing retained export capture")?;
        let file = stage.stdout_file.as_ref().unwrap();
        file.policy.validate().map_err(err)?;
        file.receipt.validate().map_err(err)?;
        if files.next().is_some()
            || stage.name != "lean4export"
            || file.path != EXPORT_PATH
            || stage.output.exit_code != Some(0)
            || !stage.output.stdout.is_empty()
            || !stage.output.stderr.is_empty()
            || file.receipt.bytes == 0
            || file.receipt.bytes != self.export_bytes
            || file.receipt.sha256 != self.export_sha256
        {
            return Err("retained export capture does not match the accepted artifact".into());
        }
        Ok(())
    }
}

pub(super) fn command(args: &[OsString]) -> Result<()> {
    if args.is_empty() || args.iter().any(|a| a == "--help") {
        print!("{HELP}");
        return Ok(());
    }
    let action = args[0].to_str().ok_or("command must be UTF-8")?;
    let allowed: &[&str] = match action {
        "bind" => &["--project", "--lean-bin", "--output"],
        "inspect" => &[
            "--task",
            "--task-sha256",
            "--prefix",
            "--limit",
            "--lean-bin",
            "--output",
        ],
        "prove" => &[
            "--task",
            "--task-sha256",
            "--candidate",
            "--lean-bin",
            "--exporter",
            "--nanoda",
            "--output",
        ],
        "recheck" => &[
            "--record",
            "--task-sha256",
            "--lean-bin",
            "--exporter",
            "--nanoda",
            "--output",
        ],
        _ => return Err(HELP.into()),
    };
    let mut options = BTreeMap::new();
    let mut rest = &args[1..];
    while let [flag, value, tail @ ..] = rest {
        let flag = flag.to_str().ok_or("option must be UTF-8")?;
        if !allowed.contains(&flag) || options.insert(flag, PathBuf::from(value)).is_some() {
            return Err(format!("unknown or duplicate option {flag}"));
        }
        rest = tail;
    }
    if !rest.is_empty() || options.len() != allowed.len() {
        return Err(HELP.into());
    }
    let lean = options["--lean-bin"].canonicalize().map_err(err)?;
    if action == "bind" {
        return bind(&options["--project"], &lean, &options["--output"]);
    }
    let digest = options["--task-sha256"]
        .to_str()
        .ok_or("hash must be UTF-8")?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("task hash requires 64 lowercase hexadecimal digits".into());
    }
    if action == "inspect" {
        let query = inspection::Query::new(
            options["--prefix"].to_str().ok_or("prefix must be UTF-8")?,
            options["--limit"].to_str().ok_or("limit must be UTF-8")?,
        )?;
        let task: Task = read_json(&options["--task"], MAX_JSON)?;
        task.validate()?;
        if task.digest()? != digest {
            return Err("retained task does not match the selected task hash".into());
        }
        return inspection::run(task, query, &lean, &options["--output"]);
    }
    let previous: Option<ResultRecord> = if action == "recheck" {
        let value: serde_json::Value = read_json(&options["--record"], MAX_JSON)?;
        if value["schema"] == "nmlt-lean-inspection-v1" {
            return Err(
                "declaration context is not a proof acceptance record; use lean-task prove".into(),
            );
        }
        if value["schema"] != "nmlt-lean-result-v4" {
            return Err(
                "unsupported Lean result version; use its retained original CLI executable".into(),
            );
        }
        Some(serde_json::from_value(value).map_err(err)?)
    } else {
        None
    };
    let (task, candidate) = if let Some(record) = &previous {
        if record.schema != "nmlt-lean-result-v4"
            || record.status != "independently_checked"
            || record.task_sha256 != digest
        {
            return Err("unsupported result or selected task mismatch".into());
        }
        record.process_policy.validate().map_err(err)?;
        record.validate_export_capture()?;
        record.validate_dependencies()?;
        (record.task.clone(), record.candidate.clone())
    } else {
        (
            read_json(&options["--task"], MAX_JSON)?,
            read_json(&options["--candidate"], 16 * 1024)?,
        )
    };
    task.validate()?;
    if task.digest()? != digest {
        return Err("retained task does not match the selected task hash".into());
    }
    let proof = candidate.validate(digest)?;
    let tools = export::Tools::open(&options["--exporter"], &options["--nanoda"])?;
    if let Some(record) = &previous {
        if record.tools != tools.identity {
            return Err("independent checker tools changed since acceptance".into());
        }
        if record.proof_source != source::proof(&task.target, &proof)? {
            return Err("retained proof source was altered".into());
        }
    }
    prove(
        task,
        candidate,
        &lean,
        tools,
        &options["--output"],
        previous.as_ref(),
    )
}

fn bind(project: &Path, executable: &Path, output: &Path) -> Result<()> {
    let project = project.canonicalize().map_err(err)?;
    let input: serde_json::Value = read_json(&project.join("nmlt-lean.json"), 16 * 1024)?;
    enum SourcePlan {
        Explicit(Manifest),
        Discovered(discovery::Configuration),
    }
    let plan = match input["schema"].as_str() {
        Some("nmlt-lean-project-v1") => {
            let manifest: Manifest = serde_json::from_value(input).map_err(err)?;
            manifest.validate()?;
            SourcePlan::Explicit(manifest)
        }
        Some("nmlt-lean-project-v2") => {
            let configuration: discovery::Configuration =
                serde_json::from_value(input).map_err(err)?;
            configuration.validate()?;
            SourcePlan::Discovered(configuration)
        }
        _ => return Err("unsupported Lean project manifest schema".into()),
    };
    let toolchain = lean::Toolchain::open(executable).map_err(err)?;
    let mut run = Run::new(output, executable)?;
    let (manifest, sources, discovery) = match plan {
        SourcePlan::Discovered(configuration) => {
            let (manifest, sources, closure) =
                discovery::capture(&project, configuration, &mut run)?;
            (manifest, sources, Some(closure))
        }
        SourcePlan::Explicit(manifest) => {
            let sources = explicit_sources(&project, &manifest)?;
            (manifest, sources, None)
        }
    };
    run.build(&sources)?;
    let target = run.target(&manifest)?;
    let task = Task {
        schema: "nmlt-lean-task-v2".into(),
        implementation_sha256: implementation()?,
        manifest,
        sources,
        lean: toolchain.identity().clone(),
        target,
        discovery,
    };
    task.validate()?;
    toolchain.verify_unchanged().map_err(err)?;
    retain_cli(&run.directory, &task.implementation_sha256)?;
    if let Some(closure) = &task.discovery {
        write_json(&run.directory.join("source-imports.json"), closure)?;
    }
    write_json(&run.directory.join("task.json"), &task)?;
    write_json(&run.directory.join("build-log.json"), &run.stages)?;
    let digest = task.digest()?;
    write_new(
        &run.directory.join("task.sha256"),
        format!("{digest}\n").as_bytes(),
    )?;
    println!(
        "bound_task: {digest}\ntask: {}",
        run.directory.join("task.json").display()
    );
    Ok(())
}

fn explicit_sources(project: &Path, manifest: &Manifest) -> Result<Vec<Source>> {
    manifest.validate()?;
    let mut sources = vec![];
    let mut total = 0usize;
    for module in &manifest.modules {
        let path = project.join(module_path(module));
        if fs::symlink_metadata(&path)
            .map_err(err)?
            .file_type()
            .is_symlink()
            || !path.canonicalize().map_err(err)?.starts_with(project)
        {
            return Err("project source escapes its root or is a link".into());
        }
        let text = String::from_utf8(read_bounded(&path, MAX_SOURCE)?).map_err(err)?;
        total += text.len();
        if total > 16 * 1024 * 1024 {
            return Err("Lean project exceeds 16 MiB of source".into());
        }
        sources.push(Source {
            module: module.clone(),
            text,
        });
    }
    Ok(sources)
}

fn prove(
    task: Task,
    candidate: Candidate,
    executable: &Path,
    tools: export::Tools,
    output: &Path,
    previous: Option<&ResultRecord>,
) -> Result<()> {
    let digest = task.digest()?;
    let proof = candidate.validate(&digest)?;
    let toolchain = lean::Toolchain::open(executable).map_err(err)?;
    if *toolchain.identity() != task.lean {
        return Err("Lean installation differs from the bound task".into());
    }
    let mut run = Run::new(output, executable)?;
    if let Some(closure) = &task.discovery {
        discovery::verify(closure, &task.manifest, &task.sources, &mut run)?;
    }
    run.build(&task.sources)?;
    let actual = run.target(&task.manifest)?;
    if actual != task.target {
        return Err("Lean target identity differs from the bound task".into());
    }
    let proof_source = source::proof(&task.target, &proof)?;
    let output = run.compile("NMLTProof", &proof_source)?;
    let metadata: ProofMetadata = parse_marker(&output.stdout, "NMLT_PROOF=")?;
    if metadata.root != "NMLTChecked.result"
        || metadata
            .axioms
            .iter()
            .any(|a| !task.manifest.permitted_axioms.contains(a))
    {
        return Err("proof uses an axiom outside the selected transitive policy".into());
    }
    let exported = tools.check(&mut run, &task.manifest.permitted_axioms)?;
    toolchain.verify_unchanged().map_err(err)?;
    tools.verify_unchanged()?;
    let record = ResultRecord {
        schema: "nmlt-lean-result-v4".into(),
        status: "independently_checked".into(),
        task_sha256: digest.clone(),
        task,
        candidate,
        proof_source,
        proof: metadata,
        tools: tools.identity,
        export_sha256: exported.sha256,
        export_bytes: exported.bytes,
        exported_declarations: exported.declarations,
        checked_declarations: exported.count,
        proof_dependencies: exported.dependencies,
        process_policy: run.policy.ok_or("missing process policy")?,
        stages: run.stages,
    };
    record.validate_export_capture()?;
    record.validate_dependencies()?;
    if let Some(old) = previous
        && (old.proof != record.proof
            || old.export_sha256 != record.export_sha256
            || old.export_bytes != record.export_bytes
            || old.exported_declarations != record.exported_declarations
            || old.proof_dependencies != record.proof_dependencies
            || old.checked_declarations != record.checked_declarations)
    {
        return Err("fresh proof artifacts differ from the retained acceptance record".into());
    }
    retain_cli(&run.directory, &record.task.implementation_sha256)?;
    if let Some(closure) = &record.task.discovery {
        write_json(&run.directory.join("source-imports.json"), closure)?;
    }
    write_new(
        &run.directory.join("proof.patch"),
        source::patch(&source::target(&record.task.manifest), &record.proof_source).as_bytes(),
    )?;
    write_new(
        &run.directory.join("explanation.md"),
        source::explanation(&record).as_bytes(),
    )?;
    write_new(
        &run.directory.join("proof-dependencies.json"),
        &record.proof_dependencies.json()?,
    )?;
    write_new(
        &run.directory.join("proof-dependencies.md"),
        record.proof_dependencies.markdown()?.as_bytes(),
    )?;
    // Publish acceptance only after all accompanying artifacts are durable.
    write_json(&run.directory.join("result.json"), &record)?;
    println!(
        "independently_checked: {}\ntask: {digest}\ndeclarations: {}\nresult: {}",
        record.proof.root,
        record.checked_declarations,
        run.directory.join("result.json").display()
    );
    Ok(())
}

struct Run {
    directory: PathBuf,
    build: PathBuf,
    executable: PathBuf,
    stages: Vec<Stage>,
    policy: Option<process::Policy>,
}
impl Run {
    fn new(path: &Path, executable: &Path) -> Result<Self> {
        fs::create_dir(path)
            .map_err(|e| format!("output must be a new directory with an existing parent: {e}"))?;
        let directory = path.canonicalize().map_err(err)?;
        let build = directory.join("build");
        fs::create_dir(&build).map_err(err)?;
        Ok(Self {
            directory,
            build,
            executable: executable.to_path_buf(),
            stages: vec![],
            policy: None,
        })
    }
    fn build(&mut self, sources: &[Source]) -> Result<()> {
        for source in sources {
            self.compile(&source.module, &source.text)?;
        }
        Ok(())
    }
    fn command(&self) -> Result<Command> {
        let mut command = Command::new(&self.executable);
        command.env_clear().current_dir(&self.build);
        let bin = self
            .executable
            .parent()
            .ok_or("Lean executable has no parent")?;
        let root = bin.parent().ok_or("Lean installation has no root")?;
        command
            .env("LEAN_SYSROOT", root)
            .env("LEAN_PATH", &self.build)
            .env("LEAN_STACK_SIZE_KB", lean::STACK_KIB)
            .env("MIMALLOC_ARENA_RESERVE", lean::ARENA_KIB)
            .env("PATH", bin);
        #[cfg(windows)]
        for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        Ok(command)
    }
    fn execute(&mut self, name: &str, command: Command) -> Result<process::Output> {
        let output = self.capture(name, command)?;
        if output.exit_code != Some(0) {
            return Err(format!(
                "{name} rejected the task/proof:\n{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(output)
    }

    fn capture(&mut self, name: &str, command: Command) -> Result<process::Output> {
        let mut child = process::Process::start(command, vec![], Duration::from_secs(30))
            .map_err(|e| format!("{name}: {e:?}"))?;
        let policy = child.policy().clone();
        policy.validate().map_err(err)?;
        self.policy = Some(policy);
        let output = child
            .wait()
            .as_ref()
            .map_err(|e| format!("{name}: {e:?}"))?
            .clone();
        self.stages.push(Stage {
            name: name.into(),
            output: output.clone(),
            stdout_file: None,
        });
        write_json(
            &self
                .directory
                .join(format!("stage-{}.json", self.stages.len())),
            self.stages.last().unwrap(),
        )?;
        Ok(output)
    }

    fn execute_export(&mut self, command: Command) -> Result<process::FileReceipt> {
        let mut child = process::FileProcess::start(
            command,
            vec![],
            Duration::from_secs(30),
            &self.directory.join(EXPORT_PATH),
        )
        .map_err(|e| format!("lean4export: {e:?}"))?;
        let policy = child.policy().clone();
        policy.validate().map_err(err)?;
        let observation = child.wait().clone();
        write_json(
            &self.directory.join("export-observation.json"),
            &observation,
        )?;
        let captured = observation.map_err(|e| format!("lean4export: {e:?}"))?;
        self.stages.push(Stage {
            name: "lean4export".into(),
            output: captured.output.clone(),
            stdout_file: Some(FileArtifact {
                path: EXPORT_PATH.into(),
                receipt: captured.file.clone(),
                policy,
            }),
        });
        write_json(
            &self
                .directory
                .join(format!("stage-{}.json", self.stages.len())),
            self.stages.last().unwrap(),
        )?;
        if captured.output.exit_code != Some(0) || !captured.output.stderr.is_empty() {
            return Err(format!(
                "lean4export rejected the task/proof:\n{}",
                String::from_utf8_lossy(&captured.output.stderr)
            ));
        }
        Ok(captured.file)
    }
    fn compile(&mut self, module: &str, text: &str) -> Result<process::Output> {
        let relative = module_path(module);
        let path = self.build.join(&relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(err)?;
        }
        write_new(&path, text.as_bytes())?;
        let mut command = self.command()?;
        command
            .args([
                "--json",
                "--threads=1",
                "--memory=768",
                "-DmaxHeartbeats=200000",
                "-o",
            ])
            .arg(relative.with_extension("olean"))
            .arg(relative);
        let output = self.capture(module, command)?;
        let report = diagnostics::Report::parse(module, text, &path, self.stages.len(), &output)?;
        let report_path = self
            .directory
            .join(format!("diagnostics-{}.json", self.stages.len()));
        write_json(&report_path, &report)?;
        if output.exit_code != Some(0) || report.has_errors() {
            let mut detail = report.error_text();
            if detail.is_empty() {
                detail = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            return Err(format!(
                "{module} rejected the task/proof:\n{detail}\nStructured Lean diagnostics: {}",
                report_path.display()
            ));
        }
        Ok(output)
    }
    fn target(&mut self, manifest: &Manifest) -> Result<Target> {
        let text = source::target(manifest);
        let output = self.compile("NMLTTask", &text)?;
        parse_marker(&output.stdout, "NMLT_TASK=")
    }
}

fn name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.split('.').all(|part| {
            !part.is_empty()
                && part.as_bytes()[0].is_ascii_alphabetic()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_'".contains(&b))
        })
}
fn module_path(module: &str) -> PathBuf {
    PathBuf::from(module.replace('.', "/") + ".lean")
}
fn implementation() -> Result<String> {
    identity::file(&std::env::current_exe().map_err(err)?, 256 * 1024 * 1024)
        .map(|x| x.1)
        .map_err(err)
}
fn retain_cli(directory: &Path, expected: &str) -> Result<()> {
    let source = std::env::current_exe().map_err(err)?;
    let target = directory.join(if cfg!(windows) { "nmlt.exe" } else { "nmlt" });
    let input = fs::File::open(&source).map_err(err)?;
    if input.metadata().map_err(err)?.len() > 256 * 1024 * 1024 {
        return Err("CLI executable exceeds retention bound".into());
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(err)?;
    let copied = std::io::copy(&mut input.take(256 * 1024 * 1024 + 1), &mut output).map_err(err)?;
    if copied > 256 * 1024 * 1024 {
        return Err("CLI executable grew beyond retention bound".into());
    }
    output.sync_all().map_err(err)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).map_err(err)?;
    }
    if identity::file(&target, 256 * 1024 * 1024).map_err(err)?.1 != expected {
        return Err("CLI executable changed during retention".into());
    }
    Ok(())
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let input = fs::File::open(path).map_err(err)?;
    let meta = input.metadata().map_err(err)?;
    if !meta.is_file() || meta.len() > limit {
        return Err("input must be a bounded regular file".into());
    }
    let mut bytes = vec![];
    input.take(limit + 1).read_to_end(&mut bytes).map_err(err)?;
    if bytes.len() as u64 != meta.len() {
        return Err("input changed size or exceeded its bound".into());
    }
    Ok(bytes)
}
fn read_json<T: DeserializeOwned>(path: &Path, limit: u64) -> Result<T> {
    decode_json(&read_bounded(path, limit)?)
}
fn decode_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let unique: crate::strict_json::Unique = serde_json::from_slice(bytes).map_err(err)?;
    serde_json::from_value(unique.0).map_err(err)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(err)?;
    file.write_all(bytes).map_err(err)?;
    file.sync_all().map_err(err)
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(err)?;
    if bytes.len() as u64 > MAX_JSON {
        return Err("retained JSON artifact exceeds its 64 MiB input bound".into());
    }
    write_new(path, &bytes)
}
fn parse_marker<T: DeserializeOwned>(bytes: &[u8], marker: &str) -> Result<T> {
    let values = diagnostics::metadata_rows(bytes, marker)?;
    let mut rows = values.iter();
    let value = decode_json(rows.next().ok_or("missing Lean metadata")?.as_bytes())?;
    if rows.next().is_some() {
        return Err("duplicate Lean metadata".into());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manifest() -> Manifest {
        Manifest {
            schema: "nmlt-lean-project-v1".into(),
            modules: vec!["Example.Goals".into()],
            target_module: "Example.Goals".into(),
            target: "Example.goal".into(),
            permitted_axioms: vec![],
        }
    }
    #[test]
    fn project_paths_and_axiom_policy_fail_closed() {
        assert!(manifest().validate().is_ok());
        for module in [
            "../Escape",
            "Example..Goals",
            "Lean",
            "Lean.Bad",
            "NMLTTask",
            "NMLTProof.Child",
            "Example/Goals",
            "C:Escape",
        ] {
            let mut m = manifest();
            m.modules = vec![module.into()];
            m.target_module = module.into();
            assert!(m.validate().is_err(), "{module}");
        }
        let mut m = manifest();
        m.modules.push("example.goals".into());
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.permitted_axioms.push("sorryAx".into());
        assert!(m.validate().is_err());
        m.permitted_axioms = vec!["propext".into(), "propext".into()];
        assert!(m.validate().is_err());
    }
    #[test]
    fn candidates_cannot_revise_statements_pins_or_commands() {
        let mut candidate = Candidate {
            schema: "nmlt-lean-proof-candidate-v1".into(),
            task_sha256: "a".repeat(64),
            proof: "fun n => Eq.refl n".into(),
        };
        assert!(candidate.validate(&"a".repeat(64)).is_ok());
        assert!(candidate.validate(&"b".repeat(64)).is_err());
        candidate.proof = "by sorry".into();
        assert!(candidate.validate(&"a".repeat(64)).is_err());
        assert!(decode_json::<Candidate>(br#"{"schema":"nmlt-lean-proof-candidate-v1","task_sha256":"x","proof":"True.intro","statement":"True"}"#).is_err());
        assert!(
            decode_json::<Candidate>(
                br#"{"schema":"x","schema":"y","task_sha256":"x","proof":"True.intro"}"#
            )
            .is_err()
        );
    }
    #[test]
    fn target_metadata_requires_one_unambiguous_report() {
        assert_eq!(
            parse_marker::<Vec<String>>(b"diagnostic\nNMLT_TASK=[\"Nat\"]\n", "NMLT_TASK=")
                .unwrap(),
            ["Nat"]
        );
        assert!(
            parse_marker::<Vec<String>>(b"NMLT_TASK=[]\nNMLT_TASK=[]\n", "NMLT_TASK=").is_err()
        );
        assert!(parse_marker::<Vec<String>>(b"success\n", "NMLT_TASK=").is_err());
    }
}

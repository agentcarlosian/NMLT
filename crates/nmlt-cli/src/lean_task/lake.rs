//! Native Lake resolution and retained source inputs for fresh project builds.
use super::{Manifest, Result, Run, Source, err, read_bounded, read_json, write_json, write_new};
use nmlt_runtime::{identity, lean, process, sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const MAX_FILES: usize = 32_768;
const MAX_FILE: u64 = 16 * 1024 * 1024;
const MAX_TOTAL: u64 = 512 * 1024 * 1024;
const INPUTS: &str = "lake-sources";
const PROJECT: &str = "lake-project";

struct CaptureBudget {
    bytes: u64,
    files: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Configuration {
    schema: String,
    target_module: String,
    target: String,
    permitted_axioms: Vec<String>,
}

impl Configuration {
    pub fn validate(&self) -> Result<()> {
        if self.schema != "nmlt-lean-project-v3" {
            return Err("unsupported Lake project configuration".into());
        }
        self.manifest().validate()
    }

    pub fn manifest(&self) -> Manifest {
        Manifest {
            schema: "nmlt-lean-project-v1".into(),
            modules: vec![self.target_module.clone()],
            target_module: self.target_module.clone(),
            target: self.target.clone(),
            permitted_axioms: self.permitted_axioms.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Package {
    name: String,
    directory: String,
    origin: Value,
    git_commit: Option<String>,
    git_clean: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "base", rename_all = "snake_case", deny_unknown_fields)]
enum SearchRoot {
    Project { path: String },
    Toolchain { path: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Environment {
    lean_path: Vec<SearchRoot>,
    lean_src_path: Vec<SearchRoot>,
    library_path: Vec<SearchRoot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    schema: String,
    configuration: Configuration,
    packages: Vec<Package>,
    pub(super) files: Vec<identity::FileIdentity>,
    root_lake_dir: String,
    target_source: String,
    environment: Environment,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NativeEnvironment {
    lean_path: Vec<PathBuf>,
    lean_src_path: Vec<PathBuf>,
    library_path: Vec<PathBuf>,
}

/// Called only as a fixed child of the pinned Lake executable. No command or
/// source text is accepted by this internal environment query.
pub(super) fn print_environment() -> Result<()> {
    fn paths(key: &str) -> Vec<PathBuf> {
        std::env::var_os(key)
            .map(|value| std::env::split_paths(&value).collect())
            .unwrap_or_default()
    }
    let mut library_path = paths("PATH");
    library_path.extend(paths("LD_LIBRARY_PATH"));
    library_path.extend(paths("DYLD_LIBRARY_PATH"));
    println!(
        "{}",
        serde_json::to_string(&NativeEnvironment {
            lean_path: paths("LEAN_PATH"),
            lean_src_path: paths("LEAN_SRC_PATH"),
            library_path,
        })
        .map_err(err)?
    );
    Ok(())
}

fn hash(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn relative(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 2048
        && !value.contains(['\\', ':'])
        && value.split('/').count() <= 32
        && value.split('/').all(|part| {
            !part.is_empty() && part != "." && part != ".." && !part.chars().any(char::is_control)
        })
}

fn text_field<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Lake lock is missing {key}"))
}

fn portable(path: &Path) -> Result<String> {
    let text = path
        .to_str()
        .ok_or("Lake path is not UTF-8")?
        .replace('\\', "/");
    if !relative(&text) {
        return Err("Lake path is not a bounded relative path".into());
    }
    Ok(text)
}

fn canonical(path: &Path) -> Result<PathBuf> {
    // Library output directories do not exist before their first build. Resolve
    // their existing ancestor rather than trusting lexical `..` components.
    if path.exists() {
        return path.canonicalize().map_err(err);
    }
    let parent = path.parent().ok_or("Lake path has no existing ancestor")?;
    let name = path.file_name().ok_or("invalid Lake output path")?;
    Ok(canonical(parent)?.join(name))
}

fn owned(path: &Path, project: &Path, toolchain: &Path) -> Result<SearchRoot> {
    let path = canonical(path)?;
    if let Ok(relative) = path.strip_prefix(project) {
        Ok(SearchRoot::Project {
            path: portable(relative)?,
        })
    } else if let Ok(relative) = path.strip_prefix(toolchain) {
        Ok(SearchRoot::Toolchain {
            path: portable(relative)?,
        })
    } else {
        Err("Lake search path escapes the retained project and pinned Lean installation".into())
    }
}

impl Environment {
    fn capture(native: NativeEnvironment, project: &Path, toolchain: &Path) -> Result<Self> {
        fn unique(values: Vec<SearchRoot>) -> Vec<SearchRoot> {
            let mut result = vec![];
            for value in values {
                if !result.contains(&value) {
                    result.push(value);
                }
            }
            result
        }
        let lean_path = native
            .lean_path
            .iter()
            .map(|p| owned(p, project, toolchain))
            .collect::<Result<_>>()?;
        let lean_src_path = native
            .lean_src_path
            .iter()
            .map(|p| owned(p, project, toolchain))
            .collect::<Result<_>>()?;
        // OS loader locations are a stated host dependency. Only project/tool
        // directories are added to our explicit child environment.
        let library_path = native
            .library_path
            .iter()
            .filter_map(|p| owned(p, project, toolchain).ok())
            .collect();
        let environment = Self {
            lean_path: unique(lean_path),
            lean_src_path: unique(lean_src_path),
            library_path: unique(library_path),
        };
        environment.validate()?;
        Ok(environment)
    }

    fn validate(&self) -> Result<()> {
        if self.lean_path.is_empty() || self.lean_src_path.is_empty() {
            return Err("Lake omitted its source/library search path".into());
        }
        for roots in [&self.lean_path, &self.lean_src_path, &self.library_path] {
            if roots.len() > 256 {
                return Err("Lake search path count exceeds 256".into());
            }
            let mut seen = BTreeSet::new();
            for root in roots {
                let (base, path) = match root {
                    SearchRoot::Project { path } => ("project", path),
                    SearchRoot::Toolchain { path } => ("toolchain", path),
                };
                if !relative(path) || !seen.insert((base, path.to_lowercase())) {
                    return Err("invalid or duplicate retained Lake search path".into());
                }
            }
        }
        Ok(())
    }

    fn apply(&self, project: &Path, toolchain: &Path, run: &mut Run) -> Result<()> {
        self.validate()?;
        let resolve = |roots: &[SearchRoot]| {
            roots
                .iter()
                .map(|root| match root {
                    SearchRoot::Project { path } => project.join(path),
                    SearchRoot::Toolchain { path } => toolchain.join(path),
                })
                .collect::<Vec<_>>()
        };
        run.library_paths = resolve(&self.lean_path);
        run.source_paths = resolve(&self.lean_src_path);
        run.loader_paths = resolve(&self.library_path);
        Ok(())
    }
}

impl Snapshot {
    pub fn editor_manifest(&self) -> Result<Value> {
        let root = self.packages.first().ok_or("missing Lake root package")?;
        let config = if self
            .files
            .iter()
            .any(|file| file.path == "root/lakefile.lean")
        {
            "lakefile.lean"
        } else {
            "lakefile.toml"
        };
        let mut entries = vec![
            json!({"type":"path", "name":root.name, "scope":"", "dir":"../lake-project/root", "configFile":config, "manifestFile":"lake-manifest.json", "inherited":false}),
        ];
        for package in self.packages.iter().skip(1) {
            let mut entry = package.origin.clone();
            let object = entry.as_object_mut().ok_or("invalid Lake package entry")?;
            for key in ["url", "rev", "inputRev", "subDir"] {
                object.remove(key);
            }
            object.insert("type".into(), json!("path"));
            object.insert(
                "dir".into(),
                json!(format!("../lake-project/{}", package.directory)),
            );
            object.insert("inherited".into(), json!(true));
            entries.push(entry);
        }
        Ok(
            json!({"version":"1.2.0", "packagesDir":".lake/packages", "packages":entries, "name":"nmlt_editor", "lakeDir":".lake"}),
        )
    }

    pub fn validate(&self, manifest: &Manifest, sources: &[Source]) -> Result<()> {
        self.configuration.validate()?;
        self.environment.validate()?;
        if self.schema != "nmlt-lean-lake-snapshot-v1"
            || self.configuration.manifest() != *manifest
            || self.packages.is_empty()
            || self.packages.len() > 65
            || self.files.is_empty()
            || self.files.len() > MAX_FILES
            || !relative(&self.root_lake_dir)
            || !relative(&self.target_source)
            || self
                .files
                .windows(2)
                .any(|pair| pair[0].path >= pair[1].path)
        {
            return Err("invalid retained Lake project snapshot".into());
        }
        let mut packages = BTreeSet::new();
        let mut directories = BTreeSet::new();
        for (index, package) in self.packages.iter().enumerate() {
            let expected = if index == 0 {
                "root".into()
            } else {
                format!("packages/p{index}")
            };
            if package.name.is_empty()
                || package.name.len() > 256
                || package.name.chars().any(char::is_control)
                || package.directory != expected
                || !packages.insert(package.name.to_lowercase())
                || !directories.insert(package.directory.clone())
                || package
                    .git_commit
                    .as_ref()
                    .is_some_and(|commit| !hash(commit, 40))
                || package.git_commit.is_some() != package.git_clean.is_some()
            {
                return Err("invalid Lake package identity".into());
            }
            if index > 0 {
                if text_field(&package.origin, "name")? != package.name {
                    return Err("Lake package name differs from lock".into());
                }
                match text_field(&package.origin, "type")? {
                    "git"
                        if package.git_commit.as_deref()
                            == Some(text_field(&package.origin, "rev")?)
                            && package.git_clean == Some(true) => {}
                    "path" => {}
                    _ => {
                        return Err(
                            "Lake dependency is not pinned to its captured source revision".into(),
                        );
                    }
                }
                for key in ["configFile", "manifestFile"] {
                    if let Some(value) = package.origin.get(key).and_then(Value::as_str)
                        && !relative(value)
                    {
                        return Err("Lake configuration path escapes its package".into());
                    }
                }
            }
        }
        let mut total = 0u64;
        let mut paths = BTreeSet::new();
        for file in &self.files {
            total = total
                .checked_add(file.bytes)
                .ok_or("Lake source size overflow")?;
            if !relative(&file.path)
                || file.bytes > MAX_FILE
                || total > MAX_TOTAL
                || file.link_target.is_some()
                || !hash(&file.sha256, 64)
                || !paths.insert(file.path.to_lowercase())
                || !directories
                    .iter()
                    .any(|dir| file.path.starts_with(&format!("{dir}/")))
                || compiled_file(Path::new(&file.path))
            {
                return Err("invalid Lake input identity or source bound exceeded".into());
            }
        }
        let [source] = sources else {
            return Err("Lake task must retain its selected target source".into());
        };
        let file = self
            .files
            .iter()
            .find(|file| file.path == self.target_source)
            .ok_or("Lake target source is absent from captured inputs")?;
        if source.module != manifest.target_module
            || file.bytes != source.text.len() as u64
            || file.sha256 != sha256(source.text.as_bytes())
        {
            return Err("Lake target source differs from its captured input".into());
        }
        for required in ["root/lean-toolchain", "root/lake-manifest.json"] {
            if !self.files.iter().any(|file| file.path == required) {
                return Err("Lake snapshot omitted a required pin file".into());
            }
        }
        Ok(())
    }

    fn overrides(&self) -> Value {
        let packages = Value::Array(
            self.packages
                .iter()
                .skip(1)
                .map(|package| {
                    let mut entry = package.origin.clone();
                    let object = entry.as_object_mut().expect("validated package entry");
                    for key in ["url", "rev", "inputRev", "subDir"] {
                        object.remove(key);
                    }
                    object.insert("type".into(), json!("path"));
                    object.insert("dir".into(), json!(format!("../{}", package.directory)));
                    entry
                })
                .collect(),
        );
        json!({"schemaVersion":"1.2.0", "packages":packages})
    }

    pub(super) fn verify_files(&self, directory: &Path) -> Result<()> {
        let metadata = fs::symlink_metadata(directory).map_err(err)?;
        if !metadata.is_dir() || linked(&metadata) {
            return Err("retained Lake root must be an ordinary directory".into());
        }
        let directory = directory.canonicalize().map_err(err)?;
        for file in &self.files {
            let path = checked_file(&directory, &file.path)?;
            let (bytes, digest) = identity::file(&path, MAX_FILE).map_err(err)?;
            if bytes != file.bytes || digest != file.sha256 {
                return Err(format!("retained Lake input changed: {}", file.path));
            }
        }
        Ok(())
    }

    fn materialize(&self, retained: &Path, project: &Path) -> Result<()> {
        self.verify_files(retained)?;
        fs::create_dir(project).map_err(err)?;
        for file in &self.files {
            copy_file(
                &checked_file(retained, &file.path)?,
                &project.join(&file.path),
                file,
            )?;
        }
        let override_path = project
            .join("root")
            .join(&self.root_lake_dir)
            .join("package-overrides.json");
        fs::create_dir_all(
            override_path
                .parent()
                .ok_or("override path has no parent")?,
        )
        .map_err(err)?;
        write_json(&override_path, &self.overrides())?;
        Ok(())
    }

    pub fn restore(&self, origin: &Path, run: &mut Run) -> Result<()> {
        let retained = run.directory.join(INPUTS);
        fs::create_dir(&retained).map_err(err)?;
        let original = origin.join(INPUTS).canonicalize().map_err(err)?;
        self.verify_files(&original)?;
        for file in &self.files {
            copy_file(
                &checked_file(&original, &file.path)?,
                &retained.join(&file.path),
                file,
            )?;
        }
        let project = run.directory.join(PROJECT);
        self.materialize(&retained, &project)?;
        let environment = build(&project, &self.configuration.target_module, run)?;
        if environment != self.environment {
            return Err("rebuilt Lake environment differs from the bound project".into());
        }
        self.verify_files(&retained)?;
        self.verify_files(&project)?;
        Ok(())
    }

    pub fn verify_run(&self, run: &Run) -> Result<()> {
        self.verify_files(&run.directory.join(INPUTS))?;
        self.verify_files(&run.directory.join(PROJECT))
    }
}

fn checked_file(root: &Path, relative_path: &str) -> Result<PathBuf> {
    if !relative(relative_path) {
        return Err("invalid retained Lake input path".into());
    }
    let mut path = root.to_path_buf();
    for part in relative_path.split('/') {
        path.push(part);
        if linked(&fs::symlink_metadata(&path).map_err(err)?) {
            return Err("Lake inputs cannot traverse links".into());
        }
    }
    if !path.canonicalize().map_err(err)?.starts_with(root) {
        return Err("Lake input escapes its retained root".into());
    }
    Ok(path)
}

fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

fn copy_file(source: &Path, destination: &Path, expected: &identity::FileIdentity) -> Result<()> {
    fs::create_dir_all(destination.parent().ok_or("Lake input has no parent")?).map_err(err)?;
    let bytes = read_bounded(source, MAX_FILE)?;
    if bytes.len() as u64 != expected.bytes || sha256(&bytes) != expected.sha256 {
        return Err("Lake input changed during capture".into());
    }
    write_new(destination, &bytes)?;
    #[cfg(unix)]
    fs::set_permissions(
        destination,
        fs::metadata(source).map_err(err)?.permissions(),
    )
    .map_err(err)?;
    Ok(())
}

fn compiled_file(path: &Path) -> bool {
    let file = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    [
        ".olean",
        ".olean.private",
        ".olean.server",
        ".ilean",
        ".ir",
        ".setup.json",
    ]
    .iter()
    .any(|extension| file.ends_with(extension))
}

fn command(project: &Path, executable: &Path, resolve: bool) -> Result<Command> {
    let bin = executable
        .parent()
        .ok_or("Lean has no executable directory")?;
    let mut command = Command::new(bin.join(if cfg!(windows) { "lake.exe" } else { "lake" }));
    command
        .env_clear()
        .current_dir(project)
        .env(
            "LEAN_SYSROOT",
            bin.parent().ok_or("Lean installation has no root")?,
        )
        .env("LEAN_NUM_THREADS", "1")
        .env("LEAN_STACK_SIZE_KB", lean::STACK_KIB)
        .env("MIMALLOC_ARENA_RESERVE", lean::ARENA_KIB)
        .env("PATH", bin);
    if resolve {
        // Resolution may invoke Git for the manifest's exact revisions. Final
        // reconstruction uses captured paths and does not need those checkouts.
        if let Some(path) = std::env::var_os("PATH") {
            command.env("PATH", path);
        }
        for key in [
            "HOME",
            "USERPROFILE",
            "SSH_AUTH_SOCK",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_KEY_0",
            "GIT_CONFIG_VALUE_0",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
    }
    for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command.args(["--no-cache", "--rehash", "--quiet"]);
    Ok(command)
}

pub(super) fn compile_command(run: &Run, path: &Path) -> Result<Command> {
    let root = run
        .lake_project
        .as_ref()
        .ok_or("missing Lake project directory")?;
    let mut command = command(root, &run.executable, false)?;
    command
        .env("LEAN_PATH", &run.build)
        .env("LEAN_SRC_PATH", &run.build);
    command
        .arg("lean")
        .arg(path)
        .arg("--")
        .arg(format!("--root={}", run.build.display()))
        .args([
            "--json",
            "--threads=1",
            "--memory=6144",
            "-DmaxHeartbeats=200000",
            "-o",
        ])
        .arg(path.with_extension("olean"));
    Ok(command)
}

fn environment(project: &Path, run: &mut Run, resolve: bool) -> Result<NativeEnvironment> {
    let mut command = command(project, &run.executable, resolve)?;
    command
        .args(["env"])
        .arg(std::env::current_exe().map_err(err)?)
        .args(["lean-task", "_lake-environment"]);
    let output = run.execute("lake-environment", command)?;
    super::decode_json(&output.stdout)
}

fn build(project: &Path, target: &str, run: &mut Run) -> Result<Environment> {
    let root = project.join("root");
    let mut command = command(&root, &run.executable, false)?;
    command
        .arg("--reconfigure")
        .arg("build")
        .arg(format!("+{target}:olean"));
    run.execute("lake-build", command)?;
    let native = environment(&root, run, false)?;
    let toolchain = run
        .executable
        .parent()
        .and_then(Path::parent)
        .ok_or("Lean installation has no root")?
        .canonicalize()
        .map_err(err)?;
    let environment = Environment::capture(native, project, &toolchain)?;
    environment.apply(project, &toolchain, run)?;
    run.lake_project = Some(root);
    Ok(environment)
}

fn git_command(directory: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    // Process::start forwards only explicit variables. Git is needed only for
    // the initial capture, whose source revisions are retained in the task.
    command.current_dir(directory).args(args);
    for key in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SystemRoot",
        "WINDIR",
        "TEMP",
        "TMP",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_KEY_0",
        "GIT_CONFIG_VALUE_0",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
}

fn git_state(directory: &Path, run: &mut Run) -> Result<(Option<String>, Option<bool>)> {
    // A local project may live in an ignored scratch directory of an unrelated
    // enclosing repository. Its working sources then need filesystem capture;
    // the enclosing repository's empty index listing is not that project.
    let ignored = run.capture(
        "lake-git-ignore",
        git_command(directory, &["check-ignore", "--quiet", "."]),
    )?;
    if ignored.exit_code == Some(0) {
        return Ok((None, None));
    }
    let output = run.capture(
        "lake-git-revision",
        git_command(directory, &["rev-parse", "HEAD"]),
    )?;
    if output.exit_code != Some(0) {
        return Ok((None, None));
    }
    let revision = std::str::from_utf8(&output.stdout)
        .map_err(err)?
        .trim()
        .to_owned();
    if !hash(&revision, 40) {
        return Err("Git omitted an exact source commit".into());
    }
    let output = run.execute(
        "lake-git-status",
        git_command(
            directory,
            &["status", "--porcelain", "--untracked-files=all", "--", "."],
        ),
    )?;
    Ok((Some(revision), Some(output.stdout.is_empty())))
}

fn capture_files(
    directory: &Path,
    id: &str,
    git: bool,
    lake_dir: &str,
    output: &Path,
    run: &mut Run,
    remaining: CaptureBudget,
) -> Result<Vec<identity::FileIdentity>> {
    let mut paths = if git {
        let list = run
            .directory
            .join(format!("lake-files-{}.txt", run.stages.len()));
        let mut child = process::FileProcess::start_project(
            git_command(
                directory,
                &[
                    "-c",
                    "core.quotePath=false",
                    "ls-files",
                    "--cached",
                    "--others",
                    "--exclude-standard",
                    "--",
                    ".",
                ],
            ),
            vec![],
            Duration::from_secs(30),
            &list,
        )
        .map_err(|e| format!("Lake source listing: {e:?}"))?;
        let captured = child
            .wait()
            .as_ref()
            .map_err(|e| format!("Lake source listing: {e:?}"))?;
        if captured.output.exit_code != Some(0) || !captured.output.stderr.is_empty() {
            return Err("Git could not enumerate Lake source inputs".into());
        }
        let text = String::from_utf8(read_bounded(&list, 8 * 1024 * 1024)?).map_err(err)?;
        text.lines().map(str::to_owned).collect::<Vec<_>>()
    } else {
        let mut files = vec![];
        let mut pending = vec![directory.to_path_buf()];
        let mut nodes = 0;
        while let Some(path) = pending.pop() {
            for entry in fs::read_dir(&path).map_err(err)? {
                nodes += 1;
                if nodes > 65_536 {
                    return Err("Lake source directory entry bound exceeded".into());
                }
                let entry = entry.map_err(err)?;
                let path = entry.path();
                if path.starts_with(&run.directory)
                    || path.starts_with(directory.join(lake_dir))
                    || entry.file_name() == ".git"
                {
                    continue;
                }
                let relative = portable(path.strip_prefix(directory).map_err(err)?)?;
                let metadata = fs::symlink_metadata(&path).map_err(err)?;
                let kind = metadata.file_type();
                if linked(&metadata) {
                    return Err("Lake source inputs cannot be links".into());
                }
                if !path.canonicalize().map_err(err)?.starts_with(directory) {
                    return Err("Lake source input escapes its package".into());
                }
                if kind.is_dir() {
                    pending.push(path);
                } else if kind.is_file() {
                    files.push(relative);
                } else {
                    return Err("Lake source contains an unsupported filesystem entry".into());
                }
            }
        }
        files
    };
    // Pins are required inputs even when a project's ignore rules omit them.
    for pin in [
        "lean-toolchain",
        "lake-manifest.json",
        "lakefile.lean",
        "lakefile.toml",
    ] {
        if directory.join(pin).is_file() && !paths.iter().any(|path| path == pin) {
            paths.push(pin.into());
        }
    }
    paths.sort();
    paths.dedup();
    let mut result = vec![];
    let mut total = 0u64;
    for path in paths {
        if path.starts_with(&format!("{lake_dir}/"))
            || path.starts_with(".git/")
            || compiled_file(Path::new(&path))
        {
            continue;
        }
        if !relative(&path) {
            return Err("Git reported an unsupported source pathname".into());
        }
        // A deleted tracked file is not part of the selected working source.
        // Git dependency capture separately requires a clean locked checkout.
        if !directory.join(&path).try_exists().map_err(err)? {
            continue;
        }
        let source = checked_file(directory, &path)?;
        if source.starts_with(&run.directory) {
            continue;
        }
        let (bytes, sha256) = identity::file(&source, MAX_FILE).map_err(err)?;
        total = total.checked_add(bytes).ok_or("Lake input size overflow")?;
        if total > remaining.bytes || result.len() >= remaining.files {
            return Err("Lake input snapshot exceeds its remaining byte/file bound".into());
        }
        let identity = identity::FileIdentity {
            path: format!("{id}/{path}"),
            bytes,
            sha256,
            link_target: None,
        };
        copy_file(&source, &output.join(&identity.path), &identity)?;
        result.push(identity);
        if result.len() > MAX_FILES {
            return Err("Lake source file count exceeds its bound".into());
        }
    }
    Ok(result)
}

pub(super) fn capture(
    project: &Path,
    configuration: Configuration,
    run: &mut Run,
) -> Result<(Manifest, Vec<Source>, Snapshot)> {
    configuration.validate()?;
    let toolchain_text =
        String::from_utf8(read_bounded(&project.join("lean-toolchain"), 1024)?).map_err(err)?;
    if ![
        format!("v{}", lean::version()),
        format!("leanprover/lean4:v{}", lean::version()),
    ]
    .contains(&toolchain_text.trim().to_owned())
    {
        return Err("Lake project must pin the selected Lean toolchain".into());
    }
    let lock_path = project.join("lake-manifest.json");
    let lock_before = read_bounded(&lock_path, 1024 * 1024)?;
    let lock: Value = super::decode_json(&lock_before)?;
    if text_field(&lock, "version")? != "1.2.0" {
        return Err("unsupported native Lake manifest version".into());
    }
    let lake_dir = lock
        .get("lakeDir")
        .and_then(Value::as_str)
        .unwrap_or(".lake")
        .to_owned();
    let packages_dir = lock
        .get("packagesDir")
        .and_then(Value::as_str)
        .unwrap_or(".lake/packages");
    if !relative(&lake_dir) || !relative(packages_dir) {
        return Err("Lake metadata directories must stay within the selected project".into());
    }
    let entries = lock
        .get("packages")
        .and_then(Value::as_array)
        .ok_or("Lake lock omitted packages")?;
    if entries.len() > 64 {
        return Err("Lake package count exceeds 64 dependencies".into());
    }
    let mut effective = BTreeMap::new();
    for entry in entries {
        if effective
            .insert(text_field(entry, "name")?.to_owned(), entry.clone())
            .is_some()
        {
            return Err("duplicate Lake dependency name".into());
        }
    }
    let original_overrides = project.join(&lake_dir).join("package-overrides.json");
    if original_overrides.exists() {
        let overrides: Value = read_json(&original_overrides, 1024 * 1024)?;
        let entries = overrides
            .get("packages")
            .and_then(Value::as_array)
            .ok_or("Lake overrides omitted package entries")?;
        if entries.len() > 64 {
            return Err("Lake override count exceeds 64".into());
        }
        for entry in entries.iter().cloned() {
            effective.insert(text_field(&entry, "name")?.to_owned(), entry);
        }
    }
    if effective.len() > 64 {
        return Err("Lake override package count exceeds 64".into());
    }
    // Ask Lake to materialize its exact lock before inspecting those sources.
    let _ = environment(project, run, true)?;
    if read_bounded(&lock_path, 1024 * 1024)? != lock_before {
        return Err("Lake changed its selected dependency lock during resolution".into());
    }
    let retained = run.directory.join(INPUTS);
    fs::create_dir(&retained).map_err(err)?;
    let mut selected = vec![(
        text_field(&lock, "name")?.to_owned(),
        "root".to_owned(),
        project.to_path_buf(),
        json!({"type":"root"}),
        lake_dir.clone(),
    )];
    for (index, (name, entry)) in effective.into_iter().enumerate() {
        if name.is_empty()
            || name.len() > 256
            || name.contains(['/', '\\', ':'])
            || name.chars().any(char::is_control)
        {
            return Err("unsupported Lake package name".into());
        }
        let directory = match text_field(&entry, "type")? {
            "git" => {
                if !hash(text_field(&entry, "rev")?, 40) {
                    return Err("Lake dependency requires a full Git commit".into());
                }
                let mut path = project.join(packages_dir).join(&name);
                if let Some(subdir) = entry.get("subDir").and_then(Value::as_str) {
                    if !relative(subdir) {
                        return Err("Lake Git package subdirectory escapes its checkout".into());
                    }
                    path.push(subdir);
                }
                path
            }
            "path" => project.join(text_field(&entry, "dir")?),
            _ => return Err("unsupported Lake dependency source".into()),
        }
        .canonicalize()
        .map_err(err)?;
        let manifest_path = entry
            .get("manifestFile")
            .and_then(Value::as_str)
            .unwrap_or("lake-manifest.json");
        if !relative(manifest_path) {
            return Err("invalid package manifest path".into());
        }
        let manifest = directory.join(manifest_path);
        let package_lake_dir = if manifest.exists() {
            let data: Value = read_json(&manifest, 1024 * 1024)?;
            data.get("lakeDir")
                .and_then(Value::as_str)
                .unwrap_or(".lake")
                .to_owned()
        } else {
            ".lake".into()
        };
        if !relative(&package_lake_dir) {
            return Err("invalid package Lake directory".into());
        }
        selected.push((
            name,
            format!("packages/p{}", index + 1),
            directory,
            entry,
            package_lake_dir,
        ));
    }
    let mut packages = vec![];
    let mut files = vec![];
    let mut total = 0u64;
    for (name, id, directory, origin, package_lake_dir) in selected {
        let (git_commit, git_clean) = git_state(&directory, run)?;
        if origin.get("type").and_then(Value::as_str) == Some("git")
            && (git_commit.as_deref() != Some(text_field(&origin, "rev")?)
                || git_clean != Some(true))
        {
            return Err(
                "Lake dependency differs from its locked Git revision or has local changes".into(),
            );
        }
        let captured = capture_files(
            &directory,
            &id,
            git_commit.is_some(),
            &package_lake_dir,
            &retained,
            run,
            CaptureBudget {
                bytes: MAX_TOTAL - total,
                files: MAX_FILES - files.len(),
            },
        )?;
        for file in &captured {
            total = total
                .checked_add(file.bytes)
                .ok_or("Lake source size overflow")?;
        }
        files.extend(captured);
        if total > MAX_TOTAL || files.len() > MAX_FILES {
            return Err("Lake input snapshot exceeds 512 MiB or 32768 files".into());
        }
        packages.push(Package {
            name,
            directory: id,
            origin,
            git_commit,
            git_clean,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut snapshot = Snapshot {
        schema: "nmlt-lean-lake-snapshot-v1".into(),
        configuration: configuration.clone(),
        packages,
        files,
        root_lake_dir: lake_dir,
        target_source: String::new(),
        environment: Environment {
            lean_path: vec![],
            lean_src_path: vec![],
            library_path: vec![],
        },
    };
    let project = run.directory.join(PROJECT);
    snapshot.materialize(&retained, &project)?;
    snapshot.environment = build(&project, &configuration.target_module, run)?;
    let root = project.join("root");
    let mut query = command(&root, &run.executable, false)?;
    query
        .args(["--json", "query"])
        .arg(format!("+{}:lean", configuration.target_module));
    let output = run.execute("lake-target-source", query)?;
    let path: String = super::decode_json(&output.stdout)?;
    let path = root.join(path).canonicalize().map_err(err)?;
    snapshot.target_source = portable(path.strip_prefix(&project).map_err(err)?)?;
    let text = String::from_utf8(read_bounded(&path, super::MAX_SOURCE)?).map_err(err)?;
    let manifest = configuration.manifest();
    let sources = vec![Source {
        module: configuration.target_module,
        text,
    }];
    snapshot.validate(&manifest, &sources)?;
    snapshot.verify_files(&retained)?;
    snapshot.verify_files(&project)?;
    write_json(&run.directory.join("lake-inputs.json"), &snapshot)?;
    Ok((manifest, sources, snapshot))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Manifest, Vec<Source>, Snapshot) {
        let configuration = Configuration {
            schema: "nmlt-lean-project-v3".into(),
            target_module: "App.Goals".into(),
            target: "App.goal".into(),
            permitted_axioms: vec![],
        };
        let source = Source {
            module: "App.Goals".into(),
            text: "theorem App.goal : True := True.intro\n".into(),
        };
        let mut files = [
            ("root/App/Goals.lean", source.text.as_bytes()),
            (
                "root/lean-toolchain",
                b"leanprover/lean4:v4.33.1\n".as_slice(),
            ),
            ("root/lake-manifest.json", b"{}".as_slice()),
        ]
        .into_iter()
        .map(|(path, text)| identity::FileIdentity {
            path: path.into(),
            bytes: text.len() as u64,
            sha256: sha256(text),
            link_target: None,
        })
        .collect::<Vec<_>>();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let snapshot = Snapshot {
            schema: "nmlt-lean-lake-snapshot-v1".into(),
            configuration: configuration.clone(),
            packages: vec![Package {
                name: "app".into(),
                directory: "root".into(),
                origin: json!({"type":"root"}),
                git_commit: None,
                git_clean: None,
            }],
            files,
            root_lake_dir: ".lake".into(),
            target_source: "root/App/Goals.lean".into(),
            environment: Environment {
                lean_path: vec![SearchRoot::Project {
                    path: "root/.lake/build/lib/lean".into(),
                }],
                lean_src_path: vec![SearchRoot::Project {
                    path: "root".into(),
                }],
                library_path: vec![],
            },
        };
        (configuration.manifest(), vec![source], snapshot)
    }

    #[test]
    fn lake_inputs_bind_the_target_and_reject_paths_caches_and_false_revision_metadata() {
        let (manifest, sources, snapshot) = fixture();
        snapshot.validate(&manifest, &sources).unwrap();
        for path in [
            "../escape",
            "/absolute",
            "C:/absolute",
            "root/../escape",
            "root//file",
            "root\\file",
            "root/line\nfile",
        ] {
            let mut changed = snapshot.clone();
            changed.files[0].path = path.into();
            assert!(changed.validate(&manifest, &sources).is_err(), "{path}");
        }
        let mut changed = snapshot.clone();
        changed.files[0].sha256 = "0".repeat(64);
        assert!(changed.validate(&manifest, &sources).is_err());
        let mut changed = snapshot.clone();
        changed.files[0].link_target = Some("other".into());
        assert!(changed.validate(&manifest, &sources).is_err());
        let mut changed = snapshot.clone();
        changed.files[0].path = "root/App/Goals.olean".into();
        assert!(changed.validate(&manifest, &sources).is_err());
        let mut changed = snapshot.clone();
        changed.packages.push(Package {
            name: "dep".into(),
            directory: "packages/p1".into(),
            origin: json!({"type":"git", "name":"dep", "rev":"a".repeat(40)}),
            git_commit: Some("b".repeat(40)),
            git_clean: Some(true),
        });
        assert!(changed.validate(&manifest, &sources).is_err());
        changed.packages[1].git_commit = Some("a".repeat(40));
        changed.packages[1].git_clean = Some(false);
        assert!(changed.validate(&manifest, &sources).is_err());
    }

    #[test]
    fn source_capture_enforces_the_total_budget_before_writing_the_next_file() {
        let parent = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/lake-input-tests");
        fs::create_dir_all(&parent).unwrap();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = parent.join(format!("{}-{stamp}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let source = root.join("source");
        let output = root.join("inputs");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(source.join("a.lean"), b"123456").unwrap();
        fs::write(source.join("b.lean"), b"789012").unwrap();
        let mut run = Run::new(&root.join("run"), &std::env::current_exe().unwrap()).unwrap();
        let result = capture_files(
            &source,
            "root",
            false,
            ".lake",
            &output,
            &mut run,
            CaptureBudget {
                bytes: 7,
                files: 10,
            },
        );
        assert!(result.is_err());
        assert_eq!(fs::read(output.join("root/a.lean")).unwrap(), b"123456");
        assert!(!output.join("root/b.lean").exists());
        let (manifest, sources, mut snapshot) = fixture();
        snapshot.files[0].bytes = MAX_FILE + 1;
        assert!(snapshot.validate(&manifest, &sources).is_err());
    }
}

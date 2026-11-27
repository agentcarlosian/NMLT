//! Pinned Lean / lean4export / nanoda discovery and one-command local bootstrap (RFC 0040).
use nmlt_runtime::{identity, lean};
use serde::{Deserialize, Serialize};
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const LEAN4EXPORT_REPO: &str = "https://github.com/leanprover/lean4export.git";
pub const LEAN4EXPORT_COMMIT: &str = "411dce7db58a3afc60ecab2d211acd1042b593dc";
pub const NANODA_REPO: &str = "https://github.com/ammkrn/nanoda_lib.git";
pub const NANODA_COMMIT: &str = "05055695879dfebb6628a67da88ceca6cd6b0421";
pub const NANODA_RUST_TOOLCHAIN: &str = "1.94.0";

const PROVENANCE_SCHEMA: &str = "nmlt-tools-provenance-v2";
const PROVENANCE_FILE: &str = "checker-provenance-v2.json";
const MAX_PROVENANCE_BYTES: u64 = 16 * 1024;
const MAX_CHECKER_BYTES: u64 = 256 * 1024 * 1024;

const HELP: &str = "\
Pinned Lean and independent proof-checker tool management (RFC 0040):
  nmlt tools status [--dir DIR] [--json]   Inspect resolved Lean, lean4export, and nanoda paths and digests
  nmlt tools setup  [--dir DIR] [--json]   Fetch and build pinned lean4export and nanoda into the local cache

Environment overrides:
  NMLT_TOOLS_DIR     Override default tool cache (~/.nmlt/tools/v<lean-version>)
  NMLT_LEAN_BIN      Override direct Lean executable path
  NMLT_EXPORTER_BIN  Override lean4export executable path
  NMLT_NANODA_BIN    Override nanoda executable path
";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryStatus {
    pub found: bool,
    pub path: Option<String>,
    pub sha256: Option<String>,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedCommits {
    pub lean_toolchain: String,
    pub lean4export: String,
    pub nanoda: String,
    pub rust_toolchain: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusReport {
    pub schema: String,
    pub ready: bool,
    pub cache_dir: String,
    pub pinned: PinnedCommits,
    pub lean: BinaryStatus,
    pub exporter: BinaryStatus,
    pub nanoda: BinaryStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    schema: String,
    lean_toolchain: String,
    lean4export_commit: String,
    nanoda_commit: String,
    rust_toolchain: String,
    rustc_version: String,
    generation: String,
    exporter_sha256: String,
    exporter_library_sha256: String,
    nanoda_sha256: String,
}

pub fn command(args: &[OsString], global_json: bool) -> Result<(), String> {
    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{HELP}");
        return Ok(());
    }
    let action = args[0].to_str().ok_or("tools subcommand must be UTF-8")?;
    let mut dir: Option<PathBuf> = None;
    let mut json = global_json;
    let mut rest = &args[1..];
    while !rest.is_empty() {
        match rest {
            [flag, tail @ ..] if flag == "--json" => {
                json = true;
                rest = tail;
            }
            [flag, value, tail @ ..] if flag == "--dir" => {
                if dir.is_some() {
                    return Err("duplicate --dir option".into());
                }
                dir = Some(PathBuf::from(value));
                rest = tail;
            }
            _ => return Err(format!("invalid tools option\n\n{HELP}")),
        }
    }
    let report = match action {
        "status" => status(dir.as_deref())?,
        "setup" => {
            let cache_dir = match dir {
                Some(d) => d,
                None => default_cache_dir()?,
            };
            setup(&cache_dir)?
        }
        _ => return Err(format!("unknown tools subcommand '{action}'\n\n{HELP}")),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
        );
    } else {
        print_status(&report);
    }
    Ok(())
}

fn print_status(report: &StatusReport) {
    println!("ready: {}", report.ready);
    println!("cache_dir: {}", report.cache_dir);
    println!("required_lean: {}", report.pinned.lean_toolchain);
    for (label, item) in [
        ("lean", &report.lean),
        ("lean4export", &report.exporter),
        ("nanoda", &report.nanoda),
    ] {
        if let Some(path) = &item.path {
            println!(
                "{label}: {path} ({}, sha256={})",
                item.source,
                item.sha256.as_deref().unwrap_or("unhashed")
            );
        } else {
            println!(
                "{label}: missing ({})",
                item.error.as_deref().unwrap_or("not found")
            );
        }
    }
    if !report.ready {
        println!("hint: run `nmlt tools setup` to build pinned lean4export and nanoda");
    }
}

pub fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("USERPROFILE")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("HOME")
                    .filter(|v| !v.is_empty())
                    .map(PathBuf::from)
            })
    }
    #[cfg(not(windows))]
    {
        env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    }
}

pub fn default_cache_dir() -> Result<PathBuf, String> {
    if let Some(override_dir) = env::var_os("NMLT_TOOLS_DIR").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(override_dir));
    }
    let home = home_dir().ok_or("cannot determine home directory; set NMLT_TOOLS_DIR")?;
    Ok(home
        .join(".nmlt")
        .join("tools")
        .join(format!("v{}", lean::version())))
}

pub fn cached_exporter_path(cache_dir: &Path) -> PathBuf {
    let name = if cfg!(windows) {
        "lean4export.exe"
    } else {
        "lean4export"
    };
    cache_dir
        .join("lean4export")
        .join(".lake")
        .join("build")
        .join("bin")
        .join(name)
}

pub fn cached_nanoda_path(cache_dir: &Path) -> PathBuf {
    let name = if cfg!(windows) {
        "nanoda_bin.exe"
    } else {
        "nanoda_bin"
    };
    cache_dir
        .join("nanoda")
        .join("target")
        .join("release")
        .join(name)
}

fn elan_direct_lean_path(version: &str) -> Option<PathBuf> {
    let elan_home = env::var_os("ELAN_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".elan")))?;
    let exe = if cfg!(windows) { "lean.exe" } else { "lean" };
    Some(
        elan_home
            .join("toolchains")
            .join(format!("leanprover--lean4---v{version}"))
            .join("bin")
            .join(exe),
    )
}

pub fn resolve_lean(explicit: Option<&Path>) -> Result<(PathBuf, String), String> {
    if let Some(path) = explicit {
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("cannot resolve --lean-bin '{}': {e}", path.display()))?;
        return Ok((canonical, "explicit".into()));
    }
    if let Some(env_path) = env::var_os("NMLT_LEAN_BIN").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(&env_path);
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("cannot resolve NMLT_LEAN_BIN '{}': {e}", path.display()))?;
        return Ok((canonical, "env:NMLT_LEAN_BIN".into()));
    }
    let version = lean::version();
    if let Some(direct) = elan_direct_lean_path(version)
        && direct.is_file()
        && let Ok(canonical) = direct.canonicalize()
    {
        return Ok((canonical, "elan-toolchain".into()));
    }
    let toolchain_spec = format!("leanprover/lean4:v{version}");
    if let Ok(output) = Command::new("elan")
        .args(["which", "--toolchain", &toolchain_spec, "lean"])
        .output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if !stdout.is_empty() {
            let candidate = PathBuf::from(stdout);
            if candidate.is_file()
                && let Ok(canonical) = candidate.canonicalize()
            {
                return Ok((canonical, "elan-which".into()));
            }
        }
    }
    Err(format!(
        "could not locate pinned Lean v{version}; install `elan toolchain install leanprover/lean4:v{version}` or pass --lean-bin <FILE>"
    ))
}

pub fn resolve_exporter(
    explicit: Option<&Path>,
    cache_override: Option<&Path>,
) -> Result<(PathBuf, String), String> {
    if let Some(path) = explicit {
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("cannot resolve --exporter '{}': {e}", path.display()))?;
        return Ok((canonical, "explicit".into()));
    }
    if let Some(env_path) = env::var_os("NMLT_EXPORTER_BIN").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(&env_path);
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("cannot resolve NMLT_EXPORTER_BIN '{}': {e}", path.display()))?;
        return Ok((canonical, "env:NMLT_EXPORTER_BIN".into()));
    }
    let cache_dir = match cache_override {
        Some(dir) => dir.to_path_buf(),
        None => default_cache_dir()?,
    };
    resolve_cached(&cache_dir, Checker::Exporter)
        .map(|path| (path, "verified-cache".into()))
        .map_err(|error| format!(
            "cannot verify cached exporter in '{}': {error}; run `nmlt tools setup` or pass --exporter <FILE>",
            cache_dir.display()
        ))
}

pub fn resolve_nanoda(
    explicit: Option<&Path>,
    cache_override: Option<&Path>,
) -> Result<(PathBuf, String), String> {
    if let Some(path) = explicit {
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("cannot resolve --nanoda '{}': {e}", path.display()))?;
        return Ok((canonical, "explicit".into()));
    }
    if let Some(env_path) = env::var_os("NMLT_NANODA_BIN").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(&env_path);
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("cannot resolve NMLT_NANODA_BIN '{}': {e}", path.display()))?;
        return Ok((canonical, "env:NMLT_NANODA_BIN".into()));
    }
    let cache_dir = match cache_override {
        Some(dir) => dir.to_path_buf(),
        None => default_cache_dir()?,
    };
    resolve_cached(&cache_dir, Checker::Nanoda)
        .map(|path| (path, "verified-cache".into()))
        .map_err(|error| format!(
            "cannot verify cached nanoda in '{}': {error}; run `nmlt tools setup` or pass --nanoda <FILE>",
            cache_dir.display()
        ))
}

fn inspect_binary(resolved: Result<(PathBuf, String), String>) -> BinaryStatus {
    match resolved {
        Ok((path, source)) => match identity::file(&path, 256 * 1024 * 1024) {
            Ok((_, digest)) => BinaryStatus {
                found: true,
                path: Some(path.display().to_string()),
                sha256: Some(digest),
                source,
                error: None,
            },
            Err(e) => BinaryStatus {
                found: false,
                path: Some(path.display().to_string()),
                sha256: None,
                source,
                error: Some(e.to_string()),
            },
        },
        Err(message) => BinaryStatus {
            found: false,
            path: None,
            sha256: None,
            source: "missing".into(),
            error: Some(message),
        },
    }
}

pub fn status(cache_override: Option<&Path>) -> Result<StatusReport, String> {
    let cache_dir = match cache_override {
        Some(dir) => dir.to_path_buf(),
        None => default_cache_dir()?,
    };
    let lean_status = inspect_binary(resolve_lean(None));
    let exporter_status = inspect_binary(resolve_exporter(None, Some(&cache_dir)));
    let nanoda_status = inspect_binary(resolve_nanoda(None, Some(&cache_dir)));
    let ready = lean_status.found && exporter_status.found && nanoda_status.found;
    Ok(StatusReport {
        schema: "nmlt-tools-status-v1".into(),
        ready,
        cache_dir: cache_dir.display().to_string(),
        pinned: PinnedCommits {
            lean_toolchain: format!("leanprover/lean4:v{}", lean::version()),
            lean4export: LEAN4EXPORT_COMMIT.into(),
            nanoda: NANODA_COMMIT.into(),
            rust_toolchain: NANODA_RUST_TOOLCHAIN.into(),
        },
        lean: lean_status,
        exporter: exporter_status,
        nanoda: nanoda_status,
    })
}

pub fn setup(cache_dir: &Path) -> Result<StatusReport, String> {
    if let Ok(meta) = fs::symlink_metadata(cache_dir)
        && meta.file_type().is_symlink()
    {
        return Err("tool cache directory cannot be a symlink".into());
    }
    fs::create_dir_all(cache_dir)
        .map_err(|e| format!("failed to create '{}': {e}", cache_dir.display()))?;
    let root = cache_dir
        .canonicalize()
        .map_err(|e| format!("failed to canonicalize '{}': {e}", cache_dir.display()))?;
    let (lean_bin, _) = resolve_lean(None)?;
    // A path override is explicit, but setup must still build with the advertised Lean pin.
    lean::Toolchain::open(&lean_bin).map_err(|e| e.to_string())?;
    if let Ok(prov) = read_provenance(&root)
        && verify_cached(&root, &prov, Checker::Exporter).is_ok()
        && verify_cached(&root, &prov, Checker::Nanoda).is_ok()
    {
        return status(Some(&root));
    }

    let lake_bin = resolve_lake_sibling(&lean_bin);
    let (use_rustup_override, rustc_bin, rustc_version) = resolve_pinned_rust()?;
    stage_install(&root, |generation| {
        let exporter_dir = generation.join("lean4export");
        fetch_pinned_git(LEAN4EXPORT_REPO, LEAN4EXPORT_COMMIT, &exporter_dir)?;
        fs::write(
            exporter_dir.join("lean-toolchain"),
            format!("{}\n", pinned_lean()),
        )
        .map_err(|e| format!("failed to write lean4export lean-toolchain: {e}"))?;
        run_checked(
            Command::new(&lake_bin)
                .arg("build")
                .current_dir(&exporter_dir),
            "lake build (lean4export)",
        )?;

        let nanoda_dir = generation.join("nanoda");
        fetch_pinned_git(NANODA_REPO, NANODA_COMMIT, &nanoda_dir)?;
        let cargo_toml = nanoda_dir.join("Cargo.toml");
        let existing = fs::read_to_string(&cargo_toml)
            .map_err(|e| format!("failed to read nanoda Cargo.toml: {e}"))?;
        if !existing.contains("[workspace]") {
            let mut file = OpenOptions::new()
                .append(true)
                .open(&cargo_toml)
                .map_err(|e| format!("failed to open nanoda Cargo.toml: {e}"))?;
            file.write_all(b"\n[workspace]\n")
                .map_err(|e| format!("failed to patch nanoda Cargo.toml: {e}"))?;
        }
        let mut cargo_cmd = Command::new("cargo");
        if use_rustup_override {
            cargo_cmd.arg(format!("+{NANODA_RUST_TOOLCHAIN}"));
        }
        cargo_cmd
            .args(["build", "--locked", "--release", "--manifest-path"])
            .arg(&cargo_toml)
            .current_dir(&nanoda_dir)
            .env("CARGO_TARGET_DIR", nanoda_dir.join("target"))
            .env("RUSTC", &rustc_bin)
            .env("RUSTC_WRAPPER", "")
            .env("RUSTC_WORKSPACE_WRAPPER", "");
        run_checked(&mut cargo_cmd, "build nanoda with pinned Cargo")?;
        Ok(rustc_version)
    })?;
    status(Some(&root))
}

fn pinned_lean() -> String {
    format!("leanprover/lean4:v{}", lean::version())
}

#[derive(Clone, Copy)]
enum Checker {
    Exporter,
    Nanoda,
}

impl Checker {
    fn path(self, generation: &Path) -> PathBuf {
        match self {
            Self::Exporter => cached_exporter_path(generation),
            Self::Nanoda => cached_nanoda_path(generation),
        }
    }
}

fn valid_generation(name: &str) -> bool {
    name.strip_prefix("install-").is_some_and(|suffix| {
        !suffix.is_empty()
            && suffix.len() <= 80
            && suffix.bytes().all(|c| c.is_ascii_digit() || c == b'-')
    })
}

fn read_provenance(root: &Path) -> Result<Provenance, String> {
    let path = root.join(PROVENANCE_FILE);
    let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > MAX_PROVENANCE_BYTES {
        return Err("provenance must be a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_PROVENANCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_PROVENANCE_BYTES {
        return Err("provenance exceeded its size bound".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn resolve_cached(root: &Path, checker: Checker) -> Result<PathBuf, String> {
    let provenance = read_provenance(root)?;
    verify_cached(root, &provenance, checker)
}

fn verify_cached(root: &Path, prov: &Provenance, checker: Checker) -> Result<PathBuf, String> {
    if prov.schema != PROVENANCE_SCHEMA
        || !valid_generation(&prov.generation)
        || prov.lean_toolchain != pinned_lean()
        || prov.lean4export_commit != LEAN4EXPORT_COMMIT
        || prov.nanoda_commit != NANODA_COMMIT
        || prov.rust_toolchain != NANODA_RUST_TOOLCHAIN
    {
        return Err("cache provenance does not match the pinned toolchain".into());
    }
    verify_rustc_matches_pin(&prov.rustc_version)?;
    let generation = root.join(&prov.generation);
    if !fs::symlink_metadata(&generation)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        return Err("cache generation must be a real directory".into());
    }
    let generation = generation.canonicalize().map_err(|e| e.to_string())?;
    let bin = checker.path(&generation);
    let resolved = bin.canonicalize().map_err(|e| e.to_string())?;
    if resolved != bin {
        return Err("cached checker must be in its generation at the expected path".into());
    }
    let recorded = match checker {
        Checker::Exporter => &prov.exporter_sha256,
        Checker::Nanoda => &prov.nanoda_sha256,
    };
    if &checker_digest(&resolved)? != recorded {
        return Err("cached checker digest does not match provenance".into());
    }
    if matches!(checker, Checker::Exporter)
        && crate::lean_task::exporter_library_digest(&resolved)? != prov.exporter_library_sha256
    {
        return Err("cached exporter library digest does not match provenance".into());
    }
    Ok(resolved)
}

fn checker_digest(path: &Path) -> Result<String, String> {
    identity::file(path, MAX_CHECKER_BYTES)
        .map(|(_, digest)| digest)
        .map_err(|e| e.to_string())
}

/// Build into a fresh generation at its final paths. A single atomic rename publishes
/// both tools together. Existing generations and unrecognized directories are never
/// modified, even if a build, verification or publication fails.
fn stage_install(
    root: &Path,
    build: impl FnOnce(&Path) -> Result<String, String>,
) -> Result<(), String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let mut counter = 0u32;
    let (name, generation) = loop {
        let name = format!("install-{}-{stamp}-{counter}", std::process::id());
        let generation = root.join(&name);
        match fs::create_dir(&generation) {
            Ok(()) => break (name, generation),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                counter = counter
                    .checked_add(1)
                    .ok_or("no free checker cache generation")?;
            }
            Err(e) => return Err(format!("cannot create checker staging directory: {e}")),
        }
    };
    let result = (|| {
        let rustc_version = build(&generation)?;
        let exporter = cached_exporter_path(&generation);
        let nanoda = cached_nanoda_path(&generation);
        let provenance = Provenance {
            schema: PROVENANCE_SCHEMA.into(),
            lean_toolchain: pinned_lean(),
            lean4export_commit: LEAN4EXPORT_COMMIT.into(),
            nanoda_commit: NANODA_COMMIT.into(),
            rust_toolchain: NANODA_RUST_TOOLCHAIN.into(),
            rustc_version,
            generation: name,
            exporter_sha256: checker_digest(&exporter)?,
            exporter_library_sha256: crate::lean_task::exporter_library_digest(&exporter)?,
            nanoda_sha256: checker_digest(&nanoda)?,
        };
        verify_cached(root, &provenance, Checker::Exporter)?;
        verify_cached(root, &provenance, Checker::Nanoda)?;
        let pending = generation.join("provenance.pending");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)
            .map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(&provenance).map_err(|e| e.to_string())?)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(pending, root.join(PROVENANCE_FILE))
            .map_err(|e| format!("could not publish verified checker generation: {e}"))
    })();
    if result.is_err() {
        // This is the new directory we exclusively created, not an existing cache.
        let _ = fs::remove_dir_all(generation);
    }
    result
}

fn verify_rustc_matches_pin(version_line: &str) -> Result<String, String> {
    let trimmed = version_line.trim();
    let expected_prefix = format!("rustc {NANODA_RUST_TOOLCHAIN} ");
    if trimmed.starts_with(&expected_prefix) || trimmed == format!("rustc {NANODA_RUST_TOOLCHAIN}")
    {
        Ok(trimmed.to_owned())
    } else {
        Err(format!(
            "pinned Rust toolchain {NANODA_RUST_TOOLCHAIN} is required to build nanoda (found '{trimmed}'); run `rustup toolchain install {NANODA_RUST_TOOLCHAIN}`"
        ))
    }
}

fn resolve_pinned_rust() -> Result<(bool, PathBuf, String), String> {
    let pin = format!("+{NANODA_RUST_TOOLCHAIN}");
    let override_works = Command::new("rustc")
        .args([pin.as_str(), "--version"])
        .output()
        .is_ok_and(|output| output.status.success());
    let mut query = Command::new("rustc");
    if override_works {
        query.arg(&pin);
    }
    let output = query
        .args(["--print", "sysroot"])
        .output()
        .map_err(|e| format!("failed to locate pinned rustc: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "pinned Rust toolchain {NANODA_RUST_TOOLCHAIN} is unavailable; run `rustup toolchain install {NANODA_RUST_TOOLCHAIN}`"
        ));
    }
    let sysroot = std::str::from_utf8(&output.stdout)
        .map_err(|e| e.to_string())?
        .trim();
    if !Path::new(sysroot).is_absolute() {
        return Err("rustc did not report an absolute sysroot".into());
    }
    let executable =
        Path::new(sysroot)
            .join("bin")
            .join(if cfg!(windows) { "rustc.exe" } else { "rustc" });
    let executable = executable
        .canonicalize()
        .map_err(|e| format!("cannot locate direct rustc: {e}"))?;
    let output = Command::new(&executable)
        .arg("--version")
        .output()
        .map_err(|e| format!("failed to verify direct rustc: {e}"))?;
    if !output.status.success() {
        return Err("direct rustc version probe failed".into());
    }
    let version = verify_rustc_matches_pin(&String::from_utf8_lossy(&output.stdout))?;
    Ok((override_works, executable, version))
}

fn resolve_lake_sibling(lean_bin: &Path) -> PathBuf {
    let lake_name = if cfg!(windows) { "lake.exe" } else { "lake" };
    if let Some(bin_dir) = lean_bin.parent() {
        let sibling = bin_dir.join(lake_name);
        if sibling.is_file() {
            return sibling;
        }
    }
    PathBuf::from(lake_name)
}

fn fetch_pinned_git(repo: &str, commit: &str, destination: &Path) -> Result<(), String> {
    run_checked(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(destination),
        "git init",
    )?;
    run_checked(
        Command::new("git")
            .arg("-C")
            .arg(destination)
            .args(["remote", "add", "origin", repo]),
        "git remote add",
    )?;
    run_checked(
        Command::new("git")
            .arg("-C")
            .arg(destination)
            .args(["fetch", "--quiet", "--depth", "1", "origin", commit]),
        "git fetch pinned commit",
    )?;
    run_checked(
        Command::new("git").arg("-C").arg(destination).args([
            "checkout",
            "--quiet",
            "--detach",
            "FETCH_HEAD",
        ]),
        "git checkout FETCH_HEAD",
    )?;
    let head = Command::new("git")
        .arg("-C")
        .arg(destination)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("git rev-parse HEAD failed: {e}"))?;
    if !head.status.success() {
        return Err("git rev-parse HEAD failed".into());
    }
    let actual = String::from_utf8_lossy(&head.stdout).trim().to_owned();
    if actual != commit {
        return Err(format!(
            "fetched git commit '{actual}' does not match pinned '{commit}'"
        ));
    }
    Ok(())
}

fn run_checked(command: &mut Command, label: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|e| format!("{label} failed to launch: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{label} failed (exit={:?}):\n{}{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(root: &Path, tag: &str) -> Result<String, String> {
        let exporter = cached_exporter_path(root);
        let library = exporter
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("lib/lean");
        let nanoda = cached_nanoda_path(root);
        fs::create_dir_all(exporter.parent().unwrap()).unwrap();
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(nanoda.parent().unwrap()).unwrap();
        fs::write(exporter, format!("exporter-{tag}")).unwrap();
        fs::write(library.join("Main.olean"), format!("library-{tag}")).unwrap();
        fs::write(nanoda, format!("nanoda-{tag}")).unwrap();
        Ok("rustc 1.94.0 (fixture)".into())
    }

    fn root(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("nmlt-tools-{label}-{}-{stamp}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    #[test]
    fn implicit_cache_requires_provenance_binaries_and_exporter_library() {
        let base = root("verify");
        fixture(&base, "legacy").unwrap();
        assert!(resolve_cached(&base, Checker::Exporter).is_err());
        assert!(resolve_cached(&base, Checker::Nanoda).is_err());
        stage_install(&base, |path| fixture(path, "valid")).unwrap();
        let exporter = resolve_cached(&base, Checker::Exporter).unwrap();
        let nanoda = resolve_cached(&base, Checker::Nanoda).unwrap();
        assert_eq!(fs::read(&exporter).unwrap(), b"exporter-valid");
        assert_eq!(fs::read(&nanoda).unwrap(), b"nanoda-valid");
        let (explicit, source) =
            resolve_exporter(Some(&cached_exporter_path(&base)), Some(&base)).unwrap();
        assert_eq!(source, "explicit");
        assert_eq!(fs::read(explicit).unwrap(), b"exporter-legacy");
        fs::write(
            exporter
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("lib/lean/Main.olean"),
            b"changed",
        )
        .unwrap();
        assert!(
            resolve_cached(&base, Checker::Exporter)
                .unwrap_err()
                .contains("library digest")
        );
        fs::write(&nanoda, b"changed").unwrap();
        assert!(
            resolve_cached(&base, Checker::Nanoda)
                .unwrap_err()
                .contains("digest")
        );
        assert_eq!(
            fs::read(cached_exporter_path(&base)).unwrap(),
            b"exporter-legacy"
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn installation_replaces_pointer_only_after_success_and_keeps_old_files() {
        let base = root("publish");
        fixture(&base, "unmanaged").unwrap();
        fs::write(base.join("provenance.json"), b"legacy user data").unwrap();
        stage_install(&base, |path| fixture(path, "old")).unwrap();
        let previous = fs::read(base.join(PROVENANCE_FILE)).unwrap();
        let old_bin = resolve_cached(&base, Checker::Nanoda).unwrap();

        assert_eq!(
            stage_install(&base, |path| {
                fixture(path, "incomplete")?;
                Err("simulated build failure".into())
            }),
            Err("simulated build failure".into())
        );
        assert!(
            stage_install(&base, |path| {
                let version = fixture(path, "unverified")?;
                fs::remove_file(cached_exporter_path(path)).unwrap();
                Ok(version)
            })
            .is_err()
        );
        assert_eq!(fs::read(base.join(PROVENANCE_FILE)).unwrap(), previous);
        assert_eq!(resolve_cached(&base, Checker::Nanoda).unwrap(), old_bin);
        assert_eq!(fs::read_dir(&base).unwrap().count(), 5);

        stage_install(&base, |path| fixture(path, "new")).unwrap();
        let new_bin = resolve_cached(&base, Checker::Nanoda).unwrap();
        assert_ne!(new_bin, old_bin);
        assert_eq!(fs::read(new_bin).unwrap(), b"nanoda-new");
        assert_eq!(fs::read(old_bin).unwrap(), b"nanoda-old");
        assert_eq!(
            fs::read(cached_nanoda_path(&base)).unwrap(),
            b"nanoda-unmanaged"
        );
        assert_eq!(
            fs::read(base.join("provenance.json")).unwrap(),
            b"legacy user data"
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn malformed_or_unsupported_provenance_cannot_resolve_checkers() {
        let base = root("provenance");
        stage_install(&base, |path| fixture(path, "valid")).unwrap();
        let mut provenance = read_provenance(&base).unwrap();
        provenance.generation = "../invalid".into();
        fs::write(
            base.join(PROVENANCE_FILE),
            serde_json::to_vec(&provenance).unwrap(),
        )
        .unwrap();
        assert!(
            resolve_cached(&base, Checker::Exporter)
                .unwrap_err()
                .contains("pinned toolchain")
        );
        fs::write(
            base.join(PROVENANCE_FILE),
            vec![b' '; (MAX_PROVENANCE_BYTES + 1) as usize],
        )
        .unwrap();
        assert!(
            resolve_cached(&base, Checker::Nanoda)
                .unwrap_err()
                .contains("bounded")
        );
        assert!(verify_rustc_matches_pin("rustc 1.94.0 (4a4ef493e 2026-03-02)").is_ok());
        assert!(verify_rustc_matches_pin("rustc 1.95.0 (other)").is_err());
        assert!(verify_rustc_matches_pin("rustc 1.94.0-nightly (other)").is_err());
        let _ = fs::remove_dir_all(base);
    }
}

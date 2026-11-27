//! Pinned Lean / lean4export / nanoda discovery and one-command local bootstrap (RFC 0040).
use nmlt_runtime::{identity, lean};
use serde::{Deserialize, Serialize};
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const LEAN4EXPORT_REPO: &str = "https://github.com/leanprover/lean4export.git";
pub const LEAN4EXPORT_COMMIT: &str = "411dce7db58a3afc60ecab2d211acd1042b593dc";
pub const NANODA_REPO: &str = "https://github.com/ammkrn/nanoda_lib.git";
pub const NANODA_COMMIT: &str = "05055695879dfebb6628a67da88ceca6cd6b0421";
pub const NANODA_RUST_TOOLCHAIN: &str = "1.94.0";

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
    exporter_path: String,
    exporter_sha256: String,
    nanoda_path: String,
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
    let cached = cached_exporter_path(&cache_dir);
    if cached.is_file() {
        let canonical = cached
            .canonicalize()
            .map_err(|e| format!("cannot resolve cached lean4export: {e}"))?;
        return Ok((canonical, "cache".into()));
    }
    Err(format!(
        "lean4export not found in '{}'; run `nmlt tools setup` or pass --exporter <FILE>",
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
    let cached = cached_nanoda_path(&cache_dir);
    if cached.is_file() {
        let canonical = cached
            .canonicalize()
            .map_err(|e| format!("cannot resolve cached nanoda: {e}"))?;
        return Ok((canonical, "cache".into()));
    }
    Err(format!(
        "nanoda not found in '{}'; run `nmlt tools setup` or pass --nanoda <FILE>",
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
    let lake_bin = resolve_lake_sibling(&lean_bin);
    let expected_lean = format!("leanprover/lean4:v{}", lean::version());

    let exporter_dir = root.join("lean4export");
    let exporter_bin = cached_exporter_path(&root);
    let nanoda_dir = root.join("nanoda");
    let nanoda_bin = cached_nanoda_path(&root);

    let cached_provenance = read_provenance(&root);
    let reuse_exporter =
        can_reuse_exporter(cached_provenance.as_ref(), &exporter_bin, &expected_lean);
    let reuse_nanoda = can_reuse_nanoda(cached_provenance.as_ref(), &nanoda_bin);

    if !reuse_exporter {
        if exporter_dir.exists() {
            fs::remove_dir_all(&exporter_dir).map_err(|e| {
                format!(
                    "failed to clean unverified or incomplete '{}': {e}",
                    exporter_dir.display()
                )
            })?;
        }
        fetch_pinned_git(LEAN4EXPORT_REPO, LEAN4EXPORT_COMMIT, &exporter_dir)?;
        let toolchain_text = format!("{expected_lean}\n");
        fs::write(exporter_dir.join("lean-toolchain"), toolchain_text)
            .map_err(|e| format!("failed to write lean4export lean-toolchain: {e}"))?;
        run_checked(
            Command::new(&lake_bin)
                .arg("build")
                .current_dir(&exporter_dir),
            "lake build (lean4export)",
        )?;
        if !exporter_bin.is_file() {
            return Err(format!(
                "lake build succeeded but '{}' was not produced",
                exporter_bin.display()
            ));
        }
    }

    let rustc_version = if reuse_nanoda {
        cached_provenance
            .as_ref()
            .expect("verified provenance")
            .rustc_version
            .clone()
    } else {
        let (use_rustup_override, actual_rustc) = resolve_pinned_rust()?;
        if nanoda_dir.exists() {
            fs::remove_dir_all(&nanoda_dir).map_err(|e| {
                format!(
                    "failed to clean unverified or incomplete '{}': {e}",
                    nanoda_dir.display()
                )
            })?;
        }
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
            .arg(&cargo_toml);
        run_checked(
            &mut cargo_cmd,
            &format!("cargo +{NANODA_RUST_TOOLCHAIN} build (nanoda)"),
        )?;
        if !nanoda_bin.is_file() {
            return Err(format!(
                "cargo build succeeded but '{}' was not produced",
                nanoda_bin.display()
            ));
        }
        actual_rustc
    };

    let exporter_sha256 = identity::file(&exporter_bin, 256 * 1024 * 1024)
        .map_err(|e| e.to_string())?
        .1;
    let nanoda_sha256 = identity::file(&nanoda_bin, 256 * 1024 * 1024)
        .map_err(|e| e.to_string())?
        .1;
    let provenance = Provenance {
        schema: "nmlt-tools-provenance-v1".into(),
        lean_toolchain: expected_lean,
        lean4export_commit: LEAN4EXPORT_COMMIT.into(),
        nanoda_commit: NANODA_COMMIT.into(),
        rust_toolchain: NANODA_RUST_TOOLCHAIN.into(),
        rustc_version,
        exporter_path: exporter_bin.display().to_string(),
        exporter_sha256,
        nanoda_path: nanoda_bin.display().to_string(),
        nanoda_sha256,
    };
    let bytes = serde_json::to_vec_pretty(&provenance).map_err(|e| e.to_string())?;
    fs::write(root.join("provenance.json"), bytes)
        .map_err(|e| format!("failed to write provenance.json: {e}"))?;
    status(Some(&root))
}

fn read_provenance(root: &Path) -> Option<Provenance> {
    let bytes = fs::read(root.join("provenance.json")).ok()?;
    let prov: Provenance = serde_json::from_slice(&bytes).ok()?;
    (prov.schema == "nmlt-tools-provenance-v1").then_some(prov)
}

fn can_reuse_exporter(
    provenance: Option<&Provenance>,
    exporter_bin: &Path,
    expected_lean: &str,
) -> bool {
    let Some(prov) = provenance else {
        return false;
    };
    if prov.lean_toolchain != expected_lean || prov.lean4export_commit != LEAN4EXPORT_COMMIT {
        return false;
    }
    if !exporter_bin.is_file() {
        return false;
    }
    identity::file(exporter_bin, 256 * 1024 * 1024)
        .map(|(_, digest)| digest == prov.exporter_sha256)
        .unwrap_or(false)
}

fn can_reuse_nanoda(provenance: Option<&Provenance>, nanoda_bin: &Path) -> bool {
    let Some(prov) = provenance else {
        return false;
    };
    if prov.nanoda_commit != NANODA_COMMIT
        || prov.rust_toolchain != NANODA_RUST_TOOLCHAIN
        || verify_rustc_matches_pin(&prov.rustc_version).is_err()
    {
        return false;
    }
    if !nanoda_bin.is_file() {
        return false;
    }
    identity::file(nanoda_bin, 256 * 1024 * 1024)
        .map(|(_, digest)| digest == prov.nanoda_sha256)
        .unwrap_or(false)
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

fn resolve_pinned_rust() -> Result<(bool, String), String> {
    if let Ok(output) = Command::new("rustc")
        .arg(format!("+{NANODA_RUST_TOOLCHAIN}"))
        .arg("--version")
        .output()
        && output.status.success()
    {
        let line = String::from_utf8_lossy(&output.stdout);
        return Ok((true, verify_rustc_matches_pin(&line)?));
    }
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|e| {
            format!(
                "failed to invoke rustc while verifying pinned Rust {NANODA_RUST_TOOLCHAIN}: {e}"
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "pinned Rust toolchain {NANODA_RUST_TOOLCHAIN} is unavailable; run `rustup toolchain install {NANODA_RUST_TOOLCHAIN}`"
        ));
    }
    let line = String::from_utf8_lossy(&output.stdout);
    Ok((false, verify_rustc_matches_pin(&line)?))
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

    #[test]
    fn cache_layout_and_explicit_overrides_resolve_deterministically() {
        let base = std::env::temp_dir().join(format!(
            "nmlt-tools-test-{}-{}",
            std::process::id(),
            nmlt_runtime::sha256(b"tools-unit-test")[..8].to_owned()
        ));
        let _ = fs::remove_dir_all(&base);
        let exporter = cached_exporter_path(&base);
        let nanoda = cached_nanoda_path(&base);
        fs::create_dir_all(exporter.parent().unwrap()).unwrap();
        fs::create_dir_all(nanoda.parent().unwrap()).unwrap();
        fs::write(&exporter, b"fake-exporter").unwrap();
        fs::write(&nanoda, b"fake-nanoda").unwrap();

        let (resolved_exp, exp_src) = resolve_exporter(None, Some(&base)).unwrap();
        assert_eq!(resolved_exp, exporter.canonicalize().unwrap());
        assert_eq!(exp_src, "cache");

        let (resolved_nan, nan_src) = resolve_nanoda(None, Some(&base)).unwrap();
        assert_eq!(resolved_nan, nanoda.canonicalize().unwrap());
        assert_eq!(nan_src, "cache");

        // Explicit flag overrides cache.
        let custom = base.join("custom-bin");
        fs::write(&custom, b"custom").unwrap();
        let (exp_override, src_override) = resolve_exporter(Some(&custom), Some(&base)).unwrap();
        assert_eq!(exp_override, custom.canonicalize().unwrap());
        assert_eq!(src_override, "explicit");

        let report = status(Some(&base)).unwrap();
        assert_eq!(report.schema, "nmlt-tools-status-v1");
        assert!(report.exporter.found);
        assert!(report.nanoda.found);
        assert_eq!(report.pinned.lean4export, LEAN4EXPORT_COMMIT);
        assert_eq!(report.pinned.nanoda, NANODA_COMMIT);

        // Missing cache fails closed with actionable setup message.
        let missing = base.join("empty");
        fs::create_dir_all(&missing).unwrap();
        let err = resolve_exporter(None, Some(&missing)).unwrap_err();
        assert!(err.contains("nmlt tools setup"));
        let err = resolve_nanoda(None, Some(&missing)).unwrap_err();
        assert!(err.contains("nmlt tools setup"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn cached_reuse_requires_verified_provenance_and_pinned_rust() {
        let base =
            std::env::temp_dir().join(format!("nmlt-tools-prov-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let exporter = cached_exporter_path(&base);
        let nanoda = cached_nanoda_path(&base);
        fs::create_dir_all(exporter.parent().unwrap()).unwrap();
        fs::create_dir_all(nanoda.parent().unwrap()).unwrap();
        fs::write(&exporter, b"pinned-exporter-bytes").unwrap();
        fs::write(&nanoda, b"pinned-nanoda-bytes").unwrap();

        let expected_lean = format!("leanprover/lean4:v{}", lean::version());
        // Pre-existing binaries with no provenance.json must never be blindly reused.
        assert!(!can_reuse_exporter(
            read_provenance(&base).as_ref(),
            &exporter,
            &expected_lean
        ));
        assert!(!can_reuse_nanoda(read_provenance(&base).as_ref(), &nanoda));

        let exp_sha = identity::file(&exporter, 1024 * 1024).unwrap().1;
        let nan_sha = identity::file(&nanoda, 1024 * 1024).unwrap().1;
        let valid = Provenance {
            schema: "nmlt-tools-provenance-v1".into(),
            lean_toolchain: expected_lean.clone(),
            lean4export_commit: LEAN4EXPORT_COMMIT.into(),
            nanoda_commit: NANODA_COMMIT.into(),
            rust_toolchain: NANODA_RUST_TOOLCHAIN.into(),
            rustc_version: format!("rustc {NANODA_RUST_TOOLCHAIN} (pinned-test)"),
            exporter_path: exporter.display().to_string(),
            exporter_sha256: exp_sha,
            nanoda_path: nanoda.display().to_string(),
            nanoda_sha256: nan_sha,
        };
        fs::write(
            base.join("provenance.json"),
            serde_json::to_vec_pretty(&valid).unwrap(),
        )
        .unwrap();
        assert!(can_reuse_exporter(
            read_provenance(&base).as_ref(),
            &exporter,
            &expected_lean
        ));
        assert!(can_reuse_nanoda(read_provenance(&base).as_ref(), &nanoda));

        // Tampering a cached binary invalidates reuse immediately.
        fs::write(&nanoda, b"tampered-nanoda-bytes").unwrap();
        assert!(!can_reuse_nanoda(read_provenance(&base).as_ref(), &nanoda));

        // Unpinned Rust versions fail closed.
        assert!(verify_rustc_matches_pin("rustc 1.94.0 (4a4ef493e 2026-03-02)").is_ok());
        assert!(verify_rustc_matches_pin("rustc 1.95.0 (other)").is_err());
        assert!(verify_rustc_matches_pin("rustc 1.94.0-nightly (other)").is_err());

        let _ = fs::remove_dir_all(&base);
    }
}

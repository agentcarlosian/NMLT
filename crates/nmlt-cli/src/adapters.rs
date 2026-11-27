//! TOML configuration loader for pluggable subprocess adapters (`[[adapters]]`, RFC 0042).
use nmlt_runtime::custom_adapter;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_CONFIG_BYTES: u64 = 131_072;
const DEFAULT_TIMEOUT_MS: u64 = 10_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdapterSpec {
    pub alias: String,
    pub executable: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub inputs: Vec<PathBuf>,
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

#[derive(Deserialize)]
struct AdaptersFile {
    #[serde(default)]
    adapters: Vec<AdapterSpec>,
}

pub(crate) fn open_specs(
    base_dir: &Path,
    specs: &[AdapterSpec],
) -> Result<custom_adapter::Registry, String> {
    if specs.is_empty() {
        return Err("adapter configuration declares no [[adapters]] entries".into());
    }
    let resolved = specs
        .iter()
        .map(|spec| {
            let executable = if spec.executable.is_absolute() {
                spec.executable.clone()
            } else {
                base_dir.join(&spec.executable)
            };
            let inputs = spec
                .inputs
                .iter()
                .map(|input| {
                    let path = if input.is_absolute() {
                        input.clone()
                    } else {
                        base_dir.join(input)
                    };
                    let label = input.to_string_lossy().replace('\\', "/");
                    custom_adapter::InputSpec { label, path }
                })
                .collect();
            custom_adapter::Spec {
                alias: spec.alias.clone(),
                executable,
                args: spec.args.clone(),
                timeout_ms: spec.timeout_ms,
                inputs,
            }
        })
        .collect();
    custom_adapter::Registry::open(resolved).map_err(|e| e.to_string())
}

pub(crate) fn load_file(path: &Path) -> Result<custom_adapter::Registry, String> {
    let meta = fs::symlink_metadata(path)
        .map_err(|e| format!("cannot open adapters config '{}': {e}", path.display()))?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_CONFIG_BYTES {
        return Err("adapters configuration must be a regular file <= 128 KiB".into());
    }
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    let base_dir = canonical
        .parent()
        .ok_or("adapters configuration has no parent directory")?;
    let text = fs::read_to_string(&canonical).map_err(|e| e.to_string())?;
    let parsed: AdaptersFile = toml::from_str(&text)
        .map_err(|e| format!("invalid adapters TOML '{}': {e}", path.display()))?;
    open_specs(base_dir, &parsed.adapters)
}

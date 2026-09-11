use super::{EXPORT_PATH, Result, Run, decode_json, err, read_bounded, write_json, write_new};
use nmlt_runtime::{identity, lean, process, sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Identity {
    format: String,
    exporter_sha256: String,
    exporter_library_sha256: String,
    nanoda_sha256: String,
    export_adapter_sha256: String,
}
pub(super) struct Tools {
    pub identity: Identity,
    exporter: PathBuf,
    nanoda: PathBuf,
}
const ADAPTER: &str = "lean4export NMLTProof -- NMLTChecked.result; byte-exact raw stdout file <= 16777216 bytes; SHA-256 and byte-count receipt; stderr <= 65536 bytes";
impl Tools {
    pub fn open(exporter: &Path, nanoda: &Path) -> Result<Self> {
        let exporter = exporter.canonicalize().map_err(err)?;
        let nanoda = nanoda.canonicalize().map_err(err)?;
        let library_root = exporter
            .parent()
            .and_then(Path::parent)
            .ok_or("exporter must use the Lake build/bin layout")?
            .to_path_buf();
        let library = identity::tree(&library_root, &["lib"]).map_err(err)?;
        if !library.iter().any(|f| f.path == "lib/lean/Main.olean") {
            return Err("exporter library lacks Main.olean".into());
        }
        let identity = Identity {
            format: "lean4export-ndjson-3.1.0".into(),
            exporter_sha256: identity::file(&exporter, 256 * 1024 * 1024).map_err(err)?.1,
            exporter_library_sha256: sha256(&serde_json::to_vec(&library).map_err(err)?),
            nanoda_sha256: identity::file(&nanoda, 256 * 1024 * 1024).map_err(err)?.1,
            export_adapter_sha256: sha256(ADAPTER.as_bytes()),
        };
        Ok(Self {
            identity,
            exporter,
            nanoda,
        })
    }
    pub fn verify_unchanged(&self) -> Result<()> {
        if Self::open(&self.exporter, &self.nanoda)?.identity != self.identity {
            return Err("exporter/checker changed during proof checking".into());
        }
        Ok(())
    }
    pub fn check(&self, run: &mut Run, axioms: &[String]) -> Result<Checked> {
        let export_file = run.directory.join(EXPORT_PATH);
        write_new(&run.build.join("roots.txt"), b"NMLTChecked.result\n")?;
        let environment = run.command()?;
        let mut command = std::process::Command::new(&self.exporter);
        command
            .env_clear()
            .current_dir(&run.build)
            .args(["NMLTProof", "--", "NMLTChecked.result"]);
        for (key, value) in environment.get_envs() {
            if let Some(value) = value {
                command.env(key, value);
            }
        }
        let captured = run.execute_export(command)?;
        let bytes = read_bounded(&export_file, process::FILE_BYTES)?;
        let digest = sha256(&bytes);
        if captured.bytes != bytes.len() as u64 || captured.sha256 != digest {
            return Err("export file differs from its completed raw capture".into());
        }
        let declarations = inspect_export(&bytes)?;
        let config = json!({
            "export_file_path": "environment.ndjson", "use_stdin": false,
            "permitted_axioms": axioms, "unpermitted_axiom_hard_error": true,
            "unsafe_permit_all_axioms": false, "nat_extension": true,
            "string_extension": true, "print_axioms": false, "print_success_message": true
        });
        let config_path = run.build.join("nanoda-config.json");
        write_json(&config_path, &config)?;
        let command = run.command()?;
        // Preserve the explicit, bounded environment while selecting the independent executable.
        let mut checker = std::process::Command::new(&self.nanoda);
        checker
            .env_clear()
            .current_dir(&run.build)
            .arg(&config_path);
        for (key, value) in command.get_envs() {
            if let Some(value) = value {
                checker.env(key, value);
            }
        }
        let output = run.execute("nanoda", checker)?;
        let count = success_count(&output.stdout, &output.stderr)?;
        if count != declarations.len() as u64
            || identity::file(&export_file, process::FILE_BYTES).map_err(err)?
                != (captured.bytes, digest.clone())
        {
            return Err("independent checker count or exported input changed".into());
        }
        Ok(Checked {
            sha256: digest,
            declarations,
            count,
            bytes: captured.bytes,
        })
    }
}
pub(super) struct Checked {
    pub sha256: String,
    pub declarations: Vec<String>,
    pub count: u64,
    pub bytes: u64,
}

fn success_count(stdout: &[u8], stderr: &[u8]) -> Result<u64> {
    if !stderr.is_empty() {
        return Err("independent checker emitted unexpected diagnostics".into());
    }
    let text = std::str::from_utf8(stdout).map_err(err)?.trim();
    let count = text
        .strip_prefix("Checked ")
        .and_then(|s| s.strip_suffix(" declarations with no errors"))
        .and_then(|n| n.parse::<u64>().ok())
        .filter(|n| *n > 0);
    count.ok_or_else(|| "independent checker did not report an exact successful check".into())
}

fn inspect_export(bytes: &[u8]) -> Result<Vec<String>> {
    let text = std::str::from_utf8(bytes).map_err(err)?;
    let mut names = BTreeMap::from([(0, String::new())]);
    let mut expressions = BTreeMap::new();
    let mut declarations = BTreeSet::new();
    let mut metadata = false;
    let mut root = false;
    for (index, row) in text.lines().enumerate() {
        if index >= 500_000 || row.len() > 1024 * 1024 {
            return Err("export exceeds structural limits".into());
        }
        let value: Value = decode_json(row.as_bytes())?;
        if let Some(meta) = value.get("meta") {
            if index != 0
                || meta["format"]["version"] != "3.1.0"
                || meta["lean"]["version"] != lean::version()
            {
                return Err("unsupported export metadata".into());
            }
            metadata = true;
        }
        if let Some(id) = value.get("in").and_then(Value::as_u64) {
            let part = value
                .get("str")
                .or_else(|| value.get("num"))
                .ok_or("invalid exported name")?;
            let prefix = names
                .get(&number(&part["pre"])?)
                .ok_or("undefined exported name prefix")?;
            let suffix = part
                .get("str")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| part.get("i").and_then(Value::as_u64).map(|n| n.to_string()))
                .ok_or("invalid exported name component")?;
            let name = if prefix.is_empty() {
                suffix
            } else {
                format!("{prefix}.{suffix}")
            };
            if name.len() > 4096 || names.insert(id, name).is_some() {
                return Err("duplicate or oversized exported name".into());
            }
        }
        if let Some(id) = value.get("ie").and_then(Value::as_u64)
            && expressions
                .insert(
                    id,
                    value
                        .get("const")
                        .and_then(|v| v.get("name"))
                        .and_then(Value::as_u64),
                )
                .is_some()
        {
            return Err("duplicate exported expression".into());
        }
        for kind in ["axiom", "def", "thm", "opaque", "quot"] {
            if let Some(decl) = value.get(kind) {
                let name = declaration(decl, &names, &mut declarations)?;
                if name == "NMLTChecked.result" {
                    let expr = expressions
                        .get(&number(&decl["type"])?)
                        .ok_or("missing root type")?;
                    let target = expr.and_then(|id| names.get(&id));
                    if kind != "thm"
                        || target.map(String::as_str) != Some("NMLTTask.target")
                        || root
                    {
                        return Err("independent export has the wrong proof root or target".into());
                    }
                    root = true;
                }
            }
        }
        if let Some(inductive) = value.get("inductive") {
            for kind in ["types", "ctors", "recs"] {
                for decl in inductive[kind]
                    .as_array()
                    .ok_or("invalid exported inductive block")?
                {
                    declaration(decl, &names, &mut declarations)?;
                }
            }
        }
    }
    if !metadata || !root || !declarations.contains("NMLTTask.target") {
        return Err("independent export is empty or missing its fixed root/target".into());
    }
    Ok(declarations.into_iter().collect())
}
fn number(value: &Value) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| "invalid exported numeric reference".into())
}
fn declaration<'a>(
    value: &Value,
    names: &'a BTreeMap<u64, String>,
    declarations: &mut BTreeSet<String>,
) -> Result<&'a str> {
    let name = names
        .get(&number(&value["name"])?)
        .ok_or("undefined exported declaration name")?;
    if !declarations.insert(name.clone()) {
        return Err("duplicate exported declaration".into());
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_success_requires_exact_nonzero_report() {
        assert_eq!(
            success_count(b"Checked 45 declarations with no errors\n", b"").unwrap(),
            45
        );
        for text in [
            "Checked 0 declarations with no errors",
            "Checked 45 declarations with no errors, skipping exported but unpermitted axioms []",
            "Checked 45 declarations with no typechecker errors",
            "",
        ] {
            assert!(success_count(text.as_bytes(), b"").is_err());
        }
        assert!(success_count(b"Checked 45 declarations with no errors", b"warning").is_err());
    }
    #[test]
    fn empty_wrong_root_and_duplicate_exports_fail() {
        assert!(inspect_export(b"").is_err());
        assert!(inspect_export(b"{\"meta\":{\"format\":{\"version\":\"3.1.0\"},\"lean\":{\"version\":\"4.33.1\"}}}\n").is_err());
        assert!(inspect_export(b"{\"in\":1,\"in\":2}").is_err());
    }

    #[test]
    fn root_inspection_binds_the_named_proposition_before_kernel_checking() {
        // This tests export routing only. It is not a valid proof or a kernel fixture.
        let text = concat!(
            "{\"meta\":{\"format\":{\"version\":\"3.1.0\"},\"lean\":{\"version\":\"4.33.1\"}}}\n",
            "{\"in\":1,\"str\":{\"pre\":0,\"str\":\"NMLTTask\"}}\n",
            "{\"in\":2,\"str\":{\"pre\":1,\"str\":\"target\"}}\n",
            "{\"ie\":0,\"sort\":0}\n",
            "{\"def\":{\"name\":2,\"type\":0,\"value\":0}}\n",
            "{\"in\":3,\"str\":{\"pre\":0,\"str\":\"NMLTChecked\"}}\n",
            "{\"in\":4,\"str\":{\"pre\":3,\"str\":\"result\"}}\n",
            "{\"ie\":1,\"const\":{\"name\":2,\"us\":[]}}\n",
            "{\"thm\":{\"name\":4,\"type\":1,\"value\":0}}\n"
        );
        assert_eq!(inspect_export(text.as_bytes()).unwrap().len(), 2);
        assert!(inspect_export(text.replace("\"result\"", "\"other\"").as_bytes()).is_err());
        assert!(inspect_export(text.replace("\"type\":1", "\"type\":0").as_bytes()).is_err());
        assert!(inspect_export(text.replace("\"target\"", "\"changed\"").as_bytes()).is_err());
    }
}

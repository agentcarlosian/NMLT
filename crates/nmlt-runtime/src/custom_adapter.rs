//! Pluggable pinned subprocess adapters speaking `nmlt-job-response-v1` (RFC 0042).
use crate::{
    Adapter, Dispatch, Error, RESPONSE_SCHEMA, Response, ResponseOutcome, Value, ValueType, lean,
    process, sha256,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const IDENTITY_SCHEMA: &str = "nmlt-custom-adapters-v1";
pub const REQUEST_SCHEMA: &str = "nmlt-custom-adapter-request-v1";
pub const MAX_ADAPTERS: usize = 16;
pub const MAX_ARGS: usize = 16;
pub const MAX_ARG_BYTES: usize = 512;
pub const MAX_TIMEOUT_MS: u64 = 30_000;
pub const MAX_INPUT_BYTES: usize = 3072;
pub const MAX_IDENTITY_BYTES: usize = 32_768;

pub fn adapter() -> Adapter {
    Adapter {
        name: "custom-adapter".into(),
        version: 1,
        input_type: ValueType::Text,
        output_type: ValueType::Text,
    }
}

pub fn valid_alias(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.as_bytes()[0].is_ascii_alphabetic()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryIdentity {
    pub alias: String,
    pub executable_sha256: String,
    pub args: Vec<String>,
    pub timeout_ms: u64,
}

impl EntryIdentity {
    pub fn validate(&self) -> Result<(), Error> {
        if !valid_alias(&self.alias)
            || !crate::valid_digest(&self.executable_sha256)
            || self.args.len() > MAX_ARGS
            || self
                .args
                .iter()
                .any(|a| a.len() > MAX_ARG_BYTES || a.bytes().any(|b| b == 0))
            || !(1..=MAX_TIMEOUT_MS).contains(&self.timeout_ms)
        {
            return Err(Error("invalid custom adapter entry identity".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub schema: String,
    pub process_contract: String,
    pub entries: Vec<EntryIdentity>,
}

impl Identity {
    pub fn validate(&self) -> Result<(), Error> {
        if self.schema != IDENTITY_SCHEMA
            || self.process_contract != process::CONTRACT
            || !(1..=MAX_ADAPTERS).contains(&self.entries.len())
        {
            return Err(Error("invalid custom adapter registry identity".into()));
        }
        let mut seen = BTreeSet::new();
        for entry in &self.entries {
            entry.validate()?;
            if !seen.insert(&entry.alias) {
                return Err(Error(format!(
                    "duplicate custom adapter alias '{}'",
                    entry.alias
                )));
            }
        }
        if serde_json::to_vec(self)?.len() > MAX_IDENTITY_BYTES {
            return Err(Error(format!(
                "custom adapter registry identity exceeds {MAX_IDENTITY_BYTES}-byte serialized bound"
            )));
        }
        Ok(())
    }

    pub fn entry(&self, alias: &str) -> Result<&EntryIdentity, Error> {
        self.validate()?;
        self.entries
            .iter()
            .find(|e| e.alias == alias)
            .ok_or_else(|| Error(format!("custom adapter alias '{alias}' is not registered")))
    }

    pub fn request(&self, alias: &str, input: &str) -> Result<Request, Error> {
        let entry = self.entry(alias)?;
        let req = Request {
            schema: REQUEST_SCHEMA.into(),
            alias: alias.into(),
            executable_sha256: entry.executable_sha256.clone(),
            input: input.into(),
        };
        req.input()?;
        Ok(req)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub alias: String,
    pub executable_sha256: String,
    pub input: String,
}

impl Request {
    pub fn input(&self) -> Result<Value, Error> {
        if self.schema != REQUEST_SCHEMA
            || !valid_alias(&self.alias)
            || !crate::valid_digest(&self.executable_sha256)
            || self.input.len() > MAX_INPUT_BYTES
        {
            return Err(Error("invalid bounded custom adapter request".into()));
        }
        let encoded = serde_json::to_string(self)?;
        if encoded.len() > 4096 {
            return Err(Error(
                "custom adapter request exceeds 4096-byte protocol bound".into(),
            ));
        }
        Ok(Value::Text(encoded))
    }
}

pub fn request(identity: &Identity, dispatch: &Dispatch) -> Result<Request, Error> {
    let Value::Text(raw) = &dispatch.input else {
        return Err(Error(
            "custom adapter requires a canonical text request".into(),
        ));
    };
    let req: Request = serde_json::from_str(raw)?;
    if dispatch.binding.adapter != adapter()
        || dispatch.binding.input_sha256 != sha256(&serde_json::to_vec(&dispatch.input)?)
        || req.input()? != dispatch.input
        || identity.request(&req.alias, &req.input)? != req
    {
        return Err(Error("custom adapter dispatch binding mismatch".into()));
    }
    Ok(req)
}

pub fn response(
    identity: &Identity,
    dispatch: &Dispatch,
    output: &process::Output,
) -> Result<Response, Error> {
    request(identity, dispatch)?;
    if output.exit_code != Some(0) || !output.stderr.is_empty() {
        return Err(Error(
            "custom adapter did not return a clean response".into(),
        ));
    }
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|_| Error("custom adapter stdout is not UTF-8".into()))?
        .trim();
    let parsed: Response = serde_json::from_str(text)?;
    if text != serde_json::to_string(&parsed)?
        || parsed.schema != RESPONSE_SCHEMA
        || parsed.binding != dispatch.binding
        || !matches!(parsed.observed_work, None | Some(1))
    {
        return Err(Error(
            "noncanonical or misbound custom adapter response".into(),
        ));
    }
    match &parsed.outcome {
        ResponseOutcome::Completed {
            value: Value::Text(body),
        } if body.len() <= 4096 => {}
        ResponseOutcome::Failed { message } if !message.is_empty() && message.len() <= 4096 => {}
        _ => return Err(Error("invalid custom adapter response outcome".into())),
    }
    Ok(parsed)
}

#[derive(Clone, Debug)]
pub struct Spec {
    pub alias: String,
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug)]
struct Entry {
    alias: String,
    executable: PathBuf,
    executable_sha256: String,
    args: Vec<String>,
    timeout_ms: u64,
}

#[derive(Clone, Debug)]
pub struct Registry {
    entries: Vec<Entry>,
    identity: Identity,
}

impl Registry {
    pub fn open(specs: Vec<Spec>) -> Result<Self, Error> {
        if !(1..=MAX_ADAPTERS).contains(&specs.len()) {
            return Err(Error(
                "custom adapter registry requires 1..=16 entries".into(),
            ));
        }
        let mut entries = Vec::with_capacity(specs.len());
        let mut ids = Vec::with_capacity(specs.len());
        for spec in specs {
            let executable = spec.executable.canonicalize()?;
            let digest = lean::executable_digest(&executable)?;
            let id = EntryIdentity {
                alias: spec.alias.clone(),
                executable_sha256: digest.clone(),
                args: spec.args.clone(),
                timeout_ms: spec.timeout_ms,
            };
            id.validate()?;
            entries.push(Entry {
                alias: spec.alias,
                executable,
                executable_sha256: digest,
                args: spec.args,
                timeout_ms: spec.timeout_ms,
            });
            ids.push(id);
        }
        let identity = Identity {
            schema: IDENTITY_SCHEMA.into(),
            process_contract: process::CONTRACT.into(),
            entries: ids,
        };
        identity.validate()?;
        Ok(Self { entries, identity })
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn command(&self, alias: &str) -> Result<(std::process::Command, u64), Error> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.alias == alias)
            .ok_or_else(|| Error(format!("custom adapter alias '{alias}' is not registered")))?;
        if lean::executable_digest(&entry.executable)? != entry.executable_sha256 {
            return Err(Error(format!(
                "custom adapter '{}' executable changed after preflight",
                entry.alias
            )));
        }
        let mut command = std::process::Command::new(&entry.executable);
        command.env_clear().args(&entry.args);
        if let Some(parent) = entry.executable.parent() {
            command.current_dir(parent);
        }
        #[cfg(windows)]
        for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        Ok((command, entry.timeout_ms))
    }

    pub fn verify_unchanged(&self) -> Result<(), Error> {
        for entry in &self.entries {
            if lean::executable_digest(&entry.executable)? != entry.executable_sha256 {
                return Err(Error(format!(
                    "custom adapter '{}' executable changed",
                    entry.alias
                )));
            }
        }
        Ok(())
    }

    pub fn executable_path(&self, alias: &str) -> Option<&Path> {
        self.entries
            .iter()
            .find(|e| e.alias == alias)
            .map(|e| e.executable.as_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_identity() -> Identity {
        Identity {
            schema: IDENTITY_SCHEMA.into(),
            process_contract: process::CONTRACT.into(),
            entries: vec![EntryIdentity {
                alias: "echo_tool".into(),
                executable_sha256: "a".repeat(64),
                args: vec!["--strict".into()],
                timeout_ms: 5000,
            }],
        }
    }

    fn fixture_dispatch(identity: &Identity, input_text: &str) -> Dispatch {
        let input = identity
            .request("echo_tool", input_text)
            .unwrap()
            .input()
            .unwrap();
        Dispatch {
            binding: crate::Binding {
                attempt: crate::AttemptId {
                    run: "run".into(),
                    task: "task-0".into(),
                    slot: 0,
                    generation: 1,
                },
                adapter: adapter(),
                dispatched_by: "local-session".into(),
                context_sha256: "b".repeat(64),
                input_sha256: sha256(&serde_json::to_vec(&input).unwrap()),
            },
            input,
        }
    }

    #[test]
    fn custom_adapter_protocol_enforces_pinning_and_exact_binding() {
        let id = fixture_identity();
        assert!(id.validate().is_ok());
        assert!(id.request("unregistered", "hi").is_err());
        assert!(
            id.request("echo_tool", &"x".repeat(MAX_INPUT_BYTES + 1))
                .is_err()
        );

        let dispatch = fixture_dispatch(&id, "hello world");
        let envelope = Response {
            schema: RESPONSE_SCHEMA.into(),
            binding: dispatch.binding.clone(),
            outcome: ResponseOutcome::Completed {
                value: Value::Text("ECHO: hello world".into()),
            },
            observed_work: Some(1),
        };
        let mut output = process::Output {
            exit_code: Some(0),
            stdout: format!("{}\n", serde_json::to_string(&envelope).unwrap()).into_bytes(),
            stderr: vec![],
        };
        assert_eq!(response(&id, &dispatch, &output).unwrap(), envelope);

        // Non-empty stderr fails closed.
        output.stderr = b"warn".to_vec();
        assert!(response(&id, &dispatch, &output).is_err());
        output.stderr.clear();

        // Generation mismatch fails closed.
        let mut wrong = dispatch.clone();
        wrong.binding.attempt.generation += 1;
        assert!(response(&id, &wrong, &output).is_err());

        // Non-canonical JSON (extra whitespace inside) fails closed.
        output.stdout = serde_json::to_vec_pretty(&envelope).unwrap();
        assert!(response(&id, &dispatch, &output).is_err());

        // Int output rejected because adapter output type is Text.
        let bad_type = Response {
            schema: RESPONSE_SCHEMA.into(),
            binding: dispatch.binding.clone(),
            outcome: ResponseOutcome::Completed {
                value: Value::Int(42),
            },
            observed_work: Some(1),
        };
        output.stdout = serde_json::to_vec(&bad_type).unwrap();
        assert!(response(&id, &dispatch, &output).is_err());
    }

    #[test]
    fn registry_detects_tampered_executable_before_launch() {
        let path = std::env::temp_dir().join(format!(
            "nmlt-custom-adapter-test-{}.bin",
            std::process::id()
        ));
        std::fs::write(&path, b"version-1").unwrap();
        let registry = Registry::open(vec![Spec {
            alias: "demo".into(),
            executable: path.clone(),
            args: vec![],
            timeout_ms: 1000,
        }])
        .unwrap();
        assert!(registry.command("demo").is_ok());
        assert!(registry.command("missing").is_err());

        // Mutating the executable after open must fail closed!
        std::fs::write(&path, b"version-2-tampered").unwrap();
        assert!(registry.command("demo").is_err());
        assert!(registry.verify_unchanged().is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn oversized_adapter_registry_is_rejected_before_session_manifest() {
        let entries = (0..16)
            .map(|idx| EntryIdentity {
                alias: format!("adapter_{idx}"),
                executable_sha256: "a".repeat(64),
                args: vec!["x".repeat(MAX_ARG_BYTES); MAX_ARGS],
                timeout_ms: 5000,
            })
            .collect();
        let oversized = Identity {
            schema: IDENTITY_SCHEMA.into(),
            process_contract: process::CONTRACT.into(),
            entries,
        };
        assert!(oversized.validate().is_err());
    }
}

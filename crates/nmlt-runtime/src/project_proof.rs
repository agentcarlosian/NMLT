//! Fixed project-proof worker protocol. Lifecycle replay checks receipts and
//! bindings; the CLI additionally checks the retained proof artifacts.
use crate::{
    Adapter, Dispatch, Error, Response, ResponseOutcome, Value, ValueType, identity, process,
    sha256,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const MAX_INPUT_BYTES: u64 = 1024 * 1024 * 1024;
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_FILES: usize = 65_536;

pub fn adapter() -> Adapter {
    Adapter {
        name: "lean-project".into(),
        version: 1,
        input_type: ValueType::Text,
        output_type: ValueType::Text,
    }
}
pub fn alias(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && value.as_bytes()[0].is_ascii_alphabetic()
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub alias: String,
    pub task_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub schema: String,
    pub process_contract: String,
    pub executable_sha256: String,
    pub configuration_sha256: String,
    pub inputs_sha256: String,
    pub bindings: Vec<Binding>,
    pub timeout_ms: u64,
}
impl Identity {
    pub fn validate(&self) -> Result<(), Error> {
        let mut seen = BTreeSet::new();
        if self.schema != "nmlt-project-proof-tool-v1"
            || self.process_contract != process::PROJECT_WORKER_CONTRACT
            || ![
                &self.executable_sha256,
                &self.configuration_sha256,
                &self.inputs_sha256,
            ]
            .iter()
            .all(|s| crate::valid_digest(s))
            || !(1..=1_800_000).contains(&self.timeout_ms)
            || !(1..=16).contains(&self.bindings.len())
            || self.bindings.iter().any(|b| {
                !alias(&b.alias) || !crate::valid_digest(&b.task_sha256) || !seen.insert(&b.alias)
            })
        {
            return Err(Error("invalid project proof tool identity".into()));
        }
        Ok(())
    }
    pub fn request(&self, alias: &str, proof: &str) -> Result<Request, Error> {
        self.validate()?;
        let binding = self
            .bindings
            .iter()
            .find(|b| b.alias == alias)
            .ok_or_else(|| Error("project proof alias is not registered".into()))?;
        let request = Request {
            schema: "nmlt-project-proof-request-v1".into(),
            alias: alias.into(),
            task_sha256: binding.task_sha256.clone(),
            proof: proof.into(),
        };
        request.input()?;
        Ok(request)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub alias: String,
    pub task_sha256: String,
    pub proof: String,
}
impl Request {
    pub fn input(&self) -> Result<Value, Error> {
        if self.schema != "nmlt-project-proof-request-v1"
            || !alias(&self.alias)
            || !crate::valid_digest(&self.task_sha256)
            || !(1..=8192).contains(&self.proof.len())
            || !self.proof.is_ascii()
            || self
                .proof
                .bytes()
                .any(|b| b.is_ascii_control() && !b"\t\n\r".contains(&b))
        {
            return Err(Error("invalid bounded project proof request".into()));
        }
        Ok(Value::Text(serde_json::to_string(self)?))
    }
}
pub fn request(identity: &Identity, dispatch: &Dispatch) -> Result<Request, Error> {
    let Value::Text(input) = &dispatch.input else {
        return Err(Error("project proof requires text".into()));
    };
    let request: Request = serde_json::from_str(input)?;
    if dispatch.binding.adapter != adapter()
        || dispatch.binding.input_sha256 != sha256(&serde_json::to_vec(&dispatch.input)?)
        || request.input()? != dispatch.input
        || identity.request(&request.alias, &request.proof)? != request
    {
        return Err(Error("project proof dispatch binding mismatch".into()));
    }
    Ok(request)
}
pub fn result_key(dispatch: &Dispatch) -> Result<String, Error> {
    Ok(sha256(&serde_json::to_vec(&dispatch.binding)?))
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofReceipt {
    pub schema: String,
    pub status: String,
    pub alias: String,
    pub task_sha256: String,
    pub dispatch_sha256: String,
    pub result_sha256: String,
    pub export_sha256: String,
}
pub fn response(
    identity: &Identity,
    dispatch: &Dispatch,
    output: &process::Output,
) -> Result<Response, Error> {
    let request = request(identity, dispatch)?;
    if output.exit_code != Some(0) || !output.stderr.is_empty() {
        return Err(Error(
            "project worker did not return a clean response".into(),
        ));
    }
    let response: Response = serde_json::from_slice(&output.stdout)?;
    if output.stdout != serde_json::to_vec(&response)?
        || response.schema != crate::RESPONSE_SCHEMA
        || response.binding != dispatch.binding
        || response.observed_work != Some(1)
    {
        return Err(Error("noncanonical or misbound project response".into()));
    }
    match &response.outcome {
        ResponseOutcome::Completed {
            value: Value::Text(text),
        } => {
            let receipt: ProofReceipt = serde_json::from_str(text)?;
            if serde_json::to_string(&receipt)? != *text
                || receipt.schema != "nmlt-project-proof-receipt-v1"
                || receipt.status != "independently_checked"
                || receipt.alias != request.alias
                || receipt.task_sha256 != request.task_sha256
                || receipt.dispatch_sha256 != result_key(dispatch)?
                || !crate::valid_digest(&receipt.result_sha256)
                || !crate::valid_digest(&receipt.export_sha256)
            {
                return Err(Error("invalid project proof receipt".into()));
            }
        }
        ResponseOutcome::Failed { message } if !message.is_empty() && message.len() <= 4096 => {}
        _ => return Err(Error("invalid project proof response outcome".into())),
    }
    Ok(response)
}

/// A caller-preflighted set of immutable inputs. Paths are copied individually;
/// no build cache, arbitrary directory traversal or tool lookup occurs here.
pub struct Input {
    pub source: PathBuf,
    pub file: identity::FileIdentity,
}
pub struct Tool {
    executable: PathBuf,
    configuration: Vec<u8>,
    inputs: Vec<Input>,
    identity: Identity,
}
impl Tool {
    pub fn open(
        executable: &Path,
        configuration: Vec<u8>,
        inputs: Vec<Input>,
        bindings: Vec<Binding>,
        timeout_ms: u64,
    ) -> Result<Self, Error> {
        if configuration.len() > 1024 * 1024 {
            return Err(Error("project configuration exceeds 1 MiB".into()));
        }
        let files = inputs.iter().map(|i| i.file.clone()).collect::<Vec<_>>();
        validate_files(&files)?;
        for input in &inputs {
            check_file(&input.source, &input.file)?;
        }
        let identity = Identity {
            schema: "nmlt-project-proof-tool-v1".into(),
            process_contract: process::PROJECT_WORKER_CONTRACT.into(),
            executable_sha256: crate::lean::executable_digest(executable)?,
            configuration_sha256: sha256(&configuration),
            inputs_sha256: sha256(&serde_json::to_vec(&files)?),
            bindings,
            timeout_ms,
        };
        identity.validate()?;
        Ok(Self {
            executable: executable.canonicalize()?,
            configuration,
            inputs,
            identity,
        })
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn retain(&self, directory: &Path) -> Result<Self, Error> {
        std::fs::create_dir(directory)?;
        crate::session_store::write(&directory.join("configuration.json"), &self.configuration)?;
        let files = self
            .inputs
            .iter()
            .map(|i| i.file.clone())
            .collect::<Vec<_>>();
        crate::session_store::write(&directory.join("inputs.json"), &serde_json::to_vec(&files)?)?;
        crate::session_store::write(
            &directory.join("identity.json"),
            &serde_json::to_vec(&self.identity)?,
        )?;
        let executable = directory.join(if cfg!(windows) { "nmlt.exe" } else { "nmlt" });
        copy_new(&self.executable, &executable, 256 * 1024 * 1024)?;
        for input in &self.inputs {
            check_file(&input.source, &input.file)?;
            let target = directory.join(&input.file.path);
            std::fs::create_dir_all(
                target
                    .parent()
                    .ok_or_else(|| Error("missing input parent".into()))?,
            )?;
            copy_new(&input.source, &target, input.file.bytes)?;
        }
        Self::restore(directory, &self.identity)
    }
    pub fn restore(directory: &Path, expected: &Identity) -> Result<Self, Error> {
        let identity: Identity =
            crate::session_store::read(&directory.join("identity.json"), 65_536)?;
        if &identity != expected {
            return Err(Error("retained project identity changed".into()));
        }
        let files: Vec<identity::FileIdentity> =
            crate::session_store::read(&directory.join("inputs.json"), 16 * 1024 * 1024)?;
        validate_files(&files)?;
        let configuration = bounded_read(&directory.join("configuration.json"), 1024 * 1024)?;
        let inputs = files
            .into_iter()
            .map(|file| Input {
                source: directory.join(&file.path),
                file,
            })
            .collect();
        let executable = directory.join(if cfg!(windows) { "nmlt.exe" } else { "nmlt" });
        let tool = Self::open(
            &executable,
            configuration,
            inputs,
            identity.bindings.clone(),
            identity.timeout_ms,
        )?;
        if tool.identity != identity {
            return Err(Error("retained project inputs changed".into()));
        }
        Ok(tool)
    }
    pub fn command(&self, directory: &Path) -> Result<std::process::Command, Error> {
        Self::restore(directory, &self.identity)?;
        let mut command = std::process::Command::new(&self.executable);
        command
            .arg("__lean-project-worker")
            .arg("--bundle")
            .arg(directory.canonicalize()?);
        // Match the existing Lean adapter's explicit Windows OS/temp context.
        // An empty environment otherwise makes Lake's temporary-file API fall
        // back to a system directory that an ordinary user cannot write.
        #[cfg(windows)]
        for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        Ok(command)
    }
}
fn bounded_read(path: &Path, limit: u64) -> Result<Vec<u8>, Error> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut bytes = vec![];
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error("project metadata exceeds bound".into()));
    }
    Ok(bytes)
}
fn copy_new(source: &Path, target: &Path, limit: u64) -> Result<(), Error> {
    use std::io::Read;
    let input = std::fs::File::open(source)?;
    let metadata = input.metadata()?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(Error("project input exceeds copy bound".into()));
    }
    let mut output = std::fs::File::create_new(target)?;
    let copied = std::io::copy(&mut input.take(limit + 1), &mut output)?;
    if copied > limit || copied != metadata.len() {
        return Err(Error("project input changed during bounded copy".into()));
    }
    output.sync_all()?;
    std::fs::set_permissions(target, metadata.permissions())?;
    Ok(())
}
fn validate_files(files: &[identity::FileIdentity]) -> Result<(), Error> {
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    if files.is_empty() || files.len() > MAX_FILES {
        return Err(Error("project input count exceeds bound".into()));
    }
    for file in files {
        total = total
            .checked_add(file.bytes)
            .ok_or_else(|| Error("project byte count overflow".into()))?;
        if file.path.len() > 4096
            || !file.path.starts_with("tasks/")
            || file.path.contains(['\\', ':', '\0'])
            || file
                .path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with([' ', '.']))
            || file.link_target.is_some()
            || !crate::valid_digest(&file.sha256)
            || !seen.insert(file.path.to_lowercase())
            || file.bytes > MAX_FILE_BYTES
            || total > MAX_INPUT_BYTES
        {
            return Err(Error("invalid or excessive retained project inputs".into()));
        }
    }
    Ok(())
}
fn check_file(path: &Path, expected: &identity::FileIdentity) -> Result<(), Error> {
    let metadata = std::fs::symlink_metadata(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(Error("project input is a reparse point".into()));
        }
    }
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || identity::file(path, MAX_FILE_BYTES)? != (expected.bytes, expected.sha256.clone())
    {
        return Err(Error("project input differs from its pin".into()));
    }
    // Also reject links in parent components, including Windows junctions.
    for parent in path.ancestors().skip(1) {
        if parent.as_os_str().is_empty() {
            continue;
        }
        let metadata = std::fs::symlink_metadata(parent)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(Error("project input parent is a reparse point".into()));
            }
        }
        if metadata.file_type().is_symlink() {
            return Err(Error("project input parent is a link".into()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> Identity {
        Identity {
            schema: "nmlt-project-proof-tool-v1".into(),
            process_contract: process::PROJECT_WORKER_CONTRACT.into(),
            executable_sha256: "a".repeat(64),
            configuration_sha256: "b".repeat(64),
            inputs_sha256: "c".repeat(64),
            bindings: vec![Binding {
                alias: "offset".into(),
                task_sha256: "d".repeat(64),
            }],
            timeout_ms: 60_000,
        }
    }
    fn dispatch() -> Dispatch {
        let input = identity()
            .request("offset", "fun n => Support.shift_eq n")
            .unwrap()
            .input()
            .unwrap();
        Dispatch {
            binding: crate::Binding {
                attempt: crate::AttemptId {
                    run: "run".into(),
                    task: "proof".into(),
                    slot: 0,
                    generation: 1,
                },
                adapter: adapter(),
                dispatched_by: "local-session".into(),
                context_sha256: "e".repeat(64),
                input_sha256: sha256(&serde_json::to_vec(&input).unwrap()),
            },
            input,
        }
    }
    #[test]
    fn project_profile_does_not_expand_ordinary_request_bounds() {
        let long = identity().request("offset", &"x".repeat(8192)).unwrap();
        let mut request = crate::Request {
            adapter: adapter(),
            context_sha256: "a".repeat(64),
            input: long.input().unwrap(),
            reserved_work: 1,
        };
        assert!(request.validate().is_ok());
        request.adapter = crate::lean::adapter();
        assert!(request.validate().is_err());
        assert!(identity().request("offset", &"x".repeat(8193)).is_err());
        assert!(identity().request("elsewhere", "True.intro").is_err());
        let mut changed = identity();
        changed.process_contract = process::CONTRACT.into();
        assert!(changed.validate().is_err());
        changed = identity();
        changed.bindings.push(changed.bindings[0].clone());
        assert!(changed.validate().is_err());
    }
    #[test]
    fn protocol_receipt_requires_exact_task_dispatch_and_encoding() {
        let dispatch = dispatch();
        let receipt = ProofReceipt {
            schema: "nmlt-project-proof-receipt-v1".into(),
            status: "independently_checked".into(),
            alias: "offset".into(),
            task_sha256: "d".repeat(64),
            dispatch_sha256: result_key(&dispatch).unwrap(),
            result_sha256: "f".repeat(64),
            export_sha256: "0".repeat(64),
        };
        // Structural protocol fixture only; accepted proofs are tested through
        // actual Lean, NanoDA and artifact validation in the CLI integration.
        let envelope = Response {
            schema: crate::RESPONSE_SCHEMA.into(),
            binding: dispatch.binding.clone(),
            outcome: ResponseOutcome::Completed {
                value: Value::Text(serde_json::to_string(&receipt).unwrap()),
            },
            observed_work: Some(1),
        };
        let mut output = process::Output {
            exit_code: Some(0),
            stdout: serde_json::to_vec(&envelope).unwrap(),
            stderr: vec![],
        };
        assert!(response(&identity(), &dispatch, &output).is_ok());
        output.stdout.push(b'\n');
        assert!(response(&identity(), &dispatch, &output).is_err());
        let mut changed = receipt;
        changed.task_sha256 = "1".repeat(64);
        let mut envelope = envelope;
        envelope.outcome = ResponseOutcome::Completed {
            value: Value::Text(serde_json::to_string(&changed).unwrap()),
        };
        output.stdout = serde_json::to_vec(&envelope).unwrap();
        assert!(response(&identity(), &dispatch, &output).is_err());
        let mut wrong = dispatch;
        wrong.binding.attempt.generation += 1;
        assert!(response(&identity(), &wrong, &output).is_err());
    }
    #[test]
    fn retained_input_manifest_rejects_escape_links_duplicates_and_budget() {
        let file = identity::FileIdentity {
            path: "tasks/offset/task.json".into(),
            bytes: 10,
            sha256: "a".repeat(64),
            link_target: None,
        };
        assert!(validate_files(std::slice::from_ref(&file)).is_ok());
        for path in [
            "tasks/offset/../task.json",
            "/tasks/a",
            "tasks/a/.",
            "tasks/a/x.",
            "tasks/a//x",
            "tasks/a/x:stream",
            "configuration.json",
        ] {
            let mut changed = file.clone();
            changed.path = path.into();
            assert!(validate_files(&[changed]).is_err(), "{path}");
        }
        let mut changed = file.clone();
        changed.link_target = Some("elsewhere".into());
        assert!(validate_files(&[changed]).is_err());
        let mut changed = file.clone();
        changed.path = file.path.to_uppercase();
        assert!(validate_files(&[file.clone(), changed]).is_err());
        let mut changed = file;
        changed.bytes = MAX_FILE_BYTES + 1;
        assert!(validate_files(&[changed]).is_err());
    }
}

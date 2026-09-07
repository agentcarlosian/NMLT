//! Initial asynchronous host API. Handles cannot be cloned or deserialized;
//! every live operation is mediated by a locked, durable journal.
use crate::{
    Command, Control, Dispatch, Error, Journal, Lifecycle, Limits, Outcome, Phase, Receipt,
    Request, Response, ResponseOutcome, RunSpec, Value, lean, process, sha256, worker,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub slots: u32,
    pub max_attempts: u32,
    pub timeout_ms: u64,
}

pub struct SquareTool {
    executable: PathBuf,
    identity: String,
}
impl SquareTool {
    /// The trusted executable must implement the fixed __square-worker command.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let executable = path.canonicalize()?;
        let identity = lean::executable_digest(&executable)?;
        Ok(Self {
            executable,
            identity,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub schema: String,
    pub parent_context_sha256: String,
    pub bounds: Bounds,
    pub square_executable_sha256: Option<String>,
    pub lean: Option<lean::Identity>,
}
impl Configuration {
    fn validate(&self) -> Result<(), Error> {
        if self.schema != "nmlt-async-session-v1"
            || !crate::valid_digest(&self.parent_context_sha256)
            || !(1..=4).contains(&self.bounds.slots)
            || !(1..=16).contains(&self.bounds.max_attempts)
            || self.bounds.slots > self.bounds.max_attempts
            || !(1..=30_000).contains(&self.bounds.timeout_ms)
            || self
                .square_executable_sha256
                .as_ref()
                .is_some_and(|s| !crate::valid_digest(s))
        {
            return Err(Error(
                "invalid async session configuration or bounds".into(),
            ));
        }
        if let Some(identity) = &self.lean {
            // Reuse the complete Lean identity and contract validation.
            lean::Request {
                schema: "nmlt-lean-request-v1".into(),
                toolchain: identity.clone(),
                strategy: lean::Strategy::ExistingLemma,
                source_sha256: sha256(lean::source(lean::Strategy::ExistingLemma).as_bytes()),
            }
            .input()?;
        }
        Ok(())
    }
    fn limits(&self) -> Limits {
        Limits {
            slots: self.bounds.slots,
            generations_per_slot: self.bounds.max_attempts,
            max_attempts: self.bounds.max_attempts,
            max_events: self.bounds.max_attempts * 6 + 1,
            work_budget: u64::from(self.bounds.max_attempts),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub configuration: Configuration,
    pub spec: RunSpec,
}
impl Manifest {
    pub fn validate(&self) -> Result<(), Error> {
        self.configuration.validate()?;
        if self.spec.context_sha256 != sha256(&serde_json::to_vec(&self.configuration)?)
            || self.spec.limits != self.configuration.limits()
        {
            return Err(Error("async session context or limits mismatch".into()));
        }
        Lifecycle::new(self.spec.clone())?;
        Ok(())
    }
}

/// A session-specific reference to one allocation. No Clone, serde, or public
/// constructor. Journal revisions remain private and cannot be resurrected.
pub struct Handle {
    session: Arc<()>,
    index: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    Ready,
    Uncertain,
    Collected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub dispatch: Dispatch,
    pub completion: process::Result,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub manifest: Manifest,
    pub journal: String,
    pub observations: Vec<Observation>,
}

struct Task {
    dispatch: Dispatch,
    control: Control,
    process: Option<process::Process>,
}
pub struct Session {
    failed: bool,
    manifest: Manifest,
    journal: Journal,
    square: Option<SquareTool>,
    lean: Option<lean::Toolchain>,
    instance: Arc<()>,
    tasks: Vec<Task>,
    observations: Vec<Observation>,
}
impl Session {
    pub fn create(
        directory: &Path,
        run_id: String,
        parent_context_sha256: String,
        bounds: Bounds,
        square: Option<SquareTool>,
        lean: Option<lean::Toolchain>,
    ) -> Result<Self, Error> {
        use std::io::Write;
        let configuration = Configuration {
            schema: "nmlt-async-session-v1".into(),
            parent_context_sha256,
            bounds,
            square_executable_sha256: square.as_ref().map(|s| s.identity.clone()),
            lean: lean.as_ref().map(|s| s.identity().clone()),
        };
        configuration.validate()?;
        let spec = RunSpec {
            run_id,
            context_sha256: sha256(&serde_json::to_vec(&configuration)?),
            limits: configuration.limits(),
        };
        let manifest = Manifest {
            configuration,
            spec,
        };
        manifest.validate()?;
        std::fs::create_dir(directory)?;
        let mut file = std::fs::File::create_new(directory.join("manifest.json"))?;
        file.write_all(&serde_json::to_vec(&manifest)?)?;
        file.sync_all()?;
        let journal = Journal::create(&directory.join("journal.jsonl"), manifest.spec.clone())?;
        Ok(Self {
            failed: false,
            manifest,
            journal,
            square,
            lean,
            instance: Arc::new(()),
            tasks: vec![],
            observations: vec![],
        })
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn state(&self) -> &Lifecycle {
        self.journal.state()
    }
    pub fn observations(&self) -> &[Observation] {
        &self.observations
    }
    fn apply_control(&mut self, command: Command) -> Result<Control, Error> {
        match self.apply(command)? {
            Receipt::Control { control } => Ok(control),
            _ => Err(Error("expected current control receipt".into())),
        }
    }
    fn apply(&mut self, command: Command) -> Result<Receipt, Error> {
        if self.failed {
            return Err(Error(
                "session stopped after journal failure; recover explicitly".into(),
            ));
        }
        // Rejected controls/budgets leave an otherwise usable session intact.
        // A failure after this preflight may involve persistence and stops it.
        self.journal.state().step(&command)?;
        match self.journal.apply(command) {
            Ok(receipt) => Ok(receipt),
            Err(error) => {
                self.failed = true;
                // Drop requests best-effort termination; the durable log still
                // records unresolved attempts for explicit recovery.
                for task in &mut self.tasks {
                    task.process = None;
                }
                Err(error)
            }
        }
    }
    fn start(
        &mut self,
        task: &str,
        request: Request,
        command: std::process::Command,
        stdin: impl FnOnce(&Dispatch) -> Vec<u8>,
    ) -> Result<Handle, Error> {
        let control = self.apply_control(Command::Reserve {
            task: task.into(),
            owner: "local-session".into(),
            request,
        })?;
        let Receipt::Dispatch { dispatch, control } = self.apply(Command::Dispatch { control })?
        else {
            return Err(Error("expected durable dispatch receipt".into()));
        };
        let index = self.tasks.len();
        let launched = process::Process::start(
            command,
            stdin(&dispatch),
            Duration::from_millis(self.manifest.configuration.bounds.timeout_ms),
        );
        let (process, failed) = match launched {
            Ok(process) => (Some(process), None),
            Err(error) => (None, Some(error)),
        };
        self.tasks.push(Task {
            dispatch,
            control,
            process,
        });
        if let Some(failure) = failed {
            self.observe(index, Err(failure), false)?;
        }
        Ok(Handle {
            session: self.instance.clone(),
            index,
        })
    }
    pub fn start_square(&mut self, task: &str, input: i64) -> Result<Handle, Error> {
        let tool = self
            .square
            .as_ref()
            .ok_or_else(|| Error("square tool is not configured".into()))?;
        if lean::executable_digest(&tool.executable)? != tool.identity {
            return Err(Error("square executable changed after preflight".into()));
        }
        let mut command = std::process::Command::new(&tool.executable);
        command.arg("__square-worker");
        let request = Request {
            adapter: worker::adapter(),
            context_sha256: self.manifest.spec.context_sha256.clone(),
            input: Value::Int(input),
            reserved_work: 1,
        };
        self.start(task, request, command, |dispatch| {
            serde_json::to_vec(dispatch).expect("dispatch serialization")
        })
    }
    pub fn start_lean(&mut self, task: &str, strategy: lean::Strategy) -> Result<Handle, Error> {
        let (input, command, stdin) = self
            .lean
            .as_ref()
            .ok_or_else(|| Error("Lean is not configured".into()))?
            .prepare(strategy)?;
        let request = Request {
            adapter: lean::adapter(),
            context_sha256: self.manifest.spec.context_sha256.clone(),
            input: input.input()?,
            reserved_work: 1,
        };
        self.start(task, request, command, |_| stdin)
    }
    fn index(&self, handle: &Handle) -> Result<usize, Error> {
        if self.failed {
            return Err(Error(
                "session stopped after journal failure; recover explicitly".into(),
            ));
        }
        if !Arc::ptr_eq(&handle.session, &self.instance) || handle.index >= self.tasks.len() {
            return Err(Error("job handle belongs to a different session".into()));
        }
        Ok(handle.index)
    }
    fn status(&self, index: usize) -> Status {
        match &self.state().attempts()[index].phase {
            Phase::Finished { .. } => Status::Ready,
            Phase::Uncertain { .. } => Status::Uncertain,
            Phase::Collected { .. } => Status::Collected,
            _ => Status::Pending,
        }
    }
    fn observe(
        &mut self,
        index: usize,
        completion: process::Result,
        cancelled: bool,
    ) -> Result<(), Error> {
        let observation = Observation {
            dispatch: self.tasks[index].dispatch.clone(),
            completion,
        };
        let response = response(&self.manifest.configuration, &observation, cancelled)?;
        let control = self.tasks[index].control.clone();
        let next = match response {
            Some(response) => self.apply_control(Command::Deliver { response })?,
            None => self.apply_control(Command::Timeout { control })?,
        };
        self.tasks[index].control = next;
        self.tasks[index].process = None;
        self.observations.push(observation);
        Ok(())
    }
    pub fn poll(&mut self, handle: &Handle) -> Result<Status, Error> {
        let i = self.index(handle)?;
        if let Some(result) = self.tasks[i]
            .process
            .as_mut()
            .and_then(|p| p.poll().cloned())
        {
            self.observe(i, result, false)?;
        }
        Ok(self.status(i))
    }
    pub fn wait(&mut self, handle: &Handle) -> Result<Status, Error> {
        let i = self.index(handle)?;
        if let Some(process) = &mut self.tasks[i].process {
            let result = process.wait().clone();
            self.observe(i, result, false)?;
        }
        Ok(self.status(i))
    }
    /// Returns false when completion was already settled. Otherwise durable
    /// cancellation precedes signalling; only confirmed child exit can settle it.
    pub fn cancel(&mut self, handle: &mut Handle) -> Result<bool, Error> {
        let i = self.index(handle)?;
        match self.poll(handle)? {
            Status::Ready => return Ok(false),
            Status::Collected => return Err(Error("job already collected".into())),
            Status::Uncertain => {
                return Err(Error(
                    "uncertain job requires external reconciliation".into(),
                ));
            }
            Status::Pending => {}
        }
        let control = self.tasks[i].control.clone();
        self.tasks[i].control = self.apply_control(Command::Cancel { control })?;
        let result = self.tasks[i]
            .process
            .as_mut()
            .expect("pending process")
            .cancel()
            .clone();
        self.observe(i, result, true)?;
        if self.status(i) == Status::Ready {
            Ok(true)
        } else {
            Err(Error(
                "cancellation cleanup is uncertain; explicit reconciliation required".into(),
            ))
        }
    }
    /// Pending work retains its handle. Successful collection consumes the
    /// journal revision; further collection using the same handle is rejected.
    pub fn collect(&mut self, handle: &mut Handle) -> Result<Option<Outcome>, Error> {
        let i = self.index(handle)?;
        match self.poll(handle)? {
            Status::Pending => return Ok(None),
            Status::Collected => return Err(Error("job already collected".into())),
            Status::Uncertain => return Err(Error("uncertain job cannot be collected".into())),
            Status::Ready => {}
        }
        let Receipt::Collected { outcome, .. } = self.apply(Command::Collect {
            control: self.tasks[i].control.clone(),
        })?
        else {
            return Err(Error("expected collected result".into()));
        };
        Ok(Some(outcome))
    }
    pub fn snapshot(&mut self) -> Result<Snapshot, Error> {
        Ok(Snapshot {
            manifest: self.manifest.clone(),
            journal: self.journal.snapshot()?,
            observations: self.observations.clone(),
        })
    }
}

fn check_dispatch(configuration: &Configuration, dispatch: &Dispatch) -> Result<(), Error> {
    if dispatch.binding.input_sha256 != sha256(&serde_json::to_vec(&dispatch.input)?) {
        return Err(Error("dispatch input hash mismatch".into()));
    }
    if dispatch.binding.adapter == worker::adapter() {
        if configuration.square_executable_sha256.is_none()
            || !matches!(dispatch.input, Value::Int(_))
        {
            return Err(Error("unconfigured or invalid square dispatch".into()));
        }
    } else if dispatch.binding.adapter == lean::adapter() {
        let Value::Text(input) = &dispatch.input else {
            return Err(Error("Lean requires a canonical text request".into()));
        };
        let request: lean::Request = serde_json::from_str(input)?;
        if Some(&request.toolchain) != configuration.lean.as_ref()
            || request.input()? != dispatch.input
        {
            return Err(Error("Lean request or toolchain identity mismatch".into()));
        }
    } else {
        return Err(Error("unsupported asynchronous adapter".into()));
    }
    Ok(())
}
fn response(
    configuration: &Configuration,
    observation: &Observation,
    cancelled: bool,
) -> Result<Option<Response>, Error> {
    check_dispatch(configuration, &observation.dispatch)?;
    if let Ok(output) = &observation.completion
        && (output.stdout.len() > process::PIPE_BYTES || output.stderr.len() > process::PIPE_BYTES)
    {
        return Err(Error("process observation exceeds pipe bounds".into()));
    }
    if cancelled {
        let reaped = match &observation.completion {
            Ok(_) => true,
            Err(failure) => failure.child_reaped,
        };
        return Ok(reaped.then(|| Response {
            schema: crate::RESPONSE_SCHEMA.into(),
            binding: observation.dispatch.binding.clone(),
            outcome: ResponseOutcome::Cancelled,
            observed_work: None,
        }));
    }
    let Ok(output) = &observation.completion else {
        return Ok(None);
    };
    if observation.dispatch.binding.adapter == worker::adapter() {
        if output.exit_code != Some(0) || !output.stderr.is_empty() {
            return Ok(None);
        }
        // Requiring the exact worker encoding plus whitespace rejects surplus
        // and duplicate fields, even in serde's internally tagged variants.
        let expected = worker::evaluate(&observation.dispatch)?;
        if std::str::from_utf8(&output.stdout).ok().map(str::trim)
            != Some(serde_json::to_string(&expected)?.as_str())
        {
            return Ok(None);
        }
        Ok(Some(expected))
    } else {
        let Value::Text(input) = &observation.dispatch.input else {
            unreachable!()
        };
        let request: lean::Request = serde_json::from_str(input)?;
        let Ok(evidence) = lean::evidence(request, output) else {
            return Ok(None);
        };
        Ok(Some(lean::validate(&observation.dispatch, &evidence)?))
    }
}

/// Replay captured observations and the journal without starting any process.
/// The result establishes consistency, not authenticated physical observations.
pub fn verify_snapshot(snapshot: &Snapshot) -> Result<Lifecycle, Error> {
    snapshot.manifest.validate()?;
    if snapshot.observations.len() > snapshot.manifest.spec.limits.max_attempts as usize {
        return Err(Error("too many observations".into()));
    }
    let (expected, commands) =
        crate::replay_journal(snapshot.journal.as_bytes(), &snapshot.manifest.spec)?;
    let mut state = Lifecycle::new(snapshot.manifest.spec.clone())?;
    let mut cursor = 0;
    for command in commands {
        match &command {
            Command::Deliver { .. } | Command::Timeout { .. } => {
                let observation = snapshot
                    .observations
                    .get(cursor)
                    .ok_or_else(|| Error("missing process observation".into()))?;
                let attempt = match &command {
                    Command::Deliver { response } => &response.binding.attempt,
                    Command::Timeout { control } => &control.attempt,
                    _ => unreachable!(),
                };
                let current = state
                    .attempts()
                    .iter()
                    .find(|a| a.control.attempt == *attempt)
                    .ok_or_else(|| Error("unknown observed attempt".into()))?;
                if current.dispatch.as_ref() != Some(&observation.dispatch.binding) {
                    return Err(Error("observation dispatch mismatch".into()));
                }
                let expected = response(
                    &snapshot.manifest.configuration,
                    observation,
                    matches!(current.phase, Phase::CancelRequested),
                )?;
                let matches = match (&command, expected) {
                    (Command::Deliver { response }, Some(expected)) => *response == expected,
                    (Command::Timeout { .. }, None) => true,
                    _ => false,
                };
                if !matches {
                    return Err(Error(
                        "process observation disagrees with journal settlement".into(),
                    ));
                }
                cursor += 1;
            }
            Command::Reserve { owner, request, .. }
                if owner == "local-session" && request.reserved_work == 1 => {}
            Command::Dispatch { .. } | Command::Cancel { .. } | Command::Collect { .. } => {}
            _ => {
                return Err(Error(
                    "command is outside the asynchronous session profile".into(),
                ));
            }
        }
        let (next, receipt) = state.step(&command)?;
        if let Receipt::Dispatch { dispatch, .. } = receipt {
            check_dispatch(&snapshot.manifest.configuration, &dispatch)?;
        }
        state = next;
    }
    if cursor != snapshot.observations.len() || state != expected {
        return Err(Error(
            "unused observations or reconstructed state mismatch".into(),
        ));
    }
    Ok(state)
}

/// Recover journal state only. Surviving processes are not rediscovered and
/// asynchronous handles are never reconstructed or automatically redispatched.
pub fn recover(directory: &Path) -> Result<Lifecycle, Error> {
    use std::io::Read;
    let mut bytes = vec![];
    std::fs::File::open(directory.join("manifest.json"))?
        .take(65_537)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 65_536 {
        return Err(Error("session manifest exceeds bound".into()));
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    if serde_json::to_vec(&manifest)? != bytes {
        return Err(Error("session manifest is not canonical".into()));
    }
    manifest.validate()?;
    Ok(
        Journal::open(&directory.join("journal.jsonl"), &manifest.spec)?
            .state()
            .clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(slots: u32, timeout_ms: u64) -> (PathBuf, Session) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/async-session-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        let session = Session::create(
            &dir,
            "same-run-for-alias-control".into(),
            sha256(b"unit context"),
            Bounds {
                slots,
                max_attempts: 3,
                timeout_ms,
            },
            Some(SquareTool::open(&std::env::current_exe().unwrap()).unwrap()),
            None,
        )
        .unwrap();
        (dir, session)
    }
    fn sleeping(session: &mut Session, task: &str) -> Result<Handle, Error> {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "session::tests::process_probe", "--nocapture"])
            .env("NMLT_SESSION_PROBE", "sleep");
        let request = Request {
            adapter: worker::adapter(),
            context_sha256: session.manifest.spec.context_sha256.clone(),
            input: Value::Int(3),
            reserved_work: 1,
        };
        session.start(task, request, command, |_| vec![])
    }
    #[test]
    fn process_probe() {
        if std::env::var_os("NMLT_SESSION_PROBE").is_some() {
            std::thread::sleep(Duration::from_secs(5));
        }
    }
    #[test]
    fn parallel_capacity_cancellation_collection_and_generation_reuse() {
        let (_, mut session) = fixture(2, 5000);
        let mut first = sleeping(&mut session, "first").unwrap();
        let mut second = sleeping(&mut session, "second").unwrap();
        assert_eq!(session.state().accounting().dispatched_attempts, 2);
        assert!(sleeping(&mut session, "full").is_err());
        assert_eq!(session.poll(&first).unwrap(), Status::Pending);
        let events = session.state().event_count();
        assert_eq!(session.collect(&mut first).unwrap(), None);
        assert_eq!(session.state().event_count(), events);
        assert!(session.cancel(&mut first).unwrap());
        assert_eq!(
            session.collect(&mut first).unwrap(),
            Some(Outcome::Cancelled)
        );
        assert!(session.collect(&mut first).is_err());
        let mut third = sleeping(&mut session, "third").unwrap();
        assert_eq!(
            session.tasks[2].dispatch.binding.attempt.slot,
            session.tasks[0].dispatch.binding.attempt.slot
        );
        assert_eq!(session.tasks[2].dispatch.binding.attempt.generation, 2);
        for handle in [&mut second, &mut third] {
            assert!(session.cancel(handle).unwrap());
            assert_eq!(session.collect(handle).unwrap(), Some(Outcome::Cancelled));
        }
        assert!(sleeping(&mut session, "exhausted").is_err());
        assert_eq!(session.state().accounting().charged_work, 3);
        assert_eq!(
            verify_snapshot(&session.snapshot().unwrap()).unwrap(),
            *session.state()
        );
    }
    #[test]
    fn timeout_retains_charge_and_slot_even_after_confirmed_cleanup() {
        let (directory, mut session) = fixture(1, 100);
        let mut handle = sleeping(&mut session, "timeout").unwrap();
        assert_eq!(session.wait(&handle).unwrap(), Status::Uncertain);
        assert!(session.collect(&mut handle).is_err());
        assert!(sleeping(&mut session, "blocked-slot").is_err());
        assert_eq!(session.state().accounting().charged_work, 1);
        assert_eq!(session.state().attempts()[0].observed_work, None);
        assert!(matches!(
            &session.observations()[0].completion,
            Err(process::Failure {
                kind: process::FailureKind::Timeout,
                child_reaped: true
            })
        ));
        assert_eq!(
            verify_snapshot(&session.snapshot().unwrap()).unwrap(),
            *session.state()
        );
        drop(session);
        let recovered = recover(&directory).unwrap();
        assert!(matches!(
            recovered.attempts()[0].phase,
            Phase::Uncertain { .. }
        ));
        assert_eq!(recovered.accounting().dispatched_attempts, 1);
        assert_eq!(recovered.accounting().charged_work, 1);
    }
    #[test]
    fn handles_from_distinct_instances_cannot_cross_even_with_identical_contexts() {
        let (_, mut a) = fixture(1, 5000);
        let (_, mut b) = fixture(1, 5000);
        let mut ah = sleeping(&mut a, "task").unwrap();
        let mut bh = sleeping(&mut b, "task").unwrap();
        assert_eq!(a.tasks[0].dispatch, b.tasks[0].dispatch);
        assert!(a.cancel(&mut bh).is_err());
        assert!(b.collect(&mut ah).is_err());
        assert!(a.cancel(&mut ah).unwrap());
        assert!(b.cancel(&mut bh).unwrap());
    }
    #[test]
    fn captured_observation_mutations_do_not_replay() {
        let (_, mut session) = fixture(1, 5000);
        let mut handle = sleeping(&mut session, "cancel").unwrap();
        session.cancel(&mut handle).unwrap();
        session.collect(&mut handle).unwrap();
        let original = session.snapshot().unwrap();
        let mut changed = original.clone();
        changed.observations[0].dispatch.input = Value::Int(99);
        assert!(verify_snapshot(&changed).is_err());
        let mut changed = original.clone();
        changed.observations[0].completion = Err(process::Failure {
            kind: process::FailureKind::Cancelled,
            child_reaped: false,
        });
        assert!(verify_snapshot(&changed).is_err());
        let mut changed = original.clone();
        changed.observations.clear();
        assert!(verify_snapshot(&changed).is_err());
        let mut changed = original.clone();
        changed.observations.push(original.observations[0].clone());
        assert!(verify_snapshot(&changed).is_err());
        let mut changed = original.clone();
        changed.manifest.configuration.bounds.slots = 2;
        assert!(verify_snapshot(&changed).is_err());
        let mut changed = original;
        changed.journal.pop();
        assert!(verify_snapshot(&changed).is_err());
    }
    #[test]
    fn settled_result_wins_before_cancellation_and_collects_once() {
        let (_, mut session) = fixture(1, 5000);
        let mut handle = sleeping(&mut session, "injected-completion").unwrap();
        // A unit-level completion fixture; real adapter output is exercised by
        // the async_jobs example. Terminate the sleeping probe before injecting.
        assert!(
            session.tasks[0]
                .process
                .as_mut()
                .unwrap()
                .cancel()
                .as_ref()
                .unwrap_err()
                .child_reaped
        );
        let result = worker::evaluate(&session.tasks[0].dispatch).unwrap();
        session
            .observe(
                0,
                Ok(process::Output {
                    exit_code: Some(0),
                    stdout: serde_json::to_vec(&result).unwrap(),
                    stderr: vec![],
                }),
                false,
            )
            .unwrap();
        assert!(!session.cancel(&mut handle).unwrap());
        assert_eq!(
            session.collect(&mut handle).unwrap(),
            Some(Outcome::Completed {
                value: Value::Int(9)
            })
        );
        assert!(session.collect(&mut handle).is_err());
        assert_eq!(
            verify_snapshot(&session.snapshot().unwrap()).unwrap(),
            *session.state()
        );
    }
    #[test]
    fn persistence_failure_stops_live_session_before_any_new_dispatch() {
        let (directory, mut session) = fixture(2, 5000);
        let mut handle = sleeping(&mut session, "first").unwrap();
        let command = Command::Cancel {
            control: session.tasks[0].control.clone(),
        };
        assert!(session.journal.inject_failed_append(command).is_err());
        assert!(session.cancel(&mut handle).is_err());
        assert!(session.failed);
        assert!(session.tasks[0].process.is_none());
        assert!(session.collect(&mut handle).is_err());
        assert!(sleeping(&mut session, "second").is_err());
        assert_eq!(session.state().accounting().dispatched_attempts, 1);
        drop(session);
        assert!(recover(&directory).is_err());
    }
}

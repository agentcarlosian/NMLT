//! Scoped source controls over RFC 0023 sessions. Replay has no process authority.
use super::runtime::implementation_digest;
use super::workflow::{bounded_read, load};
use nmlt_runtime::{self as rt, session};
use nmlt_workflow::{
    Execution, Inputs, JobError, JobHost, JobOperation, JobRequest, Location, Program,
    SourceIdentity, Value,
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
mod ledger;
mod validation;
use ledger::{Item, Ledger};

pub(super) const SCHEMA: &str = "nmlt-async-source-run-v2";
pub(super) const MAX_BYTES: u64 = 32 * 1024 * 1024;

pub(super) struct Options {
    pub source: PathBuf,
    pub project_context: Option<String>,
    pub directory: PathBuf,
    pub bounds: session::Bounds,
    pub lean: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    schema: String,
    assurance: String,
    implementation_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project_context_sha256: Option<String>,
    source_path: String,
    sources: Vec<SourceIdentity>,
    program_sha256: String,
    entry: String,
    inputs: Inputs,
    max_steps: u32,
    bounds: session::Bounds,
}
impl Context {
    fn digest(&self) -> String {
        rt::sha256(&serde_json::to_vec(self).expect("context"))
    }
    fn validate(&self) -> Result<(), String> {
        super::workflow::validate_project_context(&self.project_context_sha256)?;
        if self.schema != "nmlt-async-source-context-v2"
            || self.assurance != "none"
            || self.implementation_sha256 != implementation_digest()?
            || !(1..=100_000).contains(&self.max_steps)
            || !(1..=4).contains(&self.bounds.slots)
            || !(1..=16).contains(&self.bounds.max_attempts)
            || self.bounds.slots > self.bounds.max_attempts
            || !(1..=30_000).contains(&self.bounds.timeout_ms)
        {
            return Err("unsupported async source context, executable identity, or bounds".into());
        }
        if self.sources.is_empty()
            || self.sources.len() > nmlt_workflow::MAX_PACKAGE_FILES
            || self.sources.iter().any(|s| {
                Path::new(&s.path)
                    .file_name()
                    .is_none_or(|name| name != s.path.as_str())
                    || !s.path.ends_with(".nmlt")
                    || s.path.contains(['/', '\\', ':'])
            })
        {
            return Err("source context requires portable package filenames".into());
        }
        Ok(())
    }
    fn bind(&self, manifest: &session::Manifest) -> Result<(), String> {
        self.validate()?;
        manifest.validate().map_err(|e| e.to_string())?;
        if manifest.configuration.parent_context_sha256 != self.digest()
            || manifest.configuration.bounds != self.bounds
            || manifest.configuration.square_executable_sha256.as_ref()
                != Some(&self.implementation_sha256)
        {
            return Err("async session/source context mismatch".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Call {
    Start { request: JobRequest },
    Control { job: u32, operation: JobOperation },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Reply {
    Started { job: u32 },
    Value { value: Value },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    at: Location,
    call: Call,
    result: Result<Reply, JobError>,
    before: u32,
    through: u32,
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: String,
    context: Context,
    events: Vec<Event>,
    trace: String,
    snapshot: session::Snapshot,
    execution: Execution,
}

fn exact<T: for<'de> Deserialize<'de> + Serialize>(bytes: &[u8]) -> Result<T, String> {
    let value: T = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let raw = serde_json::from_slice::<super::strict_json::Unique>(bytes)
        .map_err(|e| e.to_string())?
        .0;
    if raw != serde_json::to_value(&value).map_err(|e| e.to_string())? {
        return Err("unknown fields or non-exact async source values".into());
    }
    Ok(value)
}
fn persist(file: &mut File, value: &impl Serialize, limit: u64) -> Result<String, String> {
    let json = format!(
        "{}\n",
        serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
    );
    if json.len() as u64 > limit {
        return Err("async source evidence exceeds byte bound".into());
    }
    file.write_all(json.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(json)
}
fn strategy(name: &str) -> Result<rt::lean::Strategy, String> {
    match name {
        "wrong_term" => Ok(rt::lean::Strategy::WrongTerm),
        "existing_lemma" => Ok(rt::lean::Strategy::ExistingLemma),
        "induction" => Ok(rt::lean::Strategy::Induction),
        "admitted" => Ok(rt::lean::Strategy::Admitted),
        _ => Err("unsupported Lean strategy".into()),
    }
}
fn request(manifest: &session::Manifest, input: &JobRequest) -> Result<rt::Request, String> {
    let (adapter, input) = match input {
        JobRequest::Square(i) => (rt::worker::adapter(), rt::Value::Int(*i)),
        JobRequest::Lean(_) | JobRequest::LeanCheck { .. } => {
            let request = rt::lean::Request::new(
                manifest
                    .configuration
                    .lean
                    .clone()
                    .ok_or("Lean tool is not configured")?,
                candidate(input)?,
            )
            .map_err(|e| e.to_string())?;
            (
                rt::lean::adapter(),
                request.input().map_err(|e| e.to_string())?,
            )
        }
    };
    Ok(rt::Request {
        adapter,
        input,
        context_sha256: manifest.spec.context_sha256.clone(),
        reserved_work: 1,
    })
}
fn candidate(input: &JobRequest) -> Result<rt::lean::Candidate, String> {
    match input {
        JobRequest::Lean(name) => Ok(rt::lean::Candidate::Template {
            strategy: strategy(name)?,
        }),
        JobRequest::LeanCheck { statement, proof } => Ok(rt::lean::Candidate::Terms {
            statement: statement.clone(),
            proof: proof.clone(),
        }),
        _ => Err("expected Lean job request".into()),
    }
}
fn reserve(
    manifest: &session::Manifest,
    job: u32,
    input: &JobRequest,
) -> Result<rt::Command, String> {
    Ok(rt::Command::Reserve {
        task: format!("source-job-{job}"),
        owner: "local-session".into(),
        request: request(manifest, input)?,
    })
}
fn outcome(outcome: &rt::Outcome) -> Value {
    match outcome {
        rt::Outcome::Completed { value } => Value::Ok(Box::new(match value {
            rt::Value::Int(i) => Value::Int(*i),
            rt::Value::Text(s) => Value::Text(s.clone()),
            rt::Value::Bool(b) => Value::Bool(*b),
        })),
        rt::Outcome::Failed { message } => Value::Err(message.clone()),
        rt::Outcome::Cancelled => Value::Err("job cancelled".into()),
    }
}

pub(super) fn run(
    program: &Program,
    source_path: String,
    entry: String,
    inputs: Inputs,
    max_steps: u32,
    output: &Path,
    options: Options,
) -> Result<(), String> {
    nmlt_workflow::validate_inputs(program, &entry, &inputs, max_steps)?;
    if program.requires_lean_jobs(&entry)? && options.lean.is_none() {
        return Err("source Lean jobs require --lean-bin with the pinned direct executable".into());
    }
    let context = Context {
        schema: "nmlt-async-source-context-v2".into(),
        assurance: "none".into(),
        implementation_sha256: implementation_digest()?,
        project_context_sha256: options.project_context,
        source_path,
        sources: program.sources().to_vec(),
        program_sha256: program.identity(),
        entry,
        inputs,
        max_steps,
        bounds: options.bounds,
    };
    context.validate()?;
    let square = session::SquareTool::open(&std::env::current_exe().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let lean = options
        .lean
        .as_ref()
        .map(|p| rt::lean::Toolchain::open(p))
        .transpose()
        .map_err(|e| e.to_string())?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let run_id = format!("source-{}-{nonce}", std::process::id());
    let mut output =
        File::create_new(output).map_err(|e| format!("could not create new async record: {e}"))?;
    let session = session::Session::create(
        &options.directory,
        run_id,
        context.digest(),
        context.bounds.clone(),
        Some(square),
        lean,
    )
    .map_err(|e| e.to_string())?;
    persist(
        &mut File::create_new(options.directory.join("source-context.json"))
            .map_err(|e| e.to_string())?,
        &context,
        2 * 1024 * 1024,
    )?;
    capture_sources(&context, &options.source, &options.directory)?;
    let ledger = Ledger::create(
        &options.directory.join("source-journal.jsonl"),
        &context.digest(),
    )?;
    let mut host = Live {
        session,
        handles: vec![],
        ledger,
        prefix_len: 0,
        cursor: 0,
        fatal: None,
    };
    let execution = nmlt_workflow::execute_with_host(
        program,
        &context.entry,
        &context.inputs,
        max_steps,
        &mut host,
    )?;
    if let Some(error) = host.fatal {
        return Err(format!(
            "async host failed; inspect the durable session: {error}"
        ));
    }
    let snapshot = host.session.snapshot().map_err(|e| e.to_string())?;
    let record = Record {
        schema: SCHEMA.into(),
        context,
        events: host.ledger.state.events.clone(),
        trace: host.ledger.text(),
        snapshot,
        execution,
    };
    // Check the same operation/journal contract before publishing a live record.
    validate(&record)?;
    print!("{}", persist(&mut output, &record, MAX_BYTES)?);
    if record.execution.stop.returned() {
        Ok(())
    } else {
        Err("async source execution stopped; bounded outcome recorded".into())
    }
}

struct Live {
    session: session::Session,
    handles: Vec<session::Handle>,
    ledger: Ledger,
    prefix_len: usize,
    cursor: usize,
    fatal: Option<String>,
}
impl Live {
    fn perform(&mut self, call: Call, at: Location) -> Result<Reply, JobError> {
        if self.fatal.is_some() {
            return Err(JobError::HostFailure);
        }
        if self.cursor < self.prefix_len {
            let event = &self.ledger.state.events[self.cursor];
            if event.at != at || event.call != call {
                self.fatal = Some("source operation prefix differs during resumption".into());
                return Err(JobError::HostFailure);
            }
            let result = event.result.clone();
            if let Ok(Reply::Started { job }) = result {
                if job as usize != self.handles.len() {
                    self.fatal = Some("restored source job order differs".into());
                    return Err(JobError::HostFailure);
                }
                self.handles
                    .push(self.session.restored_handle(job as usize).map_err(|e| {
                        self.fatal = Some(e.to_string());
                        JobError::HostFailure
                    })?);
            }
            self.cursor += 1;
            return result;
        }
        let resumed = self.ledger.state.pending.is_some();
        if let Some(intent) = &self.ledger.state.pending {
            if intent.at != at || intent.call != call {
                self.fatal = Some("pending source intent differs during resumption".into());
                return Err(JobError::HostFailure);
            }
        } else {
            self.ledger
                .append(Item::Intent {
                    at,
                    call: call.clone(),
                    before: self.session.state().event_count(),
                })
                .map_err(|e| {
                    self.fatal = Some(e);
                    JobError::HostFailure
                })?;
        }
        let result = self.inner(&call, resumed);
        // An I/O failure can leave a complete durable transition that the live
        // state did not release. Preserve its pending intent for locked recovery.
        if self.fatal.is_some() {
            return Err(JobError::HostFailure);
        }
        self.ledger
            .append(Item::Reply {
                result: result.clone(),
                through: self.session.state().event_count(),
            })
            .map_err(|e| {
                self.fatal = Some(e);
                JobError::HostFailure
            })?;
        result
    }
    fn inner(&mut self, call: &Call, resumed: bool) -> Result<Reply, JobError> {
        match call {
            Call::Start { request: input } => {
                let job = self.handles.len() as u32;
                let command = reserve(self.session.manifest(), job, input).map_err(|e| {
                    self.fatal = Some(e);
                    JobError::HostFailure
                })?;
                if resumed && let Some(attempt) = self.session.state().attempts().get(job as usize)
                {
                    let rt::Command::Reserve {
                        task,
                        owner,
                        request,
                    } = &command
                    else {
                        unreachable!()
                    };
                    if attempt.control.attempt.task != *task
                        || attempt.control.owner != *owner
                        || attempt.request != *request
                    {
                        self.fatal =
                            Some("resumed source request differs from original reservation".into());
                        return Err(JobError::HostFailure);
                    }
                    let handle = self.session.resume_start(job as usize).map_err(|e| {
                        self.fatal = Some(e.to_string());
                        JobError::HostFailure
                    })?;
                    self.handles.push(handle);
                    return Ok(Reply::Started { job });
                }
                if self.session.state().step(&command).is_err() {
                    return Err(JobError::Limit);
                }
                let task = format!("source-job-{job}");
                let started = match input {
                    JobRequest::Square(i) => self.session.start_square(&task, *i),
                    JobRequest::Lean(_) | JobRequest::LeanCheck { .. } => {
                        self.session.start_lean_candidate(
                            &task,
                            candidate(input).expect("validated candidate"),
                        )
                    }
                };
                let handle = started.map_err(|e| {
                    self.fatal = Some(e.to_string());
                    JobError::HostFailure
                })?;
                self.handles.push(handle);
                Ok(Reply::Started { job })
            }
            Call::Control { job, operation } => {
                if resumed
                    && *operation == JobOperation::Collect
                    && let Some(result) = self
                        .session
                        .restored_outcome(*job as usize)
                        .map_err(|_| JobError::HostFailure)?
                {
                    return Ok(Reply::Value {
                        value: outcome(&result),
                    });
                }
                if resumed
                    && *operation == JobOperation::Cancel
                    && self.restored_cancellation(*job as usize)?
                {
                    return Ok(Reply::Value {
                        value: Value::Bool(true),
                    });
                }
                let handle = self
                    .handles
                    .get_mut(*job as usize)
                    .ok_or(JobError::HostFailure)?;
                let result = match operation {
                    JobOperation::Poll => self.session.poll(handle).map(|status| match status {
                        session::Status::Ready => Ok(Value::Bool(true)),
                        session::Status::Pending => Ok(Value::Bool(false)),
                        _ => Err(JobError::HostFailure),
                    }),
                    JobOperation::Cancel => self.session.cancel(handle).map(|v| Ok(Value::Bool(v))),
                    JobOperation::Collect => self
                        .session
                        .wait(handle)
                        .and_then(|_| self.session.collect(handle))
                        .map(|v| v.map(|o| outcome(&o)).ok_or(JobError::HostFailure)),
                };
                let result = result.map_err(|e| {
                    if !matches!(
                        self.session.state().attempts()[*job as usize].phase,
                        rt::Phase::Uncertain { .. }
                    ) {
                        self.fatal = Some(e.to_string());
                    }
                    JobError::HostFailure
                })??;
                Ok(Reply::Value { value: result })
            }
        }
    }
    fn restored_cancellation(&mut self, job: usize) -> Result<bool, JobError> {
        let attempt = self
            .session
            .state()
            .attempts()
            .get(job)
            .ok_or(JobError::HostFailure)?;
        if attempt.phase
            != (rt::Phase::Finished {
                outcome: rt::Outcome::Cancelled,
            })
        {
            return Ok(false);
        }
        let attempt_id = attempt.control.attempt.clone();
        let before = self
            .ledger
            .state
            .pending
            .as_ref()
            .ok_or(JobError::HostFailure)?
            .before as usize;
        let snapshot = self.session.snapshot().map_err(|_| JobError::HostFailure)?;
        let (_, commands) =
            rt::replay_journal(snapshot.journal.as_bytes(), &snapshot.manifest.spec)
                .map_err(|_| JobError::HostFailure)?;
        Ok(commands[before..]
            .iter()
            .any(|c| matches!(c, rt::Command::Cancel { control } if control.attempt == attempt_id)))
    }
}

fn capture_sources(context: &Context, source: &Path, directory: &Path) -> Result<(), String> {
    let source = source.canonicalize().map_err(|e| e.to_string())?;
    let root = source.parent().ok_or("source directory is missing")?;
    let destination = directory.join("sources");
    std::fs::create_dir(&destination).map_err(|e| e.to_string())?;
    for identity in &context.sources {
        let bytes = bounded_read(
            &root.join(&identity.path),
            nmlt_workflow::MAX_SOURCE_BYTES as u64,
        )?;
        if rt::sha256(&bytes) != identity.source_sha256 {
            return Err("source changed before durable execution snapshot".into());
        }
        let mut file =
            File::create_new(destination.join(&identity.path)).map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
    }
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let retained = directory.join(if cfg!(windows) { "nmlt.exe" } else { "nmlt" });
    let mut output = File::create_new(&retained).map_err(|e| e.to_string())?;
    std::io::copy(
        &mut File::open(executable).map_err(|e| e.to_string())?,
        &mut output,
    )
    .map_err(|e| e.to_string())?;
    output.sync_all().map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&retained, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    if rt::identity::file(&retained, 256 * 1024 * 1024)
        .map_err(|e| e.to_string())?
        .1
        != context.implementation_sha256
    {
        return Err("retained executable changed during capture".into());
    }
    Ok(())
}

trait Calls {
    fn perform(&mut self, call: Call, at: Location) -> Result<Reply, JobError>;
}
impl Calls for Live {
    fn perform(&mut self, call: Call, at: Location) -> Result<Reply, JobError> {
        self.perform(call, at)
    }
}
// The legacy synchronous builtin can be used inside the asynchronous route;
// its two source-located operations are replayed by exactly the same host.
macro_rules! job_host {
    ($host:ty) => {
        impl JobHost for $host {
            fn square(
                &mut self,
                input: i64,
                at: Location,
            ) -> Result<Result<i64, String>, JobError> {
                let job = self.start(JobRequest::Square(input), at)?;
                match self.control(job, JobOperation::Collect, at)? {
                    Value::Ok(v) => match *v {
                        Value::Int(i) => Ok(Ok(i)),
                        _ => Err(JobError::HostFailure),
                    },
                    Value::Err(s) => Ok(Err(s)),
                    _ => Err(JobError::HostFailure),
                }
            }
            fn start(&mut self, request: JobRequest, at: Location) -> Result<u32, JobError> {
                match Calls::perform(self, Call::Start { request }, at)? {
                    Reply::Started { job } => Ok(job),
                    _ => Err(JobError::HostFailure),
                }
            }
            fn control(
                &mut self,
                job: u32,
                operation: JobOperation,
                at: Location,
            ) -> Result<Value, JobError> {
                match Calls::perform(self, Call::Control { job, operation }, at)? {
                    Reply::Value { value } => Ok(value),
                    _ => Err(JobError::HostFailure),
                }
            }
        }
    };
}
job_host!(Live);

struct Replay<'a> {
    events: &'a [Event],
    cursor: usize,
    mismatch: bool,
}
impl Calls for Replay<'_> {
    fn perform(&mut self, call: Call, at: Location) -> Result<Reply, JobError> {
        let event = self.events.get(self.cursor);
        if event.is_none_or(|e| e.at != at || e.call != call) {
            self.mismatch = true;
            return Err(JobError::HostFailure);
        }
        self.cursor += 1;
        event.expect("matched event").result.clone()
    }
}
job_host!(Replay<'_>);

fn validate(record: &Record) -> Result<(), String> {
    validation::record(record)
}

pub(super) fn replay(bytes: &[u8], source: &Path) -> Result<(), String> {
    let record: Record = exact(bytes)?;
    validate(&record)?;
    let logical = &record
        .context
        .sources
        .first()
        .ok_or("missing source manifest")?
        .path;
    let program = load(source, Some(logical))?;
    if program.sources() != record.context.sources
        || program.identity() != record.context.program_sha256
    {
        return Err("async replay source package or typed program mismatch".into());
    }
    let mut host = Replay {
        events: &record.events,
        cursor: 0,
        mismatch: false,
    };
    let execution = nmlt_workflow::execute_with_host(
        &program,
        &record.context.entry,
        &record.context.inputs,
        record.context.max_steps,
        &mut host,
    )?;
    if host.mismatch || host.cursor != record.events.len() || execution != record.execution {
        return Err("async source replay operation or execution mismatch".into());
    }
    println!(
        "{}",
        serde_json::json!({"schema":"nmlt-async-source-replay-v1", "assurance":"none", "matched":true, "execution":execution,
        "meaning":"same-executable source and captured-session consistency; no redispatch or fresh Lean check"})
    );
    Ok(())
}

pub(super) fn recover(directory: &Path) -> Result<(), String> {
    let context: Context = exact(&bounded_read(
        &directory.join("source-context.json"),
        2 * 1024 * 1024,
    )?)?;
    let manifest: session::Manifest =
        exact(&bounded_read(&directory.join("manifest.json"), 65_536)?)?;
    context.bind(&manifest)?;
    let state = session::recover(directory).map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({"schema":"nmlt-async-source-recovery-v1", "assurance":"none", "state":state,
        "meaning":"saved observations settled; unresolved effects classified; no process redispatched; jobs-resume continues the saved source"})
    );
    Ok(())
}

pub(super) fn resume(args: &[std::ffi::OsString]) -> Result<(), String> {
    let directory = args
        .first()
        .map(PathBuf::from)
        .ok_or("jobs-resume requires a saved jobs directory")?;
    let mut output = None;
    let mut lean_path = None;
    let mut acknowledgement = None;
    let mut pairs = args[1..].chunks_exact(2);
    for pair in &mut pairs {
        match pair[0].to_str() {
            Some("--emit-run") if output.is_none() => output = Some(PathBuf::from(&pair[1])),
            Some("--lean-bin") if lean_path.is_none() => lean_path = Some(PathBuf::from(&pair[1])),
            Some("--acknowledge-uncertain-effects") if acknowledgement.is_none() => {
                let reason = pair[1].to_str().ok_or("acknowledgement must be UTF-8")?;
                if reason.trim().is_empty() || reason.len() > 1024 {
                    return Err(
                        "acknowledgement requires 1..1024 bytes explaining operator reconciliation"
                            .into(),
                    );
                }
                acknowledgement = Some(reason.to_owned());
            }
            _ => return Err("unsupported or duplicate jobs-resume option".into()),
        }
    }
    if !pairs.remainder().is_empty() {
        return Err("jobs-resume options require values".into());
    }
    let output = output.ok_or("jobs-resume requires --emit-run with a new record path")?;
    if output.exists() {
        return Err("resumed record path must be new".into());
    }
    resume_source(
        &directory,
        &output,
        lean_path.as_deref(),
        acknowledgement.as_deref(),
    )
}

pub(super) fn repair(args: &[std::ffi::OsString]) -> Result<(), String> {
    let [directory, flag, reason] = args else {
        return Err(
            "usage: nmlt jobs-repair <jobs-directory> --acknowledge-incomplete-tail <reason>"
                .into(),
        );
    };
    if flag != "--acknowledge-incomplete-tail" {
        return Err("tail repair requires explicit acknowledgement".into());
    }
    let reason = reason.to_str().ok_or("repair reason must be UTF-8")?;
    let directory = Path::new(directory);
    let context: Context = exact(&bounded_read(
        &directory.join("source-context.json"),
        2 * 1024 * 1024,
    )?)?;
    let manifest: session::Manifest =
        exact(&bounded_read(&directory.join("manifest.json"), 65_536)?)?;
    context.bind(&manifest)?;
    let (mut guard, runtime_tail) =
        rt::Journal::open_repair(&directory.join("journal.jsonl"), &manifest.spec, reason)
            .map_err(|e| e.to_string())?;
    let source_tail = Ledger::repair(
        &directory.join("source-journal.jsonl"),
        &context.digest(),
        reason,
    )?;
    guard.snapshot().map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({"schema":"nmlt-source-tail-repair-v1","assurance":"none","runtime_tail":runtime_tail,"source_tail":source_tail,"meaning":"only incomplete final appends were quarantined; no work dispatched or uncertainty acknowledged; run jobs-resume to reconcile and continue"})
    );
    Ok(())
}

pub(super) fn resume_source(
    directory: &Path,
    output: &Path,
    lean_path: Option<&Path>,
    acknowledgement: Option<&str>,
) -> Result<(), String> {
    let context: Context = exact(&bounded_read(
        &directory.join("source-context.json"),
        2 * 1024 * 1024,
    )?)?;
    let manifest: session::Manifest =
        exact(&bounded_read(&directory.join("manifest.json"), 65_536)?)?;
    context.bind(&manifest)?;
    let logical = &context
        .sources
        .first()
        .ok_or("saved source manifest missing")?
        .path;
    let program = load(&directory.join("sources").join(logical), Some(logical))?;
    if program.sources() != context.sources || program.identity() != context.program_sha256 {
        return Err("saved source package or program identity changed".into());
    }
    nmlt_workflow::validate_inputs(&program, &context.entry, &context.inputs, context.max_steps)?;
    if manifest.configuration.lean.is_some() && lean_path.is_none() {
        return Err(
            "resuming this run requires --lean-bin with the original pinned installation".into(),
        );
    }
    let square = session::SquareTool::open(&std::env::current_exe().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let lean = lean_path
        .map(rt::lean::Toolchain::open)
        .transpose()
        .map_err(|e| e.to_string())?;
    let mut session = session::Session::resume(directory, &context.digest(), Some(square), lean)
        .map_err(|e| e.to_string())?;
    let mut ledger = Ledger::open(&directory.join("source-journal.jsonl"), &context.digest())?;
    validation::trace(
        &context,
        &session.snapshot().map_err(|e| e.to_string())?,
        &ledger.state,
    )?;
    let uncertain: Vec<_> = session
        .state()
        .attempts()
        .iter()
        .enumerate()
        .filter(|(_, attempt)| matches!(attempt.phase, rt::Phase::Uncertain { .. }))
        .map(|(index, attempt)| (index, attempt.control.attempt.clone()))
        .collect();
    if !uncertain.is_empty() && acknowledgement.is_none() {
        return Err(format!(
            "unresolved external effects remain: {}; use --acknowledge-uncertain-effects <reason> only after operator reconciliation; old attempts will be marked failed and their spend retained",
            serde_json::to_string(&uncertain.iter().map(|(_, id)| id).collect::<Vec<_>>())
                .map_err(|e| e.to_string())?
        ));
    }
    let mut output = File::create_new(output)
        .map_err(|e| format!("could not create new resumed record: {e}"))?;
    if let Some(reason) = acknowledgement {
        for (index, _) in &uncertain {
            session
                .acknowledge_uncertain(*index, reason)
                .map_err(|e| e.to_string())?;
        }
    }
    if ledger.state.pending.is_none()
        && ledger.state.events.last().is_some_and(|event| {
            event.result == Err(JobError::HostFailure) && matches!(event.call, Call::Control { .. })
        })
    {
        ledger.append(Item::Retry)?;
    }
    let prefix_len = ledger.state.events.len();
    let mut host = Live {
        session,
        handles: vec![],
        ledger,
        prefix_len,
        cursor: 0,
        fatal: None,
    };
    let execution = nmlt_workflow::execute_with_host(
        &program,
        &context.entry,
        &context.inputs,
        context.max_steps,
        &mut host,
    )?;
    if let Some(error) = host.fatal {
        return Err(format!("source resumption stopped: {error}"));
    }
    if host.cursor != prefix_len || host.ledger.state.pending.is_some() {
        return Err("saved source prefix was not fully consumed".into());
    }
    let record = Record {
        schema: SCHEMA.into(),
        context,
        events: host.ledger.state.events.clone(),
        trace: host.ledger.text(),
        snapshot: host.session.snapshot().map_err(|e| e.to_string())?,
        execution,
    };
    validate(&record)?;
    print!("{}", persist(&mut output, &record, MAX_BYTES)?);
    if record.execution.stop.returned() {
        Ok(())
    } else {
        Err("resumed source execution stopped; bounded outcome recorded".into())
    }
}

//! Source workflows over the durable local runtime. Replay uses only the pure
//! lifecycle and captured worker evidence; it has no process-launch authority.
use super::job_process::{self, Failure, FailureKind};
use super::runtime::implementation_digest;
use super::workflow::{bounded_read, load};
use nmlt_runtime::{self as rt, Command, Journal, Lifecycle, Receipt, Response};
use nmlt_workflow::{Execution, Inputs, JobError, JobHost, Location, Program, SourceIdentity};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) const SCHEMA: &str = "nmlt-local-job-run-v1";
const PROFILE: &str = "local-job-workflow-v1";
const MAX_CONTEXT_BYTES: u64 = 1_048_576;

pub(super) struct Options {
    pub directory: PathBuf,
    pub max_jobs: u32,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    schema: String,
    profile: String,
    assurance: String,
    implementation_sha256: String,
    source_path: String,
    sources: Vec<SourceIdentity>,
    program_sha256: String,
    entry: String,
    inputs: Inputs,
    max_steps: u32,
    max_jobs: u32,
    timeout_ms: u64,
    adapter: rt::Adapter,
}

impl Context {
    fn limits(&self) -> rt::Limits {
        rt::Limits {
            slots: 1,
            generations_per_slot: self.max_jobs,
            max_attempts: self.max_jobs,
            max_events: self.max_jobs * 6 + 1,
            work_budget: u64::from(self.max_jobs),
        }
    }
    fn digest(&self) -> String {
        rt::sha256(&serde_json::to_vec(self).expect("context serialization"))
    }
    fn validate(&self) -> Result<(), String> {
        if self.schema != "nmlt-local-job-context-v1"
            || self.profile != PROFILE
            || self.assurance != "none"
            || self.adapter != rt::worker::adapter()
            || !(1..=16).contains(&self.max_jobs)
            || !(1..=30_000).contains(&self.timeout_ms)
            || !(1..=100_000).contains(&self.max_steps)
        {
            return Err("unsupported local job context, adapter, or bounds".into());
        }
        if self.implementation_sha256 != implementation_digest()? {
            return Err("job implementation mismatch: exact recorded executable required".into());
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() as u64 > MAX_CONTEXT_BYTES {
            return Err("job context exceeds byte bound".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    context: Context,
    spec: rt::RunSpec,
}
impl Manifest {
    fn validate(&self) -> Result<(), String> {
        self.context.validate()?;
        if self.spec.context_sha256 != self.context.digest()
            || self.spec.limits != self.context.limits()
        {
            return Err("job run context or limits mismatch".into());
        }
        Lifecycle::new(self.spec.clone()).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum JobResult {
    Response(Box<Response>),
    Failure(Failure),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    at: Location,
    input: i64,
    result: JobResult,
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: String,
    profile: String,
    assurance: String,
    manifest: Manifest,
    events: Vec<Event>,
    journal: String,
    accounting: rt::Accounting,
    execution: Execution,
}

fn exact<T: for<'de> Deserialize<'de> + Serialize>(bytes: &[u8]) -> Result<T, String> {
    let value: T = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let raw = serde_json::from_slice::<super::strict_json::Unique>(bytes)
        .map_err(|e| e.to_string())?
        .0;
    if serde_json::to_value(&value).map_err(|e| e.to_string())? != raw {
        return Err("unknown fields or non-exact job values".into());
    }
    Ok(value)
}
fn persist(file: &mut File, value: &impl Serialize, max: u64) -> Result<String, String> {
    let json = format!(
        "{}\n",
        serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
    );
    if json.len() as u64 > max {
        return Err("job record exceeds byte bound".into());
    }
    file.write_all(json.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(json)
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
    let context = Context {
        schema: "nmlt-local-job-context-v1".into(),
        profile: PROFILE.into(),
        assurance: "none".into(),
        implementation_sha256: implementation_digest()?,
        source_path,
        sources: program.sources().to_vec(),
        program_sha256: program.identity(),
        entry,
        inputs,
        max_steps,
        max_jobs: options.max_jobs,
        timeout_ms: options.timeout_ms,
        adapter: rt::worker::adapter(),
    };
    context.validate()?;
    let nonce = (
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos(),
        context.digest(),
    );
    let spec = rt::RunSpec {
        run_id: format!(
            "source-{}",
            rt::sha256(&serde_json::to_vec(&nonce).map_err(|e| e.to_string())?)
        ),
        context_sha256: context.digest(),
        limits: context.limits(),
    };
    let manifest = Manifest { context, spec };
    manifest.validate()?;
    // Reserve all paths before executing source. Both paths must be new. A
    // failed setup can leave an empty output or partial directory for inspection.
    let mut output = File::create_new(output)
        .map_err(|e| format!("could not create new job run record: {e}"))?;
    std::fs::create_dir(&options.directory)
        .map_err(|e| format!("could not create new jobs directory: {e}"))?;
    persist(
        &mut File::create_new(options.directory.join("context.json")).map_err(|e| e.to_string())?,
        &manifest,
        MAX_CONTEXT_BYTES * 2,
    )?;
    let journal = Journal::create(
        &options.directory.join("journal.jsonl"),
        manifest.spec.clone(),
    )
    .map_err(|e| e.to_string())?;
    let mut host = Host::new(&manifest, Some(journal), None)?;
    let execution = nmlt_workflow::execute_with_host(
        program,
        &manifest.context.entry,
        &manifest.context.inputs,
        max_steps,
        &mut host,
    )?;
    if let Some(error) = host.error {
        return Err(format!(
            "job host failed; inspect the durable context and journal: {error}"
        ));
    }
    let journal = host
        .journal
        .as_mut()
        .expect("live journal")
        .snapshot()
        .map_err(|e| e.to_string())?;
    let record = Record {
        schema: SCHEMA.into(),
        profile: PROFILE.into(),
        assurance: "none".into(),
        manifest,
        events: host.events,
        journal,
        accounting: host.state.accounting(),
        execution,
    };
    let json = persist(&mut output, &record, 4 * 1024 * 1024)?;
    print!("{json}");
    if record.execution.stop.returned() {
        Ok(())
    } else {
        Err("local job execution stopped; bounded outcome recorded".into())
    }
}

struct Host<'a> {
    #[cfg(test)]
    injected: Option<JobResult>,
    journal: Option<Journal>,
    state: Lifecycle,
    commands: Vec<Command>,
    events: Vec<Event>,
    replay: Option<&'a [Event]>,
    cursor: usize,
    timeout_ms: u64,
    max_jobs: u32,
    error: Option<String>,
}
impl<'a> Host<'a> {
    fn new(
        manifest: &Manifest,
        journal: Option<Journal>,
        replay: Option<&'a [Event]>,
    ) -> Result<Self, String> {
        if journal.is_some() == replay.is_some() {
            return Err("job host requires exactly one live journal or replay transcript".into());
        }
        Ok(Self {
            #[cfg(test)]
            injected: None,
            journal,
            state: Lifecycle::new(manifest.spec.clone()).map_err(|e| e.to_string())?,
            commands: vec![],
            events: vec![],
            replay,
            cursor: 0,
            timeout_ms: manifest.context.timeout_ms,
            max_jobs: manifest.context.max_jobs,
            error: None,
        })
    }
    fn apply(&mut self, command: Command) -> Result<Receipt, String> {
        let receipt = if let Some(journal) = &mut self.journal {
            let receipt = journal.apply(command.clone()).map_err(|e| e.to_string())?;
            self.state = journal.state().clone();
            receipt
        } else {
            let (state, receipt) = self.state.step(&command).map_err(|e| e.to_string())?;
            self.state = state;
            receipt
        };
        self.commands.push(command);
        Ok(receipt)
    }
    fn job(
        &mut self,
        input: i64,
        at: Location,
    ) -> Result<Result<Result<i64, String>, JobError>, String> {
        if self.state.accounting().allocated_attempts >= self.max_jobs {
            return Ok(Err(JobError::Limit));
        }
        let Receipt::Control { control } = self.apply(Command::Reserve {
            task: format!("source:{}:{}:{}", at.source, at.start, at.end),
            owner: "source-worker".into(),
            request: rt::Request {
                adapter: rt::worker::adapter(),
                context_sha256: self.state.spec().context_sha256.clone(),
                input: rt::Value::Int(input),
                reserved_work: 1,
            },
        })?
        else {
            return Err("reserve did not issue control".into());
        };
        let Receipt::Dispatch { dispatch, control } = self.apply(Command::Dispatch { control })?
        else {
            return Err("dispatch did not issue durable intent".into());
        };
        let result = if let Some(events) = self.replay {
            let event = events
                .get(self.cursor)
                .ok_or("missing recorded job event")?;
            if event.input != input || event.at != at {
                return Err("recorded job input or source location mismatch".into());
            }
            event.result.clone()
        } else {
            self.launch(&dispatch)
        };
        self.cursor += 1;
        if let JobResult::Response(response) = &result {
            rt::worker::validate(&dispatch, response).map_err(|e| e.to_string())?;
        }
        self.events.push(Event {
            at,
            input,
            result: result.clone(),
        });
        match result {
            JobResult::Failure(_) => {
                // Even confirmed child cleanup is not a typed worker result.
                // Retain charged work and the uncertain attempt for inspection.
                self.apply(Command::Timeout { control })?;
                Ok(Err(JobError::HostFailure))
            }
            JobResult::Response(response) => {
                let Receipt::Control { control } = self.apply(Command::Deliver {
                    response: *response,
                })?
                else {
                    return Err("worker response was not accepted".into());
                };
                let Receipt::Collected { outcome, .. } =
                    self.apply(Command::Collect { control })?
                else {
                    return Err("worker result was not collected".into());
                };
                match outcome {
                    rt::Outcome::Completed {
                        value: rt::Value::Int(value),
                    } => Ok(Ok(Ok(value))),
                    rt::Outcome::Failed { message } => Ok(Ok(Err(message))),
                    _ => Err("square worker returned an invalid outcome".into()),
                }
            }
        }
    }
}
impl Host<'_> {
    fn launch(&self, dispatch: &rt::Dispatch) -> JobResult {
        #[cfg(test)]
        if let Some(result) = &self.injected {
            return result.clone();
        }
        launch_with_timeout(dispatch, self.timeout_ms)
    }
}
impl JobHost for Host<'_> {
    fn square(&mut self, input: i64, at: Location) -> Result<Result<i64, String>, JobError> {
        match self.job(input, at) {
            Ok(result) => result,
            Err(error) => {
                self.error = Some(error);
                Err(JobError::HostFailure)
            }
        }
    }
}

fn launch_with_timeout(dispatch: &rt::Dispatch, timeout_ms: u64) -> JobResult {
    let response = (|| {
        let exe = std::env::current_exe().map_err(|_| Failure {
            kind: FailureKind::Spawn,
            child_reaped: true,
        })?;
        let mut command = std::process::Command::new(exe);
        command.arg("__square-worker");
        let bytes = job_process::run(
            command,
            serde_json::to_vec(dispatch).expect("dispatch serialization"),
            Duration::from_millis(timeout_ms),
        )?;
        let response = exact::<Response>(&bytes).map_err(|_| Failure {
            kind: FailureKind::Protocol,
            child_reaped: true,
        })?;
        rt::worker::validate(dispatch, &response).map_err(|_| Failure {
            kind: FailureKind::Protocol,
            child_reaped: true,
        })?;
        Ok(response)
    })();
    match response {
        Ok(response) => JobResult::Response(Box::new(response)),
        Err(failure) => JobResult::Failure(failure),
    }
}

pub(super) fn worker() -> Result<(), String> {
    let mut bytes = vec![];
    std::io::stdin()
        .take(65_537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65_536 {
        return Err("worker request exceeds byte bound".into());
    }
    let dispatch = exact::<rt::Dispatch>(&bytes)?;
    let response = rt::worker::evaluate(&dispatch).map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string(&response).map_err(|e| e.to_string())?
    );
    Ok(())
}

pub(super) fn replay(bytes: &[u8], source: &Path) -> Result<(), String> {
    let record: Record = exact(bytes)?;
    if record.schema != SCHEMA || record.profile != PROFILE || record.assurance != "none" {
        return Err("unsupported job run schema".into());
    }
    record.manifest.validate()?;
    if record.events.len() > record.manifest.context.max_jobs as usize {
        return Err("too many job events".into());
    }
    let context = &record.manifest.context;
    let logical = &context
        .sources
        .first()
        .ok_or("job context has no source manifest")?
        .path;
    let program = load(source, Some(logical))?;
    if program.sources() != context.sources || program.identity() != context.program_sha256 {
        return Err("job replay source package or typed program mismatch".into());
    }
    let (state, commands) = rt::replay_journal(record.journal.as_bytes(), &record.manifest.spec)
        .map_err(|e| e.to_string())?;
    let mut host = Host::new(&record.manifest, None, Some(&record.events))?;
    let execution = nmlt_workflow::execute_with_host(
        &program,
        &context.entry,
        &context.inputs,
        context.max_steps,
        &mut host,
    )?;
    if host.error.is_some()
        || host.cursor != record.events.len()
        || host.commands != commands
        || host.state != state
        || execution != record.execution
        || state.accounting() != record.accounting
    {
        return Err(
            "job replay mismatch: events, lifecycle, budget, steps, or outcome differ".into(),
        );
    }
    println!(
        "{}",
        serde_json::json!({"schema":"nmlt-local-job-replay-v1", "assurance":"none", "matched":true,
        "execution":execution, "accounting":record.accounting, "meaning":"same-executable source and journal consistency; recorded host observations, no jobs launched or Lean acceptance"})
    );
    Ok(())
}

pub(super) fn recover(directory: &Path) -> Result<(), String> {
    let manifest: Manifest = exact(&bounded_read(
        &directory.join("context.json"),
        MAX_CONTEXT_BYTES * 2,
    )?)?;
    manifest.validate()?;
    let journal = Journal::open(&directory.join("journal.jsonl"), &manifest.spec)
        .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({"schema":"nmlt-local-job-recovery-v1", "assurance":"none", "state":journal.state(),
        "meaning":"unfinished journal state classified; no redispatch, process discovery, or workflow resumption"})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_failure_retains_uncertain_charged_work_and_replays_without_fallback() {
        for child_reaped in [true, false] {
            let source = "fn main() -> Outcome<Int> { match job_square(-1) { Ok(value) => Ok(value), Err(message) => job_square(5) } }";
            let program = nmlt_workflow::compile(source).unwrap();
            let context = Context {
                schema: "nmlt-local-job-context-v1".into(),
                profile: PROFILE.into(),
                assurance: "none".into(),
                implementation_sha256: implementation_digest().unwrap(),
                source_path: "source.nmlt".into(),
                sources: program.sources().to_vec(),
                program_sha256: program.identity(),
                entry: "main".into(),
                inputs: Inputs::new(),
                max_steps: 100,
                max_jobs: 2,
                timeout_ms: 200,
                adapter: rt::worker::adapter(),
            };
            let spec = rt::RunSpec {
                run_id: "injected-host-failure".into(),
                context_sha256: context.digest(),
                limits: context.limits(),
            };
            let manifest = Manifest { context, spec };
            let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/r2-source-host-failure")
                .join(format!(
                    "{}-{}",
                    std::process::id(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("source.nmlt"), source).unwrap();
            let journal =
                Journal::create(&dir.join("journal.jsonl"), manifest.spec.clone()).unwrap();
            let mut host = Host::new(&manifest, Some(journal), None).unwrap();
            host.injected = Some(JobResult::Failure(Failure {
                kind: FailureKind::Timeout,
                child_reaped,
            }));
            let execution =
                nmlt_workflow::execute_with_host(&program, "main", &Inputs::new(), 100, &mut host)
                    .unwrap();
            assert!(matches!(
                execution.stop,
                nmlt_workflow::Stop::JobStopped {
                    reason: JobError::HostFailure,
                    ..
                }
            ));
            assert!(host.error.is_none());
            assert_eq!(host.events.len(), 1);
            assert_eq!(host.state.accounting().charged_work, 1);
            assert_eq!(host.state.attempts()[0].observed_work, None);
            assert!(matches!(
                host.state.attempts()[0].phase,
                rt::Phase::Uncertain { .. }
            ));
            let journal = host.journal.as_mut().unwrap().snapshot().unwrap();
            let record = Record {
                schema: SCHEMA.into(),
                profile: PROFILE.into(),
                assurance: "none".into(),
                manifest,
                events: host.events,
                journal: journal.clone(),
                accounting: host.state.accounting(),
                execution,
            };
            replay(
                &serde_json::to_vec(&record).unwrap(),
                &dir.join("source.nmlt"),
            )
            .unwrap();
            assert_eq!(host.journal.as_mut().unwrap().snapshot().unwrap(), journal);
        }
    }
}

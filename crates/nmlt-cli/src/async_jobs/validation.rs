//! Bind every source intent/reply, including interrupted replies, to exactly
//! the permitted lifecycle segment. Administrative settlements remain explicit.
use super::*;

fn administrative(command: &rt::Command) -> bool {
    matches!(
        command,
        rt::Command::Recover | rt::Command::Reconcile { .. }
    )
}
pub(super) struct Timeline {
    pub commands: Vec<rt::Command>,
    pub states: Vec<rt::Lifecycle>,
}
impl Timeline {
    pub fn new(snapshot: &session::Snapshot) -> Result<Self, String> {
        let expected = session::verify_snapshot(snapshot).map_err(|e| e.to_string())?;
        let (_, commands) =
            rt::replay_journal(snapshot.journal.as_bytes(), &snapshot.manifest.spec)
                .map_err(|e| e.to_string())?;
        let mut states =
            vec![rt::Lifecycle::new(snapshot.manifest.spec.clone()).map_err(|e| e.to_string())?];
        for command in &commands {
            let (next, _) = states
                .last()
                .expect("initial state")
                .step(command)
                .map_err(|e| e.to_string())?;
            states.push(next);
        }
        if states.last() != Some(&expected) {
            return Err("source timeline final state differs".into());
        }
        Ok(Self { commands, states })
    }
    fn gap(&self, before: usize, through: usize) -> Result<(), String> {
        if before > through
            || through > self.commands.len()
            || !self.commands[before..through].iter().all(administrative)
        {
            return Err("unexplained lifecycle transitions outside a source intent".into());
        }
        Ok(())
    }
    fn segment(
        &self,
        manifest: &session::Manifest,
        intent: &ledger::Intent,
        through: u32,
        reply: Option<&Result<Reply, JobError>>,
    ) -> Result<(), String> {
        let before = intent.before as usize;
        let through = through as usize;
        if before > through || through > self.commands.len() {
            return Err("invalid source intent/reply boundary".into());
        }
        let mut state = self.states[before].clone();
        let mut stage = 0;
        let mut cancelled = false;
        let mut observed = false;
        let mut collected = false;
        let (job, reservation) = match &intent.call {
            Call::Start { request } => {
                let job = state.attempts().len();
                (job, Some(reserve(manifest, job as u32, request)?))
            }
            Call::Control { job, .. } if (*job as usize) < state.attempts().len() => {
                (*job as usize, None)
            }
            _ => return Err("unknown source job reference".into()),
        };
        let limited = reservation.as_ref().is_some_and(|r| state.step(r).is_err());
        if limited && before != through {
            return Err("rejected source allocation changed the journal".into());
        }
        for command in &self.commands[before..through] {
            if !administrative(command) {
                match (&intent.call, command) {
                    (Call::Start { .. }, rt::Command::Reserve { .. })
                        if stage == 0 && Some(command) == reservation.as_ref() =>
                    {
                        stage = 1
                    }
                    (Call::Start { .. }, rt::Command::Dispatch { control })
                        if stage == 1 && *control == state.attempts()[job].control =>
                    {
                        stage = 2
                    }
                    (Call::Start { .. }, rt::Command::Timeout { control })
                        if stage == 2 && *control == state.attempts()[job].control && !observed =>
                    {
                        observed = true
                    }
                    (Call::Control { .. }, rt::Command::Deliver { response })
                        if !observed
                            && response.binding.attempt
                                == state.attempts()[job].control.attempt =>
                    {
                        observed = true
                    }
                    (Call::Control { .. }, rt::Command::Timeout { control })
                        if !observed && *control == state.attempts()[job].control =>
                    {
                        observed = true
                    }
                    (
                        Call::Control {
                            operation: JobOperation::Cancel,
                            ..
                        },
                        rt::Command::Cancel { control },
                    ) if !cancelled && *control == state.attempts()[job].control => {
                        cancelled = true
                    }
                    (
                        Call::Control {
                            operation: JobOperation::Collect,
                            ..
                        },
                        rt::Command::Collect { control },
                    ) if !collected && *control == state.attempts()[job].control => {
                        collected = true
                    }
                    _ => return Err(
                        "source operation contains an unrelated or repeated lifecycle transition"
                            .into(),
                    ),
                }
            }
            state = state.step(command).map_err(|e| e.to_string())?.0;
        }
        let Some(reply) = reply else {
            return Ok(());
        };
        let expected = match &intent.call {
            Call::Start { .. } if limited => Err(JobError::Limit),
            Call::Start { .. } if stage == 2 => Ok(Reply::Started { job: job as u32 }),
            Call::Start { .. } => {
                return Err("source start returned before a durable dispatch".into());
            }
            Call::Control { operation, .. } => match (&state.attempts()[job].phase, operation) {
                (rt::Phase::Finished { .. }, JobOperation::Poll) => Ok(Reply::Value {
                    value: Value::Bool(true),
                }),
                (rt::Phase::Running, JobOperation::Poll) => Ok(Reply::Value {
                    value: Value::Bool(false),
                }),
                (rt::Phase::Finished { outcome }, JobOperation::Cancel) => Ok(Reply::Value {
                    value: Value::Bool(cancelled && *outcome == rt::Outcome::Cancelled),
                }),
                (rt::Phase::Collected { outcome: result }, JobOperation::Collect) if collected => {
                    Ok(Reply::Value {
                        value: outcome(result),
                    })
                }
                (rt::Phase::Uncertain { .. }, _) => Err(JobError::HostFailure),
                _ => return Err("source control did not reach its required phase".into()),
            },
        };
        if *reply != expected {
            return Err("source operation reply disagrees with the lifecycle".into());
        }
        Ok(())
    }
}

pub(super) fn trace(
    context: &Context,
    snapshot: &session::Snapshot,
    decoded: &ledger::Decoded,
) -> Result<Timeline, String> {
    context.bind(&snapshot.manifest)?;
    if decoded.items.len()
        > 4 * context.max_steps as usize + snapshot.manifest.spec.limits.max_events as usize
    {
        return Err("too many source journal entries".into());
    }
    let timeline = Timeline::new(snapshot)?;
    for event in &decoded.replies {
        timeline.segment(
            &snapshot.manifest,
            &ledger::Intent {
                at: event.at,
                call: event.call.clone(),
                before: event.before,
            },
            event.through,
            Some(&event.result),
        )?;
    }
    let mut last_reply = None;
    let mut reply_cursor = 0;
    for item in &decoded.items {
        match item {
            Item::Reply { .. } => {
                last_reply = decoded.replies.get(reply_cursor);
                reply_cursor += 1;
            }
            Item::Retry => {
                let event = last_reply.ok_or("retry without captured failure")?;
                let Call::Control { job, .. } = event.call else {
                    return Err("only uncertain source controls can resume".into());
                };
                if event.result != Err(JobError::HostFailure)
                    || !matches!(
                        timeline.states[event.through as usize].attempts()[job as usize].phase,
                        rt::Phase::Uncertain { .. }
                    )
                {
                    return Err("retry did not follow a captured uncertain outcome".into());
                }
            }
            _ => {}
        }
    }
    let mut cursor = 0;
    for event in &decoded.events {
        timeline.gap(cursor, event.before as usize)?;
        cursor = event.through as usize;
    }
    if let Some(intent) = &decoded.pending {
        timeline.gap(cursor, intent.before as usize)?;
        timeline.segment(
            &snapshot.manifest,
            intent,
            timeline.commands.len() as u32,
            None,
        )?;
    } else {
        timeline.gap(cursor, timeline.commands.len())?;
    }
    Ok(timeline)
}

pub(super) fn record(record: &Record) -> Result<(), String> {
    if record.schema != SCHEMA {
        return Err("unsupported async source record".into());
    }
    let decoded = ledger::decode(record.trace.as_bytes(), &record.context.digest())?;
    trace(&record.context, &record.snapshot, &decoded)?;
    if decoded.pending.is_some() || decoded.events != record.events {
        return Err("effective source events differ from the durable trace".into());
    }
    Ok(())
}

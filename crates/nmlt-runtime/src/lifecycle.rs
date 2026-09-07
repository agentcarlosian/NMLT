use serde::{Deserialize, Serialize};

use crate::{
    AttemptId, Binding, Control, Dispatch, Error, Request, Response, ResponseOutcome, Value,
    valid_digest, valid_name,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub slots: u32,
    pub generations_per_slot: u32,
    /// Includes undispatched allocations; cancelling cannot recycle identity.
    pub max_attempts: u32,
    pub max_events: u32,
    pub work_budget: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunSpec {
    pub run_id: String,
    pub context_sha256: String,
    pub limits: Limits,
}

impl RunSpec {
    fn validate(&self) -> Result<(), Error> {
        let l = &self.limits;
        if !valid_name(&self.run_id)
            || !valid_digest(&self.context_sha256)
            || !(1..=64).contains(&l.slots)
            || !(1..=10_000).contains(&l.generations_per_slot)
            || !(1..=10_000).contains(&l.max_attempts)
            || !(1..=100_000).contains(&l.max_events)
            || l.work_budget == 0
        {
            return Err(Error("invalid run identity or lifecycle bounds".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    Completed { value: Value },
    Failed { message: String },
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Phase {
    Reserved,
    Running,
    CancelRequested,
    Uncertain { cancellation_requested: bool },
    Finished { outcome: Outcome },
    Collected { outcome: Outcome },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub control: Control,
    pub request: Request,
    pub phase: Phase,
    pub dispatch: Option<Binding>,
    pub observed_work: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accounting {
    pub allocated_attempts: u32,
    pub dispatched_attempts: u32,
    pub reserved_work: u64,
    pub charged_work: u64,
    pub available_work: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Reserve {
        task: String,
        owner: String,
        request: Request,
    },
    Transfer {
        control: Control,
        to: String,
    },
    Dispatch {
        control: Control,
    },
    Cancel {
        control: Control,
    },
    Timeout {
        control: Control,
    },
    Deliver {
        response: Response,
    },
    Reconcile {
        control: Control,
        response: Response,
    },
    Collect {
        control: Control,
    },
    Recover,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Receipt {
    Control {
        control: Control,
    },
    Dispatch {
        dispatch: Dispatch,
        control: Control,
    },
    Ignored {
        reason: String,
    },
    Collected {
        attempt: AttemptId,
        outcome: Outcome,
    },
    Recovered {
        uncertain_attempts: usize,
    },
}

/// Pure transition model for tests and inspection. Only Journal commits are
/// allowed to release work to a host adapter; cloning this model is not fencing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Lifecycle {
    spec: RunSpec,
    attempts: Vec<Attempt>,
    generations: Vec<u32>,
    slots: Vec<Option<usize>>,
    event_count: u32,
}

impl Lifecycle {
    pub fn new(spec: RunSpec) -> Result<Self, Error> {
        spec.validate()?;
        let count = spec.limits.slots as usize;
        Ok(Self {
            spec,
            attempts: vec![],
            generations: vec![0; count],
            slots: vec![None; count],
            event_count: 0,
        })
    }
    #[must_use]
    pub fn spec(&self) -> &RunSpec {
        &self.spec
    }
    #[must_use]
    pub fn attempts(&self) -> &[Attempt] {
        &self.attempts
    }
    #[must_use]
    pub fn event_count(&self) -> u32 {
        self.event_count
    }
    #[must_use]
    pub fn accounting(&self) -> Accounting {
        let mut a = Accounting {
            allocated_attempts: self.attempts.len() as u32,
            dispatched_attempts: 0,
            reserved_work: 0,
            charged_work: 0,
            available_work: 0,
        };
        for attempt in &self.attempts {
            if attempt.dispatch.is_some() {
                a.dispatched_attempts += 1;
                a.charged_work += attempt.request.reserved_work;
            } else if matches!(attempt.phase, Phase::Reserved) {
                a.reserved_work += attempt.request.reserved_work;
            }
        }
        a.available_work = self.spec.limits.work_budget - a.charged_work - a.reserved_work;
        a
    }
    /// Rejected operations leave the previous value unchanged.
    pub fn step(&self, command: &Command) -> Result<(Self, Receipt), Error> {
        if self.event_count == self.spec.limits.max_events {
            return Err(Error("event limit exhausted".into()));
        }
        let mut next = self.clone();
        let receipt = next.apply(command)?;
        next.event_count += 1;
        Ok((next, receipt))
    }

    fn controlled(&self, control: &Control) -> Result<usize, Error> {
        let index = self
            .slots
            .get(control.attempt.slot as usize)
            .and_then(|i| *i)
            .ok_or_else(|| Error("stale or unallocated control".into()))?;
        if self.attempts[index].control != *control {
            return Err(Error("stale attempt, owner, or control revision".into()));
        }
        Ok(index)
    }
    fn control_receipt(&mut self, index: usize) -> Receipt {
        self.attempts[index].control.revision += 1;
        Receipt::Control {
            control: self.attempts[index].control.clone(),
        }
    }
    fn apply(&mut self, command: &Command) -> Result<Receipt, Error> {
        match command {
            Command::Reserve {
                task,
                owner,
                request,
            } => {
                request.validate()?;
                if !valid_name(task)
                    || !valid_name(owner)
                    || request.context_sha256 != self.spec.context_sha256
                {
                    return Err(Error("invalid task, owner, or request context".into()));
                }
                if self.attempts.len() >= self.spec.limits.max_attempts as usize {
                    return Err(Error("attempt allocation limit exhausted".into()));
                }
                if request.reserved_work > self.accounting().available_work {
                    return Err(Error("work reservation exceeds available budget".into()));
                }
                let slot = self
                    .slots
                    .iter()
                    .enumerate()
                    .position(|(i, s)| {
                        s.is_none() && self.generations[i] < self.spec.limits.generations_per_slot
                    })
                    .ok_or_else(|| Error("no free slot with an unused generation".into()))?;
                self.generations[slot] += 1;
                let control = Control {
                    attempt: AttemptId {
                        run: self.spec.run_id.clone(),
                        task: task.clone(),
                        slot: slot as u32,
                        generation: self.generations[slot],
                    },
                    owner: owner.clone(),
                    revision: 0,
                };
                self.slots[slot] = Some(self.attempts.len());
                self.attempts.push(Attempt {
                    control: control.clone(),
                    request: request.clone(),
                    phase: Phase::Reserved,
                    dispatch: None,
                    observed_work: None,
                });
                Ok(Receipt::Control { control })
            }
            Command::Transfer { control, to } => {
                let i = self.controlled(control)?;
                if !valid_name(to)
                    || to == &control.owner
                    || !matches!(self.attempts[i].phase, Phase::Reserved)
                {
                    return Err(Error(
                        "transfer requires a reserved attempt and a different valid owner".into(),
                    ));
                }
                self.attempts[i].control.owner.clone_from(to);
                Ok(self.control_receipt(i))
            }
            Command::Dispatch { control } => {
                let i = self.controlled(control)?;
                let attempt = &mut self.attempts[i];
                if !matches!(attempt.phase, Phase::Reserved) {
                    return Err(Error("dispatch requires unused reserved control".into()));
                }
                let binding = Binding {
                    attempt: control.attempt.clone(),
                    adapter: attempt.request.adapter.clone(),
                    dispatched_by: control.owner.clone(),
                    context_sha256: attempt.request.context_sha256.clone(),
                    input_sha256: attempt.request.input_digest()?,
                };
                attempt.dispatch = Some(binding.clone());
                attempt.phase = Phase::Running;
                attempt.control.revision += 1;
                Ok(Receipt::Dispatch {
                    dispatch: Dispatch {
                        binding,
                        input: attempt.request.input.clone(),
                    },
                    control: attempt.control.clone(),
                })
            }
            Command::Cancel { control } => {
                let i = self.controlled(control)?;
                self.attempts[i].phase = match self.attempts[i].phase {
                    Phase::Reserved => Phase::Finished {
                        outcome: Outcome::Cancelled,
                    },
                    Phase::Running => Phase::CancelRequested,
                    Phase::Uncertain {
                        cancellation_requested: false,
                    } => Phase::Uncertain {
                        cancellation_requested: true,
                    },
                    _ => {
                        return Err(Error(
                            "attempt cannot request cancellation in this phase".into(),
                        ));
                    }
                };
                Ok(self.control_receipt(i))
            }
            Command::Timeout { control } => {
                let i = self.controlled(control)?;
                let cancelled = match self.attempts[i].phase {
                    Phase::Running => false,
                    Phase::CancelRequested => true,
                    _ => return Err(Error("timeout requires in-flight work".into())),
                };
                self.attempts[i].phase = Phase::Uncertain {
                    cancellation_requested: cancelled,
                };
                Ok(self.control_receipt(i))
            }
            Command::Deliver { response } => self.response(response, None),
            Command::Reconcile { control, response } => self.response(response, Some(control)),
            Command::Collect { control } => {
                let i = self.controlled(control)?;
                let Phase::Finished { outcome } = self.attempts[i].phase.clone() else {
                    return Err(Error(
                        "collection requires a settled terminal outcome".into(),
                    ));
                };
                self.attempts[i].phase = Phase::Collected {
                    outcome: outcome.clone(),
                };
                self.attempts[i].control.revision += 1;
                self.slots[control.attempt.slot as usize] = None;
                Ok(Receipt::Collected {
                    attempt: control.attempt.clone(),
                    outcome,
                })
            }
            Command::Recover => {
                let mut uncertain_attempts = 0;
                for attempt in &mut self.attempts {
                    match attempt.phase {
                        Phase::Collected { .. } => continue,
                        Phase::Running => {
                            attempt.phase = Phase::Uncertain {
                                cancellation_requested: false,
                            }
                        }
                        Phase::CancelRequested => {
                            attempt.phase = Phase::Uncertain {
                                cancellation_requested: true,
                            }
                        }
                        _ => {}
                    }
                    if matches!(attempt.phase, Phase::Uncertain { .. }) {
                        uncertain_attempts += 1;
                    }
                    attempt.control.revision += 1;
                }
                Ok(Receipt::Recovered { uncertain_attempts })
            }
        }
    }

    fn response(
        &mut self,
        response: &Response,
        control: Option<&Control>,
    ) -> Result<Receipt, Error> {
        response.validate()?;
        // Owner commands must validate current control even when the supplied
        // response names an already collected or otherwise unallocated slot.
        let controlled = control.map(|c| self.controlled(c)).transpose()?;
        let Some(i) = self
            .slots
            .get(response.binding.attempt.slot as usize)
            .and_then(|v| *v)
        else {
            return Ok(Receipt::Ignored {
                reason: "stale or unallocated attempt".into(),
            });
        };
        if let Some(controlled) = controlled
            && controlled != i
        {
            return Err(Error("reconciliation control names another attempt".into()));
        }
        let attempt = &self.attempts[i];
        if attempt.dispatch.as_ref() != Some(&response.binding) {
            return Ok(Receipt::Ignored {
                reason: "response binding does not match the active dispatch".into(),
            });
        }
        let cancelled = match (&attempt.phase, control.is_some()) {
            (Phase::Running, false) => false,
            (Phase::CancelRequested, _) => true,
            (
                Phase::Uncertain {
                    cancellation_requested,
                },
                true,
            ) => *cancellation_requested,
            _ => {
                return Ok(Receipt::Ignored {
                    reason: "response requires reconciliation or attempt is already terminal"
                        .into(),
                });
            }
        };
        if let ResponseOutcome::Completed { value } = &response.outcome
            && value.value_type() != attempt.request.adapter.output_type
        {
            return Err(Error(
                "response output type differs from the dispatch contract".into(),
            ));
        }
        let outcome = if cancelled {
            if control.is_none() && !matches!(response.outcome, ResponseOutcome::Cancelled) {
                return Ok(Receipt::Ignored {
                    reason: "late result after cancellation requires reconciliation".into(),
                });
            }
            Outcome::Cancelled
        } else {
            match &response.outcome {
                ResponseOutcome::Completed { value } => Outcome::Completed {
                    value: value.clone(),
                },
                ResponseOutcome::Failed { message } => Outcome::Failed {
                    message: message.clone(),
                },
                ResponseOutcome::Cancelled => {
                    return Err(Error("unsolicited cancellation acknowledgement".into()));
                }
            }
        };
        self.attempts[i].phase = Phase::Finished { outcome };
        self.attempts[i].observed_work = response.observed_work;
        Ok(self.control_receipt(i))
    }
}

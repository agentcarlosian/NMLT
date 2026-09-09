//! A frozen finite Rust regression alphabet, not a Lean/model-check certificate.
use nmlt_runtime::*;
use std::collections::{BTreeSet, VecDeque};

fn invariants(state: &Lifecycle) {
    let spec = state.spec();
    let mut identities = BTreeSet::new();
    let mut occupied = BTreeSet::new();
    let mut generations = vec![0; spec.limits.slots as usize];
    let mut reserved = 0_u128;
    let mut charged = 0_u128;
    for a in state.attempts() {
        let id = &a.control.attempt;
        assert_eq!(id.run, spec.run_id);
        assert!(id.slot < spec.limits.slots);
        assert!((1..=spec.limits.generations_per_slot).contains(&id.generation));
        assert!(identities.insert((id.slot, id.generation)));
        assert!(id.generation > generations[id.slot as usize]);
        generations[id.slot as usize] = id.generation;
        if !matches!(a.phase, Phase::Collected { .. }) {
            assert!(occupied.insert(id.slot));
        }
        assert!(a.control.revision <= state.event_count() as u64);
        if let Some(binding) = &a.dispatch {
            assert_eq!(&binding.attempt, id);
            assert!(!matches!(a.phase, Phase::Reserved));
            charged += a.request.reserved_work as u128;
        } else {
            assert!(matches!(
                a.phase,
                Phase::Reserved
                    | Phase::Finished {
                        outcome: Outcome::Cancelled
                    }
                    | Phase::Collected {
                        outcome: Outcome::Cancelled
                    }
            ));
            assert!(a.observed_work.is_none());
            if matches!(a.phase, Phase::Reserved) {
                reserved += a.request.reserved_work as u128;
            }
        }
        if let Phase::Finished {
            outcome: Outcome::Completed { value },
        }
        | Phase::Collected {
            outcome: Outcome::Completed { value },
        } = &a.phase
        {
            assert_eq!(value.value_type(), a.request.adapter.output_type);
        }
    }
    assert!(state.attempts().len() <= spec.limits.max_attempts as usize);
    assert!(reserved + charged <= spec.limits.work_budget as u128);
    let accounting = state.accounting();
    assert_eq!(accounting.reserved_work as u128, reserved);
    assert_eq!(accounting.charged_work as u128, charged);
    assert_eq!(
        reserved + charged + accounting.available_work as u128,
        spec.limits.work_budget as u128
    );
}

fn commands(state: &Lifecycle) -> Vec<Command> {
    let mut commands = vec![Command::Recover];
    for owner in ["a", "b"] {
        commands.push(Command::Reserve {
            task: "task".into(),
            owner: owner.into(),
            request: Request {
                adapter: worker::adapter(),
                context_sha256: state.spec().context_sha256.clone(),
                input: Value::Int(0),
                reserved_work: 1,
            },
        });
    }
    for a in state.attempts() {
        let c = a.control.clone();
        commands.extend([
            Command::Dispatch { control: c.clone() },
            Command::Cancel { control: c.clone() },
            Command::Timeout { control: c.clone() },
            Command::Collect { control: c.clone() },
            Command::Transfer {
                control: c.clone(),
                to: if c.owner == "a" { "b" } else { "a" }.into(),
            },
        ]);
        if let Some(binding) = &a.dispatch {
            for outcome in [
                ResponseOutcome::Completed {
                    value: Value::Int(0),
                },
                ResponseOutcome::Failed {
                    message: "failure".into(),
                },
                ResponseOutcome::Cancelled,
            ] {
                let response = Response {
                    schema: RESPONSE_SCHEMA.into(),
                    binding: binding.clone(),
                    outcome,
                    observed_work: None,
                };
                commands.push(Command::Deliver {
                    response: response.clone(),
                });
                commands.push(Command::Reconcile {
                    control: c.clone(),
                    response,
                });
            }
        }
    }
    commands
}

#[test]
fn frozen_two_slot_four_event_sequences_preserve_lifecycle_invariants() {
    let initial = Lifecycle::new(RunSpec {
        run_id: "bounded".into(),
        context_sha256: sha256(b"bounded"),
        limits: Limits {
            slots: 2,
            generations_per_slot: 2,
            max_attempts: 2,
            max_events: 4,
            work_budget: 2,
        },
    })
    .unwrap();
    let mut seen = BTreeSet::from([serde_json::to_vec(&initial).unwrap()]);
    let mut queue = VecDeque::from([initial]);
    let mut transitions = 0;
    while let Some(state) = queue.pop_front() {
        invariants(&state);
        for command in commands(&state) {
            if let Ok((next, _)) = state.step(&command) {
                assert!(next.accounting().charged_work >= state.accounting().charged_work);
                assert!(
                    next.accounting().dispatched_attempts >= state.accounting().dispatched_attempts
                );
                transitions += 1;
                if seen.insert(serde_json::to_vec(&next).unwrap()) {
                    queue.push_back(next);
                }
            }
        }
        assert!(seen.len() < 50_000, "unexpected regression-alphabet growth");
    }
    println!(
        "bounded lifecycle regression: {} states, {transitions} accepted transitions",
        seen.len()
    );
    assert_eq!((seen.len(), transitions), (291, 520));
}

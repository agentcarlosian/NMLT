use nmlt_runtime::*;

fn spec() -> RunSpec {
    RunSpec {
        run_id: "run-a".into(),
        context_sha256: sha256(b"fixed context"),
        limits: Limits {
            slots: 1,
            generations_per_slot: 3,
            max_attempts: 3,
            max_events: 80,
            work_budget: 6,
        },
    }
}
fn request(value: i64) -> Request {
    Request {
        adapter: worker::adapter(),
        context_sha256: spec().context_sha256,
        input: Value::Int(value),
        reserved_work: 2,
    }
}
fn step(state: &mut Lifecycle, command: Command) -> Receipt {
    let (next, receipt) = state.step(&command).unwrap();
    *state = next;
    receipt
}
fn controlled(receipt: Receipt) -> Control {
    match receipt {
        Receipt::Control { control } | Receipt::Dispatch { control, .. } => control,
        _ => panic!("expected control"),
    }
}
fn reserve(state: &mut Lifecycle) -> Control {
    controlled(step(
        state,
        Command::Reserve {
            task: "task".into(),
            owner: "coordinator".into(),
            request: request(3),
        },
    ))
}
fn dispatch(state: &mut Lifecycle, control: Control) -> (Control, Dispatch) {
    let Receipt::Dispatch { control, dispatch } = step(state, Command::Dispatch { control }) else {
        panic!()
    };
    (control, dispatch)
}
fn rejected(state: &Lifecycle, command: Command) {
    let before = state.clone();
    assert!(state.step(&command).is_err());
    assert_eq!(state, &before);
}

#[test]
fn ownership_revisions_prevent_duplicate_and_aba_control() {
    let mut s = Lifecycle::new(spec()).unwrap();
    let initial = reserve(&mut s);
    let mut forged = initial.clone();
    forged.owner = "other".into();
    rejected(&s, Command::Dispatch { control: forged });
    let worker = controlled(step(
        &mut s,
        Command::Transfer {
            control: initial.clone(),
            to: "worker".into(),
        },
    ));
    rejected(
        &s,
        Command::Dispatch {
            control: initial.clone(),
        },
    );
    let returned = controlled(step(
        &mut s,
        Command::Transfer {
            control: worker.clone(),
            to: "coordinator".into(),
        },
    ));
    rejected(&s, Command::Dispatch { control: initial });
    rejected(&s, Command::Dispatch { control: worker });
    let (running, _) = dispatch(&mut s, returned.clone());
    rejected(&s, Command::Dispatch { control: returned });
    rejected(
        &s,
        Command::Dispatch {
            control: running.clone(),
        },
    );
    rejected(
        &s,
        Command::Transfer {
            control: running,
            to: "another".into(),
        },
    );
    assert_eq!(s.accounting().dispatched_attempts, 1);
}

#[test]
fn terminal_collection_consumes_control_but_keeps_reusable_values() {
    let mut s = Lifecycle::new(spec()).unwrap();
    let c = reserve(&mut s);
    let (running, d) = dispatch(&mut s, c);
    rejected(&s, Command::Collect { control: running });
    let response = worker::evaluate(&d).unwrap();
    let c = controlled(step(
        &mut s,
        Command::Deliver {
            response: response.clone(),
        },
    ));
    let before = s.attempts().to_vec();
    assert!(matches!(
        step(&mut s, Command::Deliver { response }),
        Receipt::Ignored { .. }
    ));
    assert_eq!(s.attempts(), before);
    let receipt = step(&mut s, Command::Collect { control: c.clone() });
    let Receipt::Collected {
        outcome: Outcome::Completed { value },
        ..
    } = receipt
    else {
        panic!()
    };
    assert_eq!([value.clone(), value], [Value::Int(9), Value::Int(9)]);
    rejected(&s, Command::Collect { control: c.clone() });
    rejected(
        &s,
        Command::Reconcile {
            control: c,
            response: worker::evaluate(&d).unwrap(),
        },
    );
    assert_eq!(s.accounting().charged_work, 2);
    assert_eq!(s.attempts()[0].observed_work, Some(1));
    assert!(matches!(s.attempts()[0].phase, Phase::Collected { .. }));
}

#[test]
fn cancellation_waits_for_ack_and_stale_completion_cannot_settle_reused_slot() {
    let mut s = Lifecycle::new(spec()).unwrap();
    let c = reserve(&mut s);
    let (running, first) = dispatch(&mut s, c);
    let cancelling = controlled(step(&mut s, Command::Cancel { control: running }));
    let before = s.accounting();
    let old_response = worker::evaluate(&first).unwrap();
    assert!(matches!(
        step(
            &mut s,
            Command::Deliver {
                response: old_response.clone()
            }
        ),
        Receipt::Ignored { .. }
    ));
    assert_eq!(s.accounting(), before);
    assert_eq!(s.attempts()[0].phase, Phase::CancelRequested);
    rejected(
        &s,
        Command::Collect {
            control: cancelling,
        },
    );
    rejected(
        &s,
        Command::Reserve {
            task: "new".into(),
            owner: "worker".into(),
            request: request(5),
        },
    );
    let mut ack = old_response.clone();
    ack.outcome = ResponseOutcome::Cancelled;
    ack.observed_work = None;
    let c = controlled(step(&mut s, Command::Deliver { response: ack }));
    step(&mut s, Command::Collect { control: c });
    let c = reserve(&mut s);
    assert_eq!(c.attempt.generation, 2);
    let (second_control, _) = dispatch(&mut s, c);
    let current = s.attempts()[1].clone();
    let accounting = s.accounting();
    assert!(matches!(
        step(
            &mut s,
            Command::Deliver {
                response: old_response
            }
        ),
        Receipt::Ignored { .. }
    ));
    assert_eq!(s.attempts()[1], current);
    assert_eq!(s.attempts()[1].control, second_control);
    assert_eq!(s.accounting(), accounting);
    assert_eq!(accounting.charged_work, 4);
    assert_eq!(s.attempts()[0].observed_work, None);
}

#[test]
fn only_undispatched_work_is_refunded_and_generations_never_reset() {
    let mut spec = spec();
    spec.limits.generations_per_slot = 2;
    let mut s = Lifecycle::new(spec).unwrap();
    for generation in 1..=2 {
        let c = reserve(&mut s);
        assert_eq!(c.attempt.generation, generation);
        assert_eq!(s.accounting().reserved_work, 2);
        let c = controlled(step(&mut s, Command::Cancel { control: c }));
        assert_eq!(s.accounting().available_work, 6);
        assert_eq!(s.accounting().dispatched_attempts, 0);
        step(&mut s, Command::Collect { control: c });
    }
    rejected(
        &s,
        Command::Reserve {
            task: "third".into(),
            owner: "coordinator".into(),
            request: request(5),
        },
    );
    assert_eq!(s.accounting().allocated_attempts, 2);
}

#[test]
fn restart_classifies_inflight_work_and_requires_explicit_reconciliation() {
    let mut s = Lifecycle::new(spec()).unwrap();
    let old = reserve(&mut s);
    let (running, d) = dispatch(&mut s, old);
    let accounting = s.accounting();
    let before = s.clone();
    let a = s.step(&Command::Recover).unwrap();
    let b = before.step(&Command::Recover).unwrap();
    assert_eq!(a, b);
    s = a.0;
    assert_eq!(
        s.attempts()[0].phase,
        Phase::Uncertain {
            cancellation_requested: false
        }
    );
    rejected(
        &s,
        Command::Dispatch {
            control: running.clone(),
        },
    );
    rejected(&s, Command::Cancel { control: running });
    assert_eq!(s.accounting(), accounting);
    let response = worker::evaluate(&d).unwrap();
    assert!(matches!(
        step(
            &mut s,
            Command::Deliver {
                response: response.clone()
            }
        ),
        Receipt::Ignored { .. }
    ));
    let control = s.attempts()[0].control.clone();
    let c = controlled(step(&mut s, Command::Reconcile { control, response }));
    step(&mut s, Command::Collect { control: c });
    assert_eq!(s.accounting().dispatched_attempts, 1);
}

#[test]
fn cancelled_uncertain_work_never_promotes_a_late_success() {
    for cancel_before_restart in [false, true] {
        let mut s = Lifecycle::new(spec()).unwrap();
        let c = reserve(&mut s);
        let (mut c, d) = dispatch(&mut s, c);
        if cancel_before_restart {
            c = controlled(step(&mut s, Command::Cancel { control: c }));
        }
        let _ = c;
        step(&mut s, Command::Recover);
        let mut c = s.attempts()[0].control.clone();
        if !cancel_before_restart {
            c = controlled(step(&mut s, Command::Cancel { control: c }));
        }
        let response = worker::evaluate(&d).unwrap();
        let c = controlled(step(
            &mut s,
            Command::Reconcile {
                control: c,
                response,
            },
        ));
        let Receipt::Collected { outcome, .. } = step(&mut s, Command::Collect { control: c })
        else {
            panic!()
        };
        assert_eq!(outcome, Outcome::Cancelled);
        assert_eq!(s.accounting().charged_work, 2);
        assert_eq!(s.attempts()[0].observed_work, Some(1));
    }
}

#[test]
fn timeout_is_unknown_and_preserves_charge_even_if_reported_usage_is_larger() {
    let mut s = Lifecycle::new(spec()).unwrap();
    let c = reserve(&mut s);
    let (c, d) = dispatch(&mut s, c);
    let c = controlled(step(&mut s, Command::Timeout { control: c }));
    assert_eq!(s.attempts()[0].observed_work, None);
    let mut response = worker::evaluate(&d).unwrap();
    response.observed_work = Some(u64::MAX);
    step(
        &mut s,
        Command::Reconcile {
            control: c,
            response,
        },
    );
    assert_eq!(s.attempts()[0].observed_work, Some(u64::MAX));
    assert_eq!(s.accounting().charged_work, 2);
}

#[test]
fn every_response_binding_dimension_is_checked_before_settlement() {
    let mut s = Lifecycle::new(spec()).unwrap();
    let c = reserve(&mut s);
    let (_, d) = dispatch(&mut s, c);
    let original = worker::evaluate(&d).unwrap();
    type BindingMutation = Box<dyn Fn(&mut Binding)>;
    let changes: Vec<BindingMutation> = vec![
        Box::new(|b| b.attempt.run = "other-run".into()),
        Box::new(|b| b.attempt.task = "other-task".into()),
        Box::new(|b| b.attempt.slot = 1),
        Box::new(|b| b.attempt.generation += 1),
        Box::new(|b| b.dispatched_by = "other-owner".into()),
        Box::new(|b| b.input_sha256 = sha256(b"other input")),
        Box::new(|b| b.context_sha256 = sha256(b"other context")),
        Box::new(|b| b.adapter.name = "other-adapter".into()),
        Box::new(|b| b.adapter.output_type = ValueType::Bool),
        Box::new(|b| b.adapter.input_type = ValueType::Text),
    ];
    for change in changes {
        let mut response = original.clone();
        change(&mut response.binding);
        let before = s.attempts().to_vec();
        let accounting = s.accounting();
        assert!(matches!(
            step(&mut s, Command::Deliver { response }),
            Receipt::Ignored { .. }
        ));
        assert_eq!(s.attempts(), before);
        assert_eq!(s.accounting(), accounting);
    }
    let mut wrong = original.clone();
    wrong.schema = "v2".into();
    rejected(&s, Command::Deliver { response: wrong });
    let mut wrong = original.clone();
    wrong.binding.adapter.version = 2;
    rejected(&s, Command::Deliver { response: wrong });
    let mut wrong = original.clone();
    wrong.outcome = ResponseOutcome::Completed {
        value: Value::Bool(true),
    };
    rejected(&s, Command::Deliver { response: wrong });
    let mut wrong = original;
    wrong.outcome = ResponseOutcome::Cancelled;
    rejected(&s, Command::Deliver { response: wrong });
}

#[test]
fn bounds_cover_slots_allocation_events_and_work_without_overflow() {
    let mut configuration = spec();
    configuration.limits.work_budget = u64::MAX;
    configuration.limits.slots = 2;
    let mut s = Lifecycle::new(configuration).unwrap();
    let mut big = request(3);
    big.reserved_work = u64::MAX;
    step(
        &mut s,
        Command::Reserve {
            task: "big".into(),
            owner: "worker".into(),
            request: big,
        },
    );
    rejected(
        &s,
        Command::Reserve {
            task: "small".into(),
            owner: "worker".into(),
            request: request(3),
        },
    );
    let mut one = spec();
    one.limits.max_events = 1;
    let mut s = Lifecycle::new(one).unwrap();
    let c = reserve(&mut s);
    rejected(&s, Command::Dispatch { control: c });
    for field in [
        "slots",
        "generations_per_slot",
        "max_attempts",
        "max_events",
        "work_budget",
    ] {
        let mut value = serde_json::to_value(spec()).unwrap();
        value["limits"][field] = 0.into();
        assert!(Lifecycle::new(serde_json::from_value(value).unwrap()).is_err());
    }
    let mut one = spec();
    one.limits.max_attempts = 1;
    let mut s = Lifecycle::new(one).unwrap();
    let c = reserve(&mut s);
    let c = controlled(step(&mut s, Command::Cancel { control: c }));
    step(&mut s, Command::Collect { control: c });
    rejected(
        &s,
        Command::Reserve {
            task: "new".into(),
            owner: "worker".into(),
            request: request(3),
        },
    );
}

#[test]
fn adapter_checks_arithmetic_in_addition_to_the_typed_envelope() {
    for (input, expected) in [(3, Some(9)), (-3, None), (i64::MAX, None)] {
        let mut s = Lifecycle::new(spec()).unwrap();
        let c = controlled(step(
            &mut s,
            Command::Reserve {
                task: "square".into(),
                owner: "worker".into(),
                request: request(input),
            },
        ));
        let (_, d) = dispatch(&mut s, c);
        let response = worker::evaluate(&d).unwrap();
        worker::validate(&d, &response).unwrap();
        assert_eq!(
            match response.outcome {
                ResponseOutcome::Completed {
                    value: Value::Int(v),
                } => Some(v),
                _ => None,
            },
            expected
        );
        let mut forged = response;
        forged.outcome = ResponseOutcome::Completed {
            value: Value::Int(123),
        };
        assert!(worker::validate(&d, &forged).is_err());
    }
}

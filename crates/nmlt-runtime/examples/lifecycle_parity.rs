//! Differential parity runner comparing Rust `nmlt_runtime::Lifecycle` against
//! Lean `NMLT.Behavior.WorkflowSessionBridge` (`mechanization/lean/tests/LifecycleParity.lean`).
use nmlt_runtime::{
    Adapter, AttemptId, Binding, Command, Control, Lifecycle, Limits, Outcome, Phase,
    RESPONSE_SCHEMA, Receipt, Request, Response, ResponseOutcome, RunSpec, Value, ValueType,
    sha256,
};

fn adapter() -> Adapter {
    Adapter {
        name: "parity".into(),
        version: 1,
        input_type: ValueType::Int,
        output_type: ValueType::Int,
    }
}

fn owner_name(id: u32) -> String {
    format!("owner_{id}")
}

fn owner_id(name: &str) -> u32 {
    name.strip_prefix("owner_")
        .and_then(|s| s.parse().ok())
        .expect("numeric owner_N")
}

fn phase_tag(phase: &Phase) -> String {
    match phase {
        Phase::Reserved => "reserved".into(),
        Phase::Running => "running".into(),
        Phase::CancelRequested => "cancel_requested".into(),
        Phase::Uncertain {
            cancellation_requested: false,
        } => "uncertain(false)".into(),
        Phase::Uncertain {
            cancellation_requested: true,
        } => "uncertain(true)".into(),
        Phase::Finished { outcome } => format!("finished({})", outcome_tag(outcome)),
        Phase::Collected { outcome } => format!("collected({})", outcome_tag(outcome)),
    }
}

fn outcome_tag(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Completed {
            value: Value::Int(v),
        } => format!("ok:{v}"),
        Outcome::Failed { message } => format!("err:{message}"),
        Outcome::Cancelled => "cancelled".into(),
        _ => unreachable!(),
    }
}

fn receipt_tag(receipt: &Receipt) -> String {
    match receipt {
        Receipt::Control { control } => format!(
            "control({},{},{},{})",
            control.attempt.slot,
            control.attempt.generation,
            owner_id(&control.owner),
            control.revision
        ),
        Receipt::Dispatch { control, .. } => format!(
            "dispatch({},{},{},{})",
            control.attempt.slot,
            control.attempt.generation,
            owner_id(&control.owner),
            control.revision
        ),
        Receipt::Ignored { .. } => "ignored".into(),
        Receipt::Collected { attempt, outcome } => format!(
            "collected({},{},{})",
            attempt.slot,
            attempt.generation,
            outcome_tag(outcome)
        ),
        Receipt::Recovered { uncertain_attempts } => format!("recovered({uncertain_attempts})"),
    }
}

fn slots_summary(lc: &Lifecycle) -> String {
    let slots = lc.spec().limits.slots as usize;
    let mut active: Vec<Option<usize>> = vec![None; slots];
    for (idx, attempt) in lc.attempts().iter().enumerate() {
        if !matches!(attempt.phase, Phase::Collected { .. }) {
            active[attempt.control.attempt.slot as usize] = Some(idx);
        }
    }
    active
        .into_iter()
        .map(|opt| match opt {
            None => "_".to_owned(),
            Some(idx) => {
                let a = &lc.attempts()[idx];
                format!("g{}:{}", a.control.attempt.generation, phase_tag(&a.phase))
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn run_step(idx: usize, lc: Lifecycle, cmd: &Command) -> (Lifecycle, Option<Receipt>) {
    match lc.step(cmd) {
        Ok((next, receipt)) => {
            let acc = next.accounting();
            println!(
                "STEP|{idx}|OK|{}|{}|{}|{}|{}",
                receipt_tag(&receipt),
                acc.charged_work,
                acc.reserved_work,
                acc.available_work,
                slots_summary(&next)
            );
            (next, Some(receipt))
        }
        Err(_) => {
            let acc = lc.accounting();
            println!(
                "STEP|{idx}|ERR|-|{}|{}|{}|{}",
                acc.charged_work,
                acc.reserved_work,
                acc.available_work,
                slots_summary(&lc)
            );
            (lc, None)
        }
    }
}

fn make_request(context: &str, reserved_work: u64) -> Request {
    Request {
        adapter: adapter(),
        context_sha256: context.into(),
        input: Value::Int(1),
        reserved_work,
    }
}

fn make_response(binding: Binding, outcome: ResponseOutcome) -> Response {
    Response {
        schema: RESPONSE_SCHEMA.into(),
        binding,
        outcome,
        observed_work: Some(1),
    }
}

fn main() {
    let context = sha256(b"lifecycle-parity-v1");
    let spec = RunSpec {
        run_id: "parity-run".into(),
        context_sha256: context.clone(),
        limits: Limits {
            slots: 2,
            generations_per_slot: 2,
            max_attempts: 4,
            max_events: 32,
            work_budget: 5,
        },
    };
    let lc = Lifecycle::new(spec).expect("valid spec");

    // 1. Reserve slot 0 (owner 1, work 2)
    let (lc, r1) = run_step(
        1,
        lc,
        &Command::Reserve {
            task: "t1".into(),
            owner: owner_name(1),
            request: make_request(&context, 2),
        },
    );
    let Receipt::Control { control: c0_r0 } = r1.unwrap() else {
        unreachable!()
    };

    // 2. Transfer slot 0 from owner 1 to owner 2
    let (lc, r2) = run_step(
        2,
        lc,
        &Command::Transfer {
            control: c0_r0,
            to: owner_name(2),
        },
    );
    let Receipt::Control { control: c0_r1 } = r2.unwrap() else {
        unreachable!()
    };

    // 3. Dispatch slot 0
    let (lc, r3) = run_step(3, lc, &Command::Dispatch { control: c0_r1 });
    let Receipt::Dispatch {
        dispatch: d0,
        control: c0_r2,
    } = r3.unwrap()
    else {
        unreachable!()
    };

    // 4. Reserve slot 1 (owner 1, work 2)
    let (lc, r4) = run_step(
        4,
        lc,
        &Command::Reserve {
            task: "t2".into(),
            owner: owner_name(1),
            request: make_request(&context, 2),
        },
    );
    let Receipt::Control { control: c1_r0 } = r4.unwrap() else {
        unreachable!()
    };

    // 5. Reserve exceeding budget (work 2 > avail 1) -> ERR
    let (lc, _) = run_step(
        5,
        lc,
        &Command::Reserve {
            task: "t_over".into(),
            owner: owner_name(1),
            request: make_request(&context, 2),
        },
    );

    // 6. Deliver stale generation (slot 0, gen 2) -> IGNORED
    let mut stale_binding = d0.binding.clone();
    stale_binding.attempt = AttemptId {
        generation: 2,
        ..stale_binding.attempt
    };
    let (lc, _) = run_step(
        6,
        lc,
        &Command::Deliver {
            response: make_response(
                stale_binding,
                ResponseOutcome::Completed {
                    value: Value::Int(9),
                },
            ),
        },
    );

    // 7. Dispatch slot 1
    let (lc, r7) = run_step(7, lc, &Command::Dispatch { control: c1_r0 });
    let Receipt::Dispatch {
        dispatch: d1,
        control: c1_r1,
    } = r7.unwrap()
    else {
        unreachable!()
    };

    // 8. Cancel slot 1 -> cancel_requested
    let (lc, _) = run_step(8, lc, &Command::Cancel { control: c1_r1 });

    // 9. Timeout slot 0 -> uncertain(false)
    let (lc, r9) = run_step(9, lc, &Command::Timeout { control: c0_r2 });
    let Receipt::Control { control: c0_r3 } = r9.unwrap() else {
        unreachable!()
    };

    // 10. Collect slot 0 while uncertain -> ERR
    let (lc, _) = run_step(
        10,
        lc,
        &Command::Collect {
            control: c0_r3.clone(),
        },
    );

    // 11. Reconcile slot 0 with completed(42)
    let (lc, r11) = run_step(
        11,
        lc,
        &Command::Reconcile {
            control: c0_r3,
            response: make_response(
                d0.binding,
                ResponseOutcome::Completed {
                    value: Value::Int(42),
                },
            ),
        },
    );
    let Receipt::Control { control: c0_r4 } = r11.unwrap() else {
        unreachable!()
    };

    // 12. Collect slot 0 -> slot 0 freed
    let (lc, _) = run_step(
        12,
        lc,
        &Command::Collect {
            control: c0_r4.clone(),
        },
    );

    // 13. Double collect slot 0 -> ERR
    let (lc, _) = run_step(13, lc, &Command::Collect { control: c0_r4 });

    // 14. Recover -> marks slot 1 uncertain(true), bumps rev to 3
    let (lc, _) = run_step(14, lc, &Command::Recover);
    let c1_r3 = Control {
        attempt: d1.binding.attempt.clone(),
        owner: owner_name(1),
        revision: 3,
    };

    // 15. Reconcile slot 1 with completed(7) (cancellation requested -> settles as cancelled)
    let (lc, r15) = run_step(
        15,
        lc,
        &Command::Reconcile {
            control: c1_r3,
            response: make_response(
                d1.binding,
                ResponseOutcome::Completed {
                    value: Value::Int(7),
                },
            ),
        },
    );
    let Receipt::Control { control: c1_r4 } = r15.unwrap() else {
        unreachable!()
    };

    // 16. Collect slot 1 -> slot 1 freed
    let (lc, _) = run_step(16, lc, &Command::Collect { control: c1_r4 });

    // 17. Reserve slot 0 again (work 1) -> reuses slot 0 at generation 2
    let (lc, r17) = run_step(
        17,
        lc,
        &Command::Reserve {
            task: "t3".into(),
            owner: owner_name(1),
            request: make_request(&context, 1),
        },
    );
    let Receipt::Control { control: c0_g2_r0 } = r17.unwrap() else {
        unreachable!()
    };

    // 18. Cancel undispatched reserved slot 0 -> releases reserved work back to avail=1
    let (lc, r18) = run_step(18, lc, &Command::Cancel { control: c0_g2_r0 });
    let Receipt::Control { control: c0_g2_r1 } = r18.unwrap() else {
        unreachable!()
    };

    // 19. Collect cancelled slot 0 -> slot 0 freed
    let _ = run_step(19, lc, &Command::Collect { control: c0_g2_r1 });
}

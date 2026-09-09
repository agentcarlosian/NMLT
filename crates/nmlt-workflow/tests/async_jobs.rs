use nmlt_workflow::*;

#[derive(Default)]
struct Host {
    requests: Vec<JobRequest>,
    calls: Vec<(u32, JobOperation)>,
    bad_output: bool,
}
impl JobHost for Host {
    fn square(&mut self, _: i64, _: Location) -> Result<Result<i64, String>, JobError> {
        unreachable!()
    }
    fn start(&mut self, request: JobRequest, _: Location) -> Result<u32, JobError> {
        let id = self.requests.len() as u32;
        self.requests.push(request);
        Ok(id)
    }
    fn control(&mut self, id: u32, op: JobOperation, _: Location) -> Result<Value, JobError> {
        self.calls.push((id, op));
        if self.bad_output {
            return Ok(Value::Text("wrong output type".into()));
        }
        match op {
            JobOperation::Poll | JobOperation::Cancel => Ok(Value::Bool(false)),
            JobOperation::Collect => Ok(match &self.requests[id as usize] {
                JobRequest::Square(i) if *i < 0 => Value::Err("negative input".into()),
                JobRequest::Square(i) => Value::Ok(Box::new(Value::Int(i * i))),
                JobRequest::Lean(_) | JobRequest::LeanCheck { .. } => {
                    Value::Ok(Box::new(Value::Text("receipt".into())))
                }
            }),
        }
    }
}

#[test]
fn lean_terms_are_typed_dynamic_inputs_and_handles_transfer() {
    let program = compile(include_str!("../../../examples/pivot/lean_terms.nmlt")).unwrap();
    assert!(program.requires_lean_jobs("check").unwrap());
    let statement = "forall n : Nat, n + 0 = n";
    let proof = "fun n => Nat.add_zero n";
    let inputs = Inputs::from([
        ("statement".into(), Value::Text(statement.into())),
        ("proof".into(), Value::Text(proof.into())),
    ]);
    let mut host = Host::default();
    let result = execute_with_host(&program, "check", &inputs, 100, &mut host).unwrap();
    assert!(result.stop.returned());
    assert_eq!(
        host.requests,
        vec![JobRequest::LeanCheck {
            statement: statement.into(),
            proof: proof.into()
        }]
    );
    assert_eq!(host.calls, vec![(0, JobOperation::Collect)]);
    assert!(compile("fn main() -> Outcome<Text> { let h = job_start_lean_check(2, \"True.intro\"); job_collect(h) }").is_err());
}

#[test]
fn concurrent_source_calls_handle_failure_and_reuse_data() {
    let program = compile(include_str!("../../../examples/pivot/async_fallback.nmlt")).unwrap();
    assert!(program.requires_async_jobs("main").unwrap());
    assert!(!program.requires_lean_jobs("main").unwrap());
    assert!(program.requires_lean_jobs("lean").unwrap());
    let mut host = Host::default();
    let inputs = Inputs::from([
        ("input".into(), Value::Int(-3)),
        ("fallback".into(), Value::Int(5)),
    ]);
    let result = execute_with_host(&program, "main", &inputs, 100, &mut host).unwrap();
    assert_eq!(
        result.stop,
        Stop::Returned {
            value: Value::Ok(Box::new(Value::Int(50)))
        }
    );
    assert_eq!(
        host.requests,
        vec![JobRequest::Square(-3), JobRequest::Square(5)]
    );
    assert_eq!(
        host.calls,
        vec![
            (0, JobOperation::Poll),
            (0, JobOperation::Collect),
            (1, JobOperation::Collect)
        ]
    );
}

#[test]
fn affinity_rejects_reuse_escape_and_path_imbalances() {
    let bodies = [
        "let h = job_start_square(3); let a = job_collect(h); job_collect(h)",
        "let h = job_start_square(3); let a = job_collect(h); let b = job_poll(h); a",
        "let h = job_start_square(3); let a = job_collect(h); let b = job_cancel(h); a",
        "let h = job_start_square(3); let other = h; let old = job_collect(h); job_collect(other)",
        "let h = job_start_square(3); Ok(1)",
        "let h = job_start_square(3); if true { job_collect(h) } else { Ok(1) }",
        "let h = job_start_square(3); let a = if true { job_collect(h) } else { Ok(1) }; job_collect(h)",
        "let h = job_start_square(3); let a = [h]; job_collect(h)",
        "let h = job_start_square(3); let a = Ok(h); job_collect(h)",
        "job_collect(job_start_square(3))",
        "let h = job_start_square(3); let a = false and (match job_collect(h) { Ok(v) => true, Err(e) => false }); job_collect(h)",
        "let h = job_start_square(3); let a = fold([1,2], 0, acc, item => match job_collect(h) { Ok(v) => v, Err(e) => 0 }); job_collect(h)",
    ];
    for body in bodies {
        let source = format!("fn main() -> Outcome<Int> {{ {body} }}");
        let error = compile(&source).expect_err(&source);
        assert!(error.span.is_some(), "{source}");
    }
    for source in [
        "fn main(h: Job<Int>) -> Int { 0 }",
        "record Bad { h: Job<Int> } fn main() -> Int { 0 }",
        "fn main(h: List<Job<Int>>) -> Int { 0 }",
        "fn main(h: Outcome<Job<Int>>) -> Int { 0 }",
        "fn main() -> Outcome<Text> { let h = job_start_lean(\"unknown\"); job_collect(h) }",
        "fn main(s: Text) -> Outcome<Text> { let h = job_start_lean(s); job_collect(h) }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn branches_and_iteration_local_jobs_preserve_affinity() {
    for source in [
        "fn main() -> Outcome<Int> { let h = job_start_square(3); let moved = h; job_collect(moved) }",
        "fn main() -> Outcome<Int> { let h = if true { job_start_square(3) } else { job_start_square(4) }; job_collect(h) }",
        "fn main() -> Outcome<Int> { let h = job_start_square(3); if true { job_collect(h) } else { let c = job_cancel(h); job_collect(h) } }",
        "fn main() -> Outcome<Int> { let h = job_start_square(3); match Ok(true) { Ok(b) => job_collect(h), Err(e) => job_collect(h) } }",
        "fn main() -> Int { fold([1,2,3], 0, acc, item => { let h = job_start_square(item); match job_collect(h) { Ok(v) => acc + v, Err(e) => acc } }) }",
        "fn main() -> Outcome<Int> { let h = job_start_square(3); let a = fold([1,2], 0, acc, item => if job_poll(h) { acc + 1 } else { acc }); job_collect(h) }",
    ] {
        let program = compile(source).unwrap_or_else(|e| panic!("{source}: {}", e.message));
        assert!(
            execute_with_host(&program, "main", &Inputs::new(), 300, &mut Host::default())
                .unwrap()
                .stop
                .returned()
        );
    }
}

#[test]
fn host_type_errors_and_limits_stop_before_fallback_or_dispatch() {
    let program =
        compile("fn main() -> Outcome<Int> { let h = job_start_square(3); job_collect(h) }")
            .unwrap();
    for steps in 1..=2 {
        let mut host = Host::default();
        assert!(matches!(
            execute_with_host(&program, "main", &Inputs::new(), steps, &mut host)
                .unwrap()
                .stop,
            Stop::StepLimit { .. }
        ));
        assert!(host.requests.is_empty());
    }
    let mut host = Host {
        bad_output: true,
        ..Host::default()
    };
    assert!(matches!(
        execute_with_host(&program, "main", &Inputs::new(), 30, &mut host)
            .unwrap()
            .stop,
        Stop::JobStopped {
            reason: JobError::HostFailure,
            ..
        }
    ));
    assert_eq!(host.requests.len(), 1);
}

#[test]
fn imported_async_effects_and_control_locations_are_retained() {
    let program = compile_package("main.nmlt", |name| Ok(match name {
        "main.nmlt" => "import Jobs\nfn main() -> Outcome<Text> { Jobs.run() }",
        "Jobs.nmlt" => "fn run() -> Outcome<Text> { let h = job_start_lean(\"induction\"); job_collect(h) }",
        _ => panic!("unexpected import"),
    }.into())).unwrap();
    assert!(program.requires_async_jobs("main").unwrap());
    assert!(program.requires_lean_jobs("main").unwrap());
    let mut host = Host {
        bad_output: true,
        ..Host::default()
    };
    let execution = execute_with_host(&program, "main", &Inputs::new(), 30, &mut host).unwrap();
    let Stop::JobStopped { at, .. } = execution.stop else {
        panic!("expected host stop")
    };
    assert_eq!(program.source_path(at), Some("Jobs.nmlt"));
}

#[test]
fn handles_move_through_imported_functions_and_cannot_cross_external_entries() {
    let program = compile_package("main.nmlt", |name| Ok(match name {
        "main.nmlt" => "import Jobs\nfn main() -> Outcome<Int> { let job = Jobs.launch(5); let moved = Jobs.forward(job); Jobs.finish(moved) }",
        "Jobs.nmlt" => "fn launch(input: Int) -> Job<Int> { job_start_square(input) } fn forward(job: Job<Int>) -> Job<Int> { let ready = job_poll(job); job } fn finish(job: Job<Int>) -> Outcome<Int> { job_collect(job) }",
        _ => panic!("unknown import"),
    }.into())).unwrap();
    assert!(program.can_run_as_entry("main"));
    assert!(!program.can_run_as_entry("Jobs.launch"));
    assert!(!program.can_run_as_entry("Jobs.finish"));
    let mut host = Host::default();
    let result = execute_with_host(&program, "main", &Inputs::new(), 100, &mut host).unwrap();
    assert_eq!(
        result.stop,
        Stop::Returned {
            value: Value::Ok(Box::new(Value::Int(25)))
        }
    );
    assert_eq!(
        host.calls,
        vec![(0, JobOperation::Poll), (0, JobOperation::Collect)]
    );
    let mut host = Host::default();
    assert!(
        execute_with_host(
            &program,
            "Jobs.launch",
            &Inputs::from([("input".into(), Value::Int(5))]),
            100,
            &mut host
        )
        .is_err()
    );
    assert!(host.requests.is_empty());
    assert!(
        execute_with_host(
            &program,
            "Jobs.finish",
            &Inputs::from([("job".into(), Value::Int(0))]),
            100,
            &mut host
        )
        .is_err()
    );
}

#[test]
fn job_accumulators_and_branch_results_transfer_exactly_once() {
    for source in [
        "fn forward(job: Job<Int>, ignored: Int) -> Job<Int> { job } fn main() -> Outcome<Int> { let job = job_start_square(6); let moved = fold([1,2,3], job, acc, item => forward(acc, item)); job_collect(moved) }",
        "fn main() -> Outcome<Int> { let empty: List<Int> = []; let job = job_start_square(6); let moved = fold(empty, job, acc, item => acc); job_collect(moved) }",
        "fn choose(a: Job<Int>, b: Job<Int>, flag: Bool) -> Job<Int> { if flag { let other = job_collect(b); a } else { let other = job_collect(a); b } } fn main() -> Outcome<Int> { let a = job_start_square(6); let b = job_start_square(3); let chosen = choose(a, b, true); job_collect(chosen) }",
    ] {
        let program = compile(source).unwrap_or_else(|e| panic!("{source}: {}", e.message));
        let mut host = Host::default();
        let result = execute_with_host(&program, "main", &Inputs::new(), 100, &mut host).unwrap();
        assert_eq!(
            result.stop,
            Stop::Returned {
                value: Value::Ok(Box::new(Value::Int(36)))
            }
        );
        let mut collected: Vec<_> = host
            .calls
            .iter()
            .filter(|(_, op)| *op == JobOperation::Collect)
            .map(|(id, _)| *id)
            .collect();
        collected.sort();
        assert_eq!(
            collected,
            (0..host.requests.len() as u32).collect::<Vec<_>>()
        );
    }
}

#[test]
fn transferred_parameter_reuse_discard_and_loop_imbalance_fail_statically() {
    for source in [
        "fn discard(h: Job<Int>) -> Int { 0 } fn main() -> Int { discard(job_start_square(3)) }",
        "fn use(a: Job<Int>, b: Job<Int>) -> Outcome<Int> { let first = job_collect(a); job_collect(b) } fn main() -> Outcome<Int> { let h = job_start_square(3); use(h, h) }",
        "fn forward(h: Job<Int>) -> Job<Int> { let moved = h; let old = job_collect(h); moved }",
        "fn main() -> Outcome<Int> { let h = job_start_square(3); let moved = fold([1,2], h, acc, item => job_start_square(item)); job_collect(moved) }",
        "fn main() -> Outcome<Int> { let h = job_start_square(3); let moved = fold([1,2], h, acc, item => acc); let old = job_collect(h); job_collect(moved) }",
        "fn main() -> Outcome<Int> { let h = job_start_square(3); let xs = [h]; job_collect(h) }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

use nmlt_workflow::*;

#[derive(Default)]
struct Host {
    calls: Vec<(i64, Location)>,
    stop: bool,
}
impl JobHost for Host {
    fn square(&mut self, input: i64, at: Location) -> Result<Result<i64, String>, JobError> {
        self.calls.push((input, at));
        if self.stop {
            Err(JobError::HostFailure)
        } else if input < 0 {
            Ok(Err("negative input".into()))
        } else {
            Ok(Ok(input * input))
        }
    }
}

#[test]
fn effects_are_transitive_conservative_and_entry_specific() {
    let program = compile(
        r#"
fn main() -> Outcome<Int> { helper() }
fn helper() -> Outcome<Int> { if false { job_square(5) } else { Ok(1) } }
fn pure() -> Int { 7 }
"#,
    )
    .unwrap();
    assert!(program.requires_jobs("main").unwrap());
    assert!(program.requires_jobs("helper").unwrap());
    assert!(!program.requires_jobs("pure").unwrap());
    assert!(
        execute(&program, "main", &Inputs::new(), 100)
            .unwrap_err()
            .contains("requires local jobs")
    );
    assert_eq!(
        execute(&program, "pure", &Inputs::new(), 100).unwrap().stop,
        Stop::Returned {
            value: Value::Int(7)
        }
    );
    let mut host = Host::default();
    assert!(
        execute_with_host(&program, "main", &Inputs::new(), 100, &mut host)
            .unwrap()
            .stop
            .returned()
    );
    assert!(host.calls.is_empty());
}

#[test]
fn real_outcomes_choose_fallback_and_reuse_only_the_collected_value() {
    let program = compile(include_str!("../../../examples/pivot/job_fallback.nmlt")).unwrap();
    for (input, fallback, calls, value) in [
        (4, 5, vec![4], Value::Ok(Box::new(Value::Int(32)))),
        (-3, 5, vec![-3, 5], Value::Ok(Box::new(Value::Int(50)))),
        (-3, -5, vec![-3, -5], Value::Err("negative input".into())),
    ] {
        let mut host = Host::default();
        let inputs = Inputs::from([
            ("input".into(), Value::Int(input)),
            ("fallback".into(), Value::Int(fallback)),
        ]);
        let execution = execute_with_host(&program, "main", &inputs, 100, &mut host).unwrap();
        assert_eq!(execution.stop, Stop::Returned { value });
        assert_eq!(
            host.calls.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
            calls
        );
    }
}

#[test]
fn invalid_inputs_step_limits_and_host_failures_do_not_trigger_fallback() {
    let program = compile(include_str!("../../../examples/pivot/job_fallback.nmlt")).unwrap();
    let mut host = Host::default();
    assert!(execute_with_host(&program, "main", &Inputs::new(), 100, &mut host).is_err());
    assert!(host.calls.is_empty());
    let inputs = Inputs::from([
        ("input".into(), Value::Int(-1)),
        ("fallback".into(), Value::Int(5)),
    ]);
    assert!(matches!(
        execute_with_host(&program, "main", &inputs, 1, &mut host)
            .unwrap()
            .stop,
        Stop::StepLimit { .. }
    ));
    assert!(host.calls.is_empty());
    host.stop = true;
    assert!(matches!(
        execute_with_host(&program, "main", &inputs, 100, &mut host)
            .unwrap()
            .stop,
        Stop::JobStopped {
            reason: JobError::HostFailure,
            ..
        }
    ));
    assert_eq!(host.calls.len(), 1);
}

#[test]
fn job_types_and_reserved_names_are_checked_in_dead_code() {
    for source in [
        "fn main() -> Outcome<Int> { job_square(true) }",
        "fn main() -> Outcome<Int> { job_square() }",
        "fn main() -> Outcome<Int> { job_square(1, 2) }",
        "fn main() -> Int { job_square(1) }",
        "fn main() -> Int { if false { job_square(1) } else { 0 } }",
        "fn job_square(x: Int) -> Int { x }",
        "fn main(job_square: Int) -> Int { job_square }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn imported_effects_keep_their_source_location() {
    let program = compile_package("main.nmlt", |name| match name {
        "main.nmlt" => Ok("import Worker\nfn main() -> Outcome<Int> { Worker.run() }".into()),
        "Worker.nmlt" => Ok("fn run() -> Outcome<Int> { job_square(3) }".into()),
        _ => unreachable!(),
    })
    .unwrap();
    assert!(program.requires_jobs("main").unwrap());
    let mut host = Host::default();
    execute_with_host(&program, "main", &Inputs::new(), 100, &mut host).unwrap();
    assert_eq!(program.source_path(host.calls[0].1), Some("Worker.nmlt"));
}

#[test]
fn folds_account_for_each_job_without_duplicating_value_reuse() {
    let program = compile("fn main() -> Int { fold([2, 3], 0, total, item => match job_square(item) { Ok(value) => total + value + value, Err(message) => total }) }").unwrap();
    let mut host = Host::default();
    assert_eq!(
        execute_with_host(&program, "main", &Inputs::new(), 100, &mut host)
            .unwrap()
            .stop,
        Stop::Returned {
            value: Value::Int(26)
        }
    );
    assert_eq!(
        host.calls.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
        vec![2, 3]
    );
}

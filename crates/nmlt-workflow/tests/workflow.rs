use nmlt_workflow::*;

const SOURCE: &str = include_str!("../../../examples/pivot/pure_fallback.nmlt");
fn run(source: &str, inputs: Inputs, limit: u32) -> Execution {
    execute(&compile(source).unwrap(), "main", &inputs, limit).unwrap()
}
fn returned(value: Value) -> Stop {
    Stop::Returned { value }
}

#[test]
fn real_inputs_branch_handle_failure_and_reuse_a_value() {
    let program = compile(SOURCE).unwrap();
    for (input, fallback, expected) in [
        (4, 5, Value::Ok(Box::new(Value::Int(32)))),
        (-3, 5, Value::Ok(Box::new(Value::Int(50)))),
        (-3, -5, Value::Err("negative input".into())),
    ] {
        let inputs = Inputs::from([
            ("input".into(), Value::Int(input)),
            ("fallback".into(), Value::Int(fallback)),
        ]);
        let result = execute(&program, "main", &inputs, 1000).unwrap();
        assert_eq!(result.stop, returned(expected));
        assert_eq!(execute(&program, "main", &inputs, 1000).unwrap(), result);
    }
    assert_eq!(program.identity(), compile(SOURCE).unwrap().identity());
}

#[test]
fn scope_resolution_shadowing_forward_calls_and_payload_types() {
    let source = r#"
module Lib {
 fn helper(x: Int) -> Int { x * 2 }
 fn nested(x: Int) -> Int { helper(x) }
}
fn main() -> Int {
 let v: Outcome<Int> = Err("failure");
 let x = 3;
 match v { Err(message) => { let x = Lib.nested(x); x + 1 }, Ok(x) => x }
}
"#;
    assert_eq!(
        run(source, Inputs::new(), 100).stop,
        returned(Value::Int(7))
    );
    let source = "fn main() -> Outcome<Outcome<Int>> { Ok(Err(\"missing\")) }";
    assert_eq!(
        run(source, Inputs::new(), 100).stop,
        returned(Value::Ok(Box::new(Value::Err("missing".into()))))
    );
}

#[test]
fn budget_overflow_short_circuit_and_integer_endpoints_are_explicit() {
    assert!(matches!(
        run(
            SOURCE,
            Inputs::from([
                ("input".into(), Value::Int(4)),
                ("fallback".into(), Value::Int(5))
            ]),
            1
        )
        .stop,
        Stop::StepLimit { .. }
    ));
    for source in [
        "fn main() -> Int { 9223372036854775807 + 1 }",
        "fn main() -> Int { -(-9223372036854775808) }",
        "fn main() -> Int { -9223372036854775808 - 1 }",
        "fn main() -> Int { 9223372036854775807 * 2 }",
    ] {
        assert!(matches!(
            run(source, Inputs::new(), 100).stop,
            Stop::IntegerOverflow { .. }
        ));
    }
    assert_eq!(
        run(
            "fn main() -> Int { -9223372036854775808 }",
            Inputs::new(),
            1
        )
        .stop,
        returned(Value::Int(i64::MIN))
    );
    assert_eq!(
        run(
            "fn main() -> Bool { false and (9223372036854775807 + 1 == 0) }",
            Inputs::new(),
            100
        )
        .stop,
        returned(Value::Bool(false))
    );
    assert_eq!(
        run("fn main() -> Int { 1 + 2 * 3 - 4 }", Inputs::new(), 100).stop,
        returned(Value::Int(3))
    );
}

#[test]
fn all_declarations_and_branches_are_checked_with_source_spans() {
    let invalid = [
        "fn main() -> Int { absent }",
        "fn main() -> Int { if true { 1 } else { false } }",
        "fn main() -> Int { 1 }\nfn unused() -> Int { false }",
        "fn main(x: Int, x: Int) -> Int { x }",
        "fn main() -> Int { 1 }\nfn main() -> Int { 2 }",
        "fn main() -> Int { main() }",
        "fn main() -> Int { other() }\nfn other() -> Int { main() }",
        "fn main() -> Int { let v = Err(\"x\"); 0 }",
        "fn main() -> Int { match Ok(1) { Ok(v) => v } }",
        "fn main() -> Int { match Ok(1) { Ok(v) => v, Ok(w) => w } }",
        "fn main() -> Int { match 1 { Ok(v) => v, Err(e) => 0 } }",
        "fn main() -> Int { match Ok(1) { Ok(v) => v, Err(e) => e } }",
        "fn main() -> Outcome<Int> { Err(1) }",
        "fn main() -> Int { 9223372036854775808 }",
        "fn main() -> Int { -9223372036854775809 }",
        "fn main() -> Int { 1 / 0 }",
        "fn main() -> Int { 1 } trailing",
        "fn main() -> Int { if false { unknown() } else { 1 } }",
        "fn main() -> Int { helper(true) }\nfn helper(x: Int) -> Int { x }",
        "fn main() -> Int { helper() }\nfn helper(x: Int) -> Int { x }",
        "system Hidden {}\nfn main() -> Int { 1 }",
        "import Missing\nfn main() -> Int { 1 }",
        "record Data { x: Missing }\nfn main() -> Int { 1 }",
        "enum Bit { on, off }\nfn main() -> Int { 1 }",
    ];
    for source in invalid {
        let error = compile(source).expect_err(source);
        let span = error.span.expect("source location");
        assert!(span.start <= span.end && span.end <= source.len());
        assert!(source.is_char_boundary(span.start) && source.is_char_boundary(span.end));
    }
}

#[test]
fn input_names_types_text_bounds_and_execution_limits_are_validated() {
    let program = compile("fn main(x: Int) -> Int { x }").unwrap();
    for input in [
        Inputs::new(),
        Inputs::from([("y".into(), Value::Int(2))]),
        Inputs::from([("x".into(), Value::Bool(true))]),
        Inputs::from([("x".into(), Value::Int(1)), ("extra".into(), Value::Int(2))]),
    ] {
        assert!(execute(&program, "main", &input, 100).is_err());
    }
    let input = Inputs::from([("x".into(), Value::Int(1))]);
    for limit in [0, 100001] {
        assert!(execute(&program, "main", &input, limit).is_err());
    }
    assert!(execute(&program, "missing", &input, 100).is_err());
    let program = compile("fn main(x: Text) -> Text { x }").unwrap();
    assert!(
        execute(
            &program,
            "main",
            &Inputs::from([("x".into(), Value::Text("a".repeat(4097)))]),
            100
        )
        .is_err()
    );
}

#[test]
fn parser_and_call_depth_bounds_stop_without_host_stack_overflow() {
    let nested = format!(
        "fn main() -> Int {{ {}1{} }}",
        "(".repeat(200),
        ")".repeat(200)
    );
    assert!(compile(&nested).is_err());
    let long_chain = format!("fn main() -> Int {{ {}1 }}", "1 + ".repeat(100));
    assert!(compile(&long_chain).is_err());
    assert!(compile(&" ".repeat(MAX_SOURCE_BYTES + 1)).is_err());
    let mut source = "fn main() -> Int { 1 + f0() }\n".to_owned();
    for i in 0..40 {
        source.push_str(&format!("fn f{i}() -> Int {{ 1 + f{}() }}\n", i + 1));
    }
    source.push_str("fn f40() -> Int { 0 }\n");
    assert!(matches!(
        run(&source, Inputs::new(), 10000).stop,
        Stop::DepthLimit { .. }
    ));
}

#[test]
fn scalar_operators_and_text_escapes_have_the_declared_meaning() {
    for expression in [
        "not false",
        "true and true",
        "false or true",
        "1 < 2",
        "2 > 1",
        "2 <= 2",
        "2 >= 2",
        "1 != 2",
        "true == true",
        r#""a\n" == "a\n""#,
    ] {
        let source = format!("fn main() -> Bool {{ {expression} }}");
        assert_eq!(
            run(&source, Inputs::new(), 100).stop,
            returned(Value::Bool(true)),
            "{expression}"
        );
    }
    let source = "fn main() -> Bool { true or (9223372036854775807 + 1 == 0) }";
    assert_eq!(
        run(source, Inputs::new(), 100).stop,
        returned(Value::Bool(true))
    );
    assert!(compile(r#"fn main() -> Text { "invalid\q" }"#).is_err());
    assert!(compile("fn main() -> Bool { true < false }").is_err());
}

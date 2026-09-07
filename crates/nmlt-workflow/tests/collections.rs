use nmlt_workflow::*;
use serde_json::json;
use std::collections::BTreeMap;

fn run(source: &str) -> Execution {
    execute(&compile(source).unwrap(), "main", &Inputs::new(), 100_000).unwrap()
}
fn returned(source: &str, value: Value) {
    assert_eq!(run(source).stop, Stop::Returned { value });
}

#[test]
fn batch_workflow_consumes_structured_inputs_and_preserves_domain_failures() {
    let program = compile(include_str!("../../../examples/pivot/batch_summary.nmlt")).unwrap();
    let ty = &program.entries().find(|(n, _, _)| *n == "main").unwrap().1[0].ty;
    let items = program.input_value(ty, &json!([
        {"input": -3, "fallback": 5}, {"input": 4, "fallback": 9}, {"input": -1, "fallback": -2}
    ])).unwrap();
    let inputs = Inputs::from([("items".into(), items)]);
    let result = execute(&program, "main", &inputs, 1000).unwrap();
    assert_eq!(
        result.stop,
        Stop::Returned {
            value: Value::Record {
                name: "Summary".into(),
                fields: BTreeMap::from([
                    ("accepted".into(), Value::Int(2)),
                    ("rejected".into(), Value::Int(1)),
                    ("total".into(), Value::Int(41))
                ])
            }
        }
    );
    assert_eq!(result, execute(&program, "main", &inputs, 1000).unwrap());
    assert!(matches!(
        execute(&program, "main", &inputs, result.steps - 1)
            .unwrap()
            .stop,
        Stop::StepLimit { .. }
    ));
    assert_eq!(
        execute(&program, "main", &inputs, result.steps).unwrap(),
        result
    );
}

#[test]
fn nominal_records_resolve_forward_module_types_and_arbitrary_projections() {
    returned(
        r#"
module M {
 record Outer { inner: Inner }
 record Inner { x: Int }
 fn build() -> Outer { new Outer { inner: new Inner { x: 7 } } }
}
record Inner { x: Bool }
fn main() -> Int {
 let r: M.Outer = M.build();
 let r = new M.Inner { x: r.inner.x + 1 };
 (new M.Outer { inner: r }).inner.x + M.build().inner.x
}
"#,
        Value::Int(15),
    );
    returned(
        "record Empty {}\nfn main() -> Empty { new Empty {} }",
        Value::Record {
            name: "Empty".into(),
            fields: BTreeMap::new(),
        },
    );
}

#[test]
fn folds_are_ordered_bounded_and_lexically_scoped() {
    let source = "fn main() -> Int { fold([1, 2], 0, a, x => a * 10 + x) }";
    returned(source, Value::Int(12));
    let source = "fn main() -> Int { fold([1, 2], 0, a, x => a + x) }";
    let result = run(source);
    assert_eq!(result.steps, 11);
    assert_eq!(
        result.stop,
        Stop::Returned {
            value: Value::Int(3)
        }
    );
    returned(
        "fn main() -> Int { let xs: List<Int> = []; fold(xs, 7, a, x => a + 9223372036854775807) }",
        Value::Int(7),
    );
    returned(
        "fn main() -> Int { let x = 10; fold([1, 2], 0, a, x => a + fold([3, 4], x, b, y => b + y)) + x }",
        Value::Int(27),
    );
    assert!(matches!(
        run("fn main() -> Int { fold([1, 2], 9223372036854775807, a, x => a + x) }").stop,
        Stop::IntegerOverflow { .. }
    ));
}

#[test]
fn list_lookup_handles_endpoints_without_panics() {
    for (index, expected) in [
        (-1, Value::Err("list index out of bounds".into())),
        (0, Value::Ok(Box::new(Value::Int(4)))),
        (1, Value::Ok(Box::new(Value::Int(5)))),
        (2, Value::Err("list index out of bounds".into())),
        (i64::MAX, Value::Err("list index out of bounds".into())),
    ] {
        returned(
            &format!("fn main() -> Outcome<Int> {{ get([4, 5], {index}) }}"),
            expected,
        );
    }
    returned(
        "fn main() -> Int { length([true, false, true]) }",
        Value::Int(3),
    );
    returned("fn main() -> List<Int> { [] }", Value::List(vec![]));
    returned(
        "fn main() -> Outcome<Int> { let xs: List<Int> = []; get(xs, 0) }",
        Value::Err("list index out of bounds".into()),
    );
    let items = vec!["1"; MAX_LIST_ITEMS].join(",");
    returned(
        &format!("fn main() -> Int {{ fold([{items}], 0, a, x => a + x) }}"),
        Value::Int(256),
    );
}

#[test]
fn malformed_records_lists_and_folds_fail_even_in_unused_code() {
    for source in [
        "record R { x: Int, x: Bool }\nfn main() -> Int { 0 }",
        "record R {}\nrecord R {}\nfn main() -> Int { 0 }",
        "record Int {}\nfn main() -> Int { 0 }",
        "record R { x: R }\nfn main() -> Int { 0 }",
        "record A { x: List<B> }\nrecord B { x: Outcome<A> }\nfn main() -> Int { 0 }",
        "record R { x: Int }\nfn main() -> R { new R {} }",
        "record R { x: Int }\nfn main() -> R { new R { y: 1 } }",
        "record R { x: Int }\nfn main() -> R { new R { x: 1, x: 2 } }",
        "record R { x: Int }\nfn main() -> R { new R { x: true } }",
        "record R { x: Int }\nfn main() -> Int { (new R { x: 1 }).y }",
        "record A {}\nrecord B {}\nfn main() -> A { new B {} }",
        "record R {}\nfn main() -> Bool { new R {} == new R {} }",
        "fn main() -> Int { (1).x }",
        "fn main() -> List<Int> { [1, true] }",
        "fn main() -> Int { let xs = []; 0 }",
        "fn main() -> Bool { [1] == [1] }",
        "fn main() -> Int { fold([1], 0, a, a => a) }",
        "fn main() -> Int { fold(1, 0, a, x => a) }",
        "fn main() -> Int { fold([1], 0, a, x => true) }",
        "fn main() -> Int { let xs: List<Int> = []; fold(xs, 0, a, x => unknown()) }",
        "fn main() -> Int { fold([1], 0, a, x => main()) }",
        "fn main() -> Int { length(1) }",
        "fn main() -> Int { length([1], 0) }",
        "fn main() -> Int { get([1]) }",
        "fn main() -> Outcome<Int> { get([1], true) }",
        "fn main() -> Int { 0 }\nfn unused() -> Int { fold([1], 0, a, x => false) }",
    ] {
        let diagnostic = compile(source).expect_err(source);
        let span = diagnostic.span.expect("source span");
        assert!(span.start <= span.end && span.end <= source.len());
    }
    let items = vec!["0"; MAX_LIST_ITEMS + 1].join(",");
    assert!(compile(&format!("fn main() -> List<Int> {{ [{items}] }}")).is_err());
}

#[test]
fn structured_input_validation_checks_nominality_exact_fields_and_payloads() {
    let program = compile("record R { x: List<Outcome<Int>> }\nfn main(r: R) -> R { r }").unwrap();
    let ty = Type::Record("R".into());
    let valid = program
        .input_value(&ty, &json!({"x": [{"Ok": 3}, {"Err": "missing"}]}))
        .unwrap();
    assert!(program.conforms(&valid, &ty));
    assert!(!valid.conforms(&ty)); // Standalone validation cannot resolve named types.
    for json in [
        json!({}),
        json!({"x": [], "extra": 1}),
        json!({"x": [1]}),
        json!({"x": [{"Ok": true}]}),
        json!({"x": [{"Ok": 1, "Err": "x"}]}),
        json!({"x": [{"Err": 1}]}),
        json!({"x": [{"Err": "x".repeat(4097)}]}),
    ] {
        assert!(program.input_value(&ty, &json).is_err(), "{json}");
    }
    let Value::Record { fields, .. } = valid else {
        unreachable!()
    };
    for value in [
        Value::Record {
            name: "Other".into(),
            fields: fields.clone(),
        },
        Value::Record {
            name: "R".into(),
            fields: BTreeMap::new(),
        },
        Value::Record {
            name: "R".into(),
            fields: BTreeMap::from([("x".into(), Value::Int(1))]),
        },
    ] {
        assert!(execute(&program, "main", &Inputs::from([("r".into(), value)]), 100).is_err());
    }
    assert!(
        program
            .input_value(&Type::List(Box::new(Type::Int)), &json!(vec![0; 257]))
            .is_err()
    );
}

#[test]
fn aggregate_limits_stop_growth_and_repeated_copying() {
    let text = "x".repeat(4096);
    let source = format!(
        "fn main() -> List<Text> {{ let x = \"{text}\"; [{}] }}",
        vec!["x"; 17].join(",")
    );
    assert!(matches!(run(&source).stop, Stop::ValueLimit { .. }));
    let source = format!(
        "fn main() -> Text {{ fold([{}], \"{text}\", a, x => a) }}",
        vec!["0"; 256].join(",")
    );
    let result = run(&source);
    assert!(matches!(result.stop, Stop::ValueWorkLimit { .. }));
    assert_eq!(run(&source), result);
    let inner = vec![Value::Int(0); 256];
    let too_many = Value::List(vec![Value::List(inner); 16]);
    assert!(!too_many.conforms(&Type::List(Box::new(Type::List(Box::new(Type::Int))))));
    let mut source = "record R0 { x: Int }\n".to_owned();
    for i in 1..18 {
        source.push_str(&format!("record R{i} {{ x: R{} }}\n", i - 1));
    }
    source.push_str("fn main() -> Int { 0 }");
    assert!(compile(&source).is_err());
}

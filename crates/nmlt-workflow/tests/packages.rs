use nmlt_workflow::*;
use serde_json::json;
use std::collections::BTreeMap;

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, source)| ((*name).into(), (*source).into()))
        .collect()
}
fn package(files: &BTreeMap<String, String>) -> Result<Program, PackageError> {
    compile_package("main.nmlt", |name| {
        files.get(name).cloned().ok_or("missing".into())
    })
}
fn batch() -> BTreeMap<String, String> {
    files(&[
        (
            "main.nmlt",
            include_str!("../../../examples/pivot/package_batch/main.nmlt"),
        ),
        (
            "Arithmetic.nmlt",
            include_str!("../../../examples/pivot/package_batch/Arithmetic.nmlt"),
        ),
        (
            "Work.nmlt",
            include_str!("../../../examples/pivot/package_batch/Work.nmlt"),
        ),
        (
            "Reports.nmlt",
            include_str!("../../../examples/pivot/package_batch/Reports.nmlt"),
        ),
    ])
}

#[test]
fn multi_file_batch_passes_records_and_outcomes_between_libraries() {
    let program = package(&batch()).unwrap();
    let input = program
        .input_value(
            &Type::List(Box::new(Type::Record("Work.Item".into()))),
            &json!([{"input":-3,"fallback":5},{"input":4,"fallback":9},{"input":-1,"fallback":-2}]),
        )
        .unwrap();
    let inputs = Inputs::from([("items".into(), input)]);
    let result = execute(&program, "main", &inputs, 1000).unwrap();
    let Stop::Returned {
        value: Value::Record { name, fields },
    } = result.stop
    else {
        panic!("{result:?}")
    };
    assert_eq!(name, "Reports.Summary");
    assert_eq!(
        fields,
        BTreeMap::from([
            ("accepted".into(), Value::Int(2)),
            ("rejected".into(), Value::Int(1)),
            ("total".into(), Value::Int(41))
        ])
    );
    assert_eq!(
        program
            .sources()
            .iter()
            .map(|s| s.path.as_str())
            .collect::<Vec<_>>(),
        ["main.nmlt", "Reports.nmlt", "Work.nmlt", "Arithmetic.nmlt"]
    );
}

#[test]
fn imports_load_once_in_a_deterministic_closure_and_bind_unused_source_bytes() {
    let mut input = files(&[
        (
            "main.nmlt",
            "import B\nimport A\nfn main() -> Int { A.value() + B.value() }",
        ),
        (
            "A.nmlt",
            "import Shared\nfn value() -> Int { Shared.value() }",
        ),
        (
            "B.nmlt",
            "import Shared\nfn value() -> Int { Shared.value() }",
        ),
        (
            "Shared.nmlt",
            "fn value() -> Int { 7 }\nfn unused() -> Int { 8 }",
        ),
        ("Unrelated.nmlt", "invalid data outside the import closure"),
    ]);
    let mut reads = vec![];
    let first = compile_package("main.nmlt", |name| {
        reads.push(name.to_owned());
        input.get(name).cloned().ok_or("missing".into())
    })
    .unwrap();
    assert_eq!(reads, ["main.nmlt", "A.nmlt", "Shared.nmlt", "B.nmlt"]);
    assert_eq!(
        execute(&first, "main", &Inputs::new(), 100).unwrap().stop,
        Stop::Returned {
            value: Value::Int(14)
        }
    );
    assert_eq!(first.identity(), package(&input).unwrap().identity());
    input
        .get_mut("Shared.nmlt")
        .unwrap()
        .push_str("\n// an unused comment changes source identity\n");
    let second = package(&input).unwrap();
    assert_ne!(first.identity(), second.identity());
    assert_ne!(first.sources(), second.sources());
}

#[test]
fn file_scopes_preserve_local_module_lookup_and_require_direct_imports() {
    let mut input = files(&[
        (
            "main.nmlt",
            "import A\nfn value() -> Int { 100 }\nfn main() -> Int { A.Nested.value() }",
        ),
        (
            "A.nmlt",
            "import Shared\nfn value() -> Int { 2 }\nmodule Nested { fn value() -> Int { helper() }\nfn helper() -> Int { Shared.value() } }",
        ),
        (
            "Shared.nmlt",
            "fn value() -> Int { 7 }\nrecord R { x: Int }",
        ),
    ]);
    assert_eq!(
        execute(&package(&input).unwrap(), "main", &Inputs::new(), 100)
            .unwrap()
            .stop,
        Stop::Returned {
            value: Value::Int(7)
        }
    );
    for main in [
        "import A\nfn main() -> Int { Shared.value() }",
        "import A\nfn main() -> Shared.R { new Shared.R { x: 1 } }",
        "import A\nfn main() -> Int { value() }",
        "import A\nmodule Shared {}\nfn main() -> Int { 0 }",
        "import A\nrecord A {}\nfn main() -> Int { 0 }",
    ] {
        input.insert("main.nmlt".into(), main.into());
        assert!(package(&input).is_err(), "{main}");
    }
    input.insert(
        "main.nmlt".into(),
        "import A\nfn value() -> Int { 100 }\nfn main() -> Int { A.value() }".into(),
    );
    input.insert("A.nmlt".into(), "fn value() -> Int { missing() }".into());
    assert_eq!(package(&input).unwrap_err().path, "A.nmlt");
}

#[test]
fn record_only_dependencies_and_forward_types_are_supported() {
    let input = files(&[
        (
            "main.nmlt",
            "import Types\nimport Build\nfn main() -> Types.R { Build.make() }",
        ),
        (
            "Build.nmlt",
            "import Types\nfn make() -> Types.R { new Types.R { nested: new Types.Inner { value: 5 } } }",
        ),
        (
            "Types.nmlt",
            "record R { nested: Inner }\nrecord Inner { value: Int }",
        ),
    ]);
    let p = package(&input).unwrap();
    let Stop::Returned { value } = execute(&p, "main", &Inputs::new(), 100).unwrap().stop else {
        panic!()
    };
    assert!(p.conforms(&value, &Type::Record("Types.R".into())));
}

#[test]
fn dependency_diagnostics_and_runtime_stops_point_to_the_original_file() {
    let mut input = files(&[
        ("main.nmlt", "import Lib\nfn main() -> Int { Lib.work() }"),
        ("Lib.nmlt", "fn work() -> Int { 9223372036854775807 + 1 }"),
    ]);
    let p = package(&input).unwrap();
    let result = execute(&p, "main", &Inputs::new(), 100).unwrap();
    let Stop::IntegerOverflow { at } = result.stop else {
        panic!("{result:?}")
    };
    assert_eq!(p.source_path(at), Some("Lib.nmlt"));
    assert_eq!(
        &input["Lib.nmlt"][at.start..at.end],
        "9223372036854775807 + 1"
    );
    input.insert("Lib.nmlt".into(), "fn work() -> Int { false }".into());
    let error = package(&input).unwrap_err();
    assert_eq!(error.path, "Lib.nmlt");
    let span = error.diagnostic.span.unwrap();
    assert_eq!(&input["Lib.nmlt"][span.start..span.end], "false");
    input.insert(
        "Lib.nmlt".into(),
        "fn work() -> Int { 1 }\nfn unused() -> Int { false }".into(),
    );
    assert_eq!(package(&input).unwrap_err().path, "Lib.nmlt");
    input.insert(
        "Lib.nmlt".into(),
        "import Missing\nfn work() -> Int { 1 }".into(),
    );
    let error = package(&input).unwrap_err();
    assert_eq!(error.path, "Lib.nmlt");
    assert!(error.diagnostic.message.contains("Missing.nmlt"));
}

#[test]
fn malformed_cycles_aliases_and_nonportable_imports_are_rejected() {
    for root in [
        "import Lib\nimport Lib\nfn main() -> Int { 0 }",
        "module M { import Lib }\nfn main() -> Int { 0 }",
        "import Lib.extra\nfn main() -> Int { 0 }",
        "import \"Lib.nmlt\"\nfn main() -> Int { 0 }",
        "import con\nfn main() -> Int { 0 }",
        "import fold\nfn main() -> Int { 0 }",
        "import Lib\nimport lib\nfn main() -> Int { 0 }",
        "import main\nfn main() -> Int { 0 }",
    ] {
        assert!(
            package(&files(&[
                ("main.nmlt", root),
                ("Lib.nmlt", "fn f() -> Int { 1 }"),
                ("lib.nmlt", "fn f() -> Int { 1 }")
            ]))
            .is_err(),
            "{root}"
        );
    }
    let input = files(&[
        ("main.nmlt", "import A\nfn main() -> Int { 0 }"),
        ("A.nmlt", "import B\nfn a() -> Int { 1 }"),
        ("B.nmlt", "import A\nfn b() -> Int { 1 }"),
    ]);
    assert!(
        package(&input)
            .unwrap_err()
            .diagnostic
            .message
            .contains("cycle")
    );
    for entry in [
        "../main.nmlt",
        "C:main.nmlt",
        "dir/main.nmlt",
        "CON.nmlt",
        "main.NMLT",
    ] {
        assert!(compile_package(entry, |_| panic!("invalid path reached reader")).is_err());
    }
}

#[test]
fn package_limits_include_shared_subtrees_and_global_declarations() {
    let mut input = files(&[
        ("main.nmlt", "import A\nimport Z\nfn main() -> Int { 0 }"),
        ("A.nmlt", "import Tail\nrecord R {}"),
        ("Tail.nmlt", "record T {}"),
        ("Z.nmlt", "import Z1\nrecord R {}"),
    ]);
    for i in 1..7 {
        input.insert(
            format!("Z{i}.nmlt"),
            format!("import Z{}\nrecord R {{}}", i + 1),
        );
    }
    input.insert("Z7.nmlt".into(), "import A\nrecord R {}".into());
    assert!(
        package(&input)
            .unwrap_err()
            .diagnostic
            .message
            .contains("depth")
    );
    let mut input = BTreeMap::from([(
        "main.nmlt".into(),
        (0..32)
            .map(|i| format!("import F{i}\n"))
            .collect::<String>()
            + "fn main() -> Int { 0 }",
    )]);
    for i in 0..32 {
        input.insert(format!("F{i}.nmlt"), "record R {}".into());
    }
    assert!(
        package(&input)
            .unwrap_err()
            .diagnostic
            .message
            .contains("32 source")
    );
    let mut input = files(&[("main.nmlt", "import Lib\nfn main() -> Int { 0 }")]);
    input.insert(
        "Lib.nmlt".into(),
        (0..64)
            .map(|i| format!("fn f{i}() -> Int {{ 0 }}\n"))
            .collect(),
    );
    assert!(
        package(&input)
            .unwrap_err()
            .diagnostic
            .message
            .contains("64 functions")
    );
    input.insert("Lib.nmlt".into(), " ".repeat(MAX_SOURCE_BYTES + 1));
    assert!(
        package(&input)
            .unwrap_err()
            .diagnostic
            .message
            .contains("128 KiB")
    );
    let mut input = BTreeMap::from([(
        "main.nmlt".into(),
        (0..9).map(|i| format!("import F{i}\n")).collect::<String>() + "fn main() -> Int { 0 }",
    )]);
    for i in 0..9 {
        input.insert(
            format!("F{i}.nmlt"),
            "record R {}\n".to_owned() + &" ".repeat(MAX_SOURCE_BYTES - 12),
        );
    }
    let error = package(&input).unwrap_err();
    assert!(error.diagnostic.message.contains("1 MiB"), "{error:?}");
}

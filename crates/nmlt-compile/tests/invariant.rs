use nmlt_compile::{compile_behavior_v2, compile_safety, parse_predicate};
use nmlt_ir::SafetyPredicate;

const SOURCE: &str = include_str!("../../../examples/pivot/safety_invariant.nmlt");

#[test]
fn exact_source_predicates_are_retained_on_an_explicit_compilation_route() {
    assert!(compile_behavior_v2("safety.nmlt", SOURCE.as_bytes()).is_err());
    let program = compile_safety("safety.nmlt", SOURCE.as_bytes()).unwrap();
    assert_eq!(program.properties.len(), 3);
    let property = program
        .properties
        .iter()
        .find(|p| p.name == "UseAfterReceive")
        .unwrap();
    assert_eq!(
        &SOURCE[property.expression_start..property.expression_end],
        property.expression
    );
    assert!(matches!(
        property.predicate,
        SafetyPredicate::Implies { .. }
    ));
    assert_eq!(program.core.source_sha256.len(), 64);
}

#[test]
fn property_types_and_unsupported_temporal_syntax_are_rejected() {
    for expression in [
        "eventually(used)",
        "always(unknown)",
        "always(used + 1)",
        "always(unit)",
        "always(used == unit)",
        "always(used) ignored",
        "always(Receiver.used)",
    ] {
        let source = SOURCE.replace("always(used implies received)", expression);
        let error = compile_safety("safety.nmlt", source.as_bytes()).unwrap_err();
        assert!(error.code().starts_with("NMLT-SAFETY"));
        assert!(error.span().is_some());
    }
    assert!(
        compile_safety(
            "safety.nmlt",
            SOURCE
                .replace("safety NeverUsed", "temporal NeverUsed")
                .as_bytes()
        )
        .is_err()
    );
}

#[test]
fn boolean_precedence_comments_and_finite_enum_equality_are_supported() {
    let source = "enum Phase { Idle, Ready }\nsystem Worker {\nstate phase: Phase = Idle\nstate flag: Bool = false\nsafety Safe = always(phase == Phase.Idle or flag and not false)\n}\nsystem Peer {}\ncompose Network { Worker | Peer }\n";
    // Predicate parsing uses the same finite type table even when only a source
    // system (and no connected product) is being inspected.
    let simpler = source.split("system Peer").next().unwrap();
    let program = compile_safety("enum.nmlt", simpler.as_bytes()).unwrap();
    assert!(matches!(
        program.properties[0].predicate,
        SafetyPredicate::Or { .. }
    ));
    let a = parse_predicate(
        "always(/* literal /* marker */ flag || !false)",
        &program.core,
        "Worker",
    )
    .unwrap();
    let b = parse_predicate("always(flag or not false)", &program.core, "Worker").unwrap();
    assert_eq!(a, b);
}

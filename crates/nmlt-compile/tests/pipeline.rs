use nmlt_compile::{CompileStage, compile_single};

#[test]
fn exact_source_reaches_a_kernel_issued_checked_program() {
    let source = concat!(
        "enum Phase { idle, ready }\n",
        "system S {\n",
        " state phase: Phase = idle\n",
        " state safe: Bool = true\n",
        " action step { set phase = ready }\n",
        " safety Safe = always(safe)\n",
        " observe phase, safe\n",
        "}\n",
    );
    let checked = compile_single("Main", "src/main.nmlt", source).unwrap();
    assert_eq!(
        checked.core_program().resolved_hir_id(),
        checked.resolved_program().resolution_id()
    );
    assert_eq!(checked.resolved_program().modules().len(), 1);
}

#[test]
fn unsupported_source_fails_at_the_projection_boundary() {
    let error = compile_single(
        "Main",
        "src/main.nmlt",
        "system S { resource budget: Nat }\n",
    )
    .unwrap_err();
    assert_eq!(error.stage(), CompileStage::Projection);
    assert!(error.to_string().starts_with("NMLT_COMPILE_PROJECTION:"));
}

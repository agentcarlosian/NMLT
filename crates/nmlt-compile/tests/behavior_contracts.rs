use nmlt_compile::compile_behavior_single;

fn rejects(source: &str, code: &str) {
    let error = compile_behavior_single("contract.nmlt", source.as_bytes())
        .expect_err("unsupported behavior syntax must be rejected");
    assert_eq!(error.code(), code, "{error}\n{source}");
}

#[test]
fn properties_are_rejected_at_the_behavior_boundary() {
    for kind in ["safety", "temporal", "resource"] {
        let source = format!(
            "system S {{\n  state bit: Bool = false\n  {kind} Obligation = always(true)\n}}\n"
        );
        rejects(&source, "NMLT-BHV-UNSUPPORTED-PROPERTY");
        rejects(
            &format!("module M {{\n{source}\n}}\n"),
            "NMLT-BHV-UNSUPPORTED-PROPERTY",
        );
    }
}

#[test]
fn behavior_lists_require_complete_name_syntax() {
    for expression in ["bit + other", "\"bit\"", "bit other", "bit,", "bit,,other"] {
        rejects(
            &format!(
                "system S {{\n  state bit: Bool = false\n  state other: Bool = true\n  observe {expression}\n}}\n"
            ),
            "NMLT-BHV-OBSERVATION-SYNTAX",
        );
    }
    for expression in ["action tick + tock", "action tick tock", "action \"tick\""] {
        rejects(
            &format!(
                "system S {{\n  action tick {{}}\n  action tock {{}}\n  hide {expression}\n}}\n"
            ),
            "NMLT-BHV-OBSERVATION-SYNTAX",
        );
        rejects(
            &format!(
                "system Abstract {{}}\nsystem Concrete {{\n  action tick {{}}\n  action tock {{}}\n}}\nrefine Concrete refines Abstract {{\n  hide {expression}\n}}\n"
            ),
            "NMLT-BHV-REFINE-HIDE",
        );
    }
}

#[test]
fn action_payloads_respect_direction_and_exact_unit_shape() {
    for members in [
        "action tick(value: Unit) {}",
        "action tick {\n  emit unit\n}",
        "port input tick: Unit\naction input tick {\n  emit unit\n}",
        "port output tick: Unit\naction output tick(value: Unit) {}",
    ] {
        rejects(
            &format!("system S {{\n{members}\n}}\n"),
            "NMLT-BHV-ACTION-PAYLOAD",
        );
    }
    for members in [
        "port input tick: Unit\naction input tick(value: Unit) {}",
        "port input tick: Bool\naction input tick(value: Bool) {}",
    ] {
        rejects(
            &format!("system S {{\n{members}\n}}\n"),
            "NMLT-BHV-INPUT-PAYLOAD",
        );
    }
    rejects(
        "system S {\n  capability token: Once<Unit>\n  port input tick: Once<Unit>\n  action input tick(token: Once<Unit>) {}\n}\n",
        "NMLT-BHV-INPUT-OWNERSHIP",
    );
}

#[test]
fn polarity_checks_preserve_independent_consumption_and_fresh_receipts() {
    for (port, action) in [
        ("", "action tick"),
        ("port input tick: Unit", "action input tick"),
        ("port output tick: Unit", "action output tick"),
        (
            "port input tick: Once<Unit>",
            "action input tick(incoming: Once<Unit>)",
        ),
    ] {
        let source = format!(
            "system S {{\n  capability token: Once<Unit>\n  {port}\n  {action} {{\n    consume token\n  }}\n}}\n"
        );
        let program = compile_behavior_single("consumption.nmlt", source.as_bytes()).unwrap();
        let resources = &program.systems["S"].actions["tick"].resources;
        assert!(resources.consumes.contains("token"));
        assert_eq!(
            resources.receives.contains("incoming"),
            action.contains("incoming")
        );
    }
}

#[test]
fn state_maps_require_unique_abstract_targets() {
    rejects(
        "system Abstract {\n  state bit: Bool = false\n}\nsystem Concrete {\n  state x: Bool = false\n  state y: Bool = false\n}\nrefine Concrete refines Abstract {\n  map state x -> bit\n  map state y -> bit\n}\n",
        "NMLT-BHV-STATE-MAP-NONINJECTIVE",
    );
    let source = "system Abstract {\n  state bit: Bool = false\n}\nsystem Concrete {\n  state x: Bool = false\n}\nrefine Concrete refines Abstract {\n  map state x -> bit\n}\n";
    let program = compile_behavior_single("bijective.nmlt", source.as_bytes()).unwrap();
    assert_eq!(program.refinements[0].state_map["x"], "bit");
}

#[test]
fn refinement_hiding_checks_concrete_action_names() {
    rejects(
        "system Abstract {}\nsystem Concrete {}\nrefine Concrete refines Abstract {\n  hide action missing\n}\n",
        "NMLT-BHV-REFINE-HIDDEN-ACTION",
    );
    let source = "system Abstract {}\nsystem Concrete {\n  action tick {}\n  hide action tick\n}\nrefine Concrete refines Abstract {\n  hide action tick /* missing */\n}\n";
    let program = compile_behavior_single("hidden.nmlt", source.as_bytes()).unwrap();
    assert_eq!(
        program.refinements[0]
            .hidden_actions
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["tick"]
    );
    assert!(program.refinements[0].state_map.is_empty());
}

#[test]
fn hidden_updates_are_checked_as_self_reads_in_the_ast() {
    let constant_update = "enum Color { Other, shade }\nsystem Abstract {\n  state shade: Color = Other\n}\nsystem Concrete {\n  state shade: Color = Other\n  action tick {\n    set shade = Color.shade\n  }\n}\nrefine Concrete refines Abstract {\n  map state shade -> shade\n  hide action tick\n}\n";
    rejects(constant_update, "NMLT-BHV-HIDDEN-STATE");
    let self_read = constant_update.replace("set shade = Color.shade", "set shade = shade");
    compile_behavior_single("self-read.nmlt", self_read.as_bytes())
        .expect("an exact self-read remains a valid hidden update");
}

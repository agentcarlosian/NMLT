use nmlt_core::{
    ProjectionIssueKind, SyntaxKind, UntypedMember, UntypedStatement, UntypedUpdateTarget,
    parse_cst, parse_source, project_untyped, render_diagnostic_snapshot,
};

fn assert_stable_spans(source: &str, diagnostics: &[nmlt_core::Diagnostic]) {
    for diagnostic in diagnostics {
        let span = diagnostic
            .span
            .unwrap_or_else(|| panic!("{} has no source span", diagnostic.code));
        assert!(
            span.start <= span.end,
            "{} has a reversed span",
            diagnostic.code
        );
        assert!(
            span.end <= source.len(),
            "{} extends past EOF",
            diagnostic.code
        );
        assert!(source.is_char_boundary(span.start));
        assert!(source.is_char_boundary(span.end));
    }
}

#[test]
fn file_level_conditions_have_stable_zero_width_spans() {
    let empty = parse_cst("");
    assert_eq!(
        render_diagnostic_snapshot("", empty.diagnostics()),
        "error[NMLT0001] bytes 0..0 at 1:1: source file is empty\n"
    );

    let source = "data Bool = True | False";
    let no_system = parse_source(source).unwrap_err();
    assert_eq!(
        render_diagnostic_snapshot(source, &no_system),
        "error[NMLT0005] bytes 24..24 at 1:25: no `system` declaration found\n"
    );
    assert_stable_spans(source, &no_system);
}

#[test]
fn duplicate_declarations_fail_before_typed_elaboration() {
    let duplicate_systems = "system Same {} system Same {}";
    let parsed = parse_cst(duplicate_systems);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.code == "NMLT0006" && diagnostic.span.is_some() })
    );

    // Member-namespace uniqueness is checked at the untyped projection
    // boundary; the recovering parser itself only recognizes both shells.
    let duplicate_state = "system S { state x: Nat = 0\n state x: Nat = 1\n }";
    let parsed = parse_cst(duplicate_state);
    assert!(parsed.diagnostics().is_empty());
    let projected = project_untyped(&parsed);
    assert!(!projected.is_structurally_complete());
    assert!(projected.issues.iter().any(|issue| {
        matches!(
            &issue.kind,
            ProjectionIssueKind::DuplicateDeclaration { name, .. } if name == "x"
        )
    }));
}

#[test]
fn update_target_shape_is_checked_but_declaration_is_deferred() {
    let invalid_target = "system S { state x: Nat = 0\n action go { set x + y = 1 } }";
    let parsed = parse_cst(invalid_target);
    assert!(parsed.diagnostics().is_empty());
    let projected = project_untyped(&parsed);
    assert!(
        projected
            .issues
            .iter()
            .any(|issue| { matches!(issue.kind, ProjectionIssueKind::InvalidUpdateTarget) })
    );

    // `missing` has the grammar of a location. Only resolution can determine
    // that it is undeclared; structural completeness is not semantic success.
    let undeclared = "system S { action go { set missing = 1 } }";
    let projected = project_untyped(&parse_cst(undeclared));
    assert!(projected.is_structurally_complete());
    let systems = projected.file.systems();
    let UntypedMember::Action(action) = &systems[0].members[0] else {
        panic!("expected the action shell")
    };
    let UntypedStatement::Update { target, .. } = &action.statements[0] else {
        panic!("expected the update shell")
    };
    assert!(matches!(
        target,
        UntypedUpdateTarget::Location { root, .. } if root.text == "missing"
    ));
}

#[test]
fn implicit_modification_syntax_cannot_become_an_update() {
    let source = "system S { state x: Nat = 0\n action go { x := 1; set x = 2 } }";
    let parsed = parse_cst(source);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.code == "NMLT2009" && diagnostic.span.is_some() })
    );
    assert_eq!(parsed.root().descendants(SyntaxKind::UpdateStmt).len(), 1);
    assert_eq!(parsed.root().descendants(SyntaxKind::Error).len(), 1);
    assert!(!project_untyped(&parsed).is_structurally_complete());
}

#[test]
fn action_statement_terminators_cannot_smuggle_adjacent_punctuation() {
    for (statement, rejected_run) in [
        ("require x+;", "+;"),
        ("require x;;", ";;"),
        ("require x;+y", ";+"),
    ] {
        let source = format!("system S {{ action go {{ {statement} }} }}");
        let parsed = parse_cst(&source);
        assert_eq!(parsed, parse_cst(&source), "{statement}");
        assert_eq!(parsed.reconstruct(), source, "{statement}");
        let diagnostic = parsed
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.code == "NMLT2013")
            .unwrap_or_else(|| panic!("{statement} was accepted: {:#?}", parsed.diagnostics()));
        let span = diagnostic.span.expect("the rejection retains its span");
        assert_eq!(&source[span.start..span.end], rejected_run, "{statement}");
        assert_stable_spans(&source, parsed.diagnostics());
        assert!(!project_untyped(&parsed).is_structurally_complete());
    }
}

#[test]
fn module_and_import_declarations_reject_unparsed_tails() {
    for (source, code, rejected_tail) in [
        ("module App garbage", "NMLT2003", "garbage"),
        ("import Base.garbage", "NMLT2012", "."),
    ] {
        let parsed = parse_cst(source);
        assert_eq!(parsed, parse_cst(source));
        assert_eq!(parsed.reconstruct(), source);
        let diagnostic = parsed
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .unwrap_or_else(|| panic!("{source:?} was accepted: {:#?}", parsed.diagnostics()));
        let span = diagnostic.span.expect("the rejection retains its span");
        assert_eq!(&source[span.start..span.end], rejected_tail);
        assert_stable_spans(source, parsed.diagnostics());
        assert!(!project_untyped(&parsed).is_structurally_complete());
    }
}

#[test]
fn recovery_dependent_structure_is_deterministic_but_never_complete() {
    let source = concat!(
        "system S {\n",
        "  state x: Nat\n",
        "  action go { mystery; set x = 1\n",
    );
    let first = parse_cst(source);
    let second = parse_cst(source);
    assert_eq!(first, second);
    assert_eq!(
        render_diagnostic_snapshot(source, first.diagnostics()),
        render_diagnostic_snapshot(source, second.diagnostics())
    );
    assert_stable_spans(source, first.diagnostics());
    assert!(!project_untyped(&first).is_structurally_complete());
}

#[test]
fn delimiter_controls_are_lossless_spanned_and_deterministic() {
    for source in [
        "system S { action go { set x = f(1) }",
        "system S { action go(value: Nat] { set x = value } }",
        "system S { action go { set x[0 = 1 } }",
        "}",
    ] {
        let first = parse_cst(source);
        let second = parse_cst(source);
        assert_eq!(first, second);
        assert_eq!(first.reconstruct(), source);
        assert!(
            first
                .diagnostics()
                .iter()
                .any(|diagnostic| { diagnostic.code == "NMLT0002" && diagnostic.span.is_some() })
        );
        assert_stable_spans(source, first.diagnostics());
    }
}

#[test]
fn representative_parser_diagnostics_all_retain_source_spans() {
    for source in [
        "",
        " \r\n\t",
        "system",
        "system S",
        "system S { state x Nat = 0 }",
        "system S { action go { require } }",
        "system S { action go { set = 1 } }",
        "system S { action go { mystery } }",
        "system S { state λ: Nat = 0 }",
        "system S { state x: Text = \"open\n }",
        "/* open",
        "system S {",
    ] {
        let parsed = parse_cst(source);
        assert!(
            !parsed.diagnostics().is_empty(),
            "control unexpectedly clean: {source:?}"
        );
        assert_stable_spans(source, parsed.diagnostics());
    }
}

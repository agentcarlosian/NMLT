use std::fs;
use std::path::{Path, PathBuf};

use nmlt_core::{
    FormatMode, SyntaxKind, format_source, parse_cst, project_untyped, render_diagnostic_snapshot,
};

const FIXTURES: &[&str] = &[
    "examples/agents/trust_chain.nmlt",
    "examples/basics/boolean_toggle.nmlt",
    "examples/concurrency/two_process_mutex.nmlt",
    "examples/distributed/two_phase_commit.nmlt",
    "examples/hyperbook/one_bit_clock.nmlt",
    "examples/math/euclid.nmlt",
    "examples/refinement/bounded_channel.nmlt",
    "examples/resources/token_bucket.nmlt",
    "examples/runtime/durable_controller.nmlt",
    "examples/technicus/provider_attempt.nmlt",
    "benchmarks/seeded-defects/provider-attempt/blind-replay.nmlt",
    "benchmarks/seeded-defects/provider-attempt/dispatch-before-authorize.nmlt",
    "benchmarks/seeded-defects/provider-attempt/passing-selection.nmlt",
    "benchmarks/seeded-defects/provider-attempt/reference.nmlt",
    "benchmarks/seeded-defects/provider-attempt/response-binding.nmlt",
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_fixture(relative_path: &str) -> String {
    fs::read_to_string(repository_root().join(relative_path))
        .unwrap_or_else(|error| panic!("failed to read {relative_path}: {error}"))
}

#[test]
fn all_canonical_and_benchmark_files_build_lossless_trees() {
    for relative_path in FIXTURES {
        let source = read_fixture(relative_path);
        let parsed = parse_cst(&source);
        assert!(
            parsed.diagnostics().is_empty(),
            "{relative_path}:\n{}",
            render_diagnostic_snapshot(&source, parsed.diagnostics())
        );
        assert_eq!(parsed.reconstruct(), source, "{relative_path}");
        assert_eq!(parsed.root().text_len(), source.len(), "{relative_path}");
        assert_eq!(parsed.systems().len(), 1, "{relative_path}");

        let tokens = parsed.root().tokens_with_spans();
        let mut cursor = 0;
        for token in tokens {
            assert_eq!(token.span.start, cursor, "{relative_path}");
            assert_eq!(
                token.token.text(),
                &source[token.span.start..token.span.end]
            );
            cursor = token.span.end;
        }
        assert_eq!(cursor, source.len(), "{relative_path}");
    }
}

#[test]
fn canonical_corpus_exercises_required_declaration_families() {
    let mut sources = String::new();
    for relative_path in &FIXTURES[..10] {
        sources.push_str(&read_fixture(relative_path));
        sources.push('\n');
    }
    let parsed = parse_cst(&sources);
    assert!(parsed.diagnostics().is_empty());

    for kind in [
        SyntaxKind::EnumDecl,
        SyntaxKind::SystemDecl,
        SyntaxKind::ConstDecl,
        SyntaxKind::InputDecl,
        SyntaxKind::StateDecl,
        SyntaxKind::CapabilityDecl,
        SyntaxKind::PortDecl,
        SyntaxKind::ActionDecl,
        SyntaxKind::RequireStmt,
        SyntaxKind::UpdateStmt,
        SyntaxKind::EmitStmt,
        SyntaxKind::ConsumeStmt,
        SyntaxKind::SafetyDecl,
        SyntaxKind::TemporalDecl,
        SyntaxKind::ResourceDecl,
        SyntaxKind::ObserveDecl,
        SyntaxKind::HideDecl,
    ] {
        assert!(
            !parsed.root().descendants(kind).is_empty(),
            "canonical corpus did not produce {kind:?}"
        );
    }
}

#[test]
fn formatter_is_byte_preserving_and_idempotent_across_the_corpus() {
    for relative_path in FIXTURES {
        let source = read_fixture(relative_path);
        let once = format_source(&source, FormatMode::Preserve);
        let twice = format_source(once.text(), FormatMode::Preserve);
        assert!(once.diagnostics().is_empty(), "{relative_path}");
        assert_eq!(once.text(), source, "{relative_path}");
        assert_eq!(twice.text(), once.text(), "{relative_path}");
        assert_eq!(twice.diagnostics(), once.diagnostics(), "{relative_path}");
    }
}

#[test]
fn canonical_corpus_crosses_the_untyped_projection_without_recovery() {
    for relative_path in &FIXTURES[..10] {
        let source = read_fixture(relative_path);
        let parsed = parse_cst(&source);
        let projection = project_untyped(&parsed);
        assert!(
            projection.is_structurally_complete(),
            "{relative_path}: {:?}",
            projection.issues
        );
        let systems = projection.file.systems();
        assert_eq!(systems.len(), 1, "{relative_path}");
        let name = systems[0]
            .name
            .as_ref()
            .expect("a clean canonical system has a name");
        assert_eq!(&source[name.span.start..name.span.end], name.text);
    }
}

#[test]
fn malformed_input_recovers_deterministically_with_spans() {
    let source = "system Broken {\n  state x: Nat\n  action go { mystery x; set x = 1 }\n";
    let first = parse_cst(source);
    let second = parse_cst(source);

    assert_eq!(first, second);
    assert_eq!(first.reconstruct(), source);
    assert!(first.diagnostics().iter().all(|item| item.span.is_some()));
    assert!(!first.root().descendants(SyntaxKind::Error).is_empty());
    assert_eq!(
        first
            .diagnostics()
            .iter()
            .map(|item| item.code)
            .collect::<Vec<_>>(),
        ["NMLT0002", "NMLT2007", "NMLT2009"]
    );
    assert_eq!(
        render_diagnostic_snapshot(source, first.diagnostics()),
        concat!(
            "error[NMLT0002] bytes 14..15 at 1:15: unclosed opening delimiter `{`\n",
            "error[NMLT2007] bytes 30..33 at 2:15: expected `=` in state declaration\n",
            "error[NMLT2009] bytes 45..52 at 3:15: expected `require`, `set`, `emit`, or `consume` in action body\n",
        )
    );
    assert_eq!(
        render_diagnostic_snapshot(source, first.diagnostics()),
        render_diagnostic_snapshot(source, second.diagnostics())
    );
}

#[test]
fn every_utf8_prefix_of_a_representative_model_remains_lossless() {
    let source = read_fixture("examples/technicus/provider_attempt.nmlt");
    for end in (0..=source.len()).filter(|end| source.is_char_boundary(*end)) {
        let prefix = &source[..end];
        let first = parse_cst(prefix);
        let second = parse_cst(prefix);
        assert_eq!(first.reconstruct(), prefix, "prefix ending at byte {end}");
        assert_eq!(first, second, "prefix ending at byte {end}");
    }
}

#[test]
fn unsupported_unicode_is_retained_and_diagnosed() {
    let source = "system S { state λ: Nat = 0 }";
    let parsed = parse_cst(source);
    assert_eq!(parsed.reconstruct(), source);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|item| item.code == "NMLT1003" && item.span.is_some())
    );
}

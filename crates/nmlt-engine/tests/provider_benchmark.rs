use std::fs;
use std::path::{Path, PathBuf};

use nmlt_compile::compile_single;
use nmlt_engine::{CheckConfig, ResultClass, check_model, from_checked};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../benchmarks/seeded-defects/provider-attempt")
}

fn check(path: &Path) -> nmlt_engine::CheckReport {
    let source = fs::read_to_string(path).unwrap();
    let checked = compile_single("Benchmark", path_for_identity(path), source)
        .unwrap_or_else(|error| panic!("{error}"));
    let typed = from_checked(&checked).unwrap_or_else(|errors| panic!("{errors:#?}"));
    check_model(
        &typed,
        CheckConfig {
            max_states: 1_000,
            max_depth: 32,
        },
    )
    .unwrap_or_else(|errors| panic!("{errors:#?}"))
}

fn path_for_identity(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    canonical
        .strip_prefix(root)
        .unwrap_or(&canonical)
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn reference_is_exhaustively_model_checked() {
    let report = check(&corpus().join("reference.nmlt"));
    assert_eq!(report.class, ResultClass::ModelChecked);
    assert!(report.complete);
    assert!(
        report
            .properties
            .iter()
            .all(|result| result.class == ResultClass::ModelChecked)
    );
}

#[test]
fn every_seeded_semantic_defect_is_refuted_by_its_oracle() {
    let cases = [
        (
            "dispatch-before-authorize.nmlt",
            "DispatchRequiresArm",
            Some("dispatch"),
        ),
        ("blind-replay.nmlt", "NoBlindReplay", None),
        (
            "response-binding.nmlt",
            "EvaluationRequiresIntactResponse",
            Some("evaluate"),
        ),
        (
            "passing-selection.nmlt",
            "SelectionRequiresPass",
            Some("select"),
        ),
    ];

    for (file, property, final_action) in cases {
        let report = check(&corpus().join(file));
        assert_eq!(report.class, ResultClass::Refuted, "{file}");
        let result = report
            .properties
            .iter()
            .find(|result| result.property == property)
            .unwrap();
        assert_eq!(result.class, ResultClass::Refuted, "{file}");
        let witness = result.witness.as_ref().unwrap();
        assert_eq!(
            witness.steps.last().unwrap().action.as_deref(),
            final_action,
            "{file}"
        );
        if file == "blind-replay.nmlt" {
            assert_eq!(witness.steps.len(), 1, "state-local violation is zero-step");
            assert!(witness.steps[0].enabled_actions.contains("dispatch"));
        }
    }
}

#[test]
fn one_shot_replay_regression_separates_historical_and_corrected_properties() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../benchmarks/controls/provider-attempt/one-shot-replay-regression.nmlt");
    let report = check(&path);
    assert_eq!(report.class, ResultClass::Refuted);
    let historical = report
        .properties
        .iter()
        .find(|result| result.property == "HistoricalNextNoBlindReplay")
        .unwrap();
    let corrected = report
        .properties
        .iter()
        .find(|result| result.property == "CorrectedNoBlindReplay")
        .unwrap();
    assert_eq!(historical.class, ResultClass::ModelChecked);
    assert_eq!(corrected.class, ResultClass::Refuted);
    let witness = corrected.witness.as_ref().unwrap();
    assert_eq!(witness.steps.len(), 1);
    assert_eq!(witness.steps[0].action, None);
    assert!(witness.steps[0].enabled_actions.contains("dispatch"));
}

#[test]
fn counterexample_generation_is_deterministic() {
    let path = corpus().join("dispatch-before-authorize.nmlt");
    assert_eq!(check(&path), check(&path));
}

#[test]
fn four_type_level_negative_controls_fail_before_exploration() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/type-errors");
    let cases = [
        (
            "invalid-update-target.nmlt",
            "unresolved reference `missing`",
        ),
        ("invalid-initializer.nmlt", "checks against Nat"),
        (
            "duplicated-capability.nmlt",
            "affine capability consumed more than once",
        ),
        (
            "cross-system-property.nmlt",
            "unresolved reference `OtherSystemAction`",
        ),
    ];
    for (file, expected) in cases {
        let source = fs::read_to_string(fixtures.join(file)).unwrap();
        let errors = compile_single("Negative", path_for_identity(&fixtures.join(file)), source)
            .unwrap_err()
            .to_string();
        assert!(
            errors.contains(expected),
            "{file}: expected `{expected}` in {errors:#?}"
        );
    }
}

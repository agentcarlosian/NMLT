use nmlt_elaborate::{ElaborationError, JudgmentKind, elaborate};
use nmlt_hir::{project_source_module, resolve_modules};
use nmlt_ir::{CorePropertyKind, CoreType};

fn resolve(name: &str, source: &str) -> nmlt_hir::ResolvedProgram {
    let projected = project_source_module(
        name,
        format!("src/{}.nmlt", name.to_lowercase()),
        source.as_bytes(),
    );
    assert!(
        projected.projection_issues().is_empty(),
        "{:?}",
        projected.projection_issues()
    );
    resolve_modules(vec![projected]).unwrap()
}

const SUPPORTED: &str = concat!(
    "enum Phase { idle, busy }\n",
    "system Toggle {\n",
    "  state ready: Bool = false\n",
    "  state count: Nat = 0\n",
    "  capability token: Once<Effect>\n",
    "  action set(next_value: Bool) {\n",
    "    require next_value\n",
    "    consume token\n",
    "    set ready = next_value\n",
    "    set count = count + 1\n",
    "    emit count\n",
    "  }\n",
    "  safety Safe = always(ready or not enabled(set))\n",
    "  temporal ReadyAgain = eventually(ready)\n",
    "  observe ready, count\n",
    "}\n",
);

#[test]
fn supported_slice_elaborates_deterministically_with_complete_root_bindings() {
    let hir = resolve("Toggle", SUPPORTED);
    let first = elaborate(&hir).unwrap();
    let second = elaborate(&hir).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.format_version(), 1);
    assert_eq!(first.resolved_hir_id(), hir.resolution_id());
    assert_eq!(first.surface_program_id(), hir.surface_program_id());
    assert_eq!(
        first.required_roots().len(),
        hir.modules()
            .values()
            .map(|module| module.hir_roots().len())
            .sum()
    );
    assert_eq!(
        first.certificate_id().to_string(),
        "nmlt-elaboration-certificate-v1:sha256:ca79034986dd97832e6cd03d6d3667626b0e7dd1e302f3c0776c404921b14d3d"
    );
    assert_eq!(
        first.core_program_id().to_string(),
        "nmlt-core-program-v1:sha256:e43b30ebf315e7a5c24501ff54534d1ddebddfee84848b73d79cda01a76c52c0"
    );
    assert_eq!(
        first.ruleset_bundle_id().to_string(),
        "nmlt-ruleset-bundle-v1:sha256:4d5efdae575aade8e9e6896db2c376425aa578f8f7c385d8a055f1df8741412b"
    );
    assert_eq!(
        first.resource_policy_id().to_string(),
        "nmlt-kernel-policy-v1:sha256:361e1ba13505a45cbcc2cdf94cec60df3e8675079ba41efe52c01c35ca6a20c2"
    );
    assert!(
        first
            .required_roots()
            .keys()
            .any(|key| key.judgment() == JudgmentKind::UpdateTarget)
    );
    assert!(
        first
            .required_roots()
            .keys()
            .any(|key| key.judgment() == JudgmentKind::CapabilityUse)
    );
    assert!(first.derivations().values().any(|node| matches!(
        node.conclusion(),
        nmlt_elaborate::DerivationConclusion::Term {
            ty: CoreType::StateProp { .. },
            ..
        }
    )));
    let system = first
        .core_program()
        .modules()
        .values()
        .next()
        .unwrap()
        .systems()
        .values()
        .next()
        .unwrap();
    assert_eq!(
        system
            .properties()
            .values()
            .filter(|property| property.kind() == CorePropertyKind::Safety)
            .count(),
        1
    );
    assert_eq!(
        system
            .properties()
            .values()
            .filter(|property| property.kind() == CorePropertyKind::Temporal)
            .count(),
        1
    );
    assert_eq!(system.actions().values().next().unwrap().frames().len(), 0);
}

#[test]
fn changed_source_invalidates_every_bound_program_identity() {
    let first_hir = resolve("Identity", "system S { state ready: Bool = false }\n");
    let second_hir = resolve("Identity", "system S { state ready: Bool = true }\n");
    let first = elaborate(&first_hir).unwrap();
    let second = elaborate(&second_hir).unwrap();

    assert_ne!(first.source_set_id(), second.source_set_id());
    assert_ne!(first.surface_program_id(), second.surface_program_id());
    assert_ne!(first.resolved_hir_id(), second.resolved_hir_id());
    assert_ne!(first.core_program_id(), second.core_program_id());
    assert_ne!(first.certificate_id(), second.certificate_id());
}

#[test]
fn explicit_conversions_enums_and_temporal_operators_reach_core() {
    let hir = resolve(
        "Operators",
        concat!(
            "enum Phase { idle, busy }\n",
            "system S {\n",
            " state phase: Phase = idle\n",
            " state n: Nat = 1\n",
            " state i: Int = 0\n",
            " action step(x: Int) {\n",
            "  require x >= 0\n",
            "  set phase = busy\n",
            "  set n = n * 2\n",
            "  set i = i + to_int(n)\n",
            " }\n",
            " temporal Progress = always(next(phase != busy) implies eventually(until(n < 3, n >= 3)))\n",
            "}\n",
        ),
    );
    let artifact = elaborate(&hir).unwrap();
    let mut terms = artifact.core_program().terms().values();
    assert!(
        terms
            .clone()
            .any(|term| matches!(term.kind(), nmlt_ir::CoreTermKind::IntFromNat { .. }))
    );
    assert!(
        terms
            .clone()
            .any(|term| matches!(term.kind(), nmlt_ir::CoreTermKind::Next { .. }))
    );
    assert!(
        terms
            .clone()
            .any(|term| matches!(term.kind(), nmlt_ir::CoreTermKind::Until { .. }))
    );
    assert!(terms.any(|term| matches!(term.kind(), nmlt_ir::CoreTermKind::Constructor { .. })));
}

#[test]
fn expected_types_reject_invalid_initializers_and_nat_subtraction() {
    let invalid_initializer = resolve("BadInit", "system S { state ready: Bool = 0 }\n");
    assert!(matches!(
        elaborate(&invalid_initializer),
        Err(ElaborationError::TypeMismatch { .. })
    ));

    let nat_subtraction = resolve(
        "BadSub",
        "system S {\n state n: Nat = 1\n action dec { set n = n - 1 }\n}\n",
    );
    assert!(matches!(
        elaborate(&nat_subtraction),
        Err(ElaborationError::TypeMismatch { .. })
    ));
}

#[test]
fn affine_capability_consumption_fails_before_core_construction() {
    let hir = resolve(
        "DuplicateConsume",
        concat!(
            "system S {\n",
            " capability token: Once<Effect>\n",
            " action fire { consume token; consume token }\n",
            "}\n",
        ),
    );
    assert!(matches!(
        elaborate(&hir),
        Err(ElaborationError::InvalidHir {
            reason: "affine capability consumed more than once",
            ..
        })
    ));
}

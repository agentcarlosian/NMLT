use nmlt_elaborate::{DerivationConclusion, elaborate};
use nmlt_hir::{project_source_module, resolve_modules};
use nmlt_ir::CoreType;
use nmlt_kernel::{KernelCode, RawCertificate, check};

fn artifact(
    source: &str,
) -> (
    nmlt_hir::ResolvedProgram,
    nmlt_elaborate::ElaborationArtifact,
) {
    let projected = project_source_module("Toggle", "src/toggle.nmlt", source.as_bytes());
    assert!(projected.projection_issues().is_empty());
    let hir = resolve_modules(vec![projected]).unwrap();
    let artifact = elaborate(&hir).unwrap();
    (hir, artifact)
}

const SUPPORTED: &str = concat!(
    "enum Phase { idle, busy }\n",
    "system Toggle {\n",
    " state phase: Phase = idle\n",
    " state ready: Bool = false\n",
    " state count: Nat = 0\n",
    " state total: Int = 0\n",
    " capability token: Once<Effect>\n",
    " action set(next_value: Bool) {\n",
    "  require next_value\n",
    "  consume token\n",
    "  set phase = busy\n",
    "  set ready = next_value\n",
    "  set count = count * 2\n",
    "  set total = total + to_int(count)\n",
    "  emit count\n",
    " }\n",
    " safety Safe = always(ready or not enabled(set))\n",
    " temporal Progress = always(next(phase != busy) implies eventually(until(count < 3, count >= 3)))\n",
    " observe ready, count\n",
    "}\n",
);

#[test]
fn independent_kernel_accepts_the_complete_supported_slice() {
    let (hir, artifact) = artifact(SUPPORTED);
    let raw = RawCertificate::from_artifact(&artifact);
    let checked = check(&hir, artifact.core_program(), &raw).unwrap();
    assert_eq!(
        checked.kernel_profile_id().to_string(),
        "nmlt-kernel-profile-v1:sha256:f8d30d31838ac877f60425c132a06922d8ffd72e5beb44cf226cda8ed65afab5"
    );
    assert_eq!(checked.core_program(), artifact.core_program());
    assert_eq!(checked.certificate_digest(), &raw.certificate_digest);
}

#[test]
fn stale_subject_ruleset_and_policy_bindings_fail_closed() {
    let (hir, artifact) = artifact(SUPPORTED);

    let mut stale = RawCertificate::from_artifact(&artifact);
    stale.source_set_digest[0] ^= 1;
    assert_eq!(
        check(&hir, artifact.core_program(), &stale)
            .unwrap_err()
            .code(),
        KernelCode::StaleBinding
    );

    let mut version = RawCertificate::from_artifact(&artifact);
    version.format_version = 2;
    assert_eq!(
        check(&hir, artifact.core_program(), &version)
            .unwrap_err()
            .code(),
        KernelCode::Version
    );

    let mut ruleset = RawCertificate::from_artifact(&artifact);
    ruleset.ruleset_bundle_digest[0] ^= 1;
    assert_eq!(
        check(&hir, artifact.core_program(), &ruleset)
            .unwrap_err()
            .code(),
        KernelCode::Ruleset
    );

    let mut policy = RawCertificate::from_artifact(&artifact);
    policy.resource_policy_digest[0] ^= 1;
    assert_eq!(
        check(&hir, artifact.core_program(), &policy)
            .unwrap_err()
            .code(),
        KernelCode::ResourcePolicy
    );
}

#[test]
fn resource_limits_are_checked_before_identity_replay() {
    let (hir, artifact) = artifact(SUPPORTED);
    let mut oversized = RawCertificate::from_artifact(&artifact);
    oversized.derivations[0].premises = vec![[0; 32]; 33];
    let diagnostic = check(&hir, artifact.core_program(), &oversized).unwrap_err();
    assert_eq!(diagnostic.code(), KernelCode::ResourceLimit);
    assert!(
        diagnostic
            .to_string()
            .starts_with("NMLT_KERNEL_RESOURCE_LIMIT:")
    );
}

#[test]
fn unknown_duplicate_and_missing_coverage_inputs_are_rejected() {
    let (hir, artifact) = artifact(SUPPORTED);

    let mut unknown = RawCertificate::from_artifact(&artifact);
    unknown.derivations[0].rule_tag = u16::MAX;
    assert_eq!(
        check(&hir, artifact.core_program(), &unknown)
            .unwrap_err()
            .code(),
        KernelCode::UnknownTag
    );

    let mut duplicate = RawCertificate::from_artifact(&artifact);
    duplicate.derivations.push(duplicate.derivations[0].clone());
    assert_eq!(
        check(&hir, artifact.core_program(), &duplicate)
            .unwrap_err()
            .code(),
        KernelCode::DuplicateKey
    );

    let mut missing = RawCertificate::from_artifact(&artifact);
    missing.required_roots.pop();
    assert_eq!(
        check(&hir, artifact.core_program(), &missing)
            .unwrap_err()
            .code(),
        KernelCode::Coverage
    );
}

#[test]
fn cycles_unreachable_nodes_and_noncanonical_order_are_rejected() {
    let (hir, artifact) = artifact(SUPPORTED);

    let mut cyclic = RawCertificate::from_artifact(&artifact);
    let node = cyclic
        .derivations
        .iter_mut()
        .find(|node| !node.premises.is_empty())
        .unwrap();
    node.premises[0] = node.claimed_digest;
    assert_eq!(
        check(&hir, artifact.core_program(), &cyclic)
            .unwrap_err()
            .code(),
        KernelCode::Cycle
    );

    let mut unreachable = RawCertificate::from_artifact(&artifact);
    unreachable
        .derivations
        .iter_mut()
        .find(|node| node.rule_tag == 6)
        .unwrap()
        .premises
        .clear();
    assert_eq!(
        check(&hir, artifact.core_program(), &unreachable)
            .unwrap_err()
            .code(),
        KernelCode::Unreachable
    );

    let mut reordered = RawCertificate::from_artifact(&artifact);
    reordered.derivations.reverse();
    assert_eq!(
        check(&hir, artifact.core_program(), &reordered)
            .unwrap_err()
            .code(),
        KernelCode::NonCanonicalOrder
    );
}

#[test]
fn changed_conclusions_and_certificate_ids_are_rejected() {
    let (hir, artifact) = artifact(SUPPORTED);

    let mut conclusion = RawCertificate::from_artifact(&artifact);
    let node = conclusion
        .derivations
        .iter_mut()
        .find(|node| matches!(node.conclusion, DerivationConclusion::Term { .. }))
        .unwrap();
    if let DerivationConclusion::Term { ty, .. } = &mut node.conclusion {
        *ty = CoreType::Int;
    }
    assert_eq!(
        check(&hir, artifact.core_program(), &conclusion)
            .unwrap_err()
            .code(),
        KernelCode::Identity
    );

    let mut certificate_id = RawCertificate::from_artifact(&artifact);
    certificate_id.certificate_digest[0] ^= 1;
    assert_eq!(
        check(&hir, artifact.core_program(), &certificate_id)
            .unwrap_err()
            .code(),
        KernelCode::Identity
    );
}

use std::collections::BTreeMap;

use nmlt_agent::artifact::{ArtifactRole, ArtifactSet, TrustedArtifact};
use nmlt_agent::assistant::{AssistantInput, DeterministicAssistant, RepairAssistant};
use nmlt_agent::authority::{
    AuthorityError, ByteSpan, CandidateFile, Edit, EditPolicy, Proposal, validate_proposal,
};
use nmlt_agent::feedback::{Feedback, ResultClass};
use nmlt_agent::mutation::{MutationDescriptor, MutationKind};

fn fixture() -> (BTreeMap<String, CandidateFile>, EditPolicy) {
    let path = "benchmarks/agentic/candidates/case.nmlt";
    let source = "state ready: Bool = false;\nsafety Keep: always(ready);\n";
    let candidate = CandidateFile::new(path, source);
    let mut policy = EditPolicy::localized(2, 32);
    policy.allow_span(path, ByteSpan::new(0, source.len()));
    let protected = source.find("safety").unwrap();
    policy
        .protect_span(path, ByteSpan::new(protected, source.len() - 1), source)
        .unwrap();
    (BTreeMap::from([(path.into(), candidate)]), policy)
}

#[test]
fn rejects_specification_weakening_and_oracle_edits() {
    let (candidates, policy) = fixture();
    for role in [ArtifactRole::Property, ArtifactRole::Oracle] {
        let proposal = Proposal::localized(
            "malicious",
            vec![Edit {
                role,
                path: "benchmarks/agentic/trusted/claim.txt".into(),
                span: ByteSpan::new(0, 1),
                replacement: String::new(),
            }],
            "make the checker green",
        );
        assert_eq!(
            validate_proposal(&candidates, &policy, &proposal),
            Err(AuthorityError::NonCandidateRole(role))
        );
    }
}

#[test]
fn rejects_path_traversal_and_symlink_like_escape_strings() {
    let (candidates, policy) = fixture();
    for path in [
        "../trusted/property.txt",
        "benchmarks/agentic/link/../../trusted/oracle.txt",
        "/tmp/escaped.nmlt",
        "benchmarks\\agentic\\escaped.nmlt",
        "benchmarks/./agentic/escaped.nmlt",
    ] {
        let proposal = Proposal::localized(
            "escape",
            vec![Edit::candidate(path, ByteSpan::new(0, 0), "x")],
            "escape candidate root",
        );
        assert!(matches!(
            validate_proposal(&candidates, &policy, &proposal),
            Err(AuthorityError::InvalidPath(_))
        ));
    }
}

#[test]
fn rejects_whole_file_replacement_and_protected_span_changes() {
    let (candidates, policy) = fixture();
    let candidate = candidates.values().next().unwrap();
    let whole = Proposal::localized(
        "replace-all",
        vec![Edit::candidate(
            &candidate.path,
            ByteSpan::new(0, candidate.source.len()),
            "system Empty {}",
        )],
        "replace the complete program",
    );
    assert!(matches!(
        validate_proposal(&candidates, &policy, &whole),
        Err(AuthorityError::WholeFileReplacement(_))
    ));

    let start = candidate.source.find("always").unwrap();
    let weaken = Proposal::localized(
        "weaken-inline-property",
        vec![Edit::candidate(
            &candidate.path,
            ByteSpan::new(start, start + "always".len()),
            "eventually",
        )],
        "weaken inline property",
    );
    assert!(matches!(
        validate_proposal(&candidates, &policy, &weaken),
        Err(AuthorityError::ProtectedSpan(_))
    ));
}

#[test]
fn protected_span_digest_detects_pre_gate_tampering() {
    let (mut candidates, policy) = fixture();
    let candidate = candidates.values_mut().next().unwrap();
    candidate.source = candidate.source.replace("always", "eventually");
    candidate.digest = format!(
        "sha256:{}",
        nmlt_agent::digest::sha256_hex(candidate.source.as_bytes())
    );
    let proposal = Proposal::localized(
        "otherwise-local",
        vec![Edit::candidate(&candidate.path, ByteSpan::new(0, 0), " ")],
        "unrelated edit after protected tampering",
    );
    assert!(matches!(
        validate_proposal(&candidates, &policy, &proposal),
        Err(AuthorityError::ProtectedDigestChanged(_))
    ));
}

#[test]
fn protected_path_digest_detects_trusted_artifact_tampering() {
    let (_, mut policy) = fixture();
    policy.protect_path("benchmarks/agentic/trusted/property.txt", b"always(ready)");
    let changed = [(
        "benchmarks/agentic/trusted/property.txt",
        b"eventually(ready)".as_slice(),
    )];
    assert!(matches!(
        policy.verify_protected_paths(changed),
        Err(AuthorityError::ProtectedDigestChanged(_))
    ));
}

#[test]
fn rejects_dropped_controls_and_forged_results() {
    let mut frozen = ArtifactSet::default();
    let intent = TrustedArtifact::freeze(
        "intent",
        ArtifactRole::Intent,
        "intent.txt",
        b"intent".to_vec(),
    );
    let oracle = TrustedArtifact::freeze(
        "oracle",
        ArtifactRole::Oracle,
        "oracle.txt",
        b"control".to_vec(),
    );
    frozen.insert(intent.clone()).unwrap();
    frozen.insert(oracle).unwrap();
    let mut dropped = ArtifactSet::default();
    dropped.insert(intent).unwrap();
    assert!(dropped.verify_frozen(&frozen).is_err());

    let (candidates, policy) = fixture();
    let path = candidates.keys().next().unwrap();
    let forged = Proposal {
        proposal_id: "forged".into(),
        edits: vec![Edit::candidate(path, ByteSpan::new(0, 0), " ")],
        rationale: "claim success without checker".into(),
        claimed_result: Some(ResultClass::ModelChecked),
    };
    assert_eq!(
        validate_proposal(&candidates, &policy, &forged),
        Err(AuthorityError::ForgedResult)
    );
}

#[test]
fn unknown_and_conflict_never_seed_repair() {
    let assistant = DeterministicAssistant;
    for feedback in [
        Feedback::Unknown {
            reason: "state bound".into(),
            bounds_or_backend: "states=10".into(),
        },
        Feedback::Conflict {
            raw_backend_results: BTreeMap::from([
                ("a".into(), "model_checked".into()),
                ("b".into(), "refuted".into()),
            ]),
        },
    ] {
        let input = AssistantInput::bounded(
            "terminal",
            "benchmarks/agentic/candidates/case.nmlt",
            "system Case {}",
            vec![ByteSpan::new(0, 14)],
            feedback,
        );
        assert!(assistant.propose(&input).is_none());
    }
}

#[test]
fn semantic_mutation_cannot_target_property() {
    let mut descriptor = MutationDescriptor::candidate(
        "mutation:one",
        "property:one",
        MutationKind::DeleteGuard,
        "benchmarks/agentic/candidates/case.nmlt",
        ByteSpan::new(2, 8),
    );
    assert!(descriptor.validate().is_ok());
    descriptor.target_role = ArtifactRole::Property;
    assert!(descriptor.validate().is_err());
}

#[test]
fn rejects_zero_width_insertion_inside_protected_span() {
    let (candidates, policy) = fixture();
    let candidate = candidates.values().next().expect("fixture candidate");
    let insertion = candidate
        .source
        .find("always")
        .expect("protected property body")
        + 1;
    let proposal = Proposal::localized(
        "insert-inside-protected",
        vec![Edit::candidate(
            candidate.path.clone(),
            ByteSpan::new(insertion, insertion),
            "x",
        )],
        "attempt to insert bytes into a protected span",
    );

    assert!(matches!(
        validate_proposal(&candidates, &policy, &proposal),
        Err(AuthorityError::ProtectedSpan(_))
    ));
}

#[test]
fn protected_span_endpoints_require_explicit_permission() {
    for at_start in [true, false] {
        let (candidates, mut policy) = fixture();
        let candidate = candidates.values().next().expect("fixture candidate");
        let boundary = if at_start {
            candidate
                .source
                .find("safety")
                .expect("protected property start")
        } else {
            candidate.source.len() - 1
        };
        let proposal = Proposal::localized(
            "insert-at-protected-boundary",
            vec![Edit::candidate(
                candidate.path.clone(),
                ByteSpan::new(boundary, boundary),
                " ",
            )],
            "explicitly authorized boundary insertion",
        );

        assert!(matches!(
            validate_proposal(&candidates, &policy, &proposal),
            Err(AuthorityError::ProtectedBoundary(_))
        ));

        policy.allow_insertion_boundary(candidate.path.clone(), boundary);
        nmlt_agent::authority::apply_proposal(&candidates, &policy, &proposal)
            .expect("allowed boundary insertion preserves protected bytes");
    }
}

use nmlt_verify::*;

fn id(label: &str) -> Sha256Id {
    Sha256Id::digest(label.as_bytes())
}

fn backend(name: &str) -> BackendIdentity {
    BackendIdentity {
        name: name.to_owned(),
        version: "0.0.1+test.1".to_owned(),
        build_digest: id(&format!("build:{name}")),
        protocol: "nmlt-engine-result/1".to_owned(),
        trusted_components: vec![TrustedComponent {
            name: "nmlt-verify-certificate-checker".to_owned(),
            version: "0.0.1+test.1".to_owned(),
            digest: id("certificate-checker:test.1"),
            role: "finite invariant certificate checking".to_owned(),
        }],
    }
}

fn safe_dispatch_vc() -> FiniteSafetyVc {
    // Bits: 0 = authorized, 1 = dispatched.
    let authorized = BoolExpr::current(0);
    let dispatched = BoolExpr::current(1);
    let next_authorized = BoolExpr::next(0);
    let next_dispatched = BoolExpr::next(1);
    let authorize = BoolExpr::And(vec![
        BoolExpr::negate(authorized.clone()),
        next_authorized.clone(),
        BoolExpr::iff(next_dispatched.clone(), dispatched.clone()),
    ]);
    let dispatch = BoolExpr::And(vec![
        authorized.clone(),
        BoolExpr::iff(next_authorized.clone(), authorized.clone()),
        next_dispatched.clone(),
    ]);
    let stutter = BoolExpr::And(vec![
        BoolExpr::iff(next_authorized, authorized.clone()),
        BoolExpr::iff(next_dispatched, dispatched.clone()),
    ]);
    let config = VerificationConfig {
        finite_domain: true,
        terminal_stutter: true,
        assumptions: vec!["closed-system".to_owned()],
    };
    FiniteSafetyVc {
        identity: VerificationIdentity {
            model: id("model:safe-dispatch"),
            claim: id("claim:dispatch-implies-authorized"),
            configuration: config.identity(),
        },
        config,
        variables: vec!["authorized".to_owned(), "dispatched".to_owned()],
        initial: BoolExpr::And(vec![
            BoolExpr::negate(authorized.clone()),
            BoolExpr::negate(dispatched.clone()),
        ]),
        transition: BoolExpr::Or(vec![authorize, dispatch, stutter]),
        property: BoolExpr::implies(dispatched, authorized),
    }
}

fn reachability() -> ReachabilityEngine {
    ReachabilityEngine {
        identity: backend("nmlt-explicit-reachability"),
        limits: ReachabilityLimits {
            max_states: 16,
            max_depth: 16,
        },
    }
}

fn induction() -> InductiveEngine {
    InductiveEngine {
        identity: backend("nmlt-finite-inductiveness"),
    }
}

#[test]
fn two_distinct_engines_check_the_same_exact_claim() {
    let vc = safe_dispatch_vc();
    let explicit = reachability().check(&vc);
    let symbolic = induction().check(&vc);

    let replayed = normalize_result(&vc, &explicit);
    assert_eq!(replayed.class, NormalizedClass::ModelChecked);
    assert!(replayed.certificate_accepted);
    let checked = normalize_result(&vc, &symbolic);
    assert_eq!(checked.class, NormalizedClass::Proved);
    assert!(checked.certificate_accepted);

    let composed = compose_evidence(&vc, vec![explicit, symbolic]).unwrap();
    assert_eq!(composed.class, NormalizedClass::Proved);
    assert!(composed.disagreements.is_empty());
    assert_eq!(composed.raw_results.len(), 2);
    assert_ne!(
        composed.raw_results[0].method,
        composed.raw_results[1].method
    );
}

#[test]
fn forged_certificate_is_rejected() {
    let vc = safe_dispatch_vc();
    let mut forged = induction().check(&vc);
    forged
        .certificate
        .as_mut()
        .unwrap()
        .invariant_states
        .remove(0); // removes initial valuation 0

    let normalized = normalize_result(&vc, &forged);
    assert_eq!(normalized.class, NormalizedClass::Unknown);
    assert!(normalized.reason.contains("initial state 0"));
}

#[test]
fn engine_disagreement_fails_closed_and_retains_raw_results() {
    let vc = safe_dispatch_vc();
    let explicit = reachability().check(&vc);
    let mut hostile = explicit.clone();
    hostile.engine.name = "hostile-backend".to_owned();
    hostile.status = RawStatus::Refuted {
        witness_states: vec![0],
    };
    hostile.raw_output = b"refuted-even-though-the-other-engine-held".to_vec();

    let composed = compose_evidence(&vc, vec![explicit, hostile]).unwrap();
    assert_eq!(composed.class, NormalizedClass::Unknown);
    assert_eq!(composed.raw_results.len(), 2);
    assert!(composed.disagreements[0].contains("accepted"));
    assert_eq!(
        composed.raw_results[1].raw_output,
        b"refuted-even-though-the-other-engine-held"
    );
}

#[test]
fn stale_configuration_binding_is_rejected() {
    let vc = safe_dispatch_vc();
    let mut stale = reachability().check(&vc);
    stale.binding.configuration_id = id("configuration:old");

    let normalized = normalize_result(&vc, &stale);
    assert_eq!(normalized.class, NormalizedClass::Unknown);
    assert!(normalized.reason.contains("configuration identity"));
}

#[test]
fn bounded_result_cannot_be_promoted_to_proved() {
    let vc = safe_dispatch_vc();
    let mut bounded = induction().check(&vc);
    bounded.scope = ResultScope::Bounded {
        max_depth: 2,
        max_states: 4,
    };

    let normalized = normalize_result(&vc, &bounded);
    assert_eq!(normalized.class, NormalizedClass::Unknown);
    assert!(normalized.reason.contains("cannot be promoted"));
}

#[test]
fn sampled_scope_cannot_be_laundered_into_model_checked() {
    let vc = safe_dispatch_vc();
    let mut sampled = reachability().check(&vc);
    sampled.scope = ResultScope::Sampled {
        seed: 1,
        cases: 100,
        max_steps: 10,
    };
    assert_eq!(
        normalize_result(&vc, &sampled).class,
        NormalizedClass::Unknown
    );
}

#[test]
fn bounded_or_uncertified_holds_cannot_be_laundered_into_model_checked() {
    let vc = safe_dispatch_vc();
    let complete = reachability().check(&vc);

    let mut bounded = complete.clone();
    bounded.scope = ResultScope::Bounded {
        max_depth: 16,
        max_states: 16,
    };
    assert_eq!(
        normalize_result(&vc, &bounded).class,
        NormalizedClass::Unknown
    );

    let mut uncertified = complete.clone();
    uncertified.certificate = None;
    assert_eq!(
        normalize_result(&vc, &uncertified).class,
        NormalizedClass::Unknown
    );

    let mut false_coverage = complete;
    false_coverage
        .certificate
        .as_mut()
        .unwrap()
        .invariant_states = vec![0];
    assert_eq!(
        normalize_result(&vc, &false_coverage).class,
        NormalizedClass::Unknown
    );
}

#[test]
fn a_non_inductive_claim_is_not_misreported_as_a_counterexample() {
    let mut vc = safe_dispatch_vc();
    // `not dispatched` holds initially but the valid dispatch transition breaks it.
    vc.identity.claim = id("claim:never-dispatched");
    vc.property = BoolExpr::negate(BoolExpr::current(1));

    let raw = induction().check(&vc);
    assert!(matches!(raw.status, RawStatus::Unknown { .. }));
    assert_eq!(normalize_result(&vc, &raw).class, NormalizedClass::Unknown);
    assert_eq!(
        normalize_result(&vc, &reachability().check(&vc)).class,
        NormalizedClass::Refuted
    );
}

#[test]
fn smt_protocol_binds_query_and_preserves_unknown_without_certificate() {
    let vc = safe_dispatch_vc();
    let query = encode_inductiveness_query(&vc).unwrap();
    assert!(query.contains(SMT_LIB_VERSION));
    assert!(query.contains(vc.digest().unwrap().as_str()));
    assert!(query.contains(vc.identity.configuration.as_str()));
    assert!(query.contains("(get-proof)"));

    let returned = SmtBackendReturn {
        engine: BackendIdentity {
            protocol: SMT_LIB_VERSION.to_owned(),
            ..backend("external-smt")
        },
        binding: EngineBinding::for_vc(&vc).unwrap(),
        stdout: b"unsat\n(proof opaque)\n".to_vec(),
        stderr: Vec::new(),
        certificate: None,
    };
    let raw = normalize_smt_return(&vc, returned);
    assert!(matches!(raw.status, RawStatus::Unknown { .. }));
    assert_eq!(normalize_result(&vc, &raw).class, NormalizedClass::Unknown);
    assert_eq!(parse_smt_status(b"unknown\n"), Ok(SmtStatus::Unknown));
    assert!(parse_smt_status(b"sat\nunsat\n").is_err());
}

#[test]
fn proof_assistant_export_is_identity_bound_and_return_is_checked() {
    let vc = safe_dispatch_vc();
    let export = export_lean4_inductiveness(&vc, "4.30.0").unwrap();
    assert_eq!(export.protocol, PROOF_ASSISTANT_PROTOCOL);
    assert!(export.source.contains(vc.digest().unwrap().as_str()));
    assert!(export.source.contains("def NMLTInductiveObligation : Prop"));
    assert!(!export.source.contains("sorry"));
    assert!(!export.source.contains("axiom"));

    let no_certificate = normalize_proof_assistant_return(
        &vc,
        ProofAssistantReturn {
            engine: BackendIdentity {
                protocol: PROOF_ASSISTANT_PROTOCOL.to_owned(),
                ..backend("lean-backend")
            },
            binding: EngineBinding::for_vc(&vc).unwrap(),
            certificate: None,
            stdout: b"theorem checked".to_vec(),
            stderr: Vec::new(),
        },
    );
    assert_eq!(
        normalize_result(&vc, &no_certificate).class,
        NormalizedClass::Unknown
    );

    let checked_certificate = induction().check(&vc).certificate;
    let returned = normalize_proof_assistant_return(
        &vc,
        ProofAssistantReturn {
            engine: BackendIdentity {
                protocol: PROOF_ASSISTANT_PROTOCOL.to_owned(),
                ..backend("lean-backend")
            },
            binding: EngineBinding::for_vc(&vc).unwrap(),
            certificate: checked_certificate,
            stdout: b"theorem checked and certificate emitted".to_vec(),
            stderr: Vec::new(),
        },
    );
    assert_eq!(
        normalize_result(&vc, &returned).class,
        NormalizedClass::Proved
    );
}

#[test]
fn model_based_hook_has_a_distinct_tested_class_and_reproducible_scope() {
    let vc = safe_dispatch_vc();
    let plan = ModelTestPlan {
        seed: 7,
        cases: 20,
        max_steps: 8,
    };
    let first = run_model_based_tests(&vc, backend("nmlt-model-test"), plan);
    let second = run_model_based_tests(&vc, backend("nmlt-model-test"), plan);
    assert_eq!(first, second);
    assert_eq!(
        normalize_result(&vc, &first.result).class,
        NormalizedClass::Tested
    );
}

#[test]
fn exact_backend_version_and_trusted_component_are_mandatory() {
    let vc = safe_dispatch_vc();
    let mut raw = reachability().check(&vc);
    raw.engine.version = "unknown".to_owned();
    assert_eq!(normalize_result(&vc, &raw).class, NormalizedClass::Unknown);

    let mut raw = reachability().check(&vc);
    raw.engine.trusted_components.clear();
    assert_eq!(normalize_result(&vc, &raw).class, NormalizedClass::Unknown);
}

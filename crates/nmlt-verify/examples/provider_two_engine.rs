use nmlt_verify::{
    BackendIdentity, BoolExpr, CompositeEvidence, FiniteSafetyVc, InductiveEngine, NormalizedClass,
    RawEngineResult, RawStatus, ReachabilityEngine, ReachabilityLimits, ResultScope, Sha256Id,
    TrustedComponent, VerificationConfig, VerificationIdentity, compose_evidence, normalize_result,
};

const SOURCE_PATH: &str = "benchmarks/seeded-defects/provider-attempt/reference.nmlt";
const PROPERTY_PATH: &str = "benchmarks/provider-attempt/properties/dispatch-requires-arm.json";
const SOURCE: &[u8] =
    include_bytes!("../../../benchmarks/seeded-defects/provider-attempt/reference.nmlt");
const PROPERTY: &[u8] =
    include_bytes!("../../../benchmarks/provider-attempt/properties/dispatch-requires-arm.json");

fn implementation_digest() -> Sha256Id {
    let files: &[(&str, &[u8])] = &[
        ("certificate.rs", include_bytes!("../src/certificate.rs")),
        ("evidence.rs", include_bytes!("../src/evidence.rs")),
        ("identity.rs", include_bytes!("../src/identity.rs")),
        ("inductive.rs", include_bytes!("../src/inductive.rs")),
        ("ir.rs", include_bytes!("../src/ir.rs")),
        ("proof.rs", include_bytes!("../src/proof.rs")),
        ("reachability.rs", include_bytes!("../src/reachability.rs")),
        ("smt.rs", include_bytes!("../src/smt.rs")),
        ("test_hook.rs", include_bytes!("../src/test_hook.rs")),
    ];
    let mut canonical = Vec::new();
    for (path, bytes) in files {
        canonical.extend_from_slice(&(path.len() as u64).to_be_bytes());
        canonical.extend_from_slice(path.as_bytes());
        canonical.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        canonical.extend_from_slice(bytes);
    }
    Sha256Id::digest(&canonical)
}

fn backend(name: &str) -> BackendIdentity {
    BackendIdentity {
        name: name.to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        build_digest: implementation_digest(),
        protocol: "nmlt-engine-result/1".to_owned(),
        trusted_components: vec![TrustedComponent {
            name: "nmlt-finite-invariant-certificate-checker".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            digest: Sha256Id::digest(include_bytes!("../src/certificate.rs")),
            role: "independent finite-invariant and witness replay".to_owned(),
        }],
    }
}

/// A documented projection of the provider property onto the two observables named
/// by its frozen property contract. This is deliberately not presented as a verified
/// compiler translation of the complete provider model.
fn provider_dispatch_vc() -> FiniteSafetyVc {
    // Bit 0 = armed, bit 1 = dispatched.
    let armed = BoolExpr::current(0);
    let dispatched = BoolExpr::current(1);
    let next_armed = BoolExpr::next(0);
    let next_dispatched = BoolExpr::next(1);
    let arm = BoolExpr::And(vec![
        BoolExpr::negate(armed.clone()),
        next_armed.clone(),
        BoolExpr::iff(next_dispatched.clone(), dispatched.clone()),
    ]);
    let dispatch = BoolExpr::And(vec![
        armed.clone(),
        BoolExpr::iff(next_armed.clone(), armed.clone()),
        next_dispatched.clone(),
    ]);
    let projected_stutter = BoolExpr::And(vec![
        BoolExpr::iff(next_armed, armed.clone()),
        BoolExpr::iff(next_dispatched, dispatched.clone()),
    ]);
    let config = VerificationConfig {
        finite_domain: true,
        terminal_stutter: true,
        assumptions: vec![
            "manual two-observable projection of the frozen provider source".to_owned(),
            "unrepresented actions stutter armed and dispatched".to_owned(),
        ],
    };
    FiniteSafetyVc {
        identity: VerificationIdentity {
            model: Sha256Id::digest(SOURCE),
            claim: Sha256Id::digest(PROPERTY),
            configuration: config.identity(),
        },
        config,
        variables: vec!["armed".to_owned(), "dispatched".to_owned()],
        initial: BoolExpr::And(vec![
            BoolExpr::negate(armed.clone()),
            BoolExpr::negate(dispatched.clone()),
        ]),
        transition: BoolExpr::Or(vec![arm, dispatch, projected_stutter]),
        property: BoolExpr::implies(dispatched, armed),
    }
}

fn reachability() -> ReachabilityEngine {
    ReachabilityEngine {
        identity: backend("nmlt-explicit-reachability"),
        limits: ReachabilityLimits {
            max_states: 4,
            max_depth: 4,
        },
    }
}

fn induction() -> InductiveEngine {
    InductiveEngine {
        identity: backend("nmlt-finite-inductiveness"),
    }
}

fn class_name(class: NormalizedClass) -> &'static str {
    match class {
        NormalizedClass::Proved => "proved",
        NormalizedClass::ModelChecked => "model_checked",
        NormalizedClass::Tested => "tested",
        NormalizedClass::Refuted => "refuted",
        NormalizedClass::Unknown => "unknown",
    }
}

fn json_string(value: &str) -> String {
    let mut result = String::from("\"");
    for character in value.chars() {
        match character {
            '\"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            character if character.is_control() => {
                result.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => result.push(character),
        }
    }
    result.push('\"');
    result
}

fn state_array(states: &[u64]) -> String {
    states
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn raw_status(result: &RawEngineResult) -> (&'static str, Vec<u64>) {
    match &result.status {
        RawStatus::Holds { requested_class } => (class_name(*requested_class), Vec::new()),
        RawStatus::Refuted { witness_states } => ("refuted", witness_states.clone()),
        RawStatus::Unknown { .. } => ("unknown", Vec::new()),
        RawStatus::BackendFailure { .. } => ("backend_failure", Vec::new()),
    }
}

fn scope_json(scope: &ResultScope) -> String {
    match scope {
        ResultScope::CompleteFinite { states } => {
            format!("{{\"kind\":\"complete_finite\",\"states\":{states}}}")
        }
        ResultScope::Bounded {
            max_depth,
            max_states,
        } => format!(
            "{{\"kind\":\"bounded\",\"max_depth\":{max_depth},\"max_states\":{max_states}}}"
        ),
        ResultScope::Sampled {
            seed,
            cases,
            max_steps,
        } => format!(
            "{{\"kind\":\"sampled\",\"seed\":{seed},\"cases\":{cases},\"max_steps\":{max_steps}}}"
        ),
    }
}

fn engine_json(raw: &RawEngineResult, vc: &FiniteSafetyVc) -> String {
    let normalized = normalize_result(vc, raw);
    let (status, witness) = raw_status(raw);
    let certificate = raw.certificate.as_ref().map_or_else(
        || "null".to_owned(),
        |certificate| {
            format!(
                "{{\"format\":{},\"vc_digest\":{},\"invariant_states\":[{}]}}",
                json_string(&certificate.format),
                json_string(certificate.vc_digest.as_str()),
                state_array(&certificate.invariant_states)
            )
        },
    );
    format!(
        "{{\"name\":{},\"version\":{},\"method\":{},\"build_digest\":{},\"raw_status\":{},\"raw_output\":{},\"scope\":{},\"normalized\":{},\"certificate_accepted\":{},\"certificate\":{},\"witness_states\":[{}]}}",
        json_string(&raw.engine.name),
        json_string(&raw.engine.version),
        json_string(&raw.method),
        json_string(raw.engine.build_digest.as_str()),
        json_string(status),
        json_string(std::str::from_utf8(&raw.raw_output).expect("fixture raw output is UTF-8")),
        scope_json(&raw.scope),
        json_string(class_name(normalized.class)),
        normalized.certificate_accepted,
        certificate,
        state_array(&witness),
    )
}

fn composite_class(composite: &CompositeEvidence) -> &'static str {
    class_name(composite.class)
}

fn main() {
    let vc = provider_dispatch_vc();
    let mut explicit = reachability().check(&vc);
    explicit.raw_output = b"holds; frontier exhausted; reachable_states=3".to_vec();
    let mut inductive = induction().check(&vc);
    inductive.raw_output = b"holds; initiation and consecution enumerated; valuations=4".to_vec();
    let composite =
        compose_evidence(&vc, vec![explicit.clone(), inductive.clone()]).expect("valid fixture VC");

    assert_eq!(composite.class, NormalizedClass::Proved);
    assert_eq!(composite.raw_results.len(), 2);
    assert!(composite.disagreements.is_empty());

    let mut anti_vacuity_vc = vc.clone();
    anti_vacuity_vc.identity.claim = Sha256Id::digest(b"anti-vacuity:dispatch-is-reachable");
    anti_vacuity_vc.property = BoolExpr::negate(BoolExpr::current(1));
    let anti_vacuity = reachability().check(&anti_vacuity_vc);
    let anti_vacuity_normalized = normalize_result(&anti_vacuity_vc, &anti_vacuity);
    let (_, anti_vacuity_witness) = raw_status(&anti_vacuity);
    assert_eq!(anti_vacuity_normalized.class, NormalizedClass::Refuted);
    assert_eq!(anti_vacuity_witness, vec![0, 1, 3]);

    let mut hostile = explicit.clone();
    hostile.engine = backend("contradictory-negative-control");
    hostile.status = RawStatus::Refuted {
        witness_states: vec![0],
    };
    hostile.raw_output = b"refuted; deliberately invalid witness=0".to_vec();
    let disagreement =
        compose_evidence(&vc, vec![explicit.clone(), hostile]).expect("valid fixture VC");
    assert_eq!(disagreement.class, NormalizedClass::Unknown);
    assert_eq!(disagreement.raw_results.len(), 2);
    assert!(!disagreement.disagreements.is_empty());

    let mut bounded_proof = inductive.clone();
    bounded_proof.scope = ResultScope::Bounded {
        max_depth: 1,
        max_states: 2,
    };
    let bounded_normalized = normalize_result(&vc, &bounded_proof);
    assert_eq!(bounded_normalized.class, NormalizedClass::Unknown);

    println!(
        "{{\"schema_version\":\"1.0.0\",\"fixture\":\"provider-dispatch-two-engine-v1\",\"source\":{{\"path\":{},\"content_sha256\":{}}},\"claim\":{{\"path\":{},\"handle\":\"ProviderAttempt.DispatchRequiresArm\",\"content_sha256\":{}}},\"abstraction\":{{\"variables\":[\"armed\",\"dispatched\"],\"kind\":\"manual-observable-projection\",\"compiler_verified\":false}},\"vc_digest\":{},\"engines\":[{},{}],\"composite\":{{\"classification\":{},\"assurance_subject\":\"finite_vc_only\",\"raw_results_retained\":{},\"disagreements\":[]}},\"anti_vacuity\":{{\"classification\":{},\"witness_states\":[{}]}},\"negative_controls\":{{\"disagreement\":{{\"classification\":{},\"raw_results_retained\":{},\"reason_count\":{}}},\"bounded_proof_laundering\":{{\"classification\":{}}}}},\"residual_gaps\":[\"the full NMLT-to-VC translation is not yet verified\",\"the manual projection omits phase, authority provenance, and attempt identity\",\"both engines and the checker execute in one Rust process\"]}}",
        json_string(SOURCE_PATH),
        json_string(Sha256Id::digest(SOURCE).as_str()),
        json_string(PROPERTY_PATH),
        json_string(Sha256Id::digest(PROPERTY).as_str()),
        json_string(vc.digest().expect("valid fixture VC").as_str()),
        engine_json(&explicit, &vc),
        engine_json(&inductive, &vc),
        json_string(composite_class(&composite)),
        composite.raw_results.len(),
        json_string(class_name(anti_vacuity_normalized.class)),
        state_array(&anti_vacuity_witness),
        json_string(composite_class(&disagreement)),
        disagreement.raw_results.len(),
        disagreement.disagreements.len(),
        json_string(class_name(bounded_normalized.class)),
    );
}

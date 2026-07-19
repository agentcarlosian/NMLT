use crate::evidence::{
    BackendIdentity, EngineBinding, NormalizedClass, RawEngineResult, RawStatus, ResultScope,
};
use crate::ir::{FiniteSafetyVc, eval};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelTestPlan {
    pub seed: u64,
    pub cases: usize,
    pub max_steps: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelTestReport {
    pub executed_cases: usize,
    pub executed_steps: usize,
    pub result: RawEngineResult,
}

/// Deterministic model-based trace generation hook. Passing is `tested`, never
/// model-checked or proved; a discovered concrete bad trace is a refutation.
pub fn run_model_based_tests(
    vc: &FiniteSafetyVc,
    engine: BackendIdentity,
    plan: ModelTestPlan,
) -> ModelTestReport {
    let binding = EngineBinding::for_vc(vc).unwrap_or_else(|_| EngineBinding {
        vc_digest: crate::identity::Sha256Id::digest(b"invalid-vc"),
        model_id: vc.identity.model.clone(),
        claim_id: vc.identity.claim.clone(),
        configuration_id: vc.identity.configuration.clone(),
    });
    let scope = ResultScope::Sampled {
        seed: plan.seed,
        cases: plan.cases,
        max_steps: plan.max_steps,
    };
    if let Err(error) = vc.validate() {
        return report(
            engine,
            binding,
            scope,
            0,
            0,
            RawStatus::Unknown {
                reason: format!("invalid VC: {error}"),
            },
        );
    }
    let initial: Vec<_> = (0..vc.state_count())
        .filter(|&state| eval(&vc.initial, state, 0))
        .collect();
    if initial.is_empty() {
        return report(
            engine,
            binding,
            scope,
            0,
            0,
            RawStatus::Unknown {
                reason: "model has no initial valuation".to_owned(),
            },
        );
    }

    let mut random = Lcg(plan.seed);
    let mut executed_steps = 0;
    for case in 0..plan.cases {
        let mut state = initial[random.index(initial.len())];
        let mut trace = vec![state as u64];
        for _ in 0..=plan.max_steps {
            if !eval(&vc.property, state, 0) {
                return report(
                    engine,
                    binding,
                    scope,
                    case + 1,
                    executed_steps,
                    RawStatus::Refuted {
                        witness_states: trace,
                    },
                );
            }
            let successors: Vec<_> = (0..vc.state_count())
                .filter(|&next| eval(&vc.transition, state, next))
                .collect();
            if successors.is_empty() || trace.len() > plan.max_steps {
                break;
            }
            state = successors[random.index(successors.len())];
            trace.push(state as u64);
            executed_steps += 1;
        }
    }
    report(
        engine,
        binding,
        scope,
        plan.cases,
        executed_steps,
        RawStatus::Holds {
            requested_class: NormalizedClass::Tested,
        },
    )
}

fn report(
    engine: BackendIdentity,
    binding: EngineBinding,
    scope: ResultScope,
    executed_cases: usize,
    executed_steps: usize,
    status: RawStatus,
) -> ModelTestReport {
    ModelTestReport {
        executed_cases,
        executed_steps,
        result: RawEngineResult {
            engine,
            binding,
            method: "deterministic-model-traces/1".to_owned(),
            scope,
            status,
            certificate: None,
            raw_output: Vec::new(),
        },
    }
}

struct Lcg(u64);

impl Lcg {
    fn index(&mut self, length: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 as usize) % length
    }
}

use crate::certificate::{FINITE_INVARIANT_CERTIFICATE_VERSION, FiniteInvariantCertificate};
use crate::evidence::{
    BackendIdentity, EngineBinding, NormalizedClass, RawEngineResult, RawStatus, ResultScope,
};
use crate::ir::{BoolExpr, FiniteSafetyVc, StateRef};

pub struct InductiveEngine {
    pub identity: BackendIdentity,
}

impl InductiveEngine {
    /// Enumerates the complete valuation relation and tests whether the claim itself
    /// is an inductive invariant. This is not graph reachability: an unreachable
    /// bad inductive step returns unknown, never a false counterexample.
    pub fn check(&self, vc: &FiniteSafetyVc) -> RawEngineResult {
        let binding = EngineBinding::for_vc(vc).unwrap_or_else(|_| EngineBinding {
            vc_digest: crate::identity::Sha256Id::digest(b"invalid-vc"),
            model_id: vc.identity.model.clone(),
            claim_id: vc.identity.claim.clone(),
            configuration_id: vc.identity.configuration.clone(),
        });
        if let Err(error) = vc.validate() {
            return self.raw(
                binding,
                ResultScope::CompleteFinite { states: 0 },
                RawStatus::Unknown {
                    reason: format!("invalid VC: {error}"),
                },
                None,
            );
        }

        let count = vc.state_count();
        for state in 0..count {
            if induction_eval(&vc.initial, state, 0) && !induction_eval(&vc.property, state, 0) {
                return self.raw(
                    binding,
                    ResultScope::CompleteFinite { states: count },
                    RawStatus::Refuted {
                        witness_states: vec![state as u64],
                    },
                    None,
                );
            }
        }

        let invariant_states: Vec<u64> = (0..count)
            .filter(|&state| induction_eval(&vc.property, state, 0))
            .map(|state| state as u64)
            .collect();
        for state in 0..count {
            if !induction_eval(&vc.property, state, 0) {
                continue;
            }
            for next in 0..count {
                if induction_eval(&vc.transition, state, next)
                    && !induction_eval(&vc.property, next, 0)
                {
                    return self.raw(
                        binding,
                        ResultScope::CompleteFinite { states: count },
                        RawStatus::Unknown {
                            reason: format!(
                                "property is not inductive: valuation {state} permits bad successor {next}; reachability was not assumed"
                            ),
                        },
                        None,
                    );
                }
            }
        }

        let certificate = FiniteInvariantCertificate {
            format: FINITE_INVARIANT_CERTIFICATE_VERSION.to_owned(),
            vc_digest: binding.vc_digest.clone(),
            model_id: binding.model_id.clone(),
            claim_id: binding.claim_id.clone(),
            configuration_id: binding.configuration_id.clone(),
            invariant_states,
        };
        self.raw(
            binding,
            ResultScope::CompleteFinite { states: count },
            RawStatus::Holds {
                requested_class: NormalizedClass::Proved,
            },
            Some(certificate),
        )
    }

    fn raw(
        &self,
        binding: EngineBinding,
        scope: ResultScope,
        status: RawStatus,
        certificate: Option<FiniteInvariantCertificate>,
    ) -> RawEngineResult {
        RawEngineResult {
            engine: self.identity.clone(),
            binding,
            method: "finite-inductiveness-enumeration/1".to_owned(),
            scope,
            status,
            certificate,
            raw_output: Vec::new(),
        }
    }
}

fn induction_eval(expression: &BoolExpr, current: usize, next: usize) -> bool {
    match expression {
        BoolExpr::Const(value) => *value,
        BoolExpr::Var(StateRef::Current(index)) => current & (1_usize << index) != 0,
        BoolExpr::Var(StateRef::Next(index)) => next & (1_usize << index) != 0,
        BoolExpr::Not(inner) => !induction_eval(inner, current, next),
        BoolExpr::And(items) => {
            for item in items {
                if !induction_eval(item, current, next) {
                    return false;
                }
            }
            true
        }
        BoolExpr::Or(items) => {
            for item in items {
                if induction_eval(item, current, next) {
                    return true;
                }
            }
            false
        }
        BoolExpr::Implies(left, right) => {
            if induction_eval(left, current, next) {
                induction_eval(right, current, next)
            } else {
                true
            }
        }
        BoolExpr::Iff(left, right) => {
            induction_eval(left, current, next) == induction_eval(right, current, next)
        }
    }
}

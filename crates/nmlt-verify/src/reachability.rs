use crate::certificate::{FINITE_INVARIANT_CERTIFICATE_VERSION, FiniteInvariantCertificate};
use crate::evidence::{
    BackendIdentity, EngineBinding, NormalizedClass, RawEngineResult, RawStatus, ResultScope,
};
use crate::ir::{BoolExpr, FiniteSafetyVc, StateRef};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReachabilityLimits {
    pub max_states: usize,
    pub max_depth: usize,
}

pub struct ReachabilityEngine {
    pub identity: BackendIdentity,
    pub limits: ReachabilityLimits,
}

impl ReachabilityEngine {
    /// Deterministic breadth-first exploration. This implementation intentionally has
    /// its own evaluator and result construction rather than sharing the inductive
    /// engine's valuation loop or holds logic.
    pub fn check(&self, vc: &FiniteSafetyVc) -> RawEngineResult {
        let binding = EngineBinding::for_vc(vc).unwrap_or_else(|_| placeholder_binding(vc));
        let bounded_scope = ResultScope::Bounded {
            max_depth: self.limits.max_depth,
            max_states: self.limits.max_states,
        };
        if let Err(error) = vc.validate() {
            return result(
                self.identity.clone(),
                binding,
                bounded_scope,
                RawStatus::Unknown {
                    reason: format!("invalid VC: {error}"),
                },
            );
        }

        let count = vc.state_count();
        let mut queue = VecDeque::new();
        let mut seen = BTreeSet::new();
        let mut depth = BTreeMap::new();
        let mut predecessor = BTreeMap::new();
        for state in 0..count {
            if reach_eval(&vc.initial, state, 0) {
                seen.insert(state);
                depth.insert(state, 0_usize);
                queue.push_back(state);
            }
        }
        if seen.len() > self.limits.max_states {
            return result(
                self.identity.clone(),
                binding,
                bounded_scope,
                RawStatus::Unknown {
                    reason: "initial states exceed the state budget".to_owned(),
                },
            );
        }

        while let Some(state) = queue.pop_front() {
            if !reach_eval(&vc.property, state, 0) {
                let witness_states = trace_to(state, &predecessor)
                    .into_iter()
                    .map(|state| state as u64)
                    .collect();
                return result(
                    self.identity.clone(),
                    binding,
                    bounded_scope,
                    RawStatus::Refuted { witness_states },
                );
            }
            let state_depth = depth[&state];
            for next in 0..count {
                if !reach_eval(&vc.transition, state, next) || seen.contains(&next) {
                    continue;
                }
                if state_depth >= self.limits.max_depth {
                    return result(
                        self.identity.clone(),
                        binding,
                        bounded_scope,
                        RawStatus::Unknown {
                            reason: format!(
                                "reachable successor exists beyond depth {}",
                                self.limits.max_depth
                            ),
                        },
                    );
                }
                if seen.len() >= self.limits.max_states {
                    return result(
                        self.identity.clone(),
                        binding,
                        bounded_scope,
                        RawStatus::Unknown {
                            reason: format!(
                                "reachable successor exceeds the {}-state budget",
                                self.limits.max_states
                            ),
                        },
                    );
                }
                seen.insert(next);
                depth.insert(next, state_depth + 1);
                predecessor.insert(next, state);
                queue.push_back(next);
            }
        }

        let certificate = FiniteInvariantCertificate {
            format: FINITE_INVARIANT_CERTIFICATE_VERSION.to_owned(),
            vc_digest: binding.vc_digest.clone(),
            model_id: binding.model_id.clone(),
            claim_id: binding.claim_id.clone(),
            configuration_id: binding.configuration_id.clone(),
            invariant_states: seen.iter().map(|&state| state as u64).collect(),
        };
        let mut checked = result(
            self.identity.clone(),
            binding,
            ResultScope::CompleteFinite { states: seen.len() },
            RawStatus::Holds {
                requested_class: NormalizedClass::ModelChecked,
            },
        );
        checked.certificate = Some(certificate);
        checked
    }
}

fn result(
    engine: BackendIdentity,
    binding: EngineBinding,
    scope: ResultScope,
    status: RawStatus,
) -> RawEngineResult {
    RawEngineResult {
        engine,
        binding,
        method: "deterministic-bfs/1".to_owned(),
        scope,
        status,
        certificate: None,
        raw_output: Vec::new(),
    }
}

fn placeholder_binding(vc: &FiniteSafetyVc) -> EngineBinding {
    EngineBinding {
        vc_digest: crate::identity::Sha256Id::digest(b"invalid-vc"),
        model_id: vc.identity.model.clone(),
        claim_id: vc.identity.claim.clone(),
        configuration_id: vc.identity.configuration.clone(),
    }
}

fn trace_to(mut state: usize, predecessor: &BTreeMap<usize, usize>) -> Vec<usize> {
    let mut trace = vec![state];
    while let Some(&previous) = predecessor.get(&state) {
        trace.push(previous);
        state = previous;
    }
    trace.reverse();
    trace
}

fn reach_eval(expression: &BoolExpr, current: usize, next: usize) -> bool {
    match expression {
        BoolExpr::Const(value) => *value,
        BoolExpr::Var(StateRef::Current(index)) => (current >> index) & 1 == 1,
        BoolExpr::Var(StateRef::Next(index)) => (next >> index) & 1 == 1,
        BoolExpr::Not(item) => !reach_eval(item, current, next),
        BoolExpr::And(items) => items.iter().all(|item| reach_eval(item, current, next)),
        BoolExpr::Or(items) => items.iter().any(|item| reach_eval(item, current, next)),
        BoolExpr::Implies(left, right) => {
            !reach_eval(left, current, next) || reach_eval(right, current, next)
        }
        BoolExpr::Iff(left, right) => {
            reach_eval(left, current, next) == reach_eval(right, current, next)
        }
    }
}

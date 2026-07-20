//! Label-aware finite open refinement for M11-001b.
//!
//! The relation combines the crate's finite forward simulation with a total,
//! injective visible-boundary renaming. Boundary labels preserve polarity,
//! channel, and exact nominal payload identity. Input assumptions are
//! contravariant and output guarantees are covariant under finite-set
//! inclusion. This is not payload subtyping and makes no compositionality,
//! liveness, fairness, or circular-contract claim.

use std::collections::BTreeMap;

use crate::observation::{ActionHiding, ObservationMap};
use crate::open::{ActionPolarity, OpenSystem};
use crate::refinement::{RefinementChecker, RefinementReport, RefinementSpec};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenRefinementIssue {
    BehaviorRejected,
    UnknownConcreteAction(String),
    ConcreteActionUnmapped(String),
    BoundaryActionHidden(String),
    MappedAbstractActionMissing {
        concrete_action: String,
        abstract_action: String,
    },
    AbstractBoundaryActionUncovered(String),
    NonInjectiveBoundaryMapping {
        first_concrete_action: String,
        second_concrete_action: String,
        abstract_action: String,
    },
    PolarityNotPreserved {
        concrete_action: String,
        abstract_action: String,
    },
    ChannelNotPreserved {
        concrete_action: String,
        abstract_action: String,
    },
    PayloadTypeNotPreserved {
        concrete_action: String,
        abstract_action: String,
    },
    AssumptionNotContravariant {
        concrete_action: String,
        abstract_action: String,
        rejected_value: String,
    },
    GuaranteeNotCovariant {
        concrete_action: String,
        abstract_action: String,
        rejected_value: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenRefinementReport {
    pub accepted: bool,
    pub behavior: RefinementReport,
    pub issues: Vec<OpenRefinementIssue>,
}

pub struct OpenRefinementChecker;

impl OpenRefinementChecker {
    /// Checks one complete finite open-refinement instance.
    #[must_use]
    pub fn check(
        concrete: &OpenSystem,
        abstract_system: &OpenSystem,
        spec: &RefinementSpec,
    ) -> OpenRefinementReport {
        let behavior = RefinementChecker::check(concrete.graph(), abstract_system.graph(), spec);
        let mut issues = Vec::new();
        if !behavior.accepted {
            issues.push(OpenRefinementIssue::BehaviorRejected);
        }

        for mapped_action in spec.actions.mappings().keys() {
            if concrete.interface().get(mapped_action).is_none() {
                issues.push(OpenRefinementIssue::UnknownConcreteAction(
                    mapped_action.clone(),
                ));
            }
        }

        let mut boundary_images = BTreeMap::<String, String>::new();
        for (concrete_action, concrete_signature) in concrete.interface().actions() {
            let Some(mapping) = spec.actions.get(concrete_action) else {
                issues.push(OpenRefinementIssue::ConcreteActionUnmapped(
                    concrete_action.clone(),
                ));
                continue;
            };
            let Some(abstract_action) = mapping else {
                if concrete_signature.polarity != ActionPolarity::Internal {
                    issues.push(OpenRefinementIssue::BoundaryActionHidden(
                        concrete_action.clone(),
                    ));
                }
                continue;
            };
            let Some(abstract_signature) = abstract_system.interface().get(abstract_action) else {
                issues.push(OpenRefinementIssue::MappedAbstractActionMissing {
                    concrete_action: concrete_action.clone(),
                    abstract_action: abstract_action.to_owned(),
                });
                continue;
            };

            if concrete_signature.polarity != ActionPolarity::Internal {
                if let Some(first) =
                    boundary_images.insert(abstract_action.to_owned(), concrete_action.clone())
                {
                    issues.push(OpenRefinementIssue::NonInjectiveBoundaryMapping {
                        first_concrete_action: first,
                        second_concrete_action: concrete_action.clone(),
                        abstract_action: abstract_action.to_owned(),
                    });
                }
            }
            if concrete_signature.polarity != abstract_signature.polarity {
                issues.push(OpenRefinementIssue::PolarityNotPreserved {
                    concrete_action: concrete_action.clone(),
                    abstract_action: abstract_action.to_owned(),
                });
                continue;
            }
            if concrete_signature.channel != abstract_signature.channel {
                issues.push(OpenRefinementIssue::ChannelNotPreserved {
                    concrete_action: concrete_action.clone(),
                    abstract_action: abstract_action.to_owned(),
                });
            }
            if concrete_signature.payload_type.as_ref().map(|ty| ty.id())
                != abstract_signature.payload_type.as_ref().map(|ty| ty.id())
            {
                issues.push(OpenRefinementIssue::PayloadTypeNotPreserved {
                    concrete_action: concrete_action.clone(),
                    abstract_action: abstract_action.to_owned(),
                });
                continue;
            }

            match concrete_signature.polarity {
                ActionPolarity::Input => {
                    let concrete_predicate = concrete
                        .contract()
                        .assumptions()
                        .get(concrete_action)
                        .expect("OpenSystem validates total input assumptions");
                    let abstract_predicate = abstract_system
                        .contract()
                        .assumptions()
                        .get(abstract_action)
                        .expect("OpenSystem validates total input assumptions");
                    if !abstract_predicate.is_subset_of(concrete_predicate) {
                        let rejected_value = abstract_predicate
                            .accepted()
                            .difference(concrete_predicate.accepted())
                            .next()
                            .expect("failed same-type inclusion has a witness")
                            .clone();
                        issues.push(OpenRefinementIssue::AssumptionNotContravariant {
                            concrete_action: concrete_action.clone(),
                            abstract_action: abstract_action.to_owned(),
                            rejected_value,
                        });
                    }
                }
                ActionPolarity::Output => {
                    let concrete_predicate = concrete
                        .contract()
                        .guarantees()
                        .get(concrete_action)
                        .expect("OpenSystem validates total output guarantees");
                    let abstract_predicate = abstract_system
                        .contract()
                        .guarantees()
                        .get(abstract_action)
                        .expect("OpenSystem validates total output guarantees");
                    if !concrete_predicate.is_subset_of(abstract_predicate) {
                        let rejected_value = concrete_predicate
                            .accepted()
                            .difference(abstract_predicate.accepted())
                            .next()
                            .expect("failed same-type inclusion has a witness")
                            .clone();
                        issues.push(OpenRefinementIssue::GuaranteeNotCovariant {
                            concrete_action: concrete_action.clone(),
                            abstract_action: abstract_action.to_owned(),
                            rejected_value,
                        });
                    }
                }
                ActionPolarity::Internal => {}
            }
        }

        for (abstract_action, signature) in abstract_system.interface().actions() {
            if signature.polarity != ActionPolarity::Internal
                && !boundary_images.contains_key(abstract_action)
            {
                issues.push(OpenRefinementIssue::AbstractBoundaryActionUncovered(
                    abstract_action.clone(),
                ));
            }
        }

        OpenRefinementReport {
            accepted: issues.is_empty(),
            behavior,
            issues,
        }
    }
}

/// Builds the canonical identity witness for an open system.
#[must_use]
pub fn identity_refinement_spec(
    system: &OpenSystem,
    observation: ObservationMap,
) -> RefinementSpec {
    RefinementSpec {
        state_map: (0..system.graph().states().len()).collect(),
        concrete_observation: observation.clone(),
        abstract_observation: observation,
        actions: ActionHiding::new(
            system
                .interface()
                .actions()
                .keys()
                .map(|action| (action.clone(), Some(action.clone()))),
        ),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefinementCompositionError {
    ObservationBoundaryMismatch,
    IntermediateStateOutOfRange {
        concrete_state: usize,
        intermediate_state: usize,
    },
    IntermediateActionUnmapped {
        concrete_action: String,
        intermediate_action: String,
    },
}

/// Composes two finite refinement witnesses without rechecking either premise.
pub fn compose_refinement_specs(
    concrete_to_middle: &RefinementSpec,
    middle_to_abstract: &RefinementSpec,
) -> Result<RefinementSpec, RefinementCompositionError> {
    if concrete_to_middle.abstract_observation != middle_to_abstract.concrete_observation {
        return Err(RefinementCompositionError::ObservationBoundaryMismatch);
    }
    let mut state_map = Vec::with_capacity(concrete_to_middle.state_map.len());
    for (concrete_state, &middle_state) in concrete_to_middle.state_map.iter().enumerate() {
        let Some(&abstract_state) = middle_to_abstract.state_map.get(middle_state) else {
            return Err(RefinementCompositionError::IntermediateStateOutOfRange {
                concrete_state,
                intermediate_state: middle_state,
            });
        };
        state_map.push(abstract_state);
    }

    let mut actions = Vec::with_capacity(concrete_to_middle.actions.mappings().len());
    for (concrete_action, middle_action) in concrete_to_middle.actions.mappings() {
        let abstract_action = match middle_action {
            None => None,
            Some(middle_action) => {
                let Some(mapped) = middle_to_abstract.actions.get(middle_action) else {
                    return Err(RefinementCompositionError::IntermediateActionUnmapped {
                        concrete_action: concrete_action.clone(),
                        intermediate_action: middle_action.clone(),
                    });
                };
                mapped.map(str::to_owned)
            }
        };
        actions.push((concrete_action.clone(), abstract_action));
    }

    Ok(RefinementSpec {
        state_map,
        concrete_observation: concrete_to_middle.concrete_observation.clone(),
        abstract_observation: middle_to_abstract.abstract_observation.clone(),
        actions: ActionHiding::new(actions),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{FiniteGraph, ModelState, Transition, Value};
    use crate::open::{ActionSignature, Interface};
    use crate::open_contract::{FiniteContract, PayloadPredicate, PayloadType};

    fn state(value: bool) -> ModelState {
        BTreeMap::from([("visible".to_owned(), Value::Bool(value))])
    }

    fn output_system(
        action: &'static str,
        payload_type: PayloadType,
        accepted: &[&str],
    ) -> OpenSystem {
        let guarantee = PayloadPredicate::new(&payload_type, accepted.iter().copied()).unwrap();
        OpenSystem::new(
            FiniteGraph::new(
                vec![state(false), state(true)],
                vec![0],
                vec![Transition::action(0, action, 1)],
            )
            .unwrap(),
            Interface::new([(action, ActionSignature::output("bus", payload_type))]).unwrap(),
            FiniteContract::new(
                Vec::<(&str, PayloadPredicate)>::new(),
                [(action, guarantee)],
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn input_system(action: &'static str, accepted: &[&str]) -> OpenSystem {
        let payload_type = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        let assumption = PayloadPredicate::new(&payload_type, accepted.iter().copied()).unwrap();
        OpenSystem::new(
            FiniteGraph::new(
                vec![state(false)],
                vec![0],
                vec![Transition::action(0, action, 0)],
            )
            .unwrap(),
            Interface::new([(action, ActionSignature::input("bus", payload_type))]).unwrap(),
            FiniteContract::new(
                [(action, assumption)],
                Vec::<(&str, PayloadPredicate)>::new(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn multi_output_system(actions: &[&'static str]) -> OpenSystem {
        let payload_type = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        let declarations = actions
            .iter()
            .map(|action| {
                (
                    *action,
                    ActionSignature::output("bus", payload_type.clone()),
                )
            })
            .collect::<Vec<_>>();
        let guarantees = actions
            .iter()
            .map(|action| (*action, PayloadPredicate::all(&payload_type)))
            .collect::<Vec<_>>();
        OpenSystem::new(
            FiniteGraph::new(
                vec![state(false), state(true)],
                vec![0],
                vec![Transition::action(0, actions[0], 1)],
            )
            .unwrap(),
            Interface::new(declarations).unwrap(),
            FiniteContract::new(Vec::<(&str, PayloadPredicate)>::new(), guarantees).unwrap(),
        )
        .unwrap()
    }

    fn spec(concrete_action: &str, abstract_action: &str, states: Vec<usize>) -> RefinementSpec {
        RefinementSpec {
            state_map: states,
            concrete_observation: ObservationMap::identity(["visible"]),
            abstract_observation: ObservationMap::identity(["visible"]),
            actions: ActionHiding::new([(concrete_action, Some(abstract_action))]),
        }
    }

    #[test]
    fn identity_is_accepted() {
        let ty = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        let system = output_system("send", ty, &["ok"]);
        let report = OpenRefinementChecker::check(
            &system,
            &system,
            &identity_refinement_spec(&system, ObservationMap::identity(["visible"])),
        );
        assert!(report.accepted, "{:#?}", report.issues);
    }

    #[test]
    fn accepts_nonidentity_covariant_guarantee() {
        let ty = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        let concrete = output_system("send", ty.clone(), &["ok"]);
        let abstract_system = output_system("commit", ty, &["ok", "error"]);
        let report = OpenRefinementChecker::check(
            &concrete,
            &abstract_system,
            &spec("send", "commit", vec![0, 1]),
        );
        assert!(report.accepted, "{:#?}", report.issues);
    }

    #[test]
    fn rejects_assumption_strengthening() {
        let concrete = input_system("receive", &["ok"]);
        let abstract_system = input_system("accept", &["ok", "error"]);
        let report = OpenRefinementChecker::check(
            &concrete,
            &abstract_system,
            &spec("receive", "accept", vec![0]),
        );
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            OpenRefinementIssue::AssumptionNotContravariant {
                rejected_value, ..
            } if rejected_value == "error"
        )));
    }

    #[test]
    fn rejects_guarantee_weakening() {
        let ty = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        let concrete = output_system("send", ty.clone(), &["ok", "error"]);
        let abstract_system = output_system("commit", ty, &["ok"]);
        let report = OpenRefinementChecker::check(
            &concrete,
            &abstract_system,
            &spec("send", "commit", vec![0, 1]),
        );
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            OpenRefinementIssue::GuaranteeNotCovariant {
                rejected_value, ..
            } if rejected_value == "error"
        )));
    }

    #[test]
    fn rejects_payload_substitution_even_with_same_variants() {
        let concrete_type = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        let abstract_type = PayloadType::enumeration("OtherMessage", ["ok", "error"]).unwrap();
        let concrete = output_system("send", concrete_type, &["ok"]);
        let abstract_system = output_system("commit", abstract_type, &["ok"]);
        let report = OpenRefinementChecker::check(
            &concrete,
            &abstract_system,
            &spec("send", "commit", vec![0, 1]),
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| matches!(issue, OpenRefinementIssue::PayloadTypeNotPreserved { .. }))
        );
    }

    #[test]
    fn rejects_hidden_boundary_action() {
        let ty = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        let concrete = output_system("send", ty.clone(), &["ok"]);
        let abstract_system = output_system("commit", ty, &["ok"]);
        let mut refinement = spec("send", "commit", vec![0, 1]);
        refinement.actions = ActionHiding::new([("send", None::<&str>)]);
        let report = OpenRefinementChecker::check(&concrete, &abstract_system, &refinement);
        assert!(
            report
                .issues
                .contains(&OpenRefinementIssue::BoundaryActionHidden(
                    "send".to_owned()
                ))
        );
    }

    #[test]
    fn rejects_incomplete_abstract_boundary_mapping() {
        let concrete = multi_output_system(&["send"]);
        let abstract_system = multi_output_system(&["commit", "audit"]);
        let report = OpenRefinementChecker::check(
            &concrete,
            &abstract_system,
            &spec("send", "commit", vec![0, 1]),
        );
        assert!(
            report
                .issues
                .contains(&OpenRefinementIssue::AbstractBoundaryActionUncovered(
                    "audit".to_owned()
                ))
        );
    }

    #[test]
    fn rejects_noninjective_boundary_mapping() {
        let concrete = multi_output_system(&["send", "send-alias"]);
        let abstract_system = multi_output_system(&["commit"]);
        let mut refinement = spec("send", "commit", vec![0, 1]);
        refinement.actions =
            ActionHiding::new([("send", Some("commit")), ("send-alias", Some("commit"))]);
        let report = OpenRefinementChecker::check(&concrete, &abstract_system, &refinement);
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            OpenRefinementIssue::NonInjectiveBoundaryMapping { .. }
        )));
    }

    #[test]
    fn composes_nonidentity_refinement_witnesses() {
        let first = spec("send", "forward", vec![1, 0]);
        let second = spec("forward", "commit", vec![1, 0]);
        let composed = compose_refinement_specs(&first, &second).unwrap();
        assert_eq!(composed.state_map, vec![0, 1]);
        assert_eq!(composed.actions.get("send"), Some(Some("commit")));
    }
}

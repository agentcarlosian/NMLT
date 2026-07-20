//! Two-sided finite open-refinement congruence for M11-001c.
//!
//! This module composes two independently checked label-aware open refinements.
//! It requires the complete concrete wiring to map bijectively onto the complete
//! abstract wiring, checks both concrete and abstract compositions, constructs
//! the product state/action map, and reuses [`OpenRefinementChecker`] to check
//! behavior plus the remaining boundary contracts on the products. Optional
//! finite invariants are transported only after the lifted refinement passes.
//!
//! The result is deliberately safety-only. It also checks the finite M11-001c
//! capability partition, resource-grade, and rely/guarantee profile. Fairness,
//! liveness, and hidden divergence remain separate obligations.

use std::collections::BTreeMap;

use crate::graph::StateId;
use crate::observation::{ActionHiding, ObservationMap};
use crate::open::{
    CompatibilityChecker, CompatibilityReport, CompositionLimits, CompositionSpec, Connection,
    OpenSystem, compose_with_limits,
};
use crate::open_encoding::{EncodingCorrespondenceChecker, EncodingCorrespondenceReport};
use crate::open_refinement::{OpenRefinementChecker, OpenRefinementReport};
use crate::open_resources::{
    ResourceCompositionChecker, ResourceCompositionReport, ResourceRefinementChecker,
    ResourceRefinementReport, SystemResourceProfile, mapped_product_resource_refinement,
};
use crate::refinement::RefinementSpec;

const LEFT_NAMESPACE: &str = "left::";
const RIGHT_NAMESPACE: &str = "right::";
#[cfg(test)]
const M11_CONGRUENCE_VECTORS: &str =
    include_str!("../../../mechanization/vectors/m11-open-congruence-v1.json");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiniteStateInvariant {
    /// One truth value for each abstract product state, in canonical product order.
    pub holds: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvariantTransportReport {
    pub accepted: bool,
    pub checked_abstract_reachable_states: usize,
    pub checked_concrete_reachable_states: usize,
    pub violating_abstract_state: Option<StateId>,
    pub violating_concrete_state: Option<StateId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TwoSidedCongruenceSpec {
    pub left_refinement: RefinementSpec,
    pub right_refinement: RefinementSpec,
    pub concrete_composition: CompositionSpec,
    pub abstract_composition: CompositionSpec,
    pub resources: TwoSidedResourceSpec,
    /// When present, the checker establishes the finite reachable-state instance
    /// of contravariant invariant transport for the lifted product refinement.
    pub abstract_invariant: Option<FiniteStateInvariant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TwoSidedResourceSpec {
    pub concrete_left: SystemResourceProfile,
    pub abstract_left: SystemResourceProfile,
    pub concrete_right: SystemResourceProfile,
    pub abstract_right: SystemResourceProfile,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TwoSidedCongruenceIssue {
    LeftRefinementRejected,
    RightRefinementRejected,
    ConcreteCompositionIncompatible,
    AbstractCompositionIncompatible,
    LeftResourceRefinementRejected,
    RightResourceRefinementRejected,
    ConcreteResourceCompositionRejected,
    AbstractResourceCompositionRejected,
    EncodingCorrespondenceRejected,
    ConcreteConnectionNotPreserved(Connection),
    AbstractConnectionNotReflected(Connection),
    CompositionConstructionFailed,
    ProductStateMapOverflow,
    ProductActionUnmapped(String),
    LiftedRefinementRejected,
    LiftedResourceRefinementRejected,
    InvariantLengthMismatch { expected: usize, actual: usize },
    AbstractInvariantRejected(StateId),
    TransportedInvariantRejected(StateId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TwoSidedCongruenceReport {
    pub accepted: bool,
    pub left_refinement: OpenRefinementReport,
    pub right_refinement: OpenRefinementReport,
    pub concrete_compatibility: CompatibilityReport,
    pub abstract_compatibility: CompatibilityReport,
    pub left_resource_refinement: ResourceRefinementReport,
    pub right_resource_refinement: ResourceRefinementReport,
    pub concrete_resource_composition: ResourceCompositionReport,
    pub abstract_resource_composition: ResourceCompositionReport,
    pub encoding_correspondence: EncodingCorrespondenceReport,
    pub lifted_refinement: Option<OpenRefinementReport>,
    pub lifted_resource_refinement: Option<ResourceRefinementReport>,
    pub invariant_transport: Option<InvariantTransportReport>,
    pub issues: Vec<TwoSidedCongruenceIssue>,
}

pub struct TwoSidedCongruenceChecker;

impl TwoSidedCongruenceChecker {
    #[must_use]
    pub fn check(
        concrete_left: &OpenSystem,
        abstract_left: &OpenSystem,
        concrete_right: &OpenSystem,
        abstract_right: &OpenSystem,
        spec: &TwoSidedCongruenceSpec,
    ) -> TwoSidedCongruenceReport {
        Self::check_with_limits(
            concrete_left,
            abstract_left,
            concrete_right,
            abstract_right,
            spec,
            CompositionLimits::default(),
        )
    }

    #[must_use]
    pub fn check_with_limits(
        concrete_left: &OpenSystem,
        abstract_left: &OpenSystem,
        concrete_right: &OpenSystem,
        abstract_right: &OpenSystem,
        spec: &TwoSidedCongruenceSpec,
        limits: CompositionLimits,
    ) -> TwoSidedCongruenceReport {
        let left_refinement =
            OpenRefinementChecker::check(concrete_left, abstract_left, &spec.left_refinement);
        let right_refinement =
            OpenRefinementChecker::check(concrete_right, abstract_right, &spec.right_refinement);
        let concrete_compatibility = CompatibilityChecker::check_with_limits(
            concrete_left,
            concrete_right,
            &spec.concrete_composition,
            limits,
        );
        let abstract_compatibility = CompatibilityChecker::check_with_limits(
            abstract_left,
            abstract_right,
            &spec.abstract_composition,
            limits,
        );
        let left_resource_refinement = ResourceRefinementChecker::check(
            concrete_left,
            abstract_left,
            &spec.resources.concrete_left,
            &spec.resources.abstract_left,
            &spec.left_refinement,
        );
        let right_resource_refinement = ResourceRefinementChecker::check(
            concrete_right,
            abstract_right,
            &spec.resources.concrete_right,
            &spec.resources.abstract_right,
            &spec.right_refinement,
        );
        let concrete_resource_composition = ResourceCompositionChecker::check(
            concrete_left,
            concrete_right,
            &spec.resources.concrete_left,
            &spec.resources.concrete_right,
            &spec.concrete_composition,
        );
        let abstract_resource_composition = ResourceCompositionChecker::check(
            abstract_left,
            abstract_right,
            &spec.resources.abstract_left,
            &spec.resources.abstract_right,
            &spec.abstract_composition,
        );
        let encoding_correspondence = EncodingCorrespondenceChecker::check(
            concrete_left,
            abstract_left,
            concrete_right,
            abstract_right,
            &spec.left_refinement,
            &spec.right_refinement,
            &spec.concrete_composition,
            &spec.abstract_composition,
            &spec.resources.concrete_left,
            &spec.resources.abstract_left,
            &spec.resources.concrete_right,
            &spec.resources.abstract_right,
        );

        let mut issues = Vec::new();
        if !left_refinement.accepted {
            issues.push(TwoSidedCongruenceIssue::LeftRefinementRejected);
        }
        if !right_refinement.accepted {
            issues.push(TwoSidedCongruenceIssue::RightRefinementRejected);
        }
        if !concrete_compatibility.accepted {
            issues.push(TwoSidedCongruenceIssue::ConcreteCompositionIncompatible);
        }
        if !abstract_compatibility.accepted {
            issues.push(TwoSidedCongruenceIssue::AbstractCompositionIncompatible);
        }
        if !left_resource_refinement.accepted {
            issues.push(TwoSidedCongruenceIssue::LeftResourceRefinementRejected);
        }
        if !right_resource_refinement.accepted {
            issues.push(TwoSidedCongruenceIssue::RightResourceRefinementRejected);
        }
        if !concrete_resource_composition.accepted {
            issues.push(TwoSidedCongruenceIssue::ConcreteResourceCompositionRejected);
        }
        if !abstract_resource_composition.accepted {
            issues.push(TwoSidedCongruenceIssue::AbstractResourceCompositionRejected);
        }
        if !encoding_correspondence.accepted {
            issues.push(TwoSidedCongruenceIssue::EncodingCorrespondenceRejected);
        }

        let synchronization_map = check_wiring(spec, &mut issues);
        let mut lifted_refinement = None;
        let mut lifted_resource_refinement = None;
        let mut invariant_transport = None;

        if issues.is_empty() {
            let concrete_product = compose_with_limits(
                concrete_left,
                concrete_right,
                &spec.concrete_composition,
                limits,
            );
            let abstract_product = compose_with_limits(
                abstract_left,
                abstract_right,
                &spec.abstract_composition,
                limits,
            );
            match (concrete_product, abstract_product) {
                (Ok(concrete_product), Ok(abstract_product)) => {
                    match lift_product_refinement(
                        &concrete_product,
                        concrete_left,
                        abstract_left,
                        concrete_right,
                        abstract_right,
                        spec,
                        &synchronization_map,
                    ) {
                        Ok(lifted_spec) => {
                            let lifted = OpenRefinementChecker::check(
                                &concrete_product,
                                &abstract_product,
                                &lifted_spec,
                            );
                            if !lifted.accepted {
                                issues.push(TwoSidedCongruenceIssue::LiftedRefinementRejected);
                            }
                            let lifted_resources = mapped_product_resource_refinement(
                                concrete_resource_composition
                                    .product
                                    .as_ref()
                                    .expect("accepted resource composition has a product"),
                                abstract_resource_composition
                                    .product
                                    .as_ref()
                                    .expect("accepted resource composition has a product"),
                                &lifted_spec.actions,
                            );
                            if !lifted_resources.accepted {
                                issues.push(
                                    TwoSidedCongruenceIssue::LiftedResourceRefinementRejected,
                                );
                            }
                            lifted_resource_refinement = Some(lifted_resources);
                            if lifted.accepted
                                && lifted_resource_refinement
                                    .as_ref()
                                    .is_some_and(|report| report.accepted)
                                && let Some(invariant) = &spec.abstract_invariant
                            {
                                let transport = check_invariant_transport(
                                    &concrete_product,
                                    &abstract_product,
                                    &lifted_spec,
                                    invariant,
                                    &mut issues,
                                );
                                invariant_transport = Some(transport);
                            }
                            lifted_refinement = Some(lifted);
                        }
                        Err(issue) => issues.push(issue),
                    }
                }
                _ => issues.push(TwoSidedCongruenceIssue::CompositionConstructionFailed),
            }
        }

        TwoSidedCongruenceReport {
            accepted: issues.is_empty()
                && lifted_refinement
                    .as_ref()
                    .is_some_and(|report| report.accepted)
                && lifted_resource_refinement
                    .as_ref()
                    .is_some_and(|report| report.accepted)
                && invariant_transport
                    .as_ref()
                    .is_none_or(|report| report.accepted),
            left_refinement,
            right_refinement,
            concrete_compatibility,
            abstract_compatibility,
            left_resource_refinement,
            right_resource_refinement,
            concrete_resource_composition,
            abstract_resource_composition,
            encoding_correspondence,
            lifted_refinement,
            lifted_resource_refinement,
            invariant_transport,
            issues,
        }
    }
}

fn check_wiring(
    spec: &TwoSidedCongruenceSpec,
    issues: &mut Vec<TwoSidedCongruenceIssue>,
) -> BTreeMap<String, String> {
    let mut synchronization_map = BTreeMap::new();
    let mut matched_abstract = vec![false; spec.abstract_composition.connections.len()];
    for concrete in &spec.concrete_composition.connections {
        let mapped_left = mapped_visible(&spec.left_refinement, &concrete.left_action);
        let mapped_right = mapped_visible(&spec.right_refinement, &concrete.right_action);
        let Some((abstract_index, abstract_connection)) = spec
            .abstract_composition
            .connections
            .iter()
            .enumerate()
            .find(|(index, edge)| {
                !matched_abstract[*index]
                    && mapped_left == Some(edge.left_action.as_str())
                    && mapped_right == Some(edge.right_action.as_str())
            })
        else {
            issues.push(TwoSidedCongruenceIssue::ConcreteConnectionNotPreserved(
                concrete.clone(),
            ));
            continue;
        };
        matched_abstract[abstract_index] = true;
        synchronization_map.insert(
            concrete.composite_action.clone(),
            abstract_connection.composite_action.clone(),
        );
    }
    for (index, abstract_connection) in spec.abstract_composition.connections.iter().enumerate() {
        if !matched_abstract[index] {
            issues.push(TwoSidedCongruenceIssue::AbstractConnectionNotReflected(
                abstract_connection.clone(),
            ));
        }
    }
    synchronization_map
}

fn mapped_visible<'a>(spec: &'a RefinementSpec, action: &str) -> Option<&'a str> {
    spec.actions.get(action).flatten()
}

fn lift_product_refinement(
    concrete_product: &OpenSystem,
    concrete_left: &OpenSystem,
    _abstract_left: &OpenSystem,
    concrete_right: &OpenSystem,
    abstract_right: &OpenSystem,
    spec: &TwoSidedCongruenceSpec,
    synchronization_map: &BTreeMap<String, String>,
) -> Result<RefinementSpec, TwoSidedCongruenceIssue> {
    let concrete_right_count = concrete_right.graph().states().len();
    let abstract_right_count = abstract_right.graph().states().len();
    let mut state_map = Vec::with_capacity(concrete_product.graph().states().len());
    for concrete_left_state in 0..concrete_left.graph().states().len() {
        for concrete_right_state in 0..concrete_right_count {
            let abstract_left_state = spec.left_refinement.state_map[concrete_left_state];
            let abstract_right_state = spec.right_refinement.state_map[concrete_right_state];
            let mapped = abstract_left_state
                .checked_mul(abstract_right_count)
                .and_then(|base| base.checked_add(abstract_right_state))
                .ok_or(TwoSidedCongruenceIssue::ProductStateMapOverflow)?;
            state_map.push(mapped);
        }
    }

    let actions = concrete_product
        .interface()
        .actions()
        .keys()
        .map(|action| {
            let mapped = if let Some(local) = action.strip_prefix(LEFT_NAMESPACE) {
                spec.left_refinement
                    .actions
                    .get(local)
                    .map(|target| target.map(|name| format!("{LEFT_NAMESPACE}{name}")))
            } else if let Some(local) = action.strip_prefix(RIGHT_NAMESPACE) {
                spec.right_refinement
                    .actions
                    .get(local)
                    .map(|target| target.map(|name| format!("{RIGHT_NAMESPACE}{name}")))
            } else {
                synchronization_map
                    .get(action)
                    .map(|target| Some(target.clone()))
            };
            mapped
                .map(|target| (action.clone(), target))
                .ok_or_else(|| TwoSidedCongruenceIssue::ProductActionUnmapped(action.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(RefinementSpec {
        state_map,
        concrete_observation: product_observation(
            &spec.left_refinement.concrete_observation,
            &spec.right_refinement.concrete_observation,
        ),
        abstract_observation: product_observation(
            &spec.left_refinement.abstract_observation,
            &spec.right_refinement.abstract_observation,
        ),
        actions: ActionHiding::new(actions),
    })
}

fn product_observation(left: &ObservationMap, right: &ObservationMap) -> ObservationMap {
    ObservationMap::new(
        left.fields()
            .iter()
            .map(|(source, output)| {
                (
                    format!("{LEFT_NAMESPACE}{source}"),
                    format!("{LEFT_NAMESPACE}{output}"),
                )
            })
            .chain(right.fields().iter().map(|(source, output)| {
                (
                    format!("{RIGHT_NAMESPACE}{source}"),
                    format!("{RIGHT_NAMESPACE}{output}"),
                )
            })),
    )
    .expect("component observation namespaces are disjoint")
}

fn check_invariant_transport(
    concrete_product: &OpenSystem,
    abstract_product: &OpenSystem,
    lifted: &RefinementSpec,
    invariant: &FiniteStateInvariant,
    issues: &mut Vec<TwoSidedCongruenceIssue>,
) -> InvariantTransportReport {
    if invariant.holds.len() != abstract_product.graph().states().len() {
        issues.push(TwoSidedCongruenceIssue::InvariantLengthMismatch {
            expected: abstract_product.graph().states().len(),
            actual: invariant.holds.len(),
        });
        return InvariantTransportReport {
            accepted: false,
            checked_abstract_reachable_states: 0,
            checked_concrete_reachable_states: 0,
            violating_abstract_state: None,
            violating_concrete_state: None,
        };
    }

    let abstract_reachable = abstract_product.graph().reachable_states();
    let violating_abstract_state = abstract_reachable
        .iter()
        .copied()
        .find(|&state| !invariant.holds[state]);
    if let Some(state) = violating_abstract_state {
        issues.push(TwoSidedCongruenceIssue::AbstractInvariantRejected(state));
    }
    let concrete_reachable = concrete_product.graph().reachable_states();
    let violating_concrete_state = concrete_reachable
        .iter()
        .copied()
        .find(|&state| !invariant.holds[lifted.state_map[state]]);
    if let Some(state) = violating_concrete_state {
        issues.push(TwoSidedCongruenceIssue::TransportedInvariantRejected(state));
    }

    InvariantTransportReport {
        accepted: violating_abstract_state.is_none() && violating_concrete_state.is_none(),
        checked_abstract_reachable_states: abstract_reachable.len(),
        checked_concrete_reachable_states: concrete_reachable.len(),
        violating_abstract_state,
        violating_concrete_state,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::graph::{FiniteGraph, ModelState, Transition, Value};
    use crate::open::{ActionSignature, ContractLink, Interface, Side};
    use crate::open_contract::{FiniteContract, PayloadPredicate, PayloadType};
    use crate::open_resources::{ActionResourceProfile, SystemResourceProfile};
    use nmlt_grades::{Grade, UncertaintyCertificate};

    fn state(field: &str, value: bool) -> ModelState {
        BTreeMap::from([(field.to_owned(), Value::Bool(value))])
    }

    fn output_system(
        action: &str,
        exposed_action: &str,
        accepted: &[&str],
        field: &str,
    ) -> OpenSystem {
        output_system_with_exposed_payload(action, exposed_action, accepted, field, "Message")
    }

    fn output_system_with_exposed_payload(
        action: &str,
        exposed_action: &str,
        accepted: &[&str],
        field: &str,
        exposed_payload_name: &str,
    ) -> OpenSystem {
        let payload = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        let exposed_payload =
            PayloadType::enumeration(exposed_payload_name, ["ok", "error"]).unwrap();
        OpenSystem::new(
            FiniteGraph::new(
                vec![state(field, false), state(field, true)],
                vec![0],
                vec![
                    Transition::action(0, action, 1),
                    Transition::action(0, exposed_action, 1),
                ],
            )
            .unwrap(),
            Interface::new([
                (action, ActionSignature::output("bus", payload.clone())),
                (
                    exposed_action,
                    ActionSignature::output("audit-bus", exposed_payload.clone()),
                ),
            ])
            .unwrap(),
            FiniteContract::new(
                Vec::<(&str, PayloadPredicate)>::new(),
                [
                    (
                        action,
                        PayloadPredicate::new(&payload, accepted.iter().copied()).unwrap(),
                    ),
                    (exposed_action, PayloadPredicate::all(&exposed_payload)),
                ],
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn input_system(action: &str, accepted: &[&str], field: &str) -> OpenSystem {
        let payload = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        OpenSystem::new(
            FiniteGraph::new(
                vec![state(field, false), state(field, true)],
                vec![0],
                vec![
                    Transition::action(0, action, 0),
                    Transition::action(1, action, 1),
                ],
            )
            .unwrap(),
            Interface::new([(action, ActionSignature::input("bus", payload.clone()))]).unwrap(),
            FiniteContract::new(
                [(
                    action,
                    PayloadPredicate::new(&payload, accepted.iter().copied()).unwrap(),
                )],
                Vec::<(&str, PayloadPredicate)>::new(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn refinement(
        concrete_action: &str,
        abstract_action: &str,
        exposed_mapping: Option<(&str, &str)>,
        field: &str,
    ) -> RefinementSpec {
        let mut actions = vec![(concrete_action, Some(abstract_action))];
        if let Some((concrete_exposed, abstract_exposed)) = exposed_mapping {
            actions.push((concrete_exposed, Some(abstract_exposed)));
        }
        RefinementSpec {
            state_map: vec![0, 1],
            concrete_observation: ObservationMap::identity([field]),
            abstract_observation: ObservationMap::identity([field]),
            actions: ActionHiding::new(actions),
        }
    }

    fn composition(left: &str, right: &str, sync: &str) -> CompositionSpec {
        CompositionSpec::new(
            vec![Connection::new(left, right, sync)],
            vec![ContractLink::new(Side::Right, right, Side::Left, left)],
        )
    }

    fn grade(cost: u64) -> Grade {
        Grade::checked(cost, 0, 0, UncertaintyCertificate::Certain).unwrap()
    }

    fn resource_side(
        output_action: &str,
        exposed_action: &str,
        output_cost: u64,
        exposed_cost: u64,
        guarantees_authorized: bool,
    ) -> SystemResourceProfile {
        let guarantees = guarantees_authorized.then_some("authorized");
        SystemResourceProfile::new(
            ["dispatch-cap"],
            [
                (
                    output_action,
                    ActionResourceProfile::new(
                        ["dispatch-cap"],
                        Vec::<&str>::new(),
                        ["dispatch-cap"],
                        Vec::<&str>::new(),
                        grade(output_cost),
                        Vec::<&str>::new(),
                        guarantees,
                    )
                    .unwrap(),
                ),
                (
                    exposed_action,
                    ActionResourceProfile::new(
                        Vec::<&str>::new(),
                        Vec::<&str>::new(),
                        Vec::<&str>::new(),
                        Vec::<&str>::new(),
                        grade(exposed_cost),
                        Vec::<&str>::new(),
                        ["auditable"],
                    )
                    .unwrap(),
                ),
            ],
        )
        .unwrap()
    }

    fn resource_input(action: &str, cost: u64) -> SystemResourceProfile {
        SystemResourceProfile::new(
            Vec::<&str>::new(),
            [(
                action,
                ActionResourceProfile::new(
                    Vec::<&str>::new(),
                    Vec::<&str>::new(),
                    Vec::<&str>::new(),
                    ["dispatch-cap"],
                    grade(cost),
                    ["authorized"],
                    Vec::<&str>::new(),
                )
                .unwrap(),
            )],
        )
        .unwrap()
    }

    fn resource_input_with_shared_owner(action: &str, cost: u64) -> SystemResourceProfile {
        SystemResourceProfile::new(
            ["dispatch-cap"],
            [(
                action,
                ActionResourceProfile::new(
                    Vec::<&str>::new(),
                    Vec::<&str>::new(),
                    Vec::<&str>::new(),
                    ["dispatch-cap"],
                    grade(cost),
                    ["authorized"],
                    Vec::<&str>::new(),
                )
                .unwrap(),
            )],
        )
        .unwrap()
    }

    fn resource_spec(
        concrete_output_cost: u64,
        abstract_output_cost: u64,
        guarantees_authorized: bool,
    ) -> TwoSidedResourceSpec {
        TwoSidedResourceSpec {
            concrete_left: resource_side(
                "send",
                "audit",
                concrete_output_cost,
                1,
                guarantees_authorized,
            ),
            abstract_left: resource_side(
                "commit",
                "record",
                abstract_output_cost,
                2,
                guarantees_authorized,
            ),
            concrete_right: resource_input("receive", 1),
            abstract_right: resource_input("accept", 2),
        }
    }

    fn fixture() -> (
        OpenSystem,
        OpenSystem,
        OpenSystem,
        OpenSystem,
        TwoSidedCongruenceSpec,
    ) {
        let concrete_left = output_system("send", "audit", &["ok"], "sent");
        let abstract_left = output_system("commit", "record", &["ok"], "sent");
        let concrete_right = input_system("receive", &["ok", "error"], "received");
        let abstract_right = input_system("accept", &["ok"], "received");
        let spec = TwoSidedCongruenceSpec {
            left_refinement: refinement("send", "commit", Some(("audit", "record")), "sent"),
            right_refinement: refinement("receive", "accept", None, "received"),
            concrete_composition: composition("send", "receive", "sync-send"),
            abstract_composition: composition("commit", "accept", "sync-commit"),
            resources: resource_spec(2, 3, true),
            abstract_invariant: Some(FiniteStateInvariant {
                holds: vec![true, true, true, true],
            }),
        };
        (
            concrete_left,
            abstract_left,
            concrete_right,
            abstract_right,
            spec,
        )
    }

    #[test]
    fn accepts_two_sided_contract_sound_congruence_and_invariant_transport() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, spec) = fixture();
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        assert!(report.accepted, "{:#?}", report.issues);
        let encoding = report.encoding_correspondence.encoding.as_ref().unwrap();
        assert_eq!(encoding.payload_variants, ["error", "ok"]);
        assert_eq!(encoding.left_refinement.action_map, [1, 0]);
        assert!(report.lifted_refinement.unwrap().accepted);
        assert!(report.invariant_transport.unwrap().accepted);
        assert!(report.lifted_resource_refinement.unwrap().accepted);
        let concrete_product = compose_with_limits(
            &concrete_left,
            &concrete_right,
            &spec.concrete_composition,
            CompositionLimits::default(),
        )
        .unwrap();
        assert!(
            concrete_product
                .contract()
                .guarantees()
                .contains_key("left::audit")
        );
    }

    #[test]
    fn canonical_certificate_validator_fails_closed_after_mutation() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, spec) = fixture();
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        let mut encoding = report.encoding_correspondence.encoding.unwrap();
        encoding.left_refinement.action_map[1] = encoding.left_refinement.action_map[0];
        let validation = crate::open_encoding::CanonicalEncodingValidator::check(&encoding);
        assert!(!validation.accepted);
        assert!(validation.issues.iter().any(|issue| matches!(
            issue,
            crate::open_encoding::CanonicalValidationIssue::ActionMapNotInjective {
                side: crate::open_encoding::CanonicalSide::Left,
                ..
            }
        )));
    }

    #[test]
    fn kernel_readback_rejects_dictionary_substitution() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, spec) = fixture();
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        let encoding = report.encoding_correspondence.encoding.unwrap();
        let mut certificate = crate::open_encoding::encode_execution_kernel(&encoding).unwrap();
        certificate.atom_dictionary.swap(0, 1);
        assert!(!crate::open_encoding::execution_kernel_readback_matches(
            &encoding,
            &certificate
        ));
    }

    #[test]
    fn kernel_readback_rejects_numeric_atom_substitution() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, spec) = fixture();
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        let encoding = report.encoding_correspondence.encoding.unwrap();
        let mut certificate = crate::open_encoding::encode_execution_kernel(&encoding).unwrap();
        certificate.input.concrete_left.owned.values[0] =
            certificate.input.concrete_left.owned.values[0].wrapping_add(1);
        assert!(!crate::open_encoding::execution_kernel_readback_matches(
            &encoding,
            &certificate
        ));
    }

    #[test]
    fn kernel_readback_rejects_active_action_omission() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, spec) = fixture();
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        let encoding = report.encoding_correspondence.encoding.unwrap();
        let mut certificate = crate::open_encoding::encode_execution_kernel(&encoding).unwrap();
        certificate.input.concrete_left.action_count -= 1;
        assert!(!crate::open_encoding::execution_kernel_readback_matches(
            &encoding,
            &certificate
        ));
    }

    #[test]
    fn canonical_validator_rejects_kernel_capacity_overflow() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, spec) = fixture();
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        let mut encoding = report.encoding_correspondence.encoding.unwrap();
        encoding.payload_variants = (0..=nmlt_open_kernel::MAX_PAYLOAD_VARIANTS)
            .map(|index| format!("variant-{index}"))
            .collect();
        let validation = crate::open_encoding::CanonicalEncodingValidator::check(&encoding);
        assert!(!validation.accepted);
        assert!(validation.issues.contains(
            &crate::open_encoding::CanonicalValidationIssue::ExecutionKernelCapacityExceeded
        ));
    }

    #[test]
    fn rejects_nonmonotone_composite_grade() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, mut spec) = fixture();
        spec.resources = resource_spec(10, 3, true);
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        assert!(
            report
                .issues
                .contains(&TwoSidedCongruenceIssue::LeftResourceRefinementRejected)
        );
    }

    #[test]
    fn rejects_undischarged_rely_condition() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, mut spec) = fixture();
        spec.resources = resource_spec(2, 3, false);
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        assert!(
            report
                .issues
                .contains(&TwoSidedCongruenceIssue::ConcreteResourceCompositionRejected)
        );
        assert!(
            report
                .issues
                .contains(&TwoSidedCongruenceIssue::AbstractResourceCompositionRejected)
        );
    }

    #[test]
    fn rejects_shared_affine_capability_between_components() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, mut spec) = fixture();
        spec.resources.concrete_right = resource_input_with_shared_owner("receive", 1);
        spec.resources.abstract_right = resource_input_with_shared_owner("accept", 2);
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        assert!(
            report
                .issues
                .contains(&TwoSidedCongruenceIssue::ConcreteResourceCompositionRejected)
        );
        assert!(
            report
                .issues
                .contains(&TwoSidedCongruenceIssue::AbstractResourceCompositionRejected)
        );
    }

    #[test]
    fn rejects_nonuniform_payload_universe_at_correspondence_boundary() {
        let (_, _, concrete_right, abstract_right, mut spec) = fixture();
        let concrete_left =
            output_system_with_exposed_payload("send", "audit", &["ok"], "sent", "AuditMessage");
        let abstract_left =
            output_system_with_exposed_payload("commit", "record", &["ok"], "sent", "AuditMessage");
        spec.resources = resource_spec(2, 3, true);
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        assert!(
            report
                .issues
                .contains(&TwoSidedCongruenceIssue::EncodingCorrespondenceRejected)
        );
        assert!(
            report
                .encoding_correspondence
                .issues
                .iter()
                .any(|issue| matches!(
                    issue,
                    crate::open_encoding::EncodingCorrespondenceIssue::NonUniformPayloadType { .. }
                ))
        );
    }

    #[test]
    fn rejects_connection_not_preserved_on_both_mapped_endpoints() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, mut spec) = fixture();
        spec.abstract_composition = composition("commit", "wrong", "sync-commit");
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            TwoSidedCongruenceIssue::ConcreteConnectionNotPreserved(_)
        )));
    }

    #[test]
    fn rejects_abstract_connection_outside_concrete_image() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, mut spec) = fixture();
        spec.abstract_composition.connections.push(Connection::new(
            "commit",
            "accept",
            "extra-sync",
        ));
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            TwoSidedCongruenceIssue::AbstractConnectionNotReflected(connection)
                if connection.composite_action == "extra-sync"
        )));
    }

    #[test]
    fn rejects_stale_invariant_domain() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, mut spec) = fixture();
        spec.abstract_invariant = Some(FiniteStateInvariant { holds: vec![true] });
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            TwoSidedCongruenceIssue::InvariantLengthMismatch { .. }
        )));
    }

    #[test]
    fn rejects_incomplete_component_boundary_mapping() {
        let (concrete_left, abstract_left, concrete_right, abstract_right, mut spec) = fixture();
        spec.left_refinement.actions = ActionHiding::new([("send", Some("commit"))]);
        let report = TwoSidedCongruenceChecker::check(
            &concrete_left,
            &abstract_left,
            &concrete_right,
            &abstract_right,
            &spec,
        );
        assert!(
            report
                .issues
                .contains(&TwoSidedCongruenceIssue::LeftRefinementRejected)
        );
        assert!(report.left_refinement.issues.iter().any(|issue| matches!(
            issue,
            crate::open_refinement::OpenRefinementIssue::AbstractBoundaryActionUncovered(action)
                if action == "record"
        )));
    }

    #[test]
    fn shared_m11_congruence_vectors_bind_the_rust_controls() {
        assert!(M11_CONGRUENCE_VECTORS.contains("nmlt-m11-open-congruence-v1"));
        for control in [
            "accepts_two_sided_contract_sound_congruence_and_invariant_transport",
            "rejects_connection_not_preserved_on_both_mapped_endpoints",
            "rejects_abstract_connection_outside_concrete_image",
            "rejects_stale_invariant_domain",
            "rejects_incomplete_component_boundary_mapping",
            "rejects_nonmonotone_composite_grade",
            "rejects_undischarged_rely_condition",
            "rejects_shared_affine_capability_between_components",
            "rejects_nonuniform_payload_universe_at_correspondence_boundary",
            "canonical_certificate_validator_fails_closed_after_mutation",
            "kernel_readback_rejects_dictionary_substitution",
            "kernel_readback_rejects_numeric_atom_substitution",
            "kernel_readback_rejects_active_action_omission",
            "canonical_validator_rejects_kernel_capacity_overflow",
        ] {
            assert!(
                M11_CONGRUENCE_VECTORS.contains(control),
                "missing {control}"
            );
        }
    }
}

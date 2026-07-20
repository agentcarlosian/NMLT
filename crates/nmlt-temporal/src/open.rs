//! Finite, safety-only open-system composition and refinement congruence checks.
//!
//! This module implements a deliberately bounded fragment of the repaired
//! composition rule proposed by RFC 0008. Interfaces classify graph actions as
//! inputs, outputs, or internal actions. A supported composition uses explicit
//! one-output/one-input synchronous connections, requires every declared input
//! to be enabled in every local graph state, and discharges canonical finite
//! input predicates from exact peer output predicates supplied by
//! assumption-free providers.
//!
//! The executable congruence check is an instance check, not a general theorem.
//! It establishes only that two finite composed graphs satisfy the existing
//! one-step, observation-preserving forward-simulation checker. Payload
//! subtyping, grades, capabilities, temporal/circular contract satisfaction,
//! fairness, divergence, and liveness transport remain separate obligations.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::graph::{FiniteGraph, GraphError, ModelState, StateId, Transition, TransitionKind};
use crate::observation::{ActionHiding, ObservationMap};
use crate::open_contract::{FiniteContract, PayloadPredicate, PayloadType, PayloadTypeId};
use crate::refinement::{RefinementChecker, RefinementReport, RefinementSpec};

const LEFT_NAMESPACE: &str = "left::";
const RIGHT_NAMESPACE: &str = "right::";

/// Default caps for the executable cartesian-product construction.
///
/// The transition cap applies to generated transition candidates before
/// `FiniteGraph` canonicalization. The work-item cap is a conservative bound
/// on explicit compatibility and product-enumeration loop items; it is not a
/// wall-clock, byte-allocation, or standard-library comparison budget.
pub const DEFAULT_MAX_COMPOSED_STATES: usize = 100_000;
pub const DEFAULT_MAX_COMPOSED_TRANSITIONS: usize = 1_000_000;
pub const DEFAULT_MAX_COMPOSITION_WORK_ITEMS: usize = 50_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositionLimits {
    /// Maximum number of states retained in the cartesian product.
    pub max_states: usize,
    /// Maximum generated transitions retained before graph canonicalization.
    pub max_transitions: usize,
    /// Conservative cap on the explicit loop items described above.
    pub max_work_items: usize,
}

impl Default for CompositionLimits {
    fn default() -> Self {
        Self {
            max_states: DEFAULT_MAX_COMPOSED_STATES,
            max_transitions: DEFAULT_MAX_COMPOSED_TRANSITIONS,
            max_work_items: DEFAULT_MAX_COMPOSITION_WORK_ITEMS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionPolarity {
    Input,
    Output,
    Internal,
}

impl ActionPolarity {
    fn is_complementary_to(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Input, Self::Output) | (Self::Output, Self::Input)
        )
    }
}

/// The finite interface information currently checked for one graph action.
///
/// Boundary actions carry an opaque channel and an exact finite payload type.
/// Internal actions carry neither.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActionSignature {
    pub polarity: ActionPolarity,
    pub channel: Option<String>,
    pub payload_type: Option<PayloadType>,
}

impl ActionSignature {
    #[must_use]
    pub fn input(channel: impl Into<String>, payload_type: PayloadType) -> Self {
        Self {
            polarity: ActionPolarity::Input,
            channel: Some(channel.into()),
            payload_type: Some(payload_type),
        }
    }

    #[must_use]
    pub fn output(channel: impl Into<String>, payload_type: PayloadType) -> Self {
        Self {
            polarity: ActionPolarity::Output,
            channel: Some(channel.into()),
            payload_type: Some(payload_type),
        }
    }

    #[must_use]
    pub fn internal() -> Self {
        Self {
            polarity: ActionPolarity::Internal,
            channel: None,
            payload_type: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterfaceBuildError {
    DuplicateAction(String),
}

impl fmt::Display for InterfaceBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateAction(action) => {
                write!(f, "interface action {action:?} is declared more than once")
            }
        }
    }
}

impl std::error::Error for InterfaceBuildError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Interface {
    actions: BTreeMap<String, ActionSignature>,
}

impl Interface {
    pub fn new<I, S>(actions: I) -> Result<Self, InterfaceBuildError>
    where
        I: IntoIterator<Item = (S, ActionSignature)>,
        S: Into<String>,
    {
        let mut declarations = BTreeMap::new();
        for (action, signature) in actions {
            let action = action.into();
            if declarations.insert(action.clone(), signature).is_some() {
                return Err(InterfaceBuildError::DuplicateAction(action));
            }
        }
        Ok(Self {
            actions: declarations,
        })
    }

    #[must_use]
    pub fn get(&self, action: &str) -> Option<&ActionSignature> {
        self.actions.get(action)
    }

    #[must_use]
    pub fn actions(&self) -> &BTreeMap<String, ActionSignature> {
        &self.actions
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenSystemIssue {
    EmptyActionName,
    UndeclaredGraphAction(String),
    BoundaryActionMissingChannel(String),
    BoundaryActionHasEmptyChannel(String),
    BoundaryActionMissingPayloadType(String),
    InternalActionHasChannel(String),
    InternalActionHasPayloadType(String),
    MissingAssumption(String),
    MissingGuarantee(String),
    UnexpectedAssumption(String),
    UnexpectedGuarantee(String),
    ContractPayloadTypeMismatch {
        action: String,
        expected: PayloadTypeId,
        actual: PayloadTypeId,
    },
}

impl fmt::Display for OpenSystemIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyActionName => write!(f, "an interface action name is empty"),
            Self::UndeclaredGraphAction(action) => {
                write!(f, "graph action {action:?} has no interface declaration")
            }
            Self::BoundaryActionMissingChannel(action) => {
                write!(f, "boundary action {action:?} has no channel")
            }
            Self::BoundaryActionHasEmptyChannel(action) => {
                write!(f, "boundary action {action:?} has an empty channel")
            }
            Self::InternalActionHasChannel(action) => {
                write!(f, "internal action {action:?} declares a channel")
            }
            Self::BoundaryActionMissingPayloadType(action) => {
                write!(f, "boundary action {action:?} has no payload type")
            }
            Self::InternalActionHasPayloadType(action) => {
                write!(f, "internal action {action:?} declares a payload type")
            }
            Self::MissingAssumption(action) => {
                write!(f, "input action {action:?} has no assumption predicate")
            }
            Self::MissingGuarantee(action) => {
                write!(f, "output action {action:?} has no guarantee predicate")
            }
            Self::UnexpectedAssumption(action) => {
                write!(f, "non-input action {action:?} has an assumption predicate")
            }
            Self::UnexpectedGuarantee(action) => {
                write!(f, "non-output action {action:?} has a guarantee predicate")
            }
            Self::ContractPayloadTypeMismatch {
                action,
                expected,
                actual,
            } => write!(
                f,
                "contract predicate for {action:?} has payload type {actual}, expected {expected}"
            ),
        }
    }
}

/// A finite graph bundled with a total action interface and finite contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenSystem {
    graph: FiniteGraph,
    interface: Interface,
    contract: FiniteContract,
}

impl OpenSystem {
    pub fn new(
        graph: FiniteGraph,
        interface: Interface,
        contract: FiniteContract,
    ) -> Result<Self, Vec<OpenSystemIssue>> {
        let mut issues = Vec::new();
        for (action, signature) in interface.actions() {
            if action.is_empty() {
                issues.push(OpenSystemIssue::EmptyActionName);
            }
            match (
                signature.polarity,
                signature.channel.as_deref(),
                signature.payload_type.as_ref(),
            ) {
                (ActionPolarity::Internal, Some(_), _) => {
                    issues.push(OpenSystemIssue::InternalActionHasChannel(action.clone()));
                }
                (ActionPolarity::Internal, _, Some(_)) => {
                    issues.push(OpenSystemIssue::InternalActionHasPayloadType(
                        action.clone(),
                    ));
                }
                (ActionPolarity::Input | ActionPolarity::Output, None, _) => {
                    issues.push(OpenSystemIssue::BoundaryActionMissingChannel(
                        action.clone(),
                    ));
                }
                (ActionPolarity::Input | ActionPolarity::Output, Some(""), _) => {
                    issues.push(OpenSystemIssue::BoundaryActionHasEmptyChannel(
                        action.clone(),
                    ));
                }
                (ActionPolarity::Input | ActionPolarity::Output, _, None) => {
                    issues.push(OpenSystemIssue::BoundaryActionMissingPayloadType(
                        action.clone(),
                    ));
                }
                _ => {}
            }
            validate_action_contract(action, signature, &contract, &mut issues);
        }
        for transition in graph.transitions() {
            if let Some(action) = transition.kind.action()
                && interface.get(action).is_none()
            {
                issues.push(OpenSystemIssue::UndeclaredGraphAction(action.to_owned()));
            }
        }
        for action in contract.assumptions().keys() {
            if interface.get(action).is_none() {
                issues.push(OpenSystemIssue::UnexpectedAssumption(action.clone()));
            }
        }
        for action in contract.guarantees().keys() {
            if interface.get(action).is_none() {
                issues.push(OpenSystemIssue::UnexpectedGuarantee(action.clone()));
            }
        }
        if issues.is_empty() {
            Ok(Self {
                graph,
                interface,
                contract,
            })
        } else {
            Err(issues)
        }
    }

    #[must_use]
    pub fn graph(&self) -> &FiniteGraph {
        &self.graph
    }

    #[must_use]
    pub fn interface(&self) -> &Interface {
        &self.interface
    }

    #[must_use]
    pub fn contract(&self) -> &FiniteContract {
        &self.contract
    }
}

fn validate_action_contract(
    action: &str,
    signature: &ActionSignature,
    contract: &FiniteContract,
    issues: &mut Vec<OpenSystemIssue>,
) {
    let assumption = contract.assumptions().get(action);
    let guarantee = contract.guarantees().get(action);
    match signature.polarity {
        ActionPolarity::Input => {
            let Some(predicate) = assumption else {
                issues.push(OpenSystemIssue::MissingAssumption(action.to_owned()));
                return;
            };
            if guarantee.is_some() {
                issues.push(OpenSystemIssue::UnexpectedGuarantee(action.to_owned()));
            }
            validate_predicate_payload(action, signature, predicate, issues);
        }
        ActionPolarity::Output => {
            let Some(predicate) = guarantee else {
                issues.push(OpenSystemIssue::MissingGuarantee(action.to_owned()));
                return;
            };
            if assumption.is_some() {
                issues.push(OpenSystemIssue::UnexpectedAssumption(action.to_owned()));
            }
            validate_predicate_payload(action, signature, predicate, issues);
        }
        ActionPolarity::Internal => {
            if assumption.is_some() {
                issues.push(OpenSystemIssue::UnexpectedAssumption(action.to_owned()));
            }
            if guarantee.is_some() {
                issues.push(OpenSystemIssue::UnexpectedGuarantee(action.to_owned()));
            }
        }
    }
}

fn validate_predicate_payload(
    action: &str,
    signature: &ActionSignature,
    predicate: &PayloadPredicate,
    issues: &mut Vec<OpenSystemIssue>,
) {
    if let Some(payload_type) = &signature.payload_type
        && predicate.payload_type() != payload_type.id()
    {
        issues.push(OpenSystemIssue::ContractPayloadTypeMismatch {
            action: action.to_owned(),
            expected: payload_type.id(),
            actual: predicate.payload_type(),
        });
    }
}

/// A synchronous connection between one left and one right boundary action.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Connection {
    pub left_action: String,
    pub right_action: String,
    pub composite_action: String,
}

impl Connection {
    #[must_use]
    pub fn new(
        left_action: impl Into<String>,
        right_action: impl Into<String>,
        composite_action: impl Into<String>,
    ) -> Self {
        Self {
            left_action: left_action.into(),
            right_action: right_action.into(),
            composite_action: composite_action.into(),
        }
    }
}

/// Links one consumer input predicate to one connected provider output
/// predicate. Discharge uses finite inclusion, never claim-name equality.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContractLink {
    pub consumer: Side,
    pub consumer_action: String,
    pub provider: Side,
    pub provider_action: String,
}

impl ContractLink {
    #[must_use]
    pub fn new(
        consumer: Side,
        consumer_action: impl Into<String>,
        provider: Side,
        provider_action: impl Into<String>,
    ) -> Self {
        Self {
            consumer,
            consumer_action: consumer_action.into(),
            provider,
            provider_action: provider_action.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompositionSpec {
    pub connections: Vec<Connection>,
    pub contract_links: Vec<ContractLink>,
}

impl CompositionSpec {
    #[must_use]
    pub fn new(connections: Vec<Connection>, contract_links: Vec<ContractLink>) -> Self {
        Self {
            connections,
            contract_links,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompatibilityIssue {
    DuplicateConnection(Connection),
    EmptyCompositeAction,
    UnknownAction {
        side: Side,
        action: String,
    },
    NonComplementaryPolarity {
        left_action: String,
        right_action: String,
    },
    ChannelMismatch {
        left_action: String,
        right_action: String,
    },
    PayloadTypeMismatch {
        left_action: String,
        right_action: String,
        left_payload: PayloadTypeId,
        right_payload: PayloadTypeId,
    },
    ActionMultiplyConnected {
        side: Side,
        action: String,
    },
    CompositeActionCollision(String),
    InputNotReceptive {
        side: Side,
        action: String,
        state: StateId,
    },
    DuplicateContractLink(ContractLink),
    SelfDischarge(ContractLink),
    UnknownAssumption {
        side: Side,
        action: String,
    },
    MissingPeerGuarantee {
        side: Side,
        action: String,
    },
    ContractLinkNotConnected(ContractLink),
    ContractPolarityMismatch(ContractLink),
    ContractPayloadTypeMismatch(ContractLink),
    GuaranteeDoesNotDischarge {
        link: ContractLink,
        rejected_value: String,
    },
    ConditionalGuaranteeProvider {
        provider: Side,
        action: String,
    },
    DuplicateAssumptionDischarge {
        side: Side,
        action: String,
    },
    UndischargedAssumption {
        side: Side,
        action: String,
    },
    WorkItemCountOverflow,
    WorkItemLimitExceeded {
        required: usize,
        limit: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompatibilityReport {
    pub accepted: bool,
    pub checked_connections: usize,
    pub checked_receptive_states: usize,
    pub issues: Vec<CompatibilityIssue>,
}

pub struct CompatibilityChecker;

impl CompatibilityChecker {
    /// Checks the strict, finite supported composition fragment.
    ///
    /// Receptiveness means that every declared input action is enabled in every
    /// graph state of its local component, including unreachable states and
    /// inputs not used by this composition. This deliberately global rule
    /// matches the frozen Lean model. It is stronger than a reachable-state or
    /// state-dependent rely condition.
    ///
    /// Contract links are intentionally conservative: a connected peer output
    /// predicate can discharge an input predicate only by exact-payload finite
    /// inclusion and only if the provider has no assumptions. This rejects
    /// payload substitution, mutual discharge, and other conditional cycles.
    /// The default logical work-item budget is checked before these loops; use
    /// [`CompatibilityChecker::check_with_limits`] for a smaller bound.
    #[must_use]
    pub fn check(
        left: &OpenSystem,
        right: &OpenSystem,
        spec: &CompositionSpec,
    ) -> CompatibilityReport {
        Self::check_with_limits(left, right, spec, CompositionLimits::default())
    }

    /// Checks compatibility under the work-item cap in `limits`.
    ///
    /// State and transition caps are used by product construction; this method
    /// consumes only `max_work_items`. Preflight failure returns a rejected
    /// report without entering the compatibility loops.
    #[must_use]
    pub fn check_with_limits(
        left: &OpenSystem,
        right: &OpenSystem,
        spec: &CompositionSpec,
        limits: CompositionLimits,
    ) -> CompatibilityReport {
        match preflight_compatibility_work(left, right, spec, limits.max_work_items) {
            Ok(_) => Self::check_preflighted(left, right, spec),
            Err(error) => compatibility_work_failure(error),
        }
    }

    fn check_preflighted(
        left: &OpenSystem,
        right: &OpenSystem,
        spec: &CompositionSpec,
    ) -> CompatibilityReport {
        let mut issues = Vec::new();
        let mut seen_connections = BTreeSet::new();
        let mut connected_left = BTreeSet::new();
        let mut connected_right = BTreeSet::new();
        let mut composite_actions = BTreeSet::new();
        let mut checked_receptive_states = 0;

        let exposed_actions = left
            .interface()
            .actions()
            .keys()
            .map(|action| namespaced(Side::Left, action))
            .chain(
                right
                    .interface()
                    .actions()
                    .keys()
                    .map(|action| namespaced(Side::Right, action)),
            )
            .collect::<BTreeSet<_>>();

        for connection in &spec.connections {
            if !seen_connections.insert(connection.clone()) {
                issues.push(CompatibilityIssue::DuplicateConnection(connection.clone()));
            }
            if connection.composite_action.is_empty() {
                issues.push(CompatibilityIssue::EmptyCompositeAction);
            }
            if !composite_actions.insert(connection.composite_action.clone())
                || exposed_actions.contains(&connection.composite_action)
            {
                issues.push(CompatibilityIssue::CompositeActionCollision(
                    connection.composite_action.clone(),
                ));
            }
            if !connected_left.insert(connection.left_action.clone()) {
                issues.push(CompatibilityIssue::ActionMultiplyConnected {
                    side: Side::Left,
                    action: connection.left_action.clone(),
                });
            }
            if !connected_right.insert(connection.right_action.clone()) {
                issues.push(CompatibilityIssue::ActionMultiplyConnected {
                    side: Side::Right,
                    action: connection.right_action.clone(),
                });
            }

            let left_signature = left.interface().get(&connection.left_action);
            let right_signature = right.interface().get(&connection.right_action);
            if left_signature.is_none() {
                issues.push(CompatibilityIssue::UnknownAction {
                    side: Side::Left,
                    action: connection.left_action.clone(),
                });
            }
            if right_signature.is_none() {
                issues.push(CompatibilityIssue::UnknownAction {
                    side: Side::Right,
                    action: connection.right_action.clone(),
                });
            }
            let (Some(left_signature), Some(right_signature)) = (left_signature, right_signature)
            else {
                continue;
            };
            if !left_signature
                .polarity
                .is_complementary_to(right_signature.polarity)
            {
                issues.push(CompatibilityIssue::NonComplementaryPolarity {
                    left_action: connection.left_action.clone(),
                    right_action: connection.right_action.clone(),
                });
            }
            if left_signature.channel != right_signature.channel {
                issues.push(CompatibilityIssue::ChannelMismatch {
                    left_action: connection.left_action.clone(),
                    right_action: connection.right_action.clone(),
                });
            }
            let left_payload = left_signature.payload_type.as_ref().map(PayloadType::id);
            let right_payload = right_signature.payload_type.as_ref().map(PayloadType::id);
            if let (Some(left_payload), Some(right_payload)) = (left_payload, right_payload)
                && left_payload != right_payload
            {
                issues.push(CompatibilityIssue::PayloadTypeMismatch {
                    left_action: connection.left_action.clone(),
                    right_action: connection.right_action.clone(),
                    left_payload,
                    right_payload,
                });
            }
        }

        for (side, system) in [(Side::Left, left), (Side::Right, right)] {
            for (action, signature) in system.interface().actions() {
                if signature.polarity != ActionPolarity::Input {
                    continue;
                }
                for state in 0..system.graph().states().len() {
                    checked_receptive_states += 1;
                    if !system.graph().action_enabled(state, action) {
                        issues.push(CompatibilityIssue::InputNotReceptive {
                            side,
                            action: action.clone(),
                            state,
                        });
                    }
                }
            }
        }

        check_contracts(left, right, spec, &mut issues);

        CompatibilityReport {
            accepted: issues.is_empty(),
            checked_connections: spec.connections.len(),
            checked_receptive_states,
            issues,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkPreflightError {
    CountOverflow,
    LimitExceeded { required: usize, limit: usize },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct WorkCounter {
    items: usize,
}

impl WorkCounter {
    fn add(&mut self, items: usize) -> Result<(), WorkPreflightError> {
        self.items = self
            .items
            .checked_add(items)
            .ok_or(WorkPreflightError::CountOverflow)?;
        Ok(())
    }

    fn add_product(&mut self, factors: &[usize]) -> Result<(), WorkPreflightError> {
        self.add(checked_work_product(factors)?)
    }

    fn enforce(self, limit: usize) -> Result<usize, WorkPreflightError> {
        if self.items > limit {
            Err(WorkPreflightError::LimitExceeded {
                required: self.items,
                limit,
            })
        } else {
            Ok(self.items)
        }
    }
}

fn checked_work_product(factors: &[usize]) -> Result<usize, WorkPreflightError> {
    if factors.contains(&0) {
        return Ok(0);
    }
    factors.iter().try_fold(1_usize, |product, &factor| {
        product
            .checked_mul(factor)
            .ok_or(WorkPreflightError::CountOverflow)
    })
}

fn compatibility_work_items(
    left: &OpenSystem,
    right: &OpenSystem,
    spec: &CompositionSpec,
) -> Result<usize, WorkPreflightError> {
    let mut work = WorkCounter::default();
    let action_count = left
        .interface()
        .actions()
        .len()
        .checked_add(right.interface().actions().len())
        .ok_or(WorkPreflightError::CountOverflow)?;

    // One scan builds the exposed-action set and one checks receptiveness.
    work.add_product(&[2, action_count])?;
    work.add(spec.connections.len())?;
    for system in [left, right] {
        // Treat every declaration as an input for a loop-safe upper bound;
        // deriving the exact input count would itself require a preflight scan.
        let input_count = system.interface().actions().len();
        // Each input visits every state and `action_enabled` may scan every
        // outgoing transition across those states.
        work.add_product(&[input_count, system.graph().states().len()])?;
        work.add_product(&[input_count, system.graph().transitions().len()])?;
    }
    work.add(spec.contract_links.len())?;
    work.add(left.contract().assumptions().len())?;
    work.add(right.contract().assumptions().len())?;
    Ok(work.items)
}

fn preflight_compatibility_work(
    left: &OpenSystem,
    right: &OpenSystem,
    spec: &CompositionSpec,
    limit: usize,
) -> Result<usize, WorkPreflightError> {
    WorkCounter {
        items: compatibility_work_items(left, right, spec)?,
    }
    .enforce(limit)
}

fn compatibility_work_failure(error: WorkPreflightError) -> CompatibilityReport {
    let issue = match error {
        WorkPreflightError::CountOverflow => CompatibilityIssue::WorkItemCountOverflow,
        WorkPreflightError::LimitExceeded { required, limit } => {
            CompatibilityIssue::WorkItemLimitExceeded { required, limit }
        }
    };
    CompatibilityReport {
        accepted: false,
        checked_connections: 0,
        checked_receptive_states: 0,
        issues: vec![issue],
    }
}

fn system_at<'a>(left: &'a OpenSystem, right: &'a OpenSystem, side: Side) -> &'a OpenSystem {
    match side {
        Side::Left => left,
        Side::Right => right,
    }
}

fn check_contracts(
    left: &OpenSystem,
    right: &OpenSystem,
    spec: &CompositionSpec,
    issues: &mut Vec<CompatibilityIssue>,
) {
    let mut seen_links = BTreeSet::new();
    let mut discharge_count = BTreeMap::<(Side, String), usize>::new();
    for link in &spec.contract_links {
        if !seen_links.insert(link.clone()) {
            issues.push(CompatibilityIssue::DuplicateContractLink(link.clone()));
        }
        if link.consumer == link.provider {
            issues.push(CompatibilityIssue::SelfDischarge(link.clone()));
            continue;
        }
        let consumer = system_at(left, right, link.consumer);
        let provider = system_at(left, right, link.provider);
        let assumption = consumer.contract().assumptions().get(&link.consumer_action);
        let guarantee = provider.contract().guarantees().get(&link.provider_action);
        if assumption.is_none() {
            issues.push(CompatibilityIssue::UnknownAssumption {
                side: link.consumer,
                action: link.consumer_action.clone(),
            });
        }
        if guarantee.is_none() {
            issues.push(CompatibilityIssue::MissingPeerGuarantee {
                side: link.provider,
                action: link.provider_action.clone(),
            });
        }
        let consumer_signature = consumer.interface().get(&link.consumer_action);
        let provider_signature = provider.interface().get(&link.provider_action);
        if !matches!(
            (consumer_signature, provider_signature),
            (Some(consumer_signature), Some(provider_signature))
                if consumer_signature.polarity == ActionPolarity::Input
                    && provider_signature.polarity == ActionPolarity::Output
        ) {
            issues.push(CompatibilityIssue::ContractPolarityMismatch(link.clone()));
        }
        if !link_is_connected(spec, link) {
            issues.push(CompatibilityIssue::ContractLinkNotConnected(link.clone()));
        }
        if let (Some(assumption), Some(guarantee)) = (assumption, guarantee) {
            if assumption.payload_type() != guarantee.payload_type() {
                issues.push(CompatibilityIssue::ContractPayloadTypeMismatch(
                    link.clone(),
                ));
            } else if !guarantee.is_subset_of(assumption) {
                let rejected_value = guarantee
                    .accepted()
                    .difference(assumption.accepted())
                    .next()
                    .expect("failed finite inclusion has a witness")
                    .clone();
                issues.push(CompatibilityIssue::GuaranteeDoesNotDischarge {
                    link: link.clone(),
                    rejected_value,
                });
            }
        }
        if !provider.contract().assumptions().is_empty() {
            issues.push(CompatibilityIssue::ConditionalGuaranteeProvider {
                provider: link.provider,
                action: link.provider_action.clone(),
            });
        }
        *discharge_count
            .entry((link.consumer, link.consumer_action.clone()))
            .or_default() += 1;
    }

    for side in [Side::Left, Side::Right] {
        let system = system_at(left, right, side);
        for action in system.contract().assumptions().keys() {
            match discharge_count.get(&(side, action.clone())).copied() {
                None | Some(0) => issues.push(CompatibilityIssue::UndischargedAssumption {
                    side,
                    action: action.clone(),
                }),
                Some(1) => {}
                Some(_) => issues.push(CompatibilityIssue::DuplicateAssumptionDischarge {
                    side,
                    action: action.clone(),
                }),
            }
        }
    }
}

fn link_is_connected(spec: &CompositionSpec, link: &ContractLink) -> bool {
    spec.connections
        .iter()
        .any(|connection| match (link.consumer, link.provider) {
            (Side::Left, Side::Right) => {
                connection.left_action == link.consumer_action
                    && connection.right_action == link.provider_action
            }
            (Side::Right, Side::Left) => {
                connection.right_action == link.consumer_action
                    && connection.left_action == link.provider_action
            }
            _ => false,
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionError {
    Incompatible(CompatibilityReport),
    StateCountOverflow {
        left_states: usize,
        right_states: usize,
    },
    StateLimitExceeded {
        required: usize,
        limit: usize,
    },
    StateIndexOverflow {
        left_state: StateId,
        right_state: StateId,
        right_states: usize,
    },
    TransitionCountOverflow,
    TransitionLimitExceeded {
        attempted: usize,
        limit: usize,
    },
    WorkItemCountOverflow,
    WorkItemLimitExceeded {
        required: usize,
        limit: usize,
    },
    Graph(GraphError),
    InvalidResult(Vec<OpenSystemIssue>),
}

impl fmt::Display for CompositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incompatible(report) => write!(
                f,
                "open systems are incompatible ({} issue(s))",
                report.issues.len()
            ),
            Self::StateCountOverflow {
                left_states,
                right_states,
            } => write!(
                f,
                "composed state count overflows usize: {left_states} * {right_states}"
            ),
            Self::StateLimitExceeded { required, limit } => write!(
                f,
                "composed state count {required} exceeds configured limit {limit}"
            ),
            Self::StateIndexOverflow {
                left_state,
                right_state,
                right_states,
            } => write!(
                f,
                "composed state index overflows usize: ({left_state} * {right_states}) + {right_state}"
            ),
            Self::TransitionCountOverflow => {
                write!(f, "generated transition count overflows usize")
            }
            Self::TransitionLimitExceeded { attempted, limit } => write!(
                f,
                "generated transition count {attempted} exceeds configured limit {limit}"
            ),
            Self::WorkItemCountOverflow => {
                write!(
                    f,
                    "conservative composition work-item count overflows usize"
                )
            }
            Self::WorkItemLimitExceeded { required, limit } => write!(
                f,
                "conservative composition work-item count {required} exceeds configured limit {limit}"
            ),
            Self::Graph(error) => write!(f, "composed graph is invalid: {error}"),
            Self::InvalidResult(issues) => write!(
                f,
                "composed open system is invalid ({} issue(s))",
                issues.len()
            ),
        }
    }
}

impl std::error::Error for CompositionError {}

/// Constructs the supported synchronous product after all checks succeed.
///
/// Connected actions never interleave independently. Unconnected actions are
/// namespaced, synchronization actions become internal, and state fields are
/// namespaced in the cartesian product. The result contains no open assumptions
/// because this strict fragment requires every local assumption to be discharged.
/// The default state, transition, and conservative work-item caps are exposed
/// as constants; callers that need smaller reviewed bounds should use
/// [`compose_with_limits`].
pub fn compose(
    left: &OpenSystem,
    right: &OpenSystem,
    spec: &CompositionSpec,
) -> Result<OpenSystem, CompositionError> {
    compose_with_limits(left, right, spec, CompositionLimits::default())
}

/// Constructs the supported synchronous product with explicit resource caps.
///
/// State cardinality, a conservative logical work-item estimate, and every
/// product-state index use checked arithmetic. State/work preflight runs before
/// compatibility or product enumeration. The transition cap is checked before
/// each candidate `Transition` and its action `String` are constructed. A zero
/// transition limit therefore still permits a transition-free product. Work
/// items bound explicit module loops, not wall-clock time, bytes, allocation
/// success, or operations internal to standard-library collections and sorting.
pub fn compose_with_limits(
    left: &OpenSystem,
    right: &OpenSystem,
    spec: &CompositionSpec,
    limits: CompositionLimits,
) -> Result<OpenSystem, CompositionError> {
    let left_count = left.graph().states().len();
    let right_count = right.graph().states().len();
    let state_count = checked_product_state_count(left_count, right_count, limits.max_states)?;
    preflight_composition_work(left, right, spec, state_count, limits)
        .map_err(composition_work_error)?;
    let report = CompatibilityChecker::check_preflighted(left, right, spec);
    if !report.accepted {
        return Err(CompositionError::Incompatible(report));
    }

    let mut states = Vec::with_capacity(state_count);
    for left_state in left.graph().states() {
        for right_state in right.graph().states() {
            states.push(product_state(left_state, right_state));
        }
    }
    let mut initial = Vec::new();
    for &left_state in left.graph().initial_states() {
        for &right_state in right.graph().initial_states() {
            initial.push(checked_product_index(left_state, right_state, right_count)?);
        }
    }

    let connected_left = spec
        .connections
        .iter()
        .map(|connection| connection.left_action.as_str())
        .collect::<BTreeSet<_>>();
    let connected_right = spec
        .connections
        .iter()
        .map(|connection| connection.right_action.as_str())
        .collect::<BTreeSet<_>>();
    let mut transitions = Vec::new();

    for left_state in 0..left_count {
        for right_state in 0..right_count {
            let from = checked_product_index(left_state, right_state, right_count)?;
            let left_outgoing = left.graph().outgoing_ids(left_state);
            let right_outgoing = right.graph().outgoing_ids(right_state);

            if left_outgoing.iter().any(|&id| {
                matches!(
                    left.graph().transition(id).kind,
                    TransitionKind::IdentityStutter
                )
            }) || right_outgoing.iter().any(|&id| {
                matches!(
                    right.graph().transition(id).kind,
                    TransitionKind::IdentityStutter
                )
            }) {
                push_transition(&mut transitions, limits.max_transitions, || {
                    Transition::identity_stutter(from)
                })?;
            }

            for &transition_id in left_outgoing {
                let transition = left.graph().transition(transition_id);
                let Some(action) = transition.kind.action() else {
                    continue;
                };
                if !connected_left.contains(action) {
                    let to = checked_product_index(transition.to, right_state, right_count)?;
                    push_transition(&mut transitions, limits.max_transitions, || {
                        Transition::action(from, namespaced(Side::Left, action), to)
                    })?;
                }
            }
            for &transition_id in right_outgoing {
                let transition = right.graph().transition(transition_id);
                let Some(action) = transition.kind.action() else {
                    continue;
                };
                if !connected_right.contains(action) {
                    let to = checked_product_index(left_state, transition.to, right_count)?;
                    push_transition(&mut transitions, limits.max_transitions, || {
                        Transition::action(from, namespaced(Side::Right, action), to)
                    })?;
                }
            }
            for connection in &spec.connections {
                for &left_transition_id in left_outgoing {
                    let left_transition = left.graph().transition(left_transition_id);
                    if left_transition.kind.action() != Some(connection.left_action.as_str()) {
                        continue;
                    }
                    for &right_transition_id in right_outgoing {
                        let right_transition = right.graph().transition(right_transition_id);
                        if right_transition.kind.action() == Some(connection.right_action.as_str())
                        {
                            let to = checked_product_index(
                                left_transition.to,
                                right_transition.to,
                                right_count,
                            )?;
                            push_transition(&mut transitions, limits.max_transitions, || {
                                Transition::action(from, connection.composite_action.clone(), to)
                            })?;
                        }
                    }
                }
            }
        }
    }

    let graph = FiniteGraph::new(states, initial, transitions).map_err(CompositionError::Graph)?;
    let connected_left = spec
        .connections
        .iter()
        .map(|connection| connection.left_action.as_str())
        .collect::<BTreeSet<_>>();
    let connected_right = spec
        .connections
        .iter()
        .map(|connection| connection.right_action.as_str())
        .collect::<BTreeSet<_>>();
    let mut actions = Vec::new();
    actions.extend(
        left.interface()
            .actions()
            .iter()
            .filter(|(action, _)| !connected_left.contains(action.as_str()))
            .map(|(action, signature)| (namespaced(Side::Left, action), signature.clone())),
    );
    actions.extend(
        right
            .interface()
            .actions()
            .iter()
            .filter(|(action, _)| !connected_right.contains(action.as_str()))
            .map(|(action, signature)| (namespaced(Side::Right, action), signature.clone())),
    );
    actions.extend(spec.connections.iter().map(|connection| {
        (
            connection.composite_action.clone(),
            ActionSignature::internal(),
        )
    }));
    let interface = Interface::new(actions)
        .expect("compatibility rejects every composite action-name collision");
    let assumptions = left
        .contract()
        .assumptions()
        .iter()
        .filter(|(action, _)| !connected_left.contains(action.as_str()))
        .map(|(action, predicate)| (namespaced(Side::Left, action), predicate.clone()))
        .chain(
            right
                .contract()
                .assumptions()
                .iter()
                .filter(|(action, _)| !connected_right.contains(action.as_str()))
                .map(|(action, predicate)| (namespaced(Side::Right, action), predicate.clone())),
        );
    let guarantees = left
        .contract()
        .guarantees()
        .iter()
        .filter(|(action, _)| !connected_left.contains(action.as_str()))
        .map(|(action, predicate)| (namespaced(Side::Left, action), predicate.clone()))
        .chain(
            right
                .contract()
                .guarantees()
                .iter()
                .filter(|(action, _)| !connected_right.contains(action.as_str()))
                .map(|(action, predicate)| (namespaced(Side::Right, action), predicate.clone())),
        );
    let contract = FiniteContract::new(assumptions, guarantees)
        .expect("namespacing preserves canonical unique contract actions");
    OpenSystem::new(graph, interface, contract).map_err(CompositionError::InvalidResult)
}

fn namespaced(side: Side, name: &str) -> String {
    match side {
        Side::Left => format!("{LEFT_NAMESPACE}{name}"),
        Side::Right => format!("{RIGHT_NAMESPACE}{name}"),
    }
}

fn checked_model_field_count(system: &OpenSystem) -> Result<usize, WorkPreflightError> {
    system
        .graph()
        .states()
        .iter()
        .try_fold(0_usize, |count, state| {
            count
                .checked_add(state.len())
                .ok_or(WorkPreflightError::CountOverflow)
        })
}

fn preflight_composition_work(
    left: &OpenSystem,
    right: &OpenSystem,
    spec: &CompositionSpec,
    state_count: usize,
    limits: CompositionLimits,
) -> Result<usize, WorkPreflightError> {
    let left_states = left.graph().states().len();
    let right_states = right.graph().states().len();
    let left_transitions = left.graph().transitions().len();
    let right_transitions = right.graph().transitions().len();
    let connection_count = spec.connections.len();
    let action_count = left
        .interface()
        .actions()
        .len()
        .checked_add(right.interface().actions().len())
        .ok_or(WorkPreflightError::CountOverflow)?;
    let mut work = WorkCounter {
        items: compatibility_work_items(left, right, spec)?,
    };

    // Preflight reads each component state header to count fields. Product-state
    // construction then visits every pair and copies every field.
    work.add(left_states)?;
    work.add(right_states)?;
    work.add(state_count)?;
    work.add_product(&[checked_model_field_count(left)?, right_states])?;
    work.add_product(&[checked_model_field_count(right)?, left_states])?;
    let initial_count = checked_work_product(&[
        left.graph().initial_states().len(),
        right.graph().initial_states().len(),
    ])?;
    // The product builds the initial vector and `FiniteGraph::new` validates it.
    work.add_product(&[2, initial_count])?;

    // Per-product-state dispatch plus worst-case stutter and independent-step
    // scans. Each component transition is revisited for every peer state.
    work.add(state_count)?;
    let left_peer_scans = checked_work_product(&[left_transitions, right_states])?;
    let right_peer_scans = checked_work_product(&[right_transitions, left_states])?;
    work.add_product(&[2, left_peer_scans])?;
    work.add_product(&[2, right_peer_scans])?;

    // Synchronization visits every connection at every product state, scans
    // left outgoing transitions, and may scan every right outgoing transition
    // for every left match.
    work.add_product(&[connection_count, state_count])?;
    work.add_product(&[connection_count, left_transitions, right_states])?;
    let synchronization_candidates =
        checked_work_product(&[connection_count, left_transitions, right_transitions])?;
    work.add(synchronization_candidates)?;

    let candidate_upper = state_count
        .checked_add(left_peer_scans)
        .and_then(|count| count.checked_add(right_peer_scans))
        .and_then(|count| count.checked_add(synchronization_candidates))
        .ok_or(WorkPreflightError::CountOverflow)?;
    let attempted_cap = limits.max_transitions.saturating_add(1);
    work.add(candidate_upper.min(attempted_cap))?;
    // If construction reaches canonicalization, `FiniteGraph` validates and
    // indexes transitions, then `OpenSystem` validates the canonical result.
    work.add_product(&[3, candidate_upper.min(limits.max_transitions)])?;
    // Interfaces are filtered, rebuilt, and validated. Connections are scanned
    // four times for the two connected-action sets, then once each for sync
    // declarations, interface construction, and final interface validation.
    work.add_product(&[3, action_count])?;
    work.add_product(&[7, connection_count])?;
    work.add(left.contract().guarantees().len())?;
    work.add(right.contract().guarantees().len())?;

    work.enforce(limits.max_work_items)
}

fn composition_work_error(error: WorkPreflightError) -> CompositionError {
    match error {
        WorkPreflightError::CountOverflow => CompositionError::WorkItemCountOverflow,
        WorkPreflightError::LimitExceeded { required, limit } => {
            CompositionError::WorkItemLimitExceeded { required, limit }
        }
    }
}

fn checked_product_state_count(
    left_states: usize,
    right_states: usize,
    limit: usize,
) -> Result<usize, CompositionError> {
    let required =
        left_states
            .checked_mul(right_states)
            .ok_or(CompositionError::StateCountOverflow {
                left_states,
                right_states,
            })?;
    if required > limit {
        Err(CompositionError::StateLimitExceeded { required, limit })
    } else {
        Ok(required)
    }
}

fn checked_product_index(
    left_state: StateId,
    right_state: StateId,
    right_states: usize,
) -> Result<StateId, CompositionError> {
    left_state
        .checked_mul(right_states)
        .and_then(|base| base.checked_add(right_state))
        .ok_or(CompositionError::StateIndexOverflow {
            left_state,
            right_state,
            right_states,
        })
}

fn push_transition<F>(
    transitions: &mut Vec<Transition>,
    limit: usize,
    make_transition: F,
) -> Result<(), CompositionError>
where
    F: FnOnce() -> Transition,
{
    let attempted = transitions
        .len()
        .checked_add(1)
        .ok_or(CompositionError::TransitionCountOverflow)?;
    if attempted > limit {
        return Err(CompositionError::TransitionLimitExceeded { attempted, limit });
    }
    transitions.push(make_transition());
    Ok(())
}

fn product_state(left: &ModelState, right: &ModelState) -> ModelState {
    left.iter()
        .map(|(field, value)| (namespaced(Side::Left, field), value.clone()))
        .chain(
            right
                .iter()
                .map(|(field, value)| (namespaced(Side::Right, field), value.clone())),
        )
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CongruenceSpec {
    pub local_refinement: RefinementSpec,
    pub concrete_composition: CompositionSpec,
    pub abstract_composition: CompositionSpec,
    pub peer_observation: ObservationMap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CongruenceIssue {
    LocalRefinementRejected,
    ConcreteCompositionIncompatible,
    AbstractCompositionIncompatible,
    LocalInterfaceActionUnmapped(String),
    MappedAbstractActionMissing {
        concrete_action: String,
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
    NonInjectiveVisibleBoundaryMapping {
        first_concrete_action: String,
        second_concrete_action: String,
        abstract_action: String,
    },
    HiddenConnectedAction(String),
    ConcreteConnectionNotPreserved(Connection),
    AbstractConnectionNotReflected(Connection),
    CompositionConstructionFailed,
    LiftedRefinementRejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CongruenceReport {
    pub accepted: bool,
    pub local_refinement: RefinementReport,
    pub concrete_compatibility: CompatibilityReport,
    pub abstract_compatibility: CompatibilityReport,
    pub lifted_refinement: Option<RefinementReport>,
    pub issues: Vec<CongruenceIssue>,
}

pub struct OpenRefinementCongruenceChecker;

impl OpenRefinementCongruenceChecker {
    /// Checks a finite instance of the repaired safety-congruence rule.
    ///
    /// The left concrete component is refined to the left abstract component;
    /// the right peer is unchanged. Acceptance checks local refinement, both
    /// compositions, injective boundary-signature preservation, connection
    /// reflection, the hidden-boundary exclusion, and finally the mechanically
    /// lifted product refinement. It makes no fairness or liveness claim.
    #[must_use]
    pub fn check(
        concrete: &OpenSystem,
        abstract_system: &OpenSystem,
        peer_system: &OpenSystem,
        spec: &CongruenceSpec,
    ) -> CongruenceReport {
        let local_refinement = RefinementChecker::check(
            concrete.graph(),
            abstract_system.graph(),
            &spec.local_refinement,
        );
        let concrete_compatibility =
            CompatibilityChecker::check(concrete, peer_system, &spec.concrete_composition);
        let abstract_compatibility =
            CompatibilityChecker::check(abstract_system, peer_system, &spec.abstract_composition);
        let mut issues = Vec::new();
        if !local_refinement.accepted {
            issues.push(CongruenceIssue::LocalRefinementRejected);
        }
        if !concrete_compatibility.accepted {
            issues.push(CongruenceIssue::ConcreteCompositionIncompatible);
        }
        if !abstract_compatibility.accepted {
            issues.push(CongruenceIssue::AbstractCompositionIncompatible);
        }

        check_interface_preservation(concrete, abstract_system, spec, &mut issues);
        let synchronization_map = check_connection_preservation(spec, &mut issues);

        let mut lifted_refinement = None;
        if issues.is_empty() {
            match (
                compose(concrete, peer_system, &spec.concrete_composition),
                compose(abstract_system, peer_system, &spec.abstract_composition),
            ) {
                (Ok(concrete_product), Ok(abstract_product)) => {
                    let lifted_spec = lifted_refinement_spec(
                        concrete,
                        abstract_system,
                        peer_system,
                        spec,
                        synchronization_map,
                    );
                    match lifted_spec {
                        Ok(lifted_spec) => {
                            let report = RefinementChecker::check(
                                concrete_product.graph(),
                                abstract_product.graph(),
                                &lifted_spec,
                            );
                            if !report.accepted {
                                issues.push(CongruenceIssue::LiftedRefinementRejected);
                            }
                            lifted_refinement = Some(report);
                        }
                        Err(_) => issues.push(CongruenceIssue::CompositionConstructionFailed),
                    }
                }
                _ => issues.push(CongruenceIssue::CompositionConstructionFailed),
            }
        }

        CongruenceReport {
            accepted: issues.is_empty()
                && lifted_refinement
                    .as_ref()
                    .is_some_and(|report| report.accepted),
            local_refinement,
            concrete_compatibility,
            abstract_compatibility,
            lifted_refinement,
            issues,
        }
    }
}

fn check_interface_preservation(
    concrete: &OpenSystem,
    abstract_system: &OpenSystem,
    spec: &CongruenceSpec,
    issues: &mut Vec<CongruenceIssue>,
) {
    let connected = spec
        .concrete_composition
        .connections
        .iter()
        .map(|connection| connection.left_action.as_str())
        .collect::<BTreeSet<_>>();
    let mut visible_boundary_images = BTreeMap::<String, String>::new();
    for (concrete_action, concrete_signature) in concrete.interface().actions() {
        match spec.local_refinement.actions.get(concrete_action) {
            None => issues.push(CongruenceIssue::LocalInterfaceActionUnmapped(
                concrete_action.clone(),
            )),
            Some(None) => {
                if connected.contains(concrete_action.as_str()) {
                    issues.push(CongruenceIssue::HiddenConnectedAction(
                        concrete_action.clone(),
                    ));
                }
            }
            Some(Some(abstract_action)) => {
                if concrete_signature.polarity != ActionPolarity::Internal {
                    if let Some(first_concrete_action) =
                        visible_boundary_images.get(abstract_action)
                    {
                        issues.push(CongruenceIssue::NonInjectiveVisibleBoundaryMapping {
                            first_concrete_action: first_concrete_action.clone(),
                            second_concrete_action: concrete_action.clone(),
                            abstract_action: abstract_action.to_owned(),
                        });
                    } else {
                        visible_boundary_images
                            .insert(abstract_action.to_owned(), concrete_action.clone());
                    }
                }
                let Some(abstract_signature) = abstract_system.interface().get(abstract_action)
                else {
                    issues.push(CongruenceIssue::MappedAbstractActionMissing {
                        concrete_action: concrete_action.clone(),
                        abstract_action: abstract_action.to_owned(),
                    });
                    continue;
                };
                if concrete_signature.polarity != abstract_signature.polarity {
                    issues.push(CongruenceIssue::PolarityNotPreserved {
                        concrete_action: concrete_action.clone(),
                        abstract_action: abstract_action.to_owned(),
                    });
                }
                if concrete_signature.channel != abstract_signature.channel {
                    issues.push(CongruenceIssue::ChannelNotPreserved {
                        concrete_action: concrete_action.clone(),
                        abstract_action: abstract_action.to_owned(),
                    });
                }
                if concrete_signature
                    .payload_type
                    .as_ref()
                    .map(PayloadType::id)
                    != abstract_signature
                        .payload_type
                        .as_ref()
                        .map(PayloadType::id)
                {
                    issues.push(CongruenceIssue::PayloadTypeNotPreserved {
                        concrete_action: concrete_action.clone(),
                        abstract_action: abstract_action.to_owned(),
                    });
                }
            }
        }
    }
}

fn check_connection_preservation(
    spec: &CongruenceSpec,
    issues: &mut Vec<CongruenceIssue>,
) -> BTreeMap<String, String> {
    let mut synchronization_map = BTreeMap::new();
    for concrete_connection in &spec.concrete_composition.connections {
        let Some(Some(mapped_action)) = spec
            .local_refinement
            .actions
            .get(&concrete_connection.left_action)
        else {
            issues.push(CongruenceIssue::ConcreteConnectionNotPreserved(
                concrete_connection.clone(),
            ));
            continue;
        };
        let matching = spec
            .abstract_composition
            .connections
            .iter()
            .find(|abstract_connection| {
                abstract_connection.left_action == mapped_action
                    && abstract_connection.right_action == concrete_connection.right_action
            });
        if let Some(abstract_connection) = matching {
            synchronization_map.insert(
                concrete_connection.composite_action.clone(),
                abstract_connection.composite_action.clone(),
            );
        } else {
            issues.push(CongruenceIssue::ConcreteConnectionNotPreserved(
                concrete_connection.clone(),
            ));
        }
    }
    for abstract_connection in &spec.abstract_composition.connections {
        let reflected = spec
            .concrete_composition
            .connections
            .iter()
            .any(|concrete_connection| {
                spec.local_refinement
                    .actions
                    .get(&concrete_connection.left_action)
                    == Some(Some(abstract_connection.left_action.as_str()))
                    && concrete_connection.right_action == abstract_connection.right_action
            });
        if !reflected {
            issues.push(CongruenceIssue::AbstractConnectionNotReflected(
                abstract_connection.clone(),
            ));
        }
    }
    synchronization_map
}

fn lifted_refinement_spec(
    concrete: &OpenSystem,
    _abstract_system: &OpenSystem,
    peer_system: &OpenSystem,
    spec: &CongruenceSpec,
    synchronization_map: BTreeMap<String, String>,
) -> Result<RefinementSpec, CompositionError> {
    let peer_count = peer_system.graph().states().len();
    let mut state_map = Vec::new();
    for concrete_state in 0..concrete.graph().states().len() {
        for peer_state in 0..peer_count {
            state_map.push(checked_product_index(
                spec.local_refinement.state_map[concrete_state],
                peer_state,
                peer_count,
            )?);
        }
    }

    let mut actions = Vec::<(String, Option<String>)>::new();
    actions.extend(spec.local_refinement.actions.mappings().iter().map(
        |(concrete_action, abstract_action)| {
            (
                namespaced(Side::Left, concrete_action),
                abstract_action
                    .as_ref()
                    .map(|action| namespaced(Side::Left, action)),
            )
        },
    ));
    actions.extend(peer_system.interface().actions().keys().map(|action| {
        let action = namespaced(Side::Right, action);
        (action.clone(), Some(action))
    }));
    actions.extend(
        synchronization_map
            .into_iter()
            .map(|(concrete_action, abstract_action)| (concrete_action, Some(abstract_action))),
    );

    Ok(RefinementSpec {
        state_map,
        concrete_observation: product_observation(
            &spec.local_refinement.concrete_observation,
            &spec.peer_observation,
        ),
        abstract_observation: product_observation(
            &spec.local_refinement.abstract_observation,
            &spec.peer_observation,
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
                    namespaced(Side::Left, source),
                    namespaced(Side::Left, output),
                )
            })
            .chain(right.fields().iter().map(|(source, output)| {
                (
                    namespaced(Side::Right, source),
                    namespaced(Side::Right, output),
                )
            })),
    )
    .expect("side namespaces preserve unique observation outputs")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Value;

    fn state(field: &str, value: bool) -> ModelState {
        BTreeMap::from([(field.to_owned(), Value::Bool(value))])
    }

    fn unit() -> PayloadType {
        PayloadType::unit()
    }

    fn system(
        graph: FiniteGraph,
        actions: impl IntoIterator<Item = (&'static str, ActionSignature)>,
    ) -> OpenSystem {
        let actions = actions.into_iter().collect::<Vec<_>>();
        let assumptions = actions
            .iter()
            .filter(|(_, signature)| signature.polarity == ActionPolarity::Input)
            .map(|(action, signature)| {
                (
                    *action,
                    PayloadPredicate::all(
                        signature
                            .payload_type
                            .as_ref()
                            .expect("input helper actions have payload types"),
                    ),
                )
            })
            .collect::<Vec<_>>();
        let guarantees = actions
            .iter()
            .filter(|(_, signature)| signature.polarity == ActionPolarity::Output)
            .map(|(action, signature)| {
                (
                    *action,
                    PayloadPredicate::all(
                        signature
                            .payload_type
                            .as_ref()
                            .expect("output helper actions have payload types"),
                    ),
                )
            })
            .collect::<Vec<_>>();
        OpenSystem::new(
            graph,
            Interface::new(actions).unwrap(),
            FiniteContract::new(assumptions, guarantees).unwrap(),
        )
        .unwrap()
    }

    fn sender(action: &'static str) -> OpenSystem {
        system(
            FiniteGraph::new(
                vec![state("visible", false), state("visible", true)],
                vec![0],
                vec![Transition::action(0, action, 1)],
            )
            .unwrap(),
            [(action, ActionSignature::output("bus", unit()))],
        )
    }

    fn peer() -> OpenSystem {
        system(
            FiniteGraph::new(
                vec![state("peer", false)],
                vec![0],
                vec![Transition::action(0, "receive", 0)],
            )
            .unwrap(),
            [("receive", ActionSignature::input("bus", unit()))],
        )
    }

    fn composition(left_action: &str, synchronized: &str) -> CompositionSpec {
        CompositionSpec::new(
            vec![Connection::new(left_action, "receive", synchronized)],
            vec![ContractLink::new(
                Side::Right,
                "receive",
                Side::Left,
                left_action,
            )],
        )
    }

    #[test]
    fn composes_connected_steps_and_discharges_by_finite_inclusion() {
        let product = compose(&sender("send"), &peer(), &composition("send", "transfer"))
            .expect("matching finite contracts compose");
        assert_eq!(product.graph().states().len(), 2);
        assert_eq!(product.graph().transitions().len(), 1);
        assert_eq!(
            product.graph().transitions()[0].kind.action(),
            Some("transfer")
        );
        assert_eq!(
            product.interface().get("transfer"),
            Some(&ActionSignature::internal())
        );
        assert!(product.contract().assumptions().is_empty());
    }

    #[test]
    fn preserves_namespaced_unconnected_boundary_contracts() {
        let right = system(
            FiniteGraph::new(vec![state("peer", false)], vec![0], vec![]).unwrap(),
            [],
        );
        let product = compose(&sender("send"), &right, &CompositionSpec::default()).unwrap();
        assert!(product.interface().get("left::send").is_some());
        assert!(product.contract().guarantees().contains_key("left::send"));
    }

    #[test]
    fn accepts_existing_finite_congruence_instance() {
        let concrete = sender("send");
        let abstract_system = sender("commit");
        let peer = peer();
        let report = OpenRefinementCongruenceChecker::check(
            &concrete,
            &abstract_system,
            &peer,
            &CongruenceSpec {
                local_refinement: RefinementSpec {
                    state_map: vec![0, 1],
                    concrete_observation: ObservationMap::identity(["visible"]),
                    abstract_observation: ObservationMap::identity(["visible"]),
                    actions: ActionHiding::new([("send", Some("commit"))]),
                },
                concrete_composition: composition("send", "transfer"),
                abstract_composition: composition("commit", "abstract-transfer"),
                peer_observation: ObservationMap::identity(["peer"]),
            },
        );
        assert!(report.accepted, "{:#?}", report.issues);
    }

    #[test]
    fn rejects_missing_contract_discharge() {
        let report = CompatibilityChecker::check(
            &sender("send"),
            &peer(),
            &CompositionSpec::new(vec![Connection::new("send", "receive", "transfer")], vec![]),
        );
        assert!(
            report
                .issues
                .contains(&CompatibilityIssue::UndischargedAssumption {
                    side: Side::Right,
                    action: "receive".to_owned(),
                })
        );
    }

    #[test]
    fn rejects_circular_contract_discharge() {
        let left = system(
            FiniteGraph::new(
                vec![state("left", false)],
                vec![0],
                vec![
                    Transition::action(0, "left-send", 0),
                    Transition::action(0, "left-receive", 0),
                ],
            )
            .unwrap(),
            [
                ("left-send", ActionSignature::output("a", unit())),
                ("left-receive", ActionSignature::input("b", unit())),
            ],
        );
        let right = system(
            FiniteGraph::new(
                vec![state("right", false)],
                vec![0],
                vec![
                    Transition::action(0, "right-receive", 0),
                    Transition::action(0, "right-send", 0),
                ],
            )
            .unwrap(),
            [
                ("right-receive", ActionSignature::input("a", unit())),
                ("right-send", ActionSignature::output("b", unit())),
            ],
        );
        let spec = CompositionSpec::new(
            vec![
                Connection::new("left-send", "right-receive", "a-transfer"),
                Connection::new("left-receive", "right-send", "b-transfer"),
            ],
            vec![
                ContractLink::new(Side::Right, "right-receive", Side::Left, "left-send"),
                ContractLink::new(Side::Left, "left-receive", Side::Right, "right-send"),
            ],
        );
        let report = CompatibilityChecker::check(&left, &right, &spec);
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            CompatibilityIssue::ConditionalGuaranteeProvider {
                provider: Side::Left,
                ..
            }
        )));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            CompatibilityIssue::ConditionalGuaranteeProvider {
                provider: Side::Right,
                ..
            }
        )));
    }

    #[test]
    fn rejects_connection_payload_substitution() {
        let other = PayloadType::enumeration("OtherUnit", ["unit"]).unwrap();
        let receiver = system(
            FiniteGraph::new(
                vec![state("peer", false)],
                vec![0],
                vec![Transition::action(0, "receive", 0)],
            )
            .unwrap(),
            [("receive", ActionSignature::input("bus", other))],
        );
        let report = CompatibilityChecker::check(
            &sender("send"),
            &receiver,
            &composition("send", "transfer"),
        );
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            CompatibilityIssue::PayloadTypeMismatch { .. }
                | CompatibilityIssue::ContractPayloadTypeMismatch(_)
        )));
    }

    #[test]
    fn rejects_nonreceptive_input_even_when_unreachable() {
        let receiver = system(
            FiniteGraph::new(
                vec![state("peer", false), state("peer", true)],
                vec![0],
                vec![Transition::action(0, "receive", 0)],
            )
            .unwrap(),
            [("receive", ActionSignature::input("bus", unit()))],
        );
        let report = CompatibilityChecker::check(
            &sender("send"),
            &receiver,
            &composition("send", "transfer"),
        );
        assert!(
            report
                .issues
                .contains(&CompatibilityIssue::InputNotReceptive {
                    side: Side::Right,
                    action: "receive".to_owned(),
                    state: 1,
                })
        );
    }

    #[test]
    fn composition_limits_fail_closed() {
        let error = compose_with_limits(
            &sender("send"),
            &peer(),
            &composition("send", "transfer"),
            CompositionLimits {
                max_work_items: 0,
                ..CompositionLimits::default()
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CompositionError::WorkItemLimitExceeded { .. }
        ));
    }
}

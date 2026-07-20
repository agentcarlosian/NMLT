//! Finite capability, grade, and rely/guarantee rules for M11-001c.
//!
//! The profile is deliberately nominal and finite. Capability identities and
//! rely/guarantee facts are canonical strings. Component refinement may not
//! widen owned or required authority, must preserve consumption and transfer,
//! must improve the resource upper bound, may rely on no additional facts,
//! and must preserve every abstract guaranteed fact. Parallel composition
//! requires disjoint ownership and exact synchronized transfer; peer facts
//! discharge the rely set of a connected input.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use nmlt_grades::{Grade, GradeError};

use crate::observation::ActionHiding;
use crate::open::{ActionPolarity, CompositionSpec, OpenSystem, Side};
use crate::refinement::RefinementSpec;

const LEFT_NAMESPACE: &str = "left::";
const RIGHT_NAMESPACE: &str = "right::";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceProfileError {
    InvalidAtom(String),
    DuplicateAtom(String),
    ConsumedWithoutRequirement(String),
    ReceivedAndRequired(String),
    DuplicateAction(String),
}

impl fmt::Display for ResourceProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAtom(atom) => write!(formatter, "invalid resource atom {atom:?}"),
            Self::DuplicateAtom(atom) => write!(formatter, "duplicate resource atom {atom:?}"),
            Self::ConsumedWithoutRequirement(capability) => write!(
                formatter,
                "consumed capability {capability:?} is not required by the action"
            ),
            Self::ReceivedAndRequired(capability) => write!(
                formatter,
                "received capability {capability:?} is also required before the action"
            ),
            Self::DuplicateAction(action) => {
                write!(formatter, "duplicate action resource profile {action:?}")
            }
        }
    }
}

impl std::error::Error for ResourceProfileError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionResourceProfile {
    required: BTreeSet<String>,
    consumed: BTreeSet<String>,
    transferred: BTreeSet<String>,
    received: BTreeSet<String>,
    grade: Grade,
    rely: BTreeSet<String>,
    guarantees: BTreeSet<String>,
}

impl ActionResourceProfile {
    pub fn new<R, C, T, I, F, G, RS, CS, TS, IS, FS, GS>(
        required: R,
        consumed: C,
        transferred: T,
        received: I,
        grade: Grade,
        rely: F,
        guarantees: G,
    ) -> Result<Self, ResourceProfileError>
    where
        R: IntoIterator<Item = RS>,
        C: IntoIterator<Item = CS>,
        T: IntoIterator<Item = TS>,
        I: IntoIterator<Item = IS>,
        F: IntoIterator<Item = FS>,
        G: IntoIterator<Item = GS>,
        RS: Into<String>,
        CS: Into<String>,
        TS: Into<String>,
        IS: Into<String>,
        FS: Into<String>,
        GS: Into<String>,
    {
        let required = collect_atoms(required)?;
        let consumed = collect_atoms(consumed)?;
        let transferred = collect_atoms(transferred)?;
        let received = collect_atoms(received)?;
        let rely = collect_atoms(rely)?;
        let guarantees = collect_atoms(guarantees)?;
        if let Some(capability) = consumed.difference(&required).next() {
            return Err(ResourceProfileError::ConsumedWithoutRequirement(
                capability.clone(),
            ));
        }
        if let Some(capability) = transferred.difference(&required).next() {
            return Err(ResourceProfileError::ConsumedWithoutRequirement(
                capability.clone(),
            ));
        }
        if let Some(capability) = received.intersection(&required).next() {
            return Err(ResourceProfileError::ReceivedAndRequired(
                capability.clone(),
            ));
        }
        Ok(Self {
            required,
            consumed,
            transferred,
            received,
            grade,
            rely,
            guarantees,
        })
    }

    #[must_use]
    pub fn inert() -> Self {
        Self {
            required: BTreeSet::new(),
            consumed: BTreeSet::new(),
            transferred: BTreeSet::new(),
            received: BTreeSet::new(),
            grade: Grade::ZERO,
            rely: BTreeSet::new(),
            guarantees: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn required(&self) -> &BTreeSet<String> {
        &self.required
    }

    #[must_use]
    pub fn consumed(&self) -> &BTreeSet<String> {
        &self.consumed
    }

    #[must_use]
    pub fn transferred(&self) -> &BTreeSet<String> {
        &self.transferred
    }

    #[must_use]
    pub fn received(&self) -> &BTreeSet<String> {
        &self.received
    }

    #[must_use]
    pub fn grade(&self) -> Grade {
        self.grade
    }

    #[must_use]
    pub fn rely(&self) -> &BTreeSet<String> {
        &self.rely
    }

    #[must_use]
    pub fn guarantees(&self) -> &BTreeSet<String> {
        &self.guarantees
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemResourceProfile {
    owned: BTreeSet<String>,
    actions: BTreeMap<String, ActionResourceProfile>,
}

impl SystemResourceProfile {
    pub fn new<O, A, OS, AS>(owned: O, actions: A) -> Result<Self, ResourceProfileError>
    where
        O: IntoIterator<Item = OS>,
        A: IntoIterator<Item = (AS, ActionResourceProfile)>,
        OS: Into<String>,
        AS: Into<String>,
    {
        let owned = collect_atoms(owned)?;
        let mut action_map = BTreeMap::new();
        for (action, profile) in actions {
            let action = action.into();
            if action.is_empty() || action.len() > 1024 {
                return Err(ResourceProfileError::InvalidAtom(action));
            }
            if action_map.insert(action.clone(), profile).is_some() {
                return Err(ResourceProfileError::DuplicateAction(action));
            }
        }
        Ok(Self {
            owned,
            actions: action_map,
        })
    }

    #[must_use]
    pub fn owned(&self) -> &BTreeSet<String> {
        &self.owned
    }

    #[must_use]
    pub fn actions(&self) -> &BTreeMap<String, ActionResourceProfile> {
        &self.actions
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceRefinementIssue {
    MissingConcreteAction(String),
    ExtraConcreteAction(String),
    MissingAbstractAction(String),
    AuthorityWidened(String),
    RequiredAuthorityWidened { action: String, capability: String },
    ConsumptionChanged(String),
    TransferChanged(String),
    GradeNotImproved(String),
    RelyStrengthened { action: String, fact: String },
    GuaranteeWeakened { action: String, fact: String },
    HiddenActionHasResources(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceRefinementReport {
    pub accepted: bool,
    pub checked_actions: usize,
    pub issues: Vec<ResourceRefinementIssue>,
}

pub struct ResourceRefinementChecker;

impl ResourceRefinementChecker {
    #[must_use]
    pub fn check(
        concrete_system: &OpenSystem,
        abstract_system: &OpenSystem,
        concrete: &SystemResourceProfile,
        abstract_profile: &SystemResourceProfile,
        refinement: &RefinementSpec,
    ) -> ResourceRefinementReport {
        let mut issues = Vec::new();
        for capability in concrete.owned.difference(&abstract_profile.owned) {
            issues.push(ResourceRefinementIssue::AuthorityWidened(
                capability.clone(),
            ));
        }
        for action in concrete_system.interface().actions().keys() {
            if !concrete.actions.contains_key(action) {
                issues.push(ResourceRefinementIssue::MissingConcreteAction(
                    action.clone(),
                ));
            }
        }
        for action in concrete.actions.keys() {
            if concrete_system.interface().get(action).is_none() {
                issues.push(ResourceRefinementIssue::ExtraConcreteAction(action.clone()));
            }
        }
        for action in abstract_system.interface().actions().keys() {
            if !abstract_profile.actions.contains_key(action) {
                issues.push(ResourceRefinementIssue::MissingAbstractAction(
                    action.clone(),
                ));
            }
        }

        let mut checked_actions = 0;
        for (concrete_action, concrete_resources) in &concrete.actions {
            let Some(mapping) = refinement.actions.get(concrete_action) else {
                continue;
            };
            let Some(abstract_action) = mapping else {
                if concrete_resources != &ActionResourceProfile::inert() {
                    issues.push(ResourceRefinementIssue::HiddenActionHasResources(
                        concrete_action.clone(),
                    ));
                }
                continue;
            };
            let Some(abstract_resources) = abstract_profile.actions.get(abstract_action) else {
                continue;
            };
            checked_actions += 1;
            for capability in concrete_resources
                .required
                .difference(&abstract_resources.required)
            {
                issues.push(ResourceRefinementIssue::RequiredAuthorityWidened {
                    action: concrete_action.clone(),
                    capability: capability.clone(),
                });
            }
            if concrete_resources.consumed != abstract_resources.consumed {
                issues.push(ResourceRefinementIssue::ConsumptionChanged(
                    concrete_action.clone(),
                ));
            }
            if concrete_resources.transferred != abstract_resources.transferred
                || concrete_resources.received != abstract_resources.received
            {
                issues.push(ResourceRefinementIssue::TransferChanged(
                    concrete_action.clone(),
                ));
            }
            if !concrete_resources
                .grade
                .componentwise_le(abstract_resources.grade)
            {
                issues.push(ResourceRefinementIssue::GradeNotImproved(
                    concrete_action.clone(),
                ));
            }
            for fact in concrete_resources.rely.difference(&abstract_resources.rely) {
                issues.push(ResourceRefinementIssue::RelyStrengthened {
                    action: concrete_action.clone(),
                    fact: fact.clone(),
                });
            }
            for fact in abstract_resources
                .guarantees
                .difference(&concrete_resources.guarantees)
            {
                issues.push(ResourceRefinementIssue::GuaranteeWeakened {
                    action: concrete_action.clone(),
                    fact: fact.clone(),
                });
            }
        }
        ResourceRefinementReport {
            accepted: issues.is_empty(),
            checked_actions,
            issues,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceCompositionIssue {
    CapabilityOwnershipOverlap(String),
    ActionRequiresUnowned {
        side: Side,
        action: String,
        capability: String,
    },
    ActionReceivesOwned {
        side: Side,
        action: String,
        capability: String,
    },
    RelyNotDischarged {
        consumer: Side,
        action: String,
        fact: String,
    },
    TransferMismatch {
        left_action: String,
        right_action: String,
    },
    GradeCompositionFailed {
        left_action: String,
        right_action: String,
    },
    UnknownAction {
        side: Side,
        action: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceCompositionReport {
    pub accepted: bool,
    pub checked_connections: usize,
    pub product: Option<SystemResourceProfile>,
    pub issues: Vec<ResourceCompositionIssue>,
}

pub struct ResourceCompositionChecker;

impl ResourceCompositionChecker {
    #[must_use]
    pub fn check(
        left_system: &OpenSystem,
        right_system: &OpenSystem,
        left: &SystemResourceProfile,
        right: &SystemResourceProfile,
        composition: &CompositionSpec,
    ) -> ResourceCompositionReport {
        let mut issues = Vec::new();
        for capability in left.owned.intersection(&right.owned) {
            issues.push(ResourceCompositionIssue::CapabilityOwnershipOverlap(
                capability.clone(),
            ));
        }
        validate_local_ownership(Side::Left, left, &mut issues);
        validate_local_ownership(Side::Right, right, &mut issues);

        let mut actions = BTreeMap::new();
        let connected_left = composition
            .connections
            .iter()
            .map(|connection| connection.left_action.as_str())
            .collect::<BTreeSet<_>>();
        let connected_right = composition
            .connections
            .iter()
            .map(|connection| connection.right_action.as_str())
            .collect::<BTreeSet<_>>();
        for (action, profile) in &left.actions {
            if !connected_left.contains(action.as_str()) {
                actions.insert(format!("{LEFT_NAMESPACE}{action}"), profile.clone());
            }
        }
        for (action, profile) in &right.actions {
            if !connected_right.contains(action.as_str()) {
                actions.insert(format!("{RIGHT_NAMESPACE}{action}"), profile.clone());
            }
        }

        let mut checked_connections = 0;
        for connection in &composition.connections {
            let Some(left_profile) = left.actions.get(&connection.left_action) else {
                issues.push(ResourceCompositionIssue::UnknownAction {
                    side: Side::Left,
                    action: connection.left_action.clone(),
                });
                continue;
            };
            let Some(right_profile) = right.actions.get(&connection.right_action) else {
                issues.push(ResourceCompositionIssue::UnknownAction {
                    side: Side::Right,
                    action: connection.right_action.clone(),
                });
                continue;
            };
            checked_connections += 1;
            discharge_connected_rely(
                left_system,
                right_system,
                connection.left_action.as_str(),
                connection.right_action.as_str(),
                left_profile,
                right_profile,
                &mut issues,
            );
            if left_profile.received != right_profile.transferred
                || right_profile.received != left_profile.transferred
            {
                issues.push(ResourceCompositionIssue::TransferMismatch {
                    left_action: connection.left_action.clone(),
                    right_action: connection.right_action.clone(),
                });
            }
            match compose_actions(left_profile, right_profile) {
                Ok(profile) => {
                    actions.insert(connection.composite_action.clone(), profile);
                }
                Err(_) => issues.push(ResourceCompositionIssue::GradeCompositionFailed {
                    left_action: connection.left_action.clone(),
                    right_action: connection.right_action.clone(),
                }),
            }
        }

        let product = if issues.is_empty() {
            let owned = left.owned.union(&right.owned).cloned().collect::<Vec<_>>();
            Some(SystemResourceProfile {
                owned: owned.into_iter().collect(),
                actions,
            })
        } else {
            None
        };
        ResourceCompositionReport {
            accepted: issues.is_empty(),
            checked_connections,
            product,
            issues,
        }
    }
}

fn validate_local_ownership(
    side: Side,
    profile: &SystemResourceProfile,
    issues: &mut Vec<ResourceCompositionIssue>,
) {
    for (action, resources) in &profile.actions {
        for capability in resources.required.difference(&profile.owned) {
            issues.push(ResourceCompositionIssue::ActionRequiresUnowned {
                side,
                action: action.clone(),
                capability: capability.clone(),
            });
        }
        for capability in resources.received.intersection(&profile.owned) {
            issues.push(ResourceCompositionIssue::ActionReceivesOwned {
                side,
                action: action.clone(),
                capability: capability.clone(),
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn discharge_connected_rely(
    left_system: &OpenSystem,
    right_system: &OpenSystem,
    left_action: &str,
    right_action: &str,
    left: &ActionResourceProfile,
    right: &ActionResourceProfile,
    issues: &mut Vec<ResourceCompositionIssue>,
) {
    let left_input = left_system
        .interface()
        .get(left_action)
        .is_some_and(|signature| signature.polarity == ActionPolarity::Input);
    let right_input = right_system
        .interface()
        .get(right_action)
        .is_some_and(|signature| signature.polarity == ActionPolarity::Input);
    if left_input {
        for fact in left.rely.difference(&right.guarantees) {
            issues.push(ResourceCompositionIssue::RelyNotDischarged {
                consumer: Side::Left,
                action: left_action.to_owned(),
                fact: fact.clone(),
            });
        }
    }
    if right_input {
        for fact in right.rely.difference(&left.guarantees) {
            issues.push(ResourceCompositionIssue::RelyNotDischarged {
                consumer: Side::Right,
                action: right_action.to_owned(),
                fact: fact.clone(),
            });
        }
    }
}

fn compose_actions(
    left: &ActionResourceProfile,
    right: &ActionResourceProfile,
) -> Result<ActionResourceProfile, GradeError> {
    let grade = left.grade.parallel(right.grade)?;
    let required = left
        .required
        .union(&right.required)
        .cloned()
        .collect::<Vec<_>>();
    let consumed = left
        .consumed
        .union(&right.consumed)
        .cloned()
        .collect::<Vec<_>>();
    let guarantees = left
        .guarantees
        .union(&right.guarantees)
        .cloned()
        .collect::<Vec<_>>();
    let rely = left
        .rely
        .difference(&right.guarantees)
        .chain(right.rely.difference(&left.guarantees))
        .cloned()
        .collect::<Vec<_>>();
    ActionResourceProfile::new(
        required,
        consumed,
        Vec::<String>::new(),
        Vec::<String>::new(),
        grade,
        rely,
        guarantees,
    )
    .map_err(|_| GradeError::ArithmeticOverflow {
        dimension: nmlt_grades::Dimension::CostTicks,
        operation: "resource-profile construction",
    })
}

fn collect_atoms<I, S>(values: I) -> Result<BTreeSet<String>, ResourceProfileError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut result = BTreeSet::new();
    for value in values {
        let value = value.into();
        if !canonical_atom(&value) {
            return Err(ResourceProfileError::InvalidAtom(value));
        }
        if !result.insert(value.clone()) {
            return Err(ResourceProfileError::DuplicateAtom(value));
        }
    }
    Ok(result)
}

fn canonical_atom(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        && value.len() <= 255
}

#[must_use]
pub fn mapped_product_resource_refinement(
    concrete: &SystemResourceProfile,
    abstract_profile: &SystemResourceProfile,
    actions: &ActionHiding,
) -> ResourceRefinementReport {
    let mut issues = Vec::new();
    for capability in concrete.owned.difference(&abstract_profile.owned) {
        issues.push(ResourceRefinementIssue::AuthorityWidened(
            capability.clone(),
        ));
    }
    let mut checked_actions = 0;
    for (concrete_action, concrete_resources) in &concrete.actions {
        let Some(Some(abstract_action)) = actions.get(concrete_action) else {
            issues.push(ResourceRefinementIssue::MissingConcreteAction(
                concrete_action.clone(),
            ));
            continue;
        };
        let Some(abstract_resources) = abstract_profile.actions.get(abstract_action) else {
            issues.push(ResourceRefinementIssue::MissingAbstractAction(
                abstract_action.to_owned(),
            ));
            continue;
        };
        checked_actions += 1;
        if !concrete_resources
            .required
            .is_subset(&abstract_resources.required)
        {
            for capability in concrete_resources
                .required
                .difference(&abstract_resources.required)
            {
                issues.push(ResourceRefinementIssue::RequiredAuthorityWidened {
                    action: concrete_action.clone(),
                    capability: capability.clone(),
                });
            }
        }
        if concrete_resources.consumed != abstract_resources.consumed {
            issues.push(ResourceRefinementIssue::ConsumptionChanged(
                concrete_action.clone(),
            ));
        }
        if concrete_resources.transferred != abstract_resources.transferred
            || concrete_resources.received != abstract_resources.received
        {
            issues.push(ResourceRefinementIssue::TransferChanged(
                concrete_action.clone(),
            ));
        }
        if !concrete_resources
            .grade
            .componentwise_le(abstract_resources.grade)
        {
            issues.push(ResourceRefinementIssue::GradeNotImproved(
                concrete_action.clone(),
            ));
        }
        for fact in concrete_resources.rely.difference(&abstract_resources.rely) {
            issues.push(ResourceRefinementIssue::RelyStrengthened {
                action: concrete_action.clone(),
                fact: fact.clone(),
            });
        }
        for fact in abstract_resources
            .guarantees
            .difference(&concrete_resources.guarantees)
        {
            issues.push(ResourceRefinementIssue::GuaranteeWeakened {
                action: concrete_action.clone(),
                fact: fact.clone(),
            });
        }
    }
    ResourceRefinementReport {
        accepted: issues.is_empty(),
        checked_actions,
        issues,
    }
}

//! Canonical finite-table encoding for the M11-001c Rust/Lean boundary.
//!
//! Rust strings are retained only in sorted dictionaries. Semantic maps and
//! predicates are emitted as natural-number indices and Boolean truth tables,
//! matching the executable reference representation in Lean. The supported
//! correspondence profile deliberately requires one exact nominal payload
//! universe across the four components, total visible action maps, and
//! surjective finite state maps.

use std::collections::BTreeSet;

use nmlt_grades::UncertaintyCertificate;
use nmlt_open_kernel as execution_kernel;

use crate::open::{ActionPolarity, CompositionSpec, OpenSystem};
use crate::open_contract::PayloadType;
use crate::open_resources::{ActionResourceProfile, SystemResourceProfile};
use crate::refinement::RefinementSpec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalGradeEncoding {
    pub cost_ticks: u64,
    pub privacy_micro_epsilon: u64,
    pub energy_microjoules: u64,
    pub uncertainty_family: Option<String>,
    pub uncertainty_profile: Option<String>,
    pub uncertainty_upper_bound_ppm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalResourceEncoding {
    pub required: Vec<String>,
    pub consumed: Vec<String>,
    pub transferred: Vec<String>,
    pub received: Vec<String>,
    pub grade: CanonicalGradeEncoding,
    pub rely: Vec<String>,
    pub guarantees: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalActionEncoding {
    pub name: String,
    pub polarity: ActionPolarity,
    pub channel: Option<String>,
    pub assumption: Vec<bool>,
    pub guarantee: Vec<bool>,
    pub resources: CanonicalResourceEncoding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalSystemEncoding {
    pub state_count: usize,
    pub owned_capabilities: Vec<String>,
    pub actions: Vec<CanonicalActionEncoding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalRefinementEncoding {
    pub state_map: Vec<usize>,
    /// Target action index for each concrete action in canonical action order.
    pub action_map: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalConnectionEncoding {
    pub left_action: usize,
    pub right_action: usize,
    pub composite_action: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalCongruenceEncoding {
    pub payload_name: String,
    pub payload_variants: Vec<String>,
    pub concrete_left: CanonicalSystemEncoding,
    pub abstract_left: CanonicalSystemEncoding,
    pub concrete_right: CanonicalSystemEncoding,
    pub abstract_right: CanonicalSystemEncoding,
    pub left_refinement: CanonicalRefinementEncoding,
    pub right_refinement: CanonicalRefinementEncoding,
    pub concrete_wiring: Vec<CanonicalConnectionEncoding>,
    pub abstract_wiring: Vec<CanonicalConnectionEncoding>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonicalSide {
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonicalValidationIssue {
    EmptyPayloadIdentity,
    DuplicatePayloadVariant(String),
    StateMapLengthMismatch {
        side: CanonicalSide,
    },
    StateMapOutOfRange {
        side: CanonicalSide,
        target: usize,
    },
    StateMapNotSurjective {
        side: CanonicalSide,
        target: usize,
    },
    ActionMapLengthMismatch {
        side: CanonicalSide,
    },
    ActionMapOutOfRange {
        side: CanonicalSide,
        target: usize,
    },
    ActionMapNotInjective {
        side: CanonicalSide,
        target: usize,
    },
    ActionMapNotSurjective {
        side: CanonicalSide,
        target: usize,
    },
    ActionIncompatible {
        side: CanonicalSide,
        action: String,
    },
    AuthorityWidened {
        side: CanonicalSide,
        capability: String,
    },
    WiringNotEquivalent,
    ExecutionKernelCapacityExceeded,
    ExecutionKernelReadbackMismatch,
    ExecutionKernelRejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalValidationReport {
    pub accepted: bool,
    pub issues: Vec<CanonicalValidationIssue>,
}

/// Executable form of the normalized finite predicate proved sound by
/// `OpenEncodingCorrespondence.check_sound`. This validator consumes only the
/// canonical certificate—never the richer Rust systems that produced it.
pub struct CanonicalEncodingValidator;

impl CanonicalEncodingValidator {
    #[must_use]
    pub fn check(encoding: &CanonicalCongruenceEncoding) -> CanonicalValidationReport {
        let mut issues = Vec::new();
        if encoding.payload_name.is_empty() {
            issues.push(CanonicalValidationIssue::EmptyPayloadIdentity);
        }
        let mut variants = BTreeSet::new();
        for variant in &encoding.payload_variants {
            if !variants.insert(variant) {
                issues.push(CanonicalValidationIssue::DuplicatePayloadVariant(
                    variant.clone(),
                ));
            }
        }
        validate_refinement(
            CanonicalSide::Left,
            encoding.payload_variants.len(),
            &encoding.concrete_left,
            &encoding.abstract_left,
            &encoding.left_refinement,
            &mut issues,
        );
        validate_refinement(
            CanonicalSide::Right,
            encoding.payload_variants.len(),
            &encoding.concrete_right,
            &encoding.abstract_right,
            &encoding.right_refinement,
            &mut issues,
        );
        validate_wiring(encoding, &mut issues);
        match encode_execution_kernel(encoding) {
            Some(certificate) if !execution_kernel_readback_matches(encoding, &certificate) => {
                issues.push(CanonicalValidationIssue::ExecutionKernelReadbackMismatch);
            }
            Some(certificate) if execution_kernel::check(certificate.input) => {}
            Some(_) => issues.push(CanonicalValidationIssue::ExecutionKernelRejected),
            None => issues.push(CanonicalValidationIssue::ExecutionKernelCapacityExceeded),
        }
        CanonicalValidationReport {
            accepted: issues.is_empty(),
            issues,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ExecutionKernelCertificate {
    pub(crate) atom_dictionary: Vec<String>,
    pub(crate) input: execution_kernel::Congruence,
}

pub(crate) fn encode_execution_kernel(
    encoding: &CanonicalCongruenceEncoding,
) -> Option<ExecutionKernelCertificate> {
    if encoding.payload_variants.len() > execution_kernel::MAX_PAYLOAD_VARIANTS {
        return None;
    }
    let dictionary = canonical_atom_dictionary(encoding);
    if dictionary.len() > u32::MAX as usize {
        return None;
    }
    let mut variants = BTreeSet::new();
    let variants_unique = encoding
        .payload_variants
        .iter()
        .all(|variant| variants.insert(variant));

    let input = execution_kernel::Congruence {
        payload_identity_present: !encoding.payload_name.is_empty(),
        payload_variants_unique: variants_unique,
        payload_cardinality: encoding.payload_variants.len(),
        concrete_left: encode_kernel_system(&encoding.concrete_left, &dictionary)?,
        abstract_left: encode_kernel_system(&encoding.abstract_left, &dictionary)?,
        concrete_right: encode_kernel_system(&encoding.concrete_right, &dictionary)?,
        abstract_right: encode_kernel_system(&encoding.abstract_right, &dictionary)?,
        left_refinement: encode_kernel_refinement(&encoding.left_refinement)?,
        right_refinement: encode_kernel_refinement(&encoding.right_refinement)?,
        concrete_wiring: encode_kernel_wiring(&encoding.concrete_wiring)?,
        abstract_wiring: encode_kernel_wiring(&encoding.abstract_wiring)?,
    };
    Some(ExecutionKernelCertificate {
        atom_dictionary: dictionary,
        input,
    })
}

pub(crate) fn execution_kernel_readback_matches(
    encoding: &CanonicalCongruenceEncoding,
    certificate: &ExecutionKernelCertificate,
) -> bool {
    let expected_dictionary = canonical_atom_dictionary(encoding);
    let mut variants = BTreeSet::new();
    let variants_unique = encoding
        .payload_variants
        .iter()
        .all(|variant| variants.insert(variant));
    certificate.atom_dictionary == expected_dictionary
        && certificate.input.payload_identity_present != encoding.payload_name.is_empty()
        && certificate.input.payload_variants_unique == variants_unique
        && certificate.input.payload_cardinality == encoding.payload_variants.len()
        && readback_kernel_system(
            &encoding.concrete_left,
            &certificate.atom_dictionary,
            &certificate.input.concrete_left,
        )
        && readback_kernel_system(
            &encoding.abstract_left,
            &certificate.atom_dictionary,
            &certificate.input.abstract_left,
        )
        && readback_kernel_system(
            &encoding.concrete_right,
            &certificate.atom_dictionary,
            &certificate.input.concrete_right,
        )
        && readback_kernel_system(
            &encoding.abstract_right,
            &certificate.atom_dictionary,
            &certificate.input.abstract_right,
        )
        && readback_kernel_refinement(
            &encoding.left_refinement,
            &certificate.input.left_refinement,
        )
        && readback_kernel_refinement(
            &encoding.right_refinement,
            &certificate.input.right_refinement,
        )
        && readback_kernel_wiring(
            &encoding.concrete_wiring,
            &certificate.input.concrete_wiring,
        )
        && readback_kernel_wiring(
            &encoding.abstract_wiring,
            &certificate.input.abstract_wiring,
        )
}

fn canonical_atom_dictionary(encoding: &CanonicalCongruenceEncoding) -> Vec<String> {
    let mut atoms = BTreeSet::new();
    for system in [
        &encoding.concrete_left,
        &encoding.abstract_left,
        &encoding.concrete_right,
        &encoding.abstract_right,
    ] {
        atoms.extend(system.owned_capabilities.iter().cloned());
        for action in &system.actions {
            if let Some(channel) = &action.channel {
                atoms.insert(channel.clone());
            }
            collect_resource_atoms(&action.resources, &mut atoms);
        }
    }
    atoms.into_iter().collect()
}

fn collect_resource_atoms(resources: &CanonicalResourceEncoding, atoms: &mut BTreeSet<String>) {
    for table in [
        &resources.required,
        &resources.consumed,
        &resources.transferred,
        &resources.received,
        &resources.rely,
        &resources.guarantees,
    ] {
        atoms.extend(table.iter().cloned());
    }
}

fn atom_id(dictionary: &[String], atom: &str) -> Option<u32> {
    dictionary
        .binary_search_by(|candidate| candidate.as_str().cmp(atom))
        .ok()
        .and_then(|index| u32::try_from(index).ok())
}

fn encode_kernel_atoms(
    atoms: &[String],
    dictionary: &[String],
) -> Option<execution_kernel::AtomTable> {
    if atoms.len() > execution_kernel::MAX_ATOMS {
        return None;
    }
    let mut values = [0; execution_kernel::MAX_ATOMS];
    for (index, atom) in atoms.iter().enumerate() {
        values[index] = atom_id(dictionary, atom)?;
    }
    Some(execution_kernel::AtomTable {
        len: atoms.len(),
        values,
    })
}

fn encode_kernel_predicate(bits: &[bool]) -> Option<execution_kernel::PredicateTable> {
    if bits.len() > execution_kernel::MAX_PAYLOAD_VARIANTS {
        return None;
    }
    let mut values = [false; execution_kernel::MAX_PAYLOAD_VARIANTS];
    values[..bits.len()].copy_from_slice(bits);
    Some(execution_kernel::PredicateTable {
        len: bits.len(),
        values,
    })
}

fn encode_kernel_resources(
    resources: &CanonicalResourceEncoding,
    dictionary: &[String],
) -> Option<execution_kernel::Resources> {
    Some(execution_kernel::Resources {
        required: encode_kernel_atoms(&resources.required, dictionary)?,
        consumed: encode_kernel_atoms(&resources.consumed, dictionary)?,
        transferred: encode_kernel_atoms(&resources.transferred, dictionary)?,
        received: encode_kernel_atoms(&resources.received, dictionary)?,
        grade: execution_kernel::Grade {
            cost: resources.grade.cost_ticks,
            privacy: resources.grade.privacy_micro_epsilon,
            energy: resources.grade.energy_microjoules,
            uncertainty: resources.grade.uncertainty_upper_bound_ppm,
        },
        rely: encode_kernel_atoms(&resources.rely, dictionary)?,
        guarantees: encode_kernel_atoms(&resources.guarantees, dictionary)?,
    })
}

fn encode_kernel_action(
    action: &CanonicalActionEncoding,
    dictionary: &[String],
) -> Option<execution_kernel::Action> {
    let polarity = match action.polarity {
        ActionPolarity::Internal => 0,
        ActionPolarity::Input => 1,
        ActionPolarity::Output => 2,
    };
    Some(execution_kernel::Action {
        polarity,
        channel: action
            .channel
            .as_ref()
            .map_or(Some(execution_kernel::NO_CHANNEL), |channel| {
                atom_id(dictionary, channel)
            })?,
        assumption: encode_kernel_predicate(&action.assumption)?,
        guarantee: encode_kernel_predicate(&action.guarantee)?,
        resources: encode_kernel_resources(&action.resources, dictionary)?,
    })
}

fn encode_kernel_system(
    system: &CanonicalSystemEncoding,
    dictionary: &[String],
) -> Option<execution_kernel::System> {
    if system.state_count > execution_kernel::MAX_STATES
        || system.actions.len() > execution_kernel::MAX_ACTIONS
    {
        return None;
    }
    let mut actions = [execution_kernel::Action::empty(); execution_kernel::MAX_ACTIONS];
    for (index, action) in system.actions.iter().enumerate() {
        actions[index] = encode_kernel_action(action, dictionary)?;
    }
    Some(execution_kernel::System {
        state_count: system.state_count,
        action_count: system.actions.len(),
        actions,
        owned: encode_kernel_atoms(&system.owned_capabilities, dictionary)?,
    })
}

fn encode_kernel_refinement(
    refinement: &CanonicalRefinementEncoding,
) -> Option<execution_kernel::Refinement> {
    if refinement.state_map.len() > execution_kernel::MAX_STATES
        || refinement.action_map.len() > execution_kernel::MAX_ACTIONS
    {
        return None;
    }
    let mut state_values = [0; execution_kernel::MAX_STATES];
    state_values[..refinement.state_map.len()].copy_from_slice(&refinement.state_map);
    let mut action_values = [0; execution_kernel::MAX_ACTIONS];
    action_values[..refinement.action_map.len()].copy_from_slice(&refinement.action_map);
    Some(execution_kernel::Refinement {
        state_map: execution_kernel::StateMap {
            len: refinement.state_map.len(),
            values: state_values,
        },
        action_map: execution_kernel::IndexTable {
            len: refinement.action_map.len(),
            values: action_values,
        },
    })
}

fn encode_kernel_wiring(
    wiring: &[CanonicalConnectionEncoding],
) -> Option<execution_kernel::ConnectionTable> {
    if wiring.len() > execution_kernel::MAX_CONNECTIONS {
        return None;
    }
    let mut left = [0; execution_kernel::MAX_CONNECTIONS];
    let mut right = [0; execution_kernel::MAX_CONNECTIONS];
    for (index, connection) in wiring.iter().enumerate() {
        left[index] = connection.left_action;
        right[index] = connection.right_action;
    }
    Some(execution_kernel::ConnectionTable {
        len: wiring.len(),
        left,
        right,
    })
}

fn readback_kernel_atoms(
    expected: &[String],
    dictionary: &[String],
    actual: &execution_kernel::AtomTable,
) -> bool {
    actual.len == expected.len()
        && actual.len <= execution_kernel::MAX_ATOMS
        && expected.iter().zip(actual.values).all(|(atom, id)| {
            usize::try_from(id)
                .ok()
                .and_then(|index| dictionary.get(index))
                == Some(atom)
        })
}

fn readback_kernel_predicate(expected: &[bool], actual: &execution_kernel::PredicateTable) -> bool {
    actual.len == expected.len()
        && actual.len <= execution_kernel::MAX_PAYLOAD_VARIANTS
        && actual.values[..actual.len] == *expected
}

fn readback_kernel_resources(
    expected: &CanonicalResourceEncoding,
    dictionary: &[String],
    actual: &execution_kernel::Resources,
) -> bool {
    readback_kernel_atoms(&expected.required, dictionary, &actual.required)
        && readback_kernel_atoms(&expected.consumed, dictionary, &actual.consumed)
        && readback_kernel_atoms(&expected.transferred, dictionary, &actual.transferred)
        && readback_kernel_atoms(&expected.received, dictionary, &actual.received)
        && actual.grade.cost == expected.grade.cost_ticks
        && actual.grade.privacy == expected.grade.privacy_micro_epsilon
        && actual.grade.energy == expected.grade.energy_microjoules
        && actual.grade.uncertainty == expected.grade.uncertainty_upper_bound_ppm
        && readback_kernel_atoms(&expected.rely, dictionary, &actual.rely)
        && readback_kernel_atoms(&expected.guarantees, dictionary, &actual.guarantees)
}

fn readback_kernel_action(
    expected: &CanonicalActionEncoding,
    dictionary: &[String],
    actual: &execution_kernel::Action,
) -> bool {
    let polarity = match expected.polarity {
        ActionPolarity::Internal => 0,
        ActionPolarity::Input => 1,
        ActionPolarity::Output => 2,
    };
    let channel_matches = match &expected.channel {
        None => actual.channel == execution_kernel::NO_CHANNEL,
        Some(channel) => {
            usize::try_from(actual.channel)
                .ok()
                .and_then(|index| dictionary.get(index))
                == Some(channel)
        }
    };
    actual.polarity == polarity
        && channel_matches
        && readback_kernel_predicate(&expected.assumption, &actual.assumption)
        && readback_kernel_predicate(&expected.guarantee, &actual.guarantee)
        && readback_kernel_resources(&expected.resources, dictionary, &actual.resources)
}

fn readback_kernel_system(
    expected: &CanonicalSystemEncoding,
    dictionary: &[String],
    actual: &execution_kernel::System,
) -> bool {
    actual.state_count == expected.state_count
        && actual.action_count == expected.actions.len()
        && actual.action_count <= execution_kernel::MAX_ACTIONS
        && expected
            .actions
            .iter()
            .zip(actual.actions)
            .all(|(expected_action, actual_action)| {
                readback_kernel_action(expected_action, dictionary, &actual_action)
            })
        && readback_kernel_atoms(&expected.owned_capabilities, dictionary, &actual.owned)
}

fn readback_kernel_refinement(
    expected: &CanonicalRefinementEncoding,
    actual: &execution_kernel::Refinement,
) -> bool {
    actual.state_map.len == expected.state_map.len()
        && actual.state_map.len <= execution_kernel::MAX_STATES
        && actual.state_map.values[..actual.state_map.len] == *expected.state_map
        && actual.action_map.len == expected.action_map.len()
        && actual.action_map.len <= execution_kernel::MAX_ACTIONS
        && actual.action_map.values[..actual.action_map.len] == *expected.action_map
}

fn readback_kernel_wiring(
    expected: &[CanonicalConnectionEncoding],
    actual: &execution_kernel::ConnectionTable,
) -> bool {
    actual.len == expected.len()
        && actual.len <= execution_kernel::MAX_CONNECTIONS
        && expected.iter().enumerate().all(|(index, connection)| {
            actual.left[index] == connection.left_action
                && actual.right[index] == connection.right_action
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncodingCorrespondenceIssue {
    NoBoundaryPayload,
    NonUniformPayloadType { action: String },
    HiddenActionUnsupported { action: String },
    ActionTargetMissing { action: String, target: String },
    StateMapNotSurjective { abstract_state: usize },
    MissingResourceAction(String),
    ConnectionEndpointMissing { action: String },
    CanonicalSourceReadbackMismatch,
    CanonicalValidationFailed(CanonicalValidationIssue),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodingCorrespondenceReport {
    pub accepted: bool,
    pub encoding: Option<CanonicalCongruenceEncoding>,
    pub issues: Vec<EncodingCorrespondenceIssue>,
}

pub struct EncodingCorrespondenceChecker;

impl EncodingCorrespondenceChecker {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn check(
        concrete_left: &OpenSystem,
        abstract_left: &OpenSystem,
        concrete_right: &OpenSystem,
        abstract_right: &OpenSystem,
        left_refinement: &RefinementSpec,
        right_refinement: &RefinementSpec,
        concrete_composition: &CompositionSpec,
        abstract_composition: &CompositionSpec,
        concrete_left_resources: &SystemResourceProfile,
        abstract_left_resources: &SystemResourceProfile,
        concrete_right_resources: &SystemResourceProfile,
        abstract_right_resources: &SystemResourceProfile,
    ) -> EncodingCorrespondenceReport {
        let mut issues = Vec::new();
        let systems = [concrete_left, abstract_left, concrete_right, abstract_right];
        let common_payload = systems
            .iter()
            .flat_map(|system| system.interface().actions().values())
            .find_map(|signature| signature.payload_type.as_ref())
            .cloned();
        let Some(common_payload) = common_payload else {
            return EncodingCorrespondenceReport {
                accepted: false,
                encoding: None,
                issues: vec![EncodingCorrespondenceIssue::NoBoundaryPayload],
            };
        };
        for system in systems {
            for (action, signature) in system.interface().actions() {
                if signature.polarity != ActionPolarity::Internal
                    && signature.payload_type.as_ref() != Some(&common_payload)
                {
                    issues.push(EncodingCorrespondenceIssue::NonUniformPayloadType {
                        action: action.clone(),
                    });
                }
            }
        }
        check_state_surjective(
            left_refinement,
            abstract_left.graph().states().len(),
            &mut issues,
        );
        check_state_surjective(
            right_refinement,
            abstract_right.graph().states().len(),
            &mut issues,
        );

        let payload_variants = common_payload
            .variants()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let concrete_left_encoding = encode_system(
            concrete_left,
            concrete_left_resources,
            &common_payload,
            &payload_variants,
            &mut issues,
        );
        let abstract_left_encoding = encode_system(
            abstract_left,
            abstract_left_resources,
            &common_payload,
            &payload_variants,
            &mut issues,
        );
        let concrete_right_encoding = encode_system(
            concrete_right,
            concrete_right_resources,
            &common_payload,
            &payload_variants,
            &mut issues,
        );
        let abstract_right_encoding = encode_system(
            abstract_right,
            abstract_right_resources,
            &common_payload,
            &payload_variants,
            &mut issues,
        );
        let left_map = encode_refinement(
            &concrete_left_encoding,
            &abstract_left_encoding,
            left_refinement,
            &mut issues,
        );
        let right_map = encode_refinement(
            &concrete_right_encoding,
            &abstract_right_encoding,
            right_refinement,
            &mut issues,
        );
        let concrete_wiring = encode_wiring(
            &concrete_left_encoding,
            &concrete_right_encoding,
            concrete_composition,
            &mut issues,
        );
        let abstract_wiring = encode_wiring(
            &abstract_left_encoding,
            &abstract_right_encoding,
            abstract_composition,
            &mut issues,
        );

        let candidate = issues.is_empty().then(|| CanonicalCongruenceEncoding {
            payload_name: common_payload.name().to_owned(),
            payload_variants,
            concrete_left: concrete_left_encoding,
            abstract_left: abstract_left_encoding,
            concrete_right: concrete_right_encoding,
            abstract_right: abstract_right_encoding,
            left_refinement: CanonicalRefinementEncoding {
                state_map: left_refinement.state_map.clone(),
                action_map: left_map,
            },
            right_refinement: CanonicalRefinementEncoding {
                state_map: right_refinement.state_map.clone(),
                action_map: right_map,
            },
            concrete_wiring,
            abstract_wiring,
        });
        if let Some(candidate) = &candidate {
            if !canonical_source_readback_matches(
                candidate,
                &common_payload,
                concrete_left,
                abstract_left,
                concrete_right,
                abstract_right,
                left_refinement,
                right_refinement,
                concrete_composition,
                abstract_composition,
                concrete_left_resources,
                abstract_left_resources,
                concrete_right_resources,
                abstract_right_resources,
            ) {
                issues.push(EncodingCorrespondenceIssue::CanonicalSourceReadbackMismatch);
            }
            let validation = CanonicalEncodingValidator::check(candidate);
            issues.extend(
                validation
                    .issues
                    .into_iter()
                    .map(EncodingCorrespondenceIssue::CanonicalValidationFailed),
            );
        }
        let encoding = issues.is_empty().then_some(candidate).flatten();
        EncodingCorrespondenceReport {
            accepted: encoding.is_some(),
            encoding,
            issues,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn canonical_source_readback_matches(
    encoding: &CanonicalCongruenceEncoding,
    payload: &PayloadType,
    concrete_left: &OpenSystem,
    abstract_left: &OpenSystem,
    concrete_right: &OpenSystem,
    abstract_right: &OpenSystem,
    left_refinement: &RefinementSpec,
    right_refinement: &RefinementSpec,
    concrete_composition: &CompositionSpec,
    abstract_composition: &CompositionSpec,
    concrete_left_resources: &SystemResourceProfile,
    abstract_left_resources: &SystemResourceProfile,
    concrete_right_resources: &SystemResourceProfile,
    abstract_right_resources: &SystemResourceProfile,
) -> bool {
    encoding.payload_name == payload.name()
        && encoding.payload_variants == payload.variants().iter().cloned().collect::<Vec<_>>()
        && canonical_system_readback_matches(
            &encoding.concrete_left,
            concrete_left,
            concrete_left_resources,
            payload,
        )
        && canonical_system_readback_matches(
            &encoding.abstract_left,
            abstract_left,
            abstract_left_resources,
            payload,
        )
        && canonical_system_readback_matches(
            &encoding.concrete_right,
            concrete_right,
            concrete_right_resources,
            payload,
        )
        && canonical_system_readback_matches(
            &encoding.abstract_right,
            abstract_right,
            abstract_right_resources,
            payload,
        )
        && canonical_refinement_readback_matches(
            &encoding.left_refinement,
            &encoding.concrete_left,
            &encoding.abstract_left,
            left_refinement,
        )
        && canonical_refinement_readback_matches(
            &encoding.right_refinement,
            &encoding.concrete_right,
            &encoding.abstract_right,
            right_refinement,
        )
        && canonical_wiring_readback_matches(
            &encoding.concrete_wiring,
            &encoding.concrete_left,
            &encoding.concrete_right,
            concrete_composition,
        )
        && canonical_wiring_readback_matches(
            &encoding.abstract_wiring,
            &encoding.abstract_left,
            &encoding.abstract_right,
            abstract_composition,
        )
}

fn canonical_system_readback_matches(
    encoding: &CanonicalSystemEncoding,
    system: &OpenSystem,
    resources: &SystemResourceProfile,
    payload: &PayloadType,
) -> bool {
    let variants = payload.variants().iter().cloned().collect::<Vec<_>>();
    encoding.state_count == system.graph().states().len()
        && encoding.owned_capabilities == resources.owned().iter().cloned().collect::<Vec<_>>()
        && encoding.actions.len() == system.interface().actions().len()
        && encoding
            .actions
            .iter()
            .zip(system.interface().actions())
            .all(|(encoded, (name, signature))| {
                let Some(resource) = resources.actions().get(name) else {
                    return false;
                };
                encoded.name == *name
                    && encoded.polarity == signature.polarity
                    && encoded.channel == signature.channel
                    && encoded.assumption
                        == predicate_bits(
                            system.contract().assumptions().get(name),
                            payload,
                            &variants,
                        )
                    && encoded.guarantee
                        == predicate_bits(
                            system.contract().guarantees().get(name),
                            payload,
                            &variants,
                        )
                    && canonical_resource_readback_matches(&encoded.resources, resource)
            })
}

fn canonical_resource_readback_matches(
    encoding: &CanonicalResourceEncoding,
    profile: &ActionResourceProfile,
) -> bool {
    let uncertainty = profile.grade().uncertainty();
    let (family, certificate_profile) = match uncertainty {
        UncertaintyCertificate::Certain => (None, None),
        UncertaintyCertificate::UpperBound {
            family, profile_id, ..
        } => (
            Some(family.as_str().to_owned()),
            Some(profile_id.to_string()),
        ),
    };
    encoding.required == profile.required().iter().cloned().collect::<Vec<_>>()
        && encoding.consumed == profile.consumed().iter().cloned().collect::<Vec<_>>()
        && encoding.transferred == profile.transferred().iter().cloned().collect::<Vec<_>>()
        && encoding.received == profile.received().iter().cloned().collect::<Vec<_>>()
        && encoding.grade.cost_ticks == profile.grade().cost_ticks()
        && encoding.grade.privacy_micro_epsilon == profile.grade().privacy_micro_epsilon()
        && encoding.grade.energy_microjoules == profile.grade().energy_microjoules()
        && encoding.grade.uncertainty_family == family
        && encoding.grade.uncertainty_profile == certificate_profile
        && encoding.grade.uncertainty_upper_bound_ppm == uncertainty.upper_bound_ppm()
        && encoding.rely == profile.rely().iter().cloned().collect::<Vec<_>>()
        && encoding.guarantees == profile.guarantees().iter().cloned().collect::<Vec<_>>()
}

fn canonical_refinement_readback_matches(
    encoding: &CanonicalRefinementEncoding,
    concrete: &CanonicalSystemEncoding,
    abstract_system: &CanonicalSystemEncoding,
    refinement: &RefinementSpec,
) -> bool {
    encoding.state_map == refinement.state_map
        && encoding.action_map.len() == concrete.actions.len()
        && concrete
            .actions
            .iter()
            .zip(&encoding.action_map)
            .all(|(action, &target_index)| {
                let Some(Some(target_name)) = refinement.actions.get(&action.name) else {
                    return false;
                };
                abstract_system
                    .actions
                    .get(target_index)
                    .is_some_and(|target| target.name == target_name)
            })
}

fn canonical_wiring_readback_matches(
    encoding: &[CanonicalConnectionEncoding],
    left: &CanonicalSystemEncoding,
    right: &CanonicalSystemEncoding,
    composition: &CompositionSpec,
) -> bool {
    encoding.len() == composition.connections.len()
        && encoding
            .iter()
            .zip(&composition.connections)
            .all(|(encoded, source)| {
                left.actions
                    .get(encoded.left_action)
                    .is_some_and(|action| action.name == source.left_action)
                    && right
                        .actions
                        .get(encoded.right_action)
                        .is_some_and(|action| action.name == source.right_action)
                    && encoded.composite_action == source.composite_action
            })
}

fn validate_refinement(
    side: CanonicalSide,
    payload_cardinality: usize,
    concrete: &CanonicalSystemEncoding,
    abstract_system: &CanonicalSystemEncoding,
    refinement: &CanonicalRefinementEncoding,
    issues: &mut Vec<CanonicalValidationIssue>,
) {
    if refinement.state_map.len() != concrete.state_count {
        issues.push(CanonicalValidationIssue::StateMapLengthMismatch { side });
    }
    let state_image = refinement
        .state_map
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for &target in &refinement.state_map {
        if target >= abstract_system.state_count {
            issues.push(CanonicalValidationIssue::StateMapOutOfRange { side, target });
        }
    }
    for target in 0..abstract_system.state_count {
        if !state_image.contains(&target) {
            issues.push(CanonicalValidationIssue::StateMapNotSurjective { side, target });
        }
    }

    if refinement.action_map.len() != concrete.actions.len() {
        issues.push(CanonicalValidationIssue::ActionMapLengthMismatch { side });
    }
    let mut action_image = BTreeSet::new();
    for &target in &refinement.action_map {
        if target >= abstract_system.actions.len() {
            issues.push(CanonicalValidationIssue::ActionMapOutOfRange { side, target });
        } else if !action_image.insert(target) {
            issues.push(CanonicalValidationIssue::ActionMapNotInjective { side, target });
        }
    }
    for target in 0..abstract_system.actions.len() {
        if !action_image.contains(&target) {
            issues.push(CanonicalValidationIssue::ActionMapNotSurjective { side, target });
        }
    }
    for (source, &target) in refinement.action_map.iter().enumerate() {
        let Some(concrete_action) = concrete.actions.get(source) else {
            continue;
        };
        let Some(abstract_action) = abstract_system.actions.get(target) else {
            continue;
        };
        if !canonical_action_compatible(payload_cardinality, concrete_action, abstract_action) {
            issues.push(CanonicalValidationIssue::ActionIncompatible {
                side,
                action: concrete_action.name.clone(),
            });
        }
    }
    for capability in &concrete.owned_capabilities {
        if !abstract_system.owned_capabilities.contains(capability) {
            issues.push(CanonicalValidationIssue::AuthorityWidened {
                side,
                capability: capability.clone(),
            });
        }
    }
}

fn canonical_action_compatible(
    payload_cardinality: usize,
    concrete: &CanonicalActionEncoding,
    abstract_action: &CanonicalActionEncoding,
) -> bool {
    concrete.polarity == abstract_action.polarity
        && concrete.channel == abstract_action.channel
        && concrete.assumption.len() == payload_cardinality
        && concrete.guarantee.len() == payload_cardinality
        && abstract_action.assumption.len() == payload_cardinality
        && abstract_action.guarantee.len() == payload_cardinality
        && predicate_subset(&abstract_action.assumption, &concrete.assumption)
        && predicate_subset(&concrete.guarantee, &abstract_action.guarantee)
        && canonical_resources_compatible(&concrete.resources, &abstract_action.resources)
}

fn predicate_subset(left: &[bool], right: &[bool]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(required, provided)| !required || *provided)
}

fn canonical_resources_compatible(
    concrete: &CanonicalResourceEncoding,
    abstract_resource: &CanonicalResourceEncoding,
) -> bool {
    concrete
        .required
        .iter()
        .all(|capability| abstract_resource.required.contains(capability))
        && concrete.consumed == abstract_resource.consumed
        && concrete.transferred == abstract_resource.transferred
        && concrete.received == abstract_resource.received
        && concrete.grade.cost_ticks <= abstract_resource.grade.cost_ticks
        && concrete.grade.privacy_micro_epsilon <= abstract_resource.grade.privacy_micro_epsilon
        && concrete.grade.energy_microjoules <= abstract_resource.grade.energy_microjoules
        && concrete.grade.uncertainty_upper_bound_ppm
            <= abstract_resource.grade.uncertainty_upper_bound_ppm
        && concrete
            .rely
            .iter()
            .all(|fact| abstract_resource.rely.contains(fact))
        && abstract_resource
            .guarantees
            .iter()
            .all(|fact| concrete.guarantees.contains(fact))
}

fn validate_wiring(
    encoding: &CanonicalCongruenceEncoding,
    issues: &mut Vec<CanonicalValidationIssue>,
) {
    let mapped = encoding
        .concrete_wiring
        .iter()
        .filter_map(|edge| {
            Some((
                *encoding.left_refinement.action_map.get(edge.left_action)?,
                *encoding
                    .right_refinement
                    .action_map
                    .get(edge.right_action)?,
            ))
        })
        .collect::<Vec<_>>();
    let abstract_edges = encoding
        .abstract_wiring
        .iter()
        .map(|edge| (edge.left_action, edge.right_action))
        .collect::<Vec<_>>();
    if mapped.len() != encoding.concrete_wiring.len()
        || mapped.iter().any(|edge| !abstract_edges.contains(edge))
        || abstract_edges.iter().any(|edge| !mapped.contains(edge))
    {
        issues.push(CanonicalValidationIssue::WiringNotEquivalent);
    }
}

fn check_state_surjective(
    refinement: &RefinementSpec,
    abstract_states: usize,
    issues: &mut Vec<EncodingCorrespondenceIssue>,
) {
    let image = refinement
        .state_map
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for abstract_state in 0..abstract_states {
        if !image.contains(&abstract_state) {
            issues.push(EncodingCorrespondenceIssue::StateMapNotSurjective { abstract_state });
        }
    }
}

fn encode_system(
    system: &OpenSystem,
    resources: &SystemResourceProfile,
    common_payload: &PayloadType,
    variants: &[String],
    issues: &mut Vec<EncodingCorrespondenceIssue>,
) -> CanonicalSystemEncoding {
    let actions = system
        .interface()
        .actions()
        .iter()
        .filter_map(|(name, signature)| {
            let Some(resource) = resources.actions().get(name) else {
                issues.push(EncodingCorrespondenceIssue::MissingResourceAction(
                    name.clone(),
                ));
                return None;
            };
            let assumption = system.contract().assumptions().get(name);
            let guarantee = system.contract().guarantees().get(name);
            Some(CanonicalActionEncoding {
                name: name.clone(),
                polarity: signature.polarity,
                channel: signature.channel.clone(),
                assumption: predicate_bits(assumption, common_payload, variants),
                guarantee: predicate_bits(guarantee, common_payload, variants),
                resources: encode_resources(resource),
            })
        })
        .collect();
    CanonicalSystemEncoding {
        state_count: system.graph().states().len(),
        owned_capabilities: resources.owned().iter().cloned().collect(),
        actions,
    }
}

fn predicate_bits(
    predicate: Option<&crate::open_contract::PayloadPredicate>,
    common_payload: &PayloadType,
    variants: &[String],
) -> Vec<bool> {
    predicate.map_or_else(
        || vec![false; variants.len()],
        |predicate| {
            if predicate.payload_type() != common_payload.id() {
                vec![false; variants.len()]
            } else {
                variants
                    .iter()
                    .map(|variant| predicate.accepted().contains(variant))
                    .collect()
            }
        },
    )
}

fn encode_resources(profile: &ActionResourceProfile) -> CanonicalResourceEncoding {
    let uncertainty = profile.grade().uncertainty();
    let (family, certificate_profile) = match uncertainty {
        UncertaintyCertificate::Certain => (None, None),
        UncertaintyCertificate::UpperBound {
            family, profile_id, ..
        } => (
            Some(family.as_str().to_owned()),
            Some(profile_id.to_string()),
        ),
    };
    CanonicalResourceEncoding {
        required: profile.required().iter().cloned().collect(),
        consumed: profile.consumed().iter().cloned().collect(),
        transferred: profile.transferred().iter().cloned().collect(),
        received: profile.received().iter().cloned().collect(),
        grade: CanonicalGradeEncoding {
            cost_ticks: profile.grade().cost_ticks(),
            privacy_micro_epsilon: profile.grade().privacy_micro_epsilon(),
            energy_microjoules: profile.grade().energy_microjoules(),
            uncertainty_family: family,
            uncertainty_profile: certificate_profile,
            uncertainty_upper_bound_ppm: uncertainty.upper_bound_ppm(),
        },
        rely: profile.rely().iter().cloned().collect(),
        guarantees: profile.guarantees().iter().cloned().collect(),
    }
}

fn encode_refinement(
    concrete: &CanonicalSystemEncoding,
    abstract_system: &CanonicalSystemEncoding,
    refinement: &RefinementSpec,
    issues: &mut Vec<EncodingCorrespondenceIssue>,
) -> Vec<usize> {
    let mut result = Vec::new();
    for action in &concrete.actions {
        let Some(Some(target)) = refinement.actions.get(&action.name) else {
            issues.push(EncodingCorrespondenceIssue::HiddenActionUnsupported {
                action: action.name.clone(),
            });
            continue;
        };
        let Some(index) = abstract_system
            .actions
            .iter()
            .position(|candidate| candidate.name == target)
        else {
            issues.push(EncodingCorrespondenceIssue::ActionTargetMissing {
                action: action.name.clone(),
                target: target.to_owned(),
            });
            continue;
        };
        result.push(index);
    }
    result
}

fn encode_wiring(
    left: &CanonicalSystemEncoding,
    right: &CanonicalSystemEncoding,
    composition: &CompositionSpec,
    issues: &mut Vec<EncodingCorrespondenceIssue>,
) -> Vec<CanonicalConnectionEncoding> {
    composition
        .connections
        .iter()
        .filter_map(|connection| {
            let left_index = left
                .actions
                .iter()
                .position(|action| action.name == connection.left_action);
            let right_index = right
                .actions
                .iter()
                .position(|action| action.name == connection.right_action);
            match (left_index, right_index) {
                (Some(left_action), Some(right_action)) => Some(CanonicalConnectionEncoding {
                    left_action,
                    right_action,
                    composite_action: connection.composite_action.clone(),
                }),
                _ => {
                    let missing = if left_index.is_none() {
                        connection.left_action.clone()
                    } else {
                        connection.right_action.clone()
                    };
                    issues.push(EncodingCorrespondenceIssue::ConnectionEndpointMissing {
                        action: missing,
                    });
                    None
                }
            }
        })
        .collect()
}

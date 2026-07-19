use std::collections::BTreeMap;
use std::fmt;

use nmlt_hir::{ModuleMapId, ResolutionId, SourceSetId, SurfaceProgramId, sha256_bytes};
use nmlt_ir::{CoreProgramId, CoreType};

use crate::model::{
    DerivationConclusion, DerivationNode, DerivationWitness, ElaborationRule, ObligationKey,
};

const DERIVATION_DOMAIN: &[u8] = b"NMLT-DERIVATION-NODE\0v1\0";
const CERTIFICATE_DOMAIN: &[u8] = b"NMLT-ELABORATION-CERTIFICATE\0v1\0";
const RULESET_DOMAIN: &[u8] = b"NMLT-RULESET-BUNDLE\0v1\0";
const POLICY_DOMAIN: &[u8] = b"NMLT-KERNEL-POLICY\0v1\0";

macro_rules! identity_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            pub const PREFIX: &'static str = $prefix;
            #[must_use]
            pub const fn digest(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, formatter)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(Self::PREFIX)?;
                for byte in self.0 {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }
    };
}

identity_type!(DerivationNodeId, "nmlt-derivation-node-v1:sha256:");
identity_type!(
    ElaborationCertificateId,
    "nmlt-elaboration-certificate-v1:sha256:"
);
identity_type!(RulesetBundleId, "nmlt-ruleset-bundle-v1:sha256:");
identity_type!(ResourcePolicyId, "nmlt-kernel-policy-v1:sha256:");

pub(crate) fn ruleset_bundle_id() -> RulesetBundleId {
    let mut encoder = Encoder::with_domain(RULESET_DOMAIN);
    encoder.count(2);
    encoder.text("nmlt-core-typing-v1");
    encoder.text("nmlt-temporal-formation-v1");
    RulesetBundleId(sha256_bytes(&encoder.finish()))
}

pub(crate) fn resource_policy_id() -> ResourcePolicyId {
    let mut encoder = Encoder::with_domain(POLICY_DOMAIN);
    for value in [
        256_u64,
        4 * 1024 * 1024,
        16 * 1024 * 1024,
        32 * 1024 * 1024,
        32 * 1024 * 1024,
        64 * 1024 * 1024,
        262_144,
        262_144,
        524_288,
        2_097_152,
        32,
        256,
        255,
        4_096,
        4_096,
        16 * 1024 * 1024,
        65_536,
    ] {
        encoder.u64(value);
    }
    ResourcePolicyId(sha256_bytes(&encoder.finish()))
}

pub(crate) fn make_derivation_node(
    rule: ElaborationRule,
    obligation: ObligationKey,
    conclusion: DerivationConclusion,
    witness: DerivationWitness,
    premises: Vec<DerivationNodeId>,
) -> DerivationNode {
    let mut encoder = Encoder::with_domain(DERIVATION_DOMAIN);
    encode_derivation_fields(
        &mut encoder,
        rule,
        obligation,
        &conclusion,
        &witness,
        &premises,
    );
    DerivationNode {
        id: DerivationNodeId(sha256_bytes(&encoder.finish())),
        rule,
        obligation,
        conclusion,
        witness,
        premises,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn certificate_id(
    source_set_id: SourceSetId,
    module_map_id: ModuleMapId,
    surface_program_id: SurfaceProgramId,
    resolved_hir_id: ResolutionId,
    core_program_id: CoreProgramId,
    ruleset_bundle_id: RulesetBundleId,
    resource_policy_id: ResourcePolicyId,
    required_roots: &BTreeMap<ObligationKey, DerivationNodeId>,
    derivations: &BTreeMap<DerivationNodeId, DerivationNode>,
) -> (ElaborationCertificateId, usize) {
    let mut encoder = Encoder::with_domain(CERTIFICATE_DOMAIN);
    encoder.u16(crate::model::ElaborationArtifact::FORMAT_VERSION);
    encoder.raw(source_set_id.digest());
    encoder.raw(module_map_id.digest());
    encoder.raw(surface_program_id.digest());
    encoder.raw(resolved_hir_id.digest());
    encoder.raw(core_program_id.digest());
    encoder.raw(ruleset_bundle_id.digest());
    encoder.raw(resource_policy_id.digest());
    encoder.count(required_roots.len());
    for (obligation, derivation) in required_roots {
        encode_obligation(&mut encoder, *obligation);
        encoder.raw(derivation.digest());
    }
    encoder.count(derivations.len());
    for (id, node) in derivations {
        encoder.raw(id.digest());
        encode_derivation_fields(
            &mut encoder,
            node.rule,
            node.obligation,
            &node.conclusion,
            &node.witness,
            &node.premises,
        );
    }
    let canonical = encoder.finish();
    let encoded_len = canonical.len();
    (
        ElaborationCertificateId(sha256_bytes(&canonical)),
        encoded_len,
    )
}

fn encode_derivation_fields(
    encoder: &mut Encoder,
    rule: ElaborationRule,
    obligation: ObligationKey,
    conclusion: &DerivationConclusion,
    witness: &DerivationWitness,
    premises: &[DerivationNodeId],
) {
    encoder.u16(rule.wire_tag());
    encode_obligation(encoder, obligation);
    encode_conclusion(encoder, conclusion);
    encode_witness(encoder, witness);
    encoder.count(premises.len());
    for premise in premises {
        encoder.raw(premise.digest());
    }
}

fn encode_obligation(encoder: &mut Encoder, obligation: ObligationKey) {
    encoder.u8(obligation.judgment.wire_tag());
    encoder.raw(obligation.origin.digest());
}

fn encode_conclusion(encoder: &mut Encoder, conclusion: &DerivationConclusion) {
    match conclusion {
        DerivationConclusion::Type(ty) => {
            encoder.u8(1);
            encode_type(encoder, ty);
        }
        DerivationConclusion::Protocol(node) => {
            encoder.u8(2);
            encoder.raw(node.digest());
        }
        DerivationConclusion::Term { node, ty } => {
            encoder.u8(3);
            encoder.raw(node.digest());
            encode_type(encoder, ty);
        }
        DerivationConclusion::Definition(definition) => {
            encoder.u8(4);
            encoder.raw(definition.digest());
        }
    }
}

fn encode_witness(encoder: &mut Encoder, witness: &DerivationWitness) {
    match witness {
        DerivationWitness::None => encoder.u8(0),
        DerivationWitness::Boolean(value) => {
            encoder.u8(1);
            encoder.u8(u8::from(*value));
        }
        DerivationWitness::Magnitude { negative, bytes } => {
            encoder.u8(2);
            encoder.u8(u8::from(*negative));
            encoder.bytes(bytes);
        }
        DerivationWitness::Definition(definition) => {
            encoder.u8(3);
            encoder.raw(definition.digest());
        }
        DerivationWitness::SystemDefinition { system, definition } => {
            encoder.u8(4);
            encoder.raw(system.digest());
            encoder.raw(definition.digest());
        }
    }
}

fn encode_type(encoder: &mut Encoder, ty: &CoreType) {
    match ty {
        CoreType::Bool => encoder.u8(1),
        CoreType::Nat => encoder.u8(2),
        CoreType::Int => encoder.u8(3),
        CoreType::Enum(id) => {
            encoder.u8(4);
            encoder.raw(id.digest());
        }
        CoreType::Once { protocol } => {
            encoder.u8(5);
            encoder.raw(protocol.digest());
        }
        CoreType::StateProp { system } => {
            encoder.u8(6);
            encoder.raw(system.digest());
        }
        CoreType::TemporalProp { system } => {
            encoder.u8(7);
            encoder.raw(system.digest());
        }
    }
}

struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    fn with_domain(domain: &[u8]) -> Self {
        Self {
            bytes: domain.to_vec(),
        }
    }
    fn raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }
    fn u16(&mut self, value: u16) {
        self.raw(&value.to_be_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.raw(&value.to_be_bytes());
    }
    fn count(&mut self, count: usize) {
        self.u64(count as u64);
    }
    fn bytes(&mut self, bytes: &[u8]) {
        self.count(bytes.len());
        self.raw(bytes);
    }
    fn text(&mut self, text: &str) {
        self.bytes(text.as_bytes());
    }
    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

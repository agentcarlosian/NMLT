use std::fmt;

use nmlt_certificate::{DerivationConclusion, DerivationWitness};
use nmlt_hir::sha256_bytes;
use nmlt_ir::CoreType;

use crate::input::{RawCertificate, RawDerivationNode, RawObligation};

const DERIVATION_DOMAIN: &[u8] = b"NMLT-DERIVATION-NODE\0v1\0";
const CERTIFICATE_DOMAIN: &[u8] = b"NMLT-ELABORATION-CERTIFICATE\0v1\0";
const RULESET_DOMAIN: &[u8] = b"NMLT-RULESET-BUNDLE\0v1\0";
const POLICY_DOMAIN: &[u8] = b"NMLT-KERNEL-POLICY\0v1\0";
const KERNEL_PROFILE_DOMAIN: &[u8] = b"NMLT-KERNEL-PROFILE\0v1\0";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KernelProfileId([u8; 32]);

impl KernelProfileId {
    pub const PREFIX: &'static str = "nmlt-kernel-profile-v1:sha256:";

    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for KernelProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for KernelProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(Self::PREFIX)?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

pub(crate) fn ruleset_digest() -> [u8; 32] {
    let mut encoder = Encoder::with_domain(RULESET_DOMAIN);
    encoder.count(2);
    encoder.text("nmlt-core-typing-v1");
    encoder.text("nmlt-temporal-formation-v1");
    sha256_bytes(&encoder.finish())
}

pub(crate) fn policy_digest() -> [u8; 32] {
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
    sha256_bytes(&encoder.finish())
}

pub(crate) fn kernel_profile_id() -> KernelProfileId {
    let mut encoder = Encoder::with_domain(KERNEL_PROFILE_DOMAIN);
    encoder.text("nmlt-kernel-v1");
    encoder.raw(&ruleset_digest());
    encoder.raw(&policy_digest());
    KernelProfileId(sha256_bytes(&encoder.finish()))
}

pub(crate) fn derivation_digest(node: &RawDerivationNode) -> [u8; 32] {
    let mut encoder = Encoder::with_domain(DERIVATION_DOMAIN);
    encode_derivation_fields(&mut encoder, node);
    sha256_bytes(&encoder.finish())
}

pub(crate) fn certificate_digest(certificate: &RawCertificate) -> ([u8; 32], usize) {
    let mut encoder = Encoder::with_domain(CERTIFICATE_DOMAIN);
    encoder.u16(certificate.format_version);
    encoder.raw(&certificate.source_set_digest);
    encoder.raw(&certificate.module_map_digest);
    encoder.raw(&certificate.surface_program_digest);
    encoder.raw(&certificate.resolved_hir_digest);
    encoder.raw(&certificate.core_program_digest);
    encoder.raw(&certificate.ruleset_bundle_digest);
    encoder.raw(&certificate.resource_policy_digest);
    encoder.count(certificate.required_roots.len());
    for root in &certificate.required_roots {
        encode_obligation(&mut encoder, root.obligation);
        encoder.raw(&root.derivation_digest);
    }
    encoder.count(certificate.derivations.len());
    for node in &certificate.derivations {
        encoder.raw(&node.claimed_digest);
        encode_derivation_fields(&mut encoder, node);
    }
    let canonical = encoder.finish();
    (sha256_bytes(&canonical), canonical.len())
}

fn encode_derivation_fields(encoder: &mut Encoder, node: &RawDerivationNode) {
    encoder.u16(node.rule_tag);
    encode_obligation(encoder, node.obligation);
    encode_conclusion(encoder, &node.conclusion);
    encode_witness(encoder, &node.witness);
    encoder.count(node.premises.len());
    for premise in &node.premises {
        encoder.raw(premise);
    }
}

fn encode_obligation(encoder: &mut Encoder, obligation: RawObligation) {
    encoder.u8(obligation.judgment_tag);
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

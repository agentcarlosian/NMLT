use std::fmt;

use nmlt_certificate::{DerivationConclusion, DerivationWitness};
use nmlt_hir::{DefId, NodeId, sha256_bytes};
use nmlt_ir::{CoreNodeId, CoreType};

use crate::identity::canonical_certificate;
use crate::{RawCertificate, RawDerivationNode, RawObligation, RawRequiredRoot};

const DOMAIN: &[u8] = b"NMLT-ELABORATION-CERTIFICATE\0v1\0";
const MAX_CERTIFICATE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ROOTS: usize = 524_288;
const MAX_DERIVATIONS: usize = 524_288;
const MAX_PREMISES: usize = 32;
const MAX_PREMISE_EDGES: usize = 2_097_152;
const MAX_MAGNITUDE_BYTES: usize = 4_096;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertificateDecodeError {
    message: &'static str,
    offset: usize,
}

impl CertificateDecodeError {
    const fn new(message: &'static str, offset: usize) -> Self {
        Self { message, offset }
    }
}

impl fmt::Display for CertificateDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "NMLT_KERNEL_DECODE at byte {}: {}",
            self.offset, self.message
        )
    }
}

impl std::error::Error for CertificateDecodeError {}

impl RawCertificate {
    /// Emit the exact domain-separated v1 bytes whose digest binds this input.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        canonical_certificate(self)
    }

    /// Decode untrusted canonical v1 bytes. Counts and byte lengths are checked
    /// before allocation; semantic acceptance still requires [`crate::check`].
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, CertificateDecodeError> {
        if bytes.len() > MAX_CERTIFICATE_BYTES {
            return Err(CertificateDecodeError::new(
                "certificate exceeds the 64 MiB policy",
                0,
            ));
        }
        let mut decoder = Decoder::new(bytes);
        decoder.domain(DOMAIN)?;
        let format_version = decoder.u16()?;
        let source_set_digest = decoder.digest()?;
        let module_map_digest = decoder.digest()?;
        let surface_program_digest = decoder.digest()?;
        let resolved_hir_digest = decoder.digest()?;
        let core_program_digest = decoder.digest()?;
        let ruleset_bundle_digest = decoder.digest()?;
        let resource_policy_digest = decoder.digest()?;

        let root_count = decoder.count(MAX_ROOTS, "required-root count exceeds policy")?;
        let mut required_roots = Vec::with_capacity(root_count);
        for _ in 0..root_count {
            required_roots.push(RawRequiredRoot {
                obligation: decoder.obligation()?,
                derivation_digest: decoder.digest()?,
            });
        }

        let derivation_count = decoder.count(MAX_DERIVATIONS, "derivation count exceeds policy")?;
        let mut derivations = Vec::with_capacity(derivation_count);
        let mut premise_edges = 0usize;
        for _ in 0..derivation_count {
            let claimed_digest = decoder.digest()?;
            let rule_tag = decoder.u16()?;
            let obligation = decoder.obligation()?;
            let conclusion = decoder.conclusion()?;
            let witness = decoder.witness()?;
            let premise_count = decoder.count(MAX_PREMISES, "premise count exceeds policy")?;
            premise_edges = premise_edges
                .checked_add(premise_count)
                .ok_or_else(|| decoder.error("premise-edge count overflow"))?;
            if premise_edges > MAX_PREMISE_EDGES {
                return Err(decoder.error("premise-edge count exceeds policy"));
            }
            let mut premises = Vec::with_capacity(premise_count);
            for _ in 0..premise_count {
                premises.push(decoder.digest()?);
            }
            derivations.push(RawDerivationNode {
                claimed_digest,
                rule_tag,
                obligation,
                conclusion,
                witness,
                premises,
            });
        }
        decoder.finish()?;
        let certificate_digest = sha256_bytes(bytes);
        Ok(Self {
            format_version,
            source_set_digest,
            module_map_digest,
            surface_program_digest,
            resolved_hir_digest,
            core_program_digest,
            ruleset_bundle_digest,
            resource_policy_digest,
            required_roots,
            derivations,
            certificate_digest,
        })
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    const fn error(&self, message: &'static str) -> CertificateDecodeError {
        CertificateDecodeError::new(message, self.offset)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], CertificateDecodeError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| self.error("declared length overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| self.error("declared length exceeds remaining input"))?;
        self.offset = end;
        Ok(value)
    }

    fn domain(&mut self, expected: &[u8]) -> Result<(), CertificateDecodeError> {
        if self.take(expected.len())? == expected {
            Ok(())
        } else {
            Err(self.error("wrong certificate domain or version"))
        }
    }

    fn u8(&mut self) -> Result<u8, CertificateDecodeError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, CertificateDecodeError> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, CertificateDecodeError> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn count(
        &mut self,
        maximum: usize,
        message: &'static str,
    ) -> Result<usize, CertificateDecodeError> {
        let value = self.u64()?;
        let count = usize::try_from(value).map_err(|_| self.error(message))?;
        if count > maximum {
            return Err(self.error(message));
        }
        Ok(count)
    }

    fn digest(&mut self) -> Result<[u8; 32], CertificateDecodeError> {
        Ok(self.take(32)?.try_into().unwrap())
    }

    fn obligation(&mut self) -> Result<RawObligation, CertificateDecodeError> {
        Ok(RawObligation {
            judgment_tag: self.u8()?,
            origin: NodeId::from_untrusted_digest(self.digest()?),
        })
    }

    fn conclusion(&mut self) -> Result<DerivationConclusion, CertificateDecodeError> {
        match self.u8()? {
            1 => Ok(DerivationConclusion::Type(self.core_type()?)),
            2 => Ok(DerivationConclusion::Protocol(
                NodeId::from_untrusted_digest(self.digest()?),
            )),
            3 => Ok(DerivationConclusion::Term {
                node: CoreNodeId::from_untrusted_digest(self.digest()?),
                ty: self.core_type()?,
            }),
            4 => Ok(DerivationConclusion::Definition(
                DefId::from_untrusted_digest(self.digest()?),
            )),
            _ => Err(self.error("unknown conclusion tag")),
        }
    }

    fn witness(&mut self) -> Result<DerivationWitness, CertificateDecodeError> {
        match self.u8()? {
            0 => Ok(DerivationWitness::None),
            1 => match self.u8()? {
                0 => Ok(DerivationWitness::Boolean(false)),
                1 => Ok(DerivationWitness::Boolean(true)),
                _ => Err(self.error("noncanonical Boolean witness")),
            },
            2 => {
                let negative = match self.u8()? {
                    0 => false,
                    1 => true,
                    _ => return Err(self.error("noncanonical magnitude sign")),
                };
                let length = self.count(
                    MAX_MAGNITUDE_BYTES,
                    "integer magnitude length exceeds policy",
                )?;
                let bytes = self.take(length)?.to_vec();
                Ok(DerivationWitness::Magnitude { negative, bytes })
            }
            3 => Ok(DerivationWitness::Definition(DefId::from_untrusted_digest(
                self.digest()?,
            ))),
            4 => Ok(DerivationWitness::SystemDefinition {
                system: DefId::from_untrusted_digest(self.digest()?),
                definition: DefId::from_untrusted_digest(self.digest()?),
            }),
            _ => Err(self.error("unknown derivation-witness tag")),
        }
    }

    fn core_type(&mut self) -> Result<CoreType, CertificateDecodeError> {
        match self.u8()? {
            1 => Ok(CoreType::Bool),
            2 => Ok(CoreType::Nat),
            3 => Ok(CoreType::Int),
            4 => Ok(CoreType::Enum(DefId::from_untrusted_digest(self.digest()?))),
            5 => Ok(CoreType::Once {
                protocol: NodeId::from_untrusted_digest(self.digest()?),
            }),
            6 => Ok(CoreType::StateProp {
                system: DefId::from_untrusted_digest(self.digest()?),
            }),
            7 => Ok(CoreType::TemporalProp {
                system: DefId::from_untrusted_digest(self.digest()?),
            }),
            _ => Err(self.error("unknown core-type tag")),
        }
    }

    fn finish(&self) -> Result<(), CertificateDecodeError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(self.error("trailing bytes after canonical certificate"))
        }
    }
}

use nmlt_certificate::{
    DerivationConclusion, DerivationWitness, ElaborationArtifact, ElaborationRule, JudgmentKind,
    ObligationKey,
};
use nmlt_hir::NodeId;

/// Untrusted wire-shaped obligation. Numeric tags permit unknown-tag controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawObligation {
    pub judgment_tag: u8,
    pub origin: NodeId,
}

impl RawObligation {
    #[must_use]
    pub const fn from_obligation(value: ObligationKey) -> Self {
        Self {
            judgment_tag: judgment_tag(value.judgment()),
            origin: value.origin(),
        }
    }
}

/// One untrusted root-to-derivation binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawRequiredRoot {
    pub obligation: RawObligation,
    pub derivation_digest: [u8; 32],
}

/// One untrusted derivation record including its claimed content identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawDerivationNode {
    pub claimed_digest: [u8; 32],
    pub rule_tag: u16,
    pub obligation: RawObligation,
    pub conclusion: DerivationConclusion,
    pub witness: DerivationWitness,
    pub premises: Vec<[u8; 32]>,
}

/// Untrusted certificate envelope consumed by the independent kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawCertificate {
    pub format_version: u16,
    pub source_set_digest: [u8; 32],
    pub module_map_digest: [u8; 32],
    pub surface_program_digest: [u8; 32],
    pub resolved_hir_digest: [u8; 32],
    pub core_program_digest: [u8; 32],
    pub ruleset_bundle_digest: [u8; 32],
    pub resource_policy_digest: [u8; 32],
    pub required_roots: Vec<RawRequiredRoot>,
    pub derivations: Vec<RawDerivationNode>,
    pub certificate_digest: [u8; 32],
}

impl RawCertificate {
    /// Copy a producer artifact into a wire-shaped, freely mutable input.
    #[must_use]
    pub fn from_artifact(artifact: &ElaborationArtifact) -> Self {
        Self {
            format_version: artifact.format_version(),
            source_set_digest: *artifact.source_set_id().digest(),
            module_map_digest: *artifact.module_map_id().digest(),
            surface_program_digest: *artifact.surface_program_id().digest(),
            resolved_hir_digest: *artifact.resolved_hir_id().digest(),
            core_program_digest: *artifact.core_program_id().digest(),
            ruleset_bundle_digest: *artifact.ruleset_bundle_id().digest(),
            resource_policy_digest: *artifact.resource_policy_id().digest(),
            required_roots: artifact
                .required_roots()
                .iter()
                .map(|(obligation, derivation)| RawRequiredRoot {
                    obligation: RawObligation::from_obligation(*obligation),
                    derivation_digest: *derivation.digest(),
                })
                .collect(),
            derivations: artifact
                .derivations()
                .values()
                .map(|node| RawDerivationNode {
                    claimed_digest: *node.id().digest(),
                    rule_tag: rule_tag(node.rule()),
                    obligation: RawObligation::from_obligation(node.obligation()),
                    conclusion: node.conclusion().clone(),
                    witness: node.witness().clone(),
                    premises: node
                        .premises()
                        .iter()
                        .map(|premise| *premise.digest())
                        .collect(),
                })
                .collect(),
            certificate_digest: *artifact.certificate_id().digest(),
        }
    }

    /// Recompute the envelope's claimed content digest after an untrusted
    /// transformation. This does not validate any derivation or confer
    /// checked status; callers must still invoke [`crate::check`].
    pub fn recompute_claimed_certificate_digest(&mut self) {
        self.certificate_digest = crate::identity::certificate_digest(self).0;
    }
}

pub(crate) const fn judgment_tag(value: JudgmentKind) -> u8 {
    match value {
        JudgmentKind::Synthesize => 1,
        JudgmentKind::Check => 2,
        JudgmentKind::Formula => 3,
        JudgmentKind::TypeFormation => 4,
        JudgmentKind::ProtocolFormation => 5,
        JudgmentKind::UpdateTarget => 6,
        JudgmentKind::CapabilityUse => 7,
        JudgmentKind::ActionUse => 8,
    }
}

pub(crate) const fn rule_tag(value: ElaborationRule) -> u16 {
    value as u16
}

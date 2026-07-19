use std::collections::BTreeMap;

use nmlt_hir::{DefId, ModuleMapId, NodeId, ResolutionId, SourceSetId, SurfaceProgramId};
use nmlt_ir::{CoreNodeId, CoreProgram, CoreProgramId, CoreType};

use crate::identity::{
    DerivationNodeId, ElaborationCertificateId, ResourcePolicyId, RulesetBundleId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JudgmentKind {
    Synthesize,
    Check,
    Formula,
    TypeFormation,
    ProtocolFormation,
    UpdateTarget,
    CapabilityUse,
    ActionUse,
}

impl JudgmentKind {
    pub(crate) const fn wire_tag(self) -> u8 {
        match self {
            Self::Synthesize => 1,
            Self::Check => 2,
            Self::Formula => 3,
            Self::TypeFormation => 4,
            Self::ProtocolFormation => 5,
            Self::UpdateTarget => 6,
            Self::CapabilityUse => 7,
            Self::ActionUse => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObligationKey {
    pub(crate) judgment: JudgmentKind,
    pub(crate) origin: NodeId,
}

impl ObligationKey {
    #[must_use]
    pub const fn new(judgment: JudgmentKind, origin: NodeId) -> Self {
        Self { judgment, origin }
    }

    #[must_use]
    pub const fn judgment(&self) -> JudgmentKind {
        self.judgment
    }
    #[must_use]
    pub const fn origin(&self) -> NodeId {
        self.origin
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum ElaborationRule {
    TypeBool = 1,
    TypeNat = 2,
    TypeInt = 3,
    TypeEnum = 4,
    ProtocolTag = 5,
    TypeOnce = 6,
    BoolLiteral = 10,
    NatLiteral = 11,
    IntLiteral = 12,
    LocalReference = 13,
    StateReference = 14,
    ConstructorReference = 15,
    CheckSynthesis = 16,
    Not = 20,
    Negate = 21,
    Or = 22,
    And = 23,
    Implies = 24,
    Equal = 25,
    NotEqual = 26,
    Less = 27,
    LessEqual = 28,
    Greater = 29,
    GreaterEqual = 30,
    Add = 31,
    Subtract = 32,
    Multiply = 33,
    IntFromNat = 34,
    StatePredicate = 40,
    Always = 41,
    Eventually = 42,
    Next = 43,
    Until = 44,
    Enabled = 45,
    UpdateTarget = 50,
    CapabilityUse = 51,
    ActionUse = 52,
}

impl ElaborationRule {
    pub(crate) const fn wire_tag(self) -> u16 {
        self as u16
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DerivationConclusion {
    Type(CoreType),
    Protocol(NodeId),
    Term { node: CoreNodeId, ty: CoreType },
    Definition(DefId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DerivationWitness {
    None,
    Boolean(bool),
    Magnitude { negative: bool, bytes: Vec<u8> },
    Definition(DefId),
    SystemDefinition { system: DefId, definition: DefId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivationNode {
    pub(crate) id: DerivationNodeId,
    pub(crate) rule: ElaborationRule,
    pub(crate) obligation: ObligationKey,
    pub(crate) conclusion: DerivationConclusion,
    pub(crate) witness: DerivationWitness,
    pub(crate) premises: Vec<DerivationNodeId>,
}

impl DerivationNode {
    #[must_use]
    pub const fn id(&self) -> DerivationNodeId {
        self.id
    }
    #[must_use]
    pub const fn rule(&self) -> ElaborationRule {
        self.rule
    }
    #[must_use]
    pub const fn obligation(&self) -> ObligationKey {
        self.obligation
    }
    #[must_use]
    pub const fn conclusion(&self) -> &DerivationConclusion {
        &self.conclusion
    }
    #[must_use]
    pub const fn witness(&self) -> &DerivationWitness {
        &self.witness
    }
    #[must_use]
    pub fn premises(&self) -> &[DerivationNodeId] {
        &self.premises
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElaborationArtifact {
    pub(crate) source_set_id: SourceSetId,
    pub(crate) module_map_id: ModuleMapId,
    pub(crate) surface_program_id: SurfaceProgramId,
    pub(crate) resolved_hir_id: ResolutionId,
    pub(crate) core_program: CoreProgram,
    pub(crate) ruleset_bundle_id: RulesetBundleId,
    pub(crate) resource_policy_id: ResourcePolicyId,
    pub(crate) required_roots: BTreeMap<ObligationKey, DerivationNodeId>,
    pub(crate) derivations: BTreeMap<DerivationNodeId, DerivationNode>,
    pub(crate) certificate_id: ElaborationCertificateId,
}

impl ElaborationArtifact {
    pub const FORMAT_VERSION: u16 = 1;

    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_set_id: SourceSetId,
        module_map_id: ModuleMapId,
        surface_program_id: SurfaceProgramId,
        resolved_hir_id: ResolutionId,
        core_program: CoreProgram,
        ruleset_bundle_id: RulesetBundleId,
        resource_policy_id: ResourcePolicyId,
        required_roots: BTreeMap<ObligationKey, DerivationNodeId>,
        derivations: BTreeMap<DerivationNodeId, DerivationNode>,
        certificate_id: ElaborationCertificateId,
    ) -> Self {
        Self {
            source_set_id,
            module_map_id,
            surface_program_id,
            resolved_hir_id,
            core_program,
            ruleset_bundle_id,
            resource_policy_id,
            required_roots,
            derivations,
            certificate_id,
        }
    }

    #[must_use]
    pub const fn format_version(&self) -> u16 {
        Self::FORMAT_VERSION
    }

    #[must_use]
    pub const fn source_set_id(&self) -> SourceSetId {
        self.source_set_id
    }
    #[must_use]
    pub const fn module_map_id(&self) -> ModuleMapId {
        self.module_map_id
    }
    #[must_use]
    pub const fn surface_program_id(&self) -> SurfaceProgramId {
        self.surface_program_id
    }
    #[must_use]
    pub const fn resolved_hir_id(&self) -> ResolutionId {
        self.resolved_hir_id
    }
    #[must_use]
    pub const fn core_program_id(&self) -> CoreProgramId {
        self.core_program.id()
    }
    #[must_use]
    pub const fn core_program(&self) -> &CoreProgram {
        &self.core_program
    }
    #[must_use]
    pub const fn ruleset_bundle_id(&self) -> RulesetBundleId {
        self.ruleset_bundle_id
    }
    #[must_use]
    pub const fn resource_policy_id(&self) -> ResourcePolicyId {
        self.resource_policy_id
    }
    #[must_use]
    pub const fn required_roots(&self) -> &BTreeMap<ObligationKey, DerivationNodeId> {
        &self.required_roots
    }
    #[must_use]
    pub const fn derivations(&self) -> &BTreeMap<DerivationNodeId, DerivationNode> {
        &self.derivations
    }
    #[must_use]
    pub const fn certificate_id(&self) -> ElaborationCertificateId {
        self.certificate_id
    }
}

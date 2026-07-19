use crate::artifact::ArtifactRole;
use crate::authority::ByteSpan;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MutationKind {
    DeleteGuard,
    ReplaceUpdateTarget,
    DuplicateAffineCapabilityUse,
    RemoveResponseBindingConjunct,
    EnableActionInAmbiguityState,
}

impl MutationKind {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::DeleteGuard => "delete_guard",
            Self::ReplaceUpdateTarget => "replace_update_target",
            Self::DuplicateAffineCapabilityUse => "duplicate_affine_capability_use",
            Self::RemoveResponseBindingConjunct => "remove_response_binding_conjunct",
            Self::EnableActionInAmbiguityState => "enable_action_in_ambiguity_state",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationDescriptor {
    pub mutation_id: String,
    pub property_id: String,
    pub operator: MutationKind,
    pub target_role: ArtifactRole,
    pub target_path: String,
    pub target_span: ByteSpan,
}

impl MutationDescriptor {
    #[must_use]
    pub fn candidate(
        mutation_id: impl Into<String>,
        property_id: impl Into<String>,
        operator: MutationKind,
        target_path: impl Into<String>,
        target_span: ByteSpan,
    ) -> Self {
        Self {
            mutation_id: mutation_id.into(),
            property_id: property_id.into(),
            operator,
            target_role: ArtifactRole::Candidate,
            target_path: target_path.into(),
            target_span,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.mutation_id.is_empty() || self.property_id.is_empty() {
            return Err("mutation and property identities are required".into());
        }
        if self.target_role != ArtifactRole::Candidate {
            return Err("semantic mutations may target candidate artifacts only".into());
        }
        if self.target_path.is_empty() || self.target_span.start > self.target_span.end {
            return Err("mutation target must be a valid localized span".into());
        }
        Ok(())
    }
}

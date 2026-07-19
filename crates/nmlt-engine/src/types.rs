use std::collections::BTreeMap;

use crate::ast::{Model, Type};

/// Exact identities accepted before runtime projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticBinding {
    pub source_set_id: String,
    pub module_map_id: String,
    pub surface_program_id: String,
    pub resolved_hir_id: String,
    pub core_program_id: String,
    pub ruleset_bundle_id: String,
    pub resource_policy_id: String,
    pub certificate_id: String,
    pub kernel_profile_id: String,
}

/// A runtime model projected exclusively from a kernel-issued
/// `CheckedProgram`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedModel {
    pub(crate) model: Model,
    pub(crate) state_types: BTreeMap<String, Type>,
    pub(crate) frames: BTreeMap<String, Vec<String>>,
    pub(crate) property_behavior: BTreeMap<String, String>,
    pub(crate) semantic_binding: SemanticBinding,
}

impl TypedModel {
    #[must_use]
    pub const fn model(&self) -> &Model {
        &self.model
    }

    #[must_use]
    pub const fn frames(&self) -> &BTreeMap<String, Vec<String>> {
        &self.frames
    }

    #[must_use]
    pub const fn property_behavior(&self) -> &BTreeMap<String, String> {
        &self.property_behavior
    }

    #[must_use]
    pub const fn semantic_binding(&self) -> &SemanticBinding {
        &self.semantic_binding
    }
}

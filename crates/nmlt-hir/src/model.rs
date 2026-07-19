//! Parser-independent input and resolved-program data model.

use std::collections::BTreeMap;

use crate::identity::{
    DefId, LocalId, ModuleId, ModuleMapId, NodeId, ResolutionId, SourceId, SourceSetId,
};
use crate::term::{LocalBinderInput, RawTermInput};
use crate::{HirNode, HirRoot, LocalBinder, ResolutionMap, SemanticPath, SemanticPathSegment};

/// Half-open byte range in one module's exact UTF-8 source bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

impl SourceSpan {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// A direct import projected from the lossless frontend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ImportInput {
    pub logical_module: String,
    pub span: SourceSpan,
}

impl ImportInput {
    #[must_use]
    pub fn new(logical_module: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            logical_module: logical_module.into(),
            span,
        }
    }
}

/// Stable namespaces used during M9 name resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Namespace {
    Type,
    Constructor,
    Value,
    System,
    State,
    Action,
    SystemInput,
    Capability,
    Property,
    Observation,
}

impl Namespace {
    /// Version-1 typed-definition path tag.
    #[must_use]
    pub const fn wire_tag(self) -> u8 {
        match self {
            Self::Type => 0x01,
            Self::Constructor => 0x02,
            Self::Value => 0x03,
            Self::System => 0x04,
            Self::State => 0x05,
            Self::Action => 0x06,
            Self::SystemInput => 0x07,
            Self::Capability => 0x08,
            Self::Property => 0x09,
            Self::Observation => 0x0a,
        }
    }

    /// Human-readable namespace name.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Constructor => "constructor",
            Self::Value => "value",
            Self::System => "system",
            Self::State => "state",
            Self::Action => "action",
            Self::SystemInput => "system-input",
            Self::Capability => "capability",
            Self::Property => "property",
            Self::Observation => "observation",
        }
    }
}

/// One typed, named segment in a full definition path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefPathSegment {
    pub namespace: Namespace,
    pub name: String,
}

impl DefPathSegment {
    #[must_use]
    pub fn new(namespace: Namespace, name: impl Into<String>) -> Self {
        Self {
            namespace,
            name: name.into(),
        }
    }
}

/// Full typed owner-and-definition path used by `DefId` version 1.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefPath {
    pub segments: Vec<DefPathSegment>,
}

impl DefPath {
    #[must_use]
    pub fn new(segments: impl IntoIterator<Item = DefPathSegment>) -> Self {
        Self {
            segments: segments.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn top_level(namespace: Namespace, name: impl Into<String>) -> Self {
        Self::new([DefPathSegment::new(namespace, name)])
    }

    #[must_use]
    pub fn terminal_namespace(&self) -> Option<Namespace> {
        self.segments.last().map(|segment| segment.namespace)
    }
}

/// A projected declaration address before stable IDs are assigned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeclarationInput {
    pub path: DefPath,
    pub span: SourceSpan,
    pub flavor: DeclarationFlavor,
}

impl DeclarationInput {
    #[must_use]
    pub const fn new(path: DefPath, span: SourceSpan) -> Self {
        Self {
            path,
            span,
            flavor: DeclarationFlavor::Ordinary,
        }
    }

    #[must_use]
    pub const fn property(path: DefPath, span: SourceSpan, flavor: DeclarationFlavor) -> Self {
        Self { path, span, flavor }
    }
}

/// Semantic declaration information that cannot be recovered from a typed
/// definition path alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclarationFlavor {
    #[default]
    Ordinary,
    SafetyProperty,
    TemporalProperty,
}

impl DeclarationFlavor {
    #[must_use]
    pub const fn wire_tag(self) -> u8 {
        match self {
            Self::Ordinary => 0x00,
            Self::SafetyProperty => 0x01,
            Self::TemporalProperty => 0x02,
        }
    }
}

/// Why a frontend projection cannot be admitted to semantic resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProjectionIssueKind {
    UnsupportedSyntax,
    RecoveryNode,
    CoverageGap,
}

/// Explicit evidence that a projected source module is incomplete or invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionIssue {
    pub kind: ProjectionIssueKind,
    pub message: String,
    pub span: Option<SourceSpan>,
}

impl ProjectionIssue {
    #[must_use]
    pub fn new(
        kind: ProjectionIssueKind,
        message: impl Into<String>,
        span: Option<SourceSpan>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
        }
    }
}

/// Complete named-declaration resolver input for one logical source module.
///
/// This structure is intentionally not a source parser. M9-002's hierarchical
/// surface projector must populate it and must represent every unsupported or
/// recovery-dependent node in `projection_issues`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModuleInput {
    pub logical_module: String,
    pub repository_path: String,
    pub exact_bytes: Vec<u8>,
    pub imports: Vec<ImportInput>,
    pub declarations: Vec<DeclarationInput>,
    pub local_binders: Vec<LocalBinderInput>,
    pub raw_terms: Vec<RawTermInput>,
    pub projection_issues: Vec<ProjectionIssue>,
}

impl ModuleInput {
    #[must_use]
    pub fn new(
        logical_module: impl Into<String>,
        repository_path: impl Into<String>,
        exact_bytes: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            logical_module: logical_module.into(),
            repository_path: repository_path.into(),
            exact_bytes: exact_bytes.into(),
            imports: Vec::new(),
            declarations: Vec::new(),
            local_binders: Vec::new(),
            raw_terms: Vec::new(),
            projection_issues: Vec::new(),
        }
    }
}

/// Opaque result of applying the canonical lossless frontend and M9 surface gate.
///
/// Its resolver input cannot be edited independently of the exact source bytes.
/// Construct values only through [`crate::project_source_module`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectedModule {
    pub(crate) input: ModuleInput,
}

impl ProjectedModule {
    pub(crate) const fn from_input(input: ModuleInput) -> Self {
        Self { input }
    }

    /// Logical name assigned by the source-set adapter.
    #[must_use]
    pub fn logical_module(&self) -> &str {
        &self.input.logical_module
    }

    /// Portable repository-relative path bound into source-set identity.
    #[must_use]
    pub fn repository_path(&self) -> &str {
        &self.input.repository_path
    }

    /// Exact source bytes passed through the frontend.
    #[must_use]
    pub fn exact_bytes(&self) -> &[u8] {
        &self.input.exact_bytes
    }

    /// Explicit reasons this source cannot enter declaration resolution.
    #[must_use]
    pub fn projection_issues(&self) -> &[ProjectionIssue] {
        &self.input.projection_issues
    }
}

/// Canonical key for one definition inside a logical module.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclarationKey {
    pub path: DefPath,
}

impl DeclarationKey {
    #[must_use]
    pub const fn new(path: DefPath) -> Self {
        Self { path }
    }

    #[must_use]
    pub fn top_level(namespace: Namespace, name: impl Into<String>) -> Self {
        Self::new(DefPath::top_level(namespace, name))
    }

    #[must_use]
    pub fn namespace(&self) -> Option<Namespace> {
        self.path.terminal_namespace()
    }
}

impl From<&DeclarationInput> for DeclarationKey {
    fn from(declaration: &DeclarationInput) -> Self {
        Self {
            path: declaration.path.clone(),
        }
    }
}

/// Frozen semantic-role tags available to `NodeId` version 1.
///
/// New roles require a central registry update and golden vectors; callers
/// cannot construct unregistered numeric tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticRole {
    Initializer,
}

impl SemanticRole {
    #[must_use]
    pub const fn wire_tag(self) -> u8 {
        match self {
            Self::Initializer => 0x03,
        }
    }
}

/// An import after its target has been resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedImport {
    pub(crate) logical_module: String,
    pub(crate) module_id: ModuleId,
    pub(crate) span: SourceSpan,
}

impl ResolvedImport {
    #[must_use]
    pub fn logical_module(&self) -> &str {
        &self.logical_module
    }

    #[must_use]
    pub const fn module_id(&self) -> ModuleId {
        self.module_id
    }

    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// A declaration with a stable, allocation-order-independent identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedDeclaration {
    pub(crate) key: DeclarationKey,
    pub(crate) id: DefId,
    pub(crate) span: SourceSpan,
    pub(crate) flavor: DeclarationFlavor,
}

impl ResolvedDeclaration {
    #[must_use]
    pub const fn key(&self) -> &DeclarationKey {
        &self.key
    }

    #[must_use]
    pub const fn id(&self) -> DefId {
        self.id
    }

    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    #[must_use]
    pub const fn flavor(&self) -> DeclarationFlavor {
        self.flavor
    }

    /// Derives a stable identity for a semantic node below this definition.
    #[must_use]
    pub fn node_id(&self, semantic_path: &[SemanticRole]) -> NodeId {
        let path = SemanticPath::new(semantic_path.iter().map(|role| match role {
            SemanticRole::Initializer => SemanticPathSegment::Initializer,
        }));
        path.node_id(self.id)
    }
}

/// One fully resolved module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedModule {
    pub(crate) logical_module: String,
    pub(crate) repository_path: String,
    pub(crate) source_id: SourceId,
    pub(crate) id: ModuleId,
    pub(crate) imports: Vec<ResolvedImport>,
    pub(crate) declarations: BTreeMap<DeclarationKey, ResolvedDeclaration>,
    pub(crate) local_binders: BTreeMap<LocalId, LocalBinder>,
    pub(crate) hir_roots: Vec<HirRoot>,
    pub(crate) hir_nodes: BTreeMap<NodeId, HirNode>,
    pub(crate) exact_bytes: Vec<u8>,
}

impl ResolvedModule {
    #[must_use]
    pub fn logical_module(&self) -> &str {
        &self.logical_module
    }

    #[must_use]
    pub fn repository_path(&self) -> &str {
        &self.repository_path
    }

    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }

    #[must_use]
    pub const fn id(&self) -> ModuleId {
        self.id
    }

    #[must_use]
    pub fn imports(&self) -> &[ResolvedImport] {
        &self.imports
    }

    #[must_use]
    pub const fn declarations(&self) -> &BTreeMap<DeclarationKey, ResolvedDeclaration> {
        &self.declarations
    }

    #[must_use]
    pub const fn local_binders(&self) -> &BTreeMap<LocalId, LocalBinder> {
        &self.local_binders
    }

    #[must_use]
    pub fn hir_roots(&self) -> &[HirRoot] {
        &self.hir_roots
    }

    #[must_use]
    pub const fn hir_nodes(&self) -> &BTreeMap<NodeId, HirNode> {
        &self.hir_nodes
    }

    /// Returns the exact source bytes whose identity is recorded by this module.
    #[must_use]
    pub fn exact_bytes(&self) -> &[u8] {
        &self.exact_bytes
    }
}

/// Deterministic, all-reference HIR produced from the closed source set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedProgram {
    pub(crate) source_set_id: SourceSetId,
    /// Binds the bijection from logical module names to source-set paths.
    pub(crate) module_map_id: ModuleMapId,
    pub(crate) surface_program_id: crate::SurfaceProgramId,
    /// Binds logical-module mapping, imports, and projected declaration keys.
    pub(crate) resolution_id: ResolutionId,
    /// Dependency-first order with UTF-8 lexical tie-breaking.
    pub(crate) dependency_order: Vec<String>,
    pub(crate) modules: BTreeMap<String, ResolvedModule>,
    pub(crate) resolution_map: ResolutionMap,
}

impl ResolvedProgram {
    #[must_use]
    pub const fn source_set_id(&self) -> SourceSetId {
        self.source_set_id
    }

    #[must_use]
    pub const fn module_map_id(&self) -> ModuleMapId {
        self.module_map_id
    }

    #[must_use]
    pub const fn surface_program_id(&self) -> crate::SurfaceProgramId {
        self.surface_program_id
    }

    #[must_use]
    pub const fn resolution_id(&self) -> ResolutionId {
        self.resolution_id
    }

    #[must_use]
    pub fn dependency_order(&self) -> &[String] {
        &self.dependency_order
    }

    #[must_use]
    pub const fn modules(&self) -> &BTreeMap<String, ResolvedModule> {
        &self.modules
    }

    #[must_use]
    pub fn module(&self, logical_module: &str) -> Option<&ResolvedModule> {
        self.modules.get(logical_module)
    }

    #[must_use]
    pub const fn resolution_map(&self) -> &ResolutionMap {
        &self.resolution_map
    }
}

/// A textual reference to resolve from one module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameReference {
    /// `None` requests unqualified lookup; `Some` requires self or a direct import.
    pub qualifier: Option<String>,
    pub key: DeclarationKey,
    pub span: SourceSpan,
}

impl NameReference {
    #[must_use]
    pub fn unqualified(key: DeclarationKey, span: SourceSpan) -> Self {
        Self {
            qualifier: None,
            key,
            span,
        }
    }

    #[must_use]
    pub fn qualified(
        logical_module: impl Into<String>,
        key: DeclarationKey,
        span: SourceSpan,
    ) -> Self {
        Self {
            qualifier: Some(logical_module.into()),
            key,
            span,
        }
    }
}

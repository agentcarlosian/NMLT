//! Fully resolved, still-untyped HIR nodes and canonical reference evidence.

use std::collections::BTreeMap;

use crate::identity::{DefId, LocalId, ModuleId, NodeId, local_id, node_id};
use crate::{Namespace, SourceSpan};

/// One allocation- and span-independent segment below a named definition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticPathSegment {
    DeclaredType,
    ActionParameter(u32),
    Initializer,
    Guard(u32),
    UpdateTarget(DefId),
    UpdateValue(DefId),
    Output(u32),
    PropertyBody,
    ObservationItem(u32),
    Operand(u32),
    CallArgument(u32),
    Consume(u32),
    CapabilityProtocol,
}

impl SemanticPathSegment {
    const fn wire_tag(&self) -> u8 {
        match self {
            Self::DeclaredType => 0x01,
            Self::ActionParameter(_) => 0x02,
            Self::Initializer => 0x03,
            Self::Guard(_) => 0x04,
            Self::UpdateTarget(_) => 0x05,
            Self::UpdateValue(_) => 0x06,
            Self::Output(_) => 0x07,
            Self::PropertyBody => 0x08,
            Self::ObservationItem(_) => 0x09,
            Self::Operand(_) => 0x0a,
            Self::CallArgument(_) => 0x0b,
            Self::Consume(_) => 0x0c,
            Self::CapabilityProtocol => 0x0d,
        }
    }
}

/// Complete semantic locator used as the unhashed input to `NodeId` v1.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticPath {
    segments: Vec<SemanticPathSegment>,
}

impl SemanticPath {
    #[must_use]
    pub fn new(segments: impl IntoIterator<Item = SemanticPathSegment>) -> Self {
        Self {
            segments: segments.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn segments(&self) -> &[SemanticPathSegment] {
        &self.segments
    }

    #[must_use]
    pub fn child(&self, segment: SemanticPathSegment) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment);
        Self { segments }
    }

    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::new();
        push_count(&mut encoded, self.segments.len());
        for segment in &self.segments {
            encoded.push(segment.wire_tag());
            match segment {
                SemanticPathSegment::ActionParameter(index)
                | SemanticPathSegment::Guard(index)
                | SemanticPathSegment::Output(index)
                | SemanticPathSegment::ObservationItem(index)
                | SemanticPathSegment::Operand(index)
                | SemanticPathSegment::CallArgument(index)
                | SemanticPathSegment::Consume(index) => {
                    encoded.extend_from_slice(&index.to_be_bytes());
                }
                SemanticPathSegment::UpdateTarget(definition)
                | SemanticPathSegment::UpdateValue(definition) => {
                    encoded.extend_from_slice(definition.digest());
                }
                SemanticPathSegment::DeclaredType
                | SemanticPathSegment::Initializer
                | SemanticPathSegment::PropertyBody
                | SemanticPathSegment::CapabilityProtocol => {}
            }
        }
        encoded
    }

    #[must_use]
    pub(crate) fn node_id(&self, owner: DefId) -> NodeId {
        node_id(owner, &self.encode())
    }
}

/// A reference target with no unresolved textual-symbol escape.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolvedRef {
    Local(LocalId),
    Definition(DefId),
    StateField {
        system: DefId,
        state: DefId,
    },
    Constructor {
        enumeration: DefId,
        constructor: DefId,
    },
    Capability {
        system: DefId,
        capability: DefId,
    },
}

impl ResolvedRef {
    #[must_use]
    pub const fn terminal_definition(&self) -> Option<DefId> {
        match self {
            Self::Local(_) => None,
            Self::Definition(definition) => Some(*definition),
            Self::StateField { state, .. } => Some(*state),
            Self::Constructor { constructor, .. } => Some(*constructor),
            Self::Capability { capability, .. } => Some(*capability),
        }
    }
}

/// Unary operator in resolved, untyped HIR.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HirUnaryOp {
    Not,
    Negate,
}

/// Binary operator in resolved, untyped HIR.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HirBinaryOp {
    Or,
    And,
    Implies,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
}

/// One node in the resolved HIR graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirNodeKind {
    TypeBool,
    TypeNat,
    TypeInt,
    TypeNamed(ResolvedRef),
    TypeOnce {
        protocol: NodeId,
    },
    ProtocolTag {
        spelling: String,
    },
    BoolLiteral(bool),
    NaturalLiteral {
        magnitude: Vec<u8>,
    },
    Reference(ResolvedRef),
    Unary {
        operator: HirUnaryOp,
        operand: NodeId,
    },
    Binary {
        operator: HirBinaryOp,
        left: NodeId,
        right: NodeId,
    },
    IntFromNat {
        operand: NodeId,
    },
    Always {
        property: NodeId,
    },
    Eventually {
        property: NodeId,
    },
    Next {
        property: NodeId,
    },
    Until {
        left: NodeId,
        right: NodeId,
    },
    Enabled {
        action_origin: NodeId,
        action: ResolvedRef,
    },
}

/// A source-derived HIR node whose identity excludes its diagnostic span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirNode {
    pub(crate) id: NodeId,
    pub(crate) owner: DefId,
    pub(crate) semantic_path: SemanticPath,
    pub(crate) span: SourceSpan,
    pub(crate) kind: HirNodeKind,
}

impl HirNode {
    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.id
    }

    #[must_use]
    pub const fn owner(&self) -> DefId {
        self.owner
    }

    #[must_use]
    pub const fn semantic_path(&self) -> &SemanticPath {
        &self.semantic_path
    }

    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    #[must_use]
    pub const fn kind(&self) -> &HirNodeKind {
        &self.kind
    }
}

/// One root connecting a definition role to a resolved HIR node.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HirRoot {
    pub(crate) owner: DefId,
    pub(crate) semantic_path: SemanticPath,
    pub(crate) node: NodeId,
}

impl HirRoot {
    #[must_use]
    pub const fn owner(&self) -> DefId {
        self.owner
    }

    #[must_use]
    pub const fn semantic_path(&self) -> &SemanticPath {
        &self.semantic_path
    }

    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.node
    }
}

/// One action-local binder with an owner-derived stable identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalBinder {
    pub(crate) id: LocalId,
    pub(crate) owner: DefId,
    pub(crate) binder_node: NodeId,
    pub(crate) name: String,
    pub(crate) declared_type: NodeId,
    pub(crate) span: SourceSpan,
}

impl LocalBinder {
    #[must_use]
    pub const fn id(&self) -> LocalId {
        self.id
    }

    #[must_use]
    pub const fn owner(&self) -> DefId {
        self.owner
    }

    #[must_use]
    pub const fn binder_node(&self) -> NodeId {
        self.binder_node
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn declared_type(&self) -> NodeId {
        self.declared_type
    }

    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    pub(crate) fn new(
        owner: DefId,
        index: u32,
        name: String,
        declared_type: NodeId,
        span: SourceSpan,
    ) -> Self {
        let path = SemanticPath::new([SemanticPathSegment::ActionParameter(index)]);
        let binder_node = path.node_id(owner);
        Self {
            id: local_id(binder_node),
            owner,
            binder_node,
            name,
            declared_type,
            span,
        }
    }
}

/// One exact textual-reference origin and the target selected for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionEntry {
    pub(crate) origin: NodeId,
    pub(crate) module: ModuleId,
    pub(crate) owner: DefId,
    pub(crate) semantic_path: SemanticPath,
    pub(crate) namespace: Namespace,
    pub(crate) qualifier: Option<String>,
    pub(crate) spelling: String,
    pub(crate) span: SourceSpan,
    pub(crate) target: ResolvedRef,
}

impl ResolutionEntry {
    #[must_use]
    pub const fn origin(&self) -> NodeId {
        self.origin
    }

    #[must_use]
    pub const fn module(&self) -> ModuleId {
        self.module
    }

    #[must_use]
    pub const fn owner(&self) -> DefId {
        self.owner
    }

    #[must_use]
    pub const fn semantic_path(&self) -> &SemanticPath {
        &self.semantic_path
    }

    #[must_use]
    pub const fn namespace(&self) -> Namespace {
        self.namespace
    }

    #[must_use]
    pub fn qualifier(&self) -> Option<&str> {
        self.qualifier.as_deref()
    }

    #[must_use]
    pub fn spelling(&self) -> &str {
        &self.spelling
    }

    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    #[must_use]
    pub const fn target(&self) -> &ResolvedRef {
        &self.target
    }
}

/// Canonical, bijective map from textual-reference origins to selected targets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolutionMap {
    pub(crate) entries: BTreeMap<NodeId, ResolutionEntry>,
}

impl ResolutionMap {
    #[must_use]
    pub const fn entries(&self) -> &BTreeMap<NodeId, ResolutionEntry> {
        &self.entries
    }

    #[must_use]
    pub fn get(&self, origin: &NodeId) -> Option<&ResolutionEntry> {
        self.entries.get(origin)
    }
}

pub(crate) fn push_count(output: &mut Vec<u8>, value: usize) {
    output.extend_from_slice(&(value as u64).to_be_bytes());
}

pub(crate) fn push_bytes(output: &mut Vec<u8>, value: &[u8]) {
    push_count(output, value.len());
    output.extend_from_slice(value);
}

pub(crate) fn push_text(output: &mut Vec<u8>, value: &str) {
    push_bytes(output, value.as_bytes());
}

pub(crate) fn encode_resolved_ref(output: &mut Vec<u8>, reference: &ResolvedRef) {
    match reference {
        ResolvedRef::Local(local) => {
            output.push(0x01);
            output.extend_from_slice(local.digest());
        }
        ResolvedRef::Definition(definition) => {
            output.push(0x02);
            output.extend_from_slice(definition.digest());
        }
        ResolvedRef::StateField { system, state } => {
            output.push(0x03);
            output.extend_from_slice(system.digest());
            output.extend_from_slice(state.digest());
        }
        ResolvedRef::Constructor {
            enumeration,
            constructor,
        } => {
            output.push(0x04);
            output.extend_from_slice(enumeration.digest());
            output.extend_from_slice(constructor.digest());
        }
        ResolvedRef::Capability { system, capability } => {
            output.push(0x05);
            output.extend_from_slice(system.digest());
            output.extend_from_slice(capability.digest());
        }
    }
}

pub(crate) fn encode_hir_node(output: &mut Vec<u8>, node: &HirNode) {
    output.extend_from_slice(node.id.digest());
    output.extend_from_slice(node.owner.digest());
    push_bytes(output, &node.semantic_path.encode());
    match &node.kind {
        HirNodeKind::TypeBool => output.push(0x01),
        HirNodeKind::TypeNat => output.push(0x02),
        HirNodeKind::TypeInt => output.push(0x03),
        HirNodeKind::TypeNamed(reference) => {
            output.push(0x04);
            encode_resolved_ref(output, reference);
        }
        HirNodeKind::TypeOnce { protocol } => {
            output.push(0x05);
            output.extend_from_slice(protocol.digest());
        }
        HirNodeKind::ProtocolTag { spelling } => {
            output.push(0x06);
            push_text(output, spelling);
        }
        HirNodeKind::BoolLiteral(value) => {
            output.push(0x10);
            output.push(u8::from(*value));
        }
        HirNodeKind::NaturalLiteral { magnitude } => {
            output.push(0x11);
            push_bytes(output, magnitude);
        }
        HirNodeKind::Reference(reference) => {
            output.push(0x12);
            encode_resolved_ref(output, reference);
        }
        HirNodeKind::Unary { operator, operand } => {
            output.push(0x13);
            output.push(match operator {
                HirUnaryOp::Not => 0x01,
                HirUnaryOp::Negate => 0x02,
            });
            output.extend_from_slice(operand.digest());
        }
        HirNodeKind::Binary {
            operator,
            left,
            right,
        } => {
            output.push(0x14);
            output.push(match operator {
                HirBinaryOp::Or => 0x01,
                HirBinaryOp::And => 0x02,
                HirBinaryOp::Implies => 0x03,
                HirBinaryOp::Equal => 0x04,
                HirBinaryOp::NotEqual => 0x05,
                HirBinaryOp::Less => 0x06,
                HirBinaryOp::LessEqual => 0x07,
                HirBinaryOp::Greater => 0x08,
                HirBinaryOp::GreaterEqual => 0x09,
                HirBinaryOp::Add => 0x0a,
                HirBinaryOp::Subtract => 0x0b,
                HirBinaryOp::Multiply => 0x0c,
            });
            output.extend_from_slice(left.digest());
            output.extend_from_slice(right.digest());
        }
        HirNodeKind::IntFromNat { operand } => {
            output.push(0x15);
            output.extend_from_slice(operand.digest());
        }
        HirNodeKind::Always { property } => {
            output.push(0x16);
            output.extend_from_slice(property.digest());
        }
        HirNodeKind::Eventually { property } => {
            output.push(0x17);
            output.extend_from_slice(property.digest());
        }
        HirNodeKind::Next { property } => {
            output.push(0x18);
            output.extend_from_slice(property.digest());
        }
        HirNodeKind::Until { left, right } => {
            output.push(0x19);
            output.extend_from_slice(left.digest());
            output.extend_from_slice(right.digest());
        }
        HirNodeKind::Enabled {
            action_origin,
            action,
        } => {
            output.push(0x1a);
            output.extend_from_slice(action_origin.digest());
            encode_resolved_ref(output, action);
        }
    }
}

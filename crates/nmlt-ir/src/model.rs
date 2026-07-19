use std::collections::{BTreeMap, BTreeSet};

use nmlt_hir::{DefId, LocalId, ModuleId, NodeId, ResolutionId};

use crate::identity::{CoreNodeId, CoreProgramId, core_program_id};
use crate::validate::{CoreValidationError, validate_program};

/// Explicit type carried by every core term.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoreType {
    Bool,
    Nat,
    Int,
    Enum(DefId),
    Once { protocol: NodeId },
    StateProp { system: DefId },
    TemporalProp { system: DefId },
}

impl CoreType {
    #[must_use]
    pub const fn is_scalar(&self) -> bool {
        matches!(self, Self::Bool | Self::Nat | Self::Int | Self::Enum(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoreUnaryOp {
    Not,
    Negate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoreBinaryOp {
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

/// Dedicated, string-free core term constructors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreTermKind {
    Bool(bool),
    Nat {
        magnitude: Vec<u8>,
    },
    Int {
        negative: bool,
        magnitude: Vec<u8>,
    },
    Local(LocalId),
    State {
        system: DefId,
        state: DefId,
    },
    Constructor {
        enumeration: DefId,
        constructor: DefId,
    },
    Unary {
        operator: CoreUnaryOp,
        operand: CoreNodeId,
    },
    Binary {
        operator: CoreBinaryOp,
        left: CoreNodeId,
        right: CoreNodeId,
    },
    IntFromNat {
        operand: CoreNodeId,
    },
    StatePredicate {
        system: DefId,
        condition: CoreNodeId,
    },
    Always {
        system: DefId,
        property: CoreNodeId,
    },
    Eventually {
        system: DefId,
        property: CoreNodeId,
    },
    Next {
        system: DefId,
        property: CoreNodeId,
    },
    Until {
        system: DefId,
        left: CoreNodeId,
        right: CoreNodeId,
    },
    Enabled {
        system: DefId,
        action: DefId,
    },
    ActionOccurred {
        system: DefId,
        action: DefId,
    },
}

/// One explicitly typed node. `owner` is the HIR definition whose semantic
/// path produced `origin`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreTerm {
    pub(crate) id: CoreNodeId,
    pub(crate) origin: NodeId,
    pub(crate) owner: DefId,
    pub(crate) ty: CoreType,
    pub(crate) kind: CoreTermKind,
}

impl CoreTerm {
    #[must_use]
    pub const fn new(
        id: CoreNodeId,
        origin: NodeId,
        owner: DefId,
        ty: CoreType,
        kind: CoreTermKind,
    ) -> Self {
        Self {
            id,
            origin,
            owner,
            ty,
            kind,
        }
    }

    #[must_use]
    pub const fn id(&self) -> CoreNodeId {
        self.id
    }

    #[must_use]
    pub const fn origin(&self) -> NodeId {
        self.origin
    }

    #[must_use]
    pub const fn owner(&self) -> DefId {
        self.owner
    }

    #[must_use]
    pub const fn ty(&self) -> &CoreType {
        &self.ty
    }

    #[must_use]
    pub const fn kind(&self) -> &CoreTermKind {
        &self.kind
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreEnum {
    pub(crate) id: DefId,
    pub(crate) constructors: BTreeSet<DefId>,
}

impl CoreEnum {
    #[must_use]
    pub const fn new(id: DefId, constructors: BTreeSet<DefId>) -> Self {
        Self { id, constructors }
    }

    #[must_use]
    pub const fn id(&self) -> DefId {
        self.id
    }

    #[must_use]
    pub const fn constructors(&self) -> &BTreeSet<DefId> {
        &self.constructors
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreStateField {
    pub(crate) id: DefId,
    pub(crate) ty: CoreType,
    pub(crate) initializer: CoreNodeId,
}

impl CoreStateField {
    #[must_use]
    pub const fn new(id: DefId, ty: CoreType, initializer: CoreNodeId) -> Self {
        Self {
            id,
            ty,
            initializer,
        }
    }

    #[must_use]
    pub const fn id(&self) -> DefId {
        self.id
    }

    #[must_use]
    pub const fn ty(&self) -> &CoreType {
        &self.ty
    }

    #[must_use]
    pub const fn initializer(&self) -> CoreNodeId {
        self.initializer
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreCapability {
    pub(crate) id: DefId,
    pub(crate) protocol: NodeId,
}

impl CoreCapability {
    #[must_use]
    pub const fn new(id: DefId, protocol: NodeId) -> Self {
        Self { id, protocol }
    }

    #[must_use]
    pub const fn id(&self) -> DefId {
        self.id
    }

    #[must_use]
    pub const fn protocol(&self) -> NodeId {
        self.protocol
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreActionParameter {
    pub(crate) id: LocalId,
    pub(crate) ty: CoreType,
}

impl CoreActionParameter {
    #[must_use]
    pub const fn new(id: LocalId, ty: CoreType) -> Self {
        Self { id, ty }
    }

    #[must_use]
    pub const fn id(&self) -> LocalId {
        self.id
    }

    #[must_use]
    pub const fn ty(&self) -> &CoreType {
        &self.ty
    }
}

/// An action with an explicit simultaneous update/frame partition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreAction {
    pub(crate) id: DefId,
    pub(crate) system: DefId,
    pub(crate) parameters: BTreeMap<LocalId, CoreActionParameter>,
    pub(crate) guards: Vec<CoreNodeId>,
    pub(crate) updates: BTreeMap<DefId, CoreNodeId>,
    pub(crate) frames: BTreeSet<DefId>,
    pub(crate) outputs: Vec<CoreNodeId>,
    pub(crate) consumes: BTreeSet<DefId>,
}

impl CoreAction {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: DefId,
        system: DefId,
        parameters: BTreeMap<LocalId, CoreActionParameter>,
        guards: Vec<CoreNodeId>,
        updates: BTreeMap<DefId, CoreNodeId>,
        frames: BTreeSet<DefId>,
        outputs: Vec<CoreNodeId>,
        consumes: BTreeSet<DefId>,
    ) -> Self {
        Self {
            id,
            system,
            parameters,
            guards,
            updates,
            frames,
            outputs,
            consumes,
        }
    }

    #[must_use]
    pub const fn id(&self) -> DefId {
        self.id
    }
    #[must_use]
    pub const fn system(&self) -> DefId {
        self.system
    }
    #[must_use]
    pub const fn parameters(&self) -> &BTreeMap<LocalId, CoreActionParameter> {
        &self.parameters
    }
    #[must_use]
    pub fn guards(&self) -> &[CoreNodeId] {
        &self.guards
    }
    #[must_use]
    pub const fn updates(&self) -> &BTreeMap<DefId, CoreNodeId> {
        &self.updates
    }
    #[must_use]
    pub const fn frames(&self) -> &BTreeSet<DefId> {
        &self.frames
    }
    #[must_use]
    pub fn outputs(&self) -> &[CoreNodeId] {
        &self.outputs
    }
    #[must_use]
    pub const fn consumes(&self) -> &BTreeSet<DefId> {
        &self.consumes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CorePropertyKind {
    Safety,
    Temporal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreProperty {
    pub(crate) id: DefId,
    pub(crate) system: DefId,
    pub(crate) kind: CorePropertyKind,
    pub(crate) body: CoreNodeId,
}

impl CoreProperty {
    #[must_use]
    pub const fn new(id: DefId, system: DefId, kind: CorePropertyKind, body: CoreNodeId) -> Self {
        Self {
            id,
            system,
            kind,
            body,
        }
    }

    #[must_use]
    pub const fn id(&self) -> DefId {
        self.id
    }
    #[must_use]
    pub const fn system(&self) -> DefId {
        self.system
    }
    #[must_use]
    pub const fn kind(&self) -> CorePropertyKind {
        self.kind
    }
    #[must_use]
    pub const fn body(&self) -> CoreNodeId {
        self.body
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreObservation {
    pub(crate) owner: DefId,
    pub(crate) origin: NodeId,
    pub(crate) items: Vec<CoreNodeId>,
}

impl CoreObservation {
    #[must_use]
    pub const fn new(owner: DefId, origin: NodeId, items: Vec<CoreNodeId>) -> Self {
        Self {
            owner,
            origin,
            items,
        }
    }

    #[must_use]
    pub const fn owner(&self) -> DefId {
        self.owner
    }

    #[must_use]
    pub const fn origin(&self) -> NodeId {
        self.origin
    }
    #[must_use]
    pub fn items(&self) -> &[CoreNodeId] {
        &self.items
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreSystem {
    pub(crate) id: DefId,
    pub(crate) state: BTreeMap<DefId, CoreStateField>,
    pub(crate) capabilities: BTreeMap<DefId, CoreCapability>,
    pub(crate) actions: BTreeMap<DefId, CoreAction>,
    pub(crate) properties: BTreeMap<DefId, CoreProperty>,
    pub(crate) observations: Vec<CoreObservation>,
}

impl CoreSystem {
    #[must_use]
    pub fn new(
        id: DefId,
        state: BTreeMap<DefId, CoreStateField>,
        capabilities: BTreeMap<DefId, CoreCapability>,
        actions: BTreeMap<DefId, CoreAction>,
        properties: BTreeMap<DefId, CoreProperty>,
        observations: Vec<CoreObservation>,
    ) -> Self {
        Self {
            id,
            state,
            capabilities,
            actions,
            properties,
            observations,
        }
    }

    #[must_use]
    pub const fn id(&self) -> DefId {
        self.id
    }
    #[must_use]
    pub const fn state(&self) -> &BTreeMap<DefId, CoreStateField> {
        &self.state
    }
    #[must_use]
    pub const fn capabilities(&self) -> &BTreeMap<DefId, CoreCapability> {
        &self.capabilities
    }
    #[must_use]
    pub const fn actions(&self) -> &BTreeMap<DefId, CoreAction> {
        &self.actions
    }
    #[must_use]
    pub const fn properties(&self) -> &BTreeMap<DefId, CoreProperty> {
        &self.properties
    }
    #[must_use]
    pub fn observations(&self) -> &[CoreObservation] {
        &self.observations
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreModule {
    pub(crate) id: ModuleId,
    pub(crate) imports: BTreeSet<ModuleId>,
    pub(crate) enumerations: BTreeMap<DefId, CoreEnum>,
    pub(crate) systems: BTreeMap<DefId, CoreSystem>,
}

impl CoreModule {
    #[must_use]
    pub fn new(
        id: ModuleId,
        imports: BTreeSet<ModuleId>,
        enumerations: BTreeMap<DefId, CoreEnum>,
        systems: BTreeMap<DefId, CoreSystem>,
    ) -> Self {
        Self {
            id,
            imports,
            enumerations,
            systems,
        }
    }

    #[must_use]
    pub const fn id(&self) -> ModuleId {
        self.id
    }
    #[must_use]
    pub const fn imports(&self) -> &BTreeSet<ModuleId> {
        &self.imports
    }
    #[must_use]
    pub const fn enumerations(&self) -> &BTreeMap<DefId, CoreEnum> {
        &self.enumerations
    }
    #[must_use]
    pub const fn systems(&self) -> &BTreeMap<DefId, CoreSystem> {
        &self.systems
    }
}

/// Structurally validated explicit core tied to one exact resolved HIR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreProgram {
    pub(crate) resolved_hir_id: ResolutionId,
    pub(crate) modules: BTreeMap<ModuleId, CoreModule>,
    pub(crate) terms: BTreeMap<CoreNodeId, CoreTerm>,
    pub(crate) id: CoreProgramId,
}

impl CoreProgram {
    pub fn new(
        resolved_hir_id: ResolutionId,
        modules: impl IntoIterator<Item = CoreModule>,
        terms: impl IntoIterator<Item = CoreTerm>,
    ) -> Result<Self, CoreValidationError> {
        let mut module_map = BTreeMap::new();
        for module in modules {
            if module_map.insert(module.id, module).is_some() {
                return Err(CoreValidationError::KeyMismatch {
                    context: "duplicate module",
                });
            }
        }
        let mut term_map = BTreeMap::new();
        for term in terms {
            let term_id = term.id;
            if term_map.insert(term_id, term).is_some() {
                return Err(CoreValidationError::DuplicateCoreNode(term_id));
            }
        }
        let mut program = Self {
            resolved_hir_id,
            modules: module_map,
            terms: term_map,
            id: CoreProgramId::zero(),
        };
        validate_program(&program)?;
        program.id = core_program_id(&program)?;
        Ok(program)
    }

    #[must_use]
    pub const fn id(&self) -> CoreProgramId {
        self.id
    }
    #[must_use]
    pub const fn resolved_hir_id(&self) -> ResolutionId {
        self.resolved_hir_id
    }
    #[must_use]
    pub const fn modules(&self) -> &BTreeMap<ModuleId, CoreModule> {
        &self.modules
    }
    #[must_use]
    pub const fn terms(&self) -> &BTreeMap<CoreNodeId, CoreTerm> {
        &self.terms
    }
}

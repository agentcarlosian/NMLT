use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use nmlt_hir::{
    DeclarationFlavor, DefId, HirBinaryOp, HirNode, HirNodeKind, HirRoot, HirUnaryOp, LocalBinder,
    LocalId, ModuleId, Namespace, NodeId, ResolvedProgram, ResolvedRef, SemanticPathSegment,
};
use nmlt_ir::{
    CoreAction, CoreActionParameter, CoreBinaryOp, CoreCapability, CoreEnum, CoreIdentityError,
    CoreModule, CoreNodeId, CoreObservation, CoreProgram, CoreProperty, CorePropertyKind,
    CoreStateField, CoreSystem, CoreTerm, CoreTermKind, CoreType, CoreUnaryOp, CoreValidationError,
};

use crate::identity::{
    DerivationNodeId, certificate_id, make_derivation_node, resource_policy_id, ruleset_bundle_id,
};
use crate::model::{
    DerivationConclusion, DerivationNode, DerivationWitness, ElaborationArtifact, ElaborationRule,
    JudgmentKind, ObligationKey,
};

const MAX_DERIVATIONS: usize = 524_288;
const MAX_PREMISE_EDGES: usize = 2_097_152;
const MAX_PREMISES: usize = 32;
const MAX_CERTIFICATE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ElaborationError {
    MissingNode(NodeId),
    MissingRoot {
        owner: DefId,
        role: &'static str,
    },
    MissingDefinition(DefId),
    InvalidHir {
        origin: NodeId,
        reason: &'static str,
    },
    TypeMismatch {
        origin: NodeId,
        expected: CoreType,
        actual: CoreType,
    },
    DuplicateObligation(ObligationKey),
    DuplicateCoreNode(CoreNodeId),
    ArtifactInvariant(&'static str),
    ResourceLimit {
        resource: &'static str,
        actual: usize,
        maximum: usize,
    },
    CoreIdentity(CoreIdentityError),
    CoreValidation(CoreValidationError),
}

impl fmt::Display for ElaborationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(id) => write!(formatter, "HIR node `{id}` is missing"),
            Self::MissingRoot { owner, role } => {
                write!(formatter, "definition `{owner}` has no {role} HIR root")
            }
            Self::MissingDefinition(id) => {
                write!(formatter, "definition `{id}` is absent from resolved HIR")
            }
            Self::InvalidHir { origin, reason } => {
                write!(formatter, "cannot elaborate HIR node `{origin}`: {reason}")
            }
            Self::TypeMismatch {
                origin,
                expected,
                actual,
            } => write!(
                formatter,
                "HIR node `{origin}` checks against {expected:?}, but synthesizes {actual:?}"
            ),
            Self::DuplicateObligation(key) => {
                write!(formatter, "duplicate elaboration obligation {key:?}")
            }
            Self::DuplicateCoreNode(id) => {
                write!(formatter, "two elaboration paths emitted core node `{id}`")
            }
            Self::ArtifactInvariant(reason) => {
                write!(formatter, "invalid elaboration artifact: {reason}")
            }
            Self::ResourceLimit {
                resource,
                actual,
                maximum,
            } => write!(
                formatter,
                "NMLT_ELABORATION_RESOURCE_LIMIT: {resource} is {actual}, maximum {maximum}"
            ),
            Self::CoreIdentity(error) => write!(formatter, "core identity failure: {error}"),
            Self::CoreValidation(error) => write!(
                formatter,
                "emitted core failed structural validation: {error}"
            ),
        }
    }
}

impl std::error::Error for ElaborationError {}

impl From<CoreIdentityError> for ElaborationError {
    fn from(value: CoreIdentityError) -> Self {
        Self::CoreIdentity(value)
    }
}

impl From<CoreValidationError> for ElaborationError {
    fn from(value: CoreValidationError) -> Self {
        Self::CoreValidation(value)
    }
}

#[derive(Clone)]
struct DeclMeta {
    module: ModuleId,
    namespace: Namespace,
    parent: Option<DefId>,
    flavor: DeclarationFlavor,
}

struct Index<'a> {
    declarations: BTreeMap<DefId, DeclMeta>,
    nodes: BTreeMap<NodeId, &'a HirNode>,
    roots: BTreeMap<DefId, Vec<&'a HirRoot>>,
    locals: BTreeMap<LocalId, &'a LocalBinder>,
}

impl<'a> Index<'a> {
    fn new(hir: &'a ResolvedProgram) -> Result<Self, ElaborationError> {
        let mut declarations = BTreeMap::new();
        let mut nodes = BTreeMap::new();
        let mut roots: BTreeMap<DefId, Vec<&HirRoot>> = BTreeMap::new();
        let mut locals = BTreeMap::new();
        for module in hir.modules().values() {
            let mut ids_by_path = BTreeMap::new();
            for declaration in module.declarations().values() {
                ids_by_path.insert(declaration.key().path.clone(), declaration.id());
            }
            for declaration in module.declarations().values() {
                let mut parent_path = declaration.key().path.clone();
                parent_path.segments.pop();
                let parent = if parent_path.segments.is_empty() {
                    None
                } else {
                    ids_by_path.get(&parent_path).copied()
                };
                declarations.insert(
                    declaration.id(),
                    DeclMeta {
                        module: module.id(),
                        namespace: declaration
                            .key()
                            .namespace()
                            .ok_or(ElaborationError::MissingDefinition(declaration.id()))?,
                        parent,
                        flavor: declaration.flavor(),
                    },
                );
            }
            for (id, node) in module.hir_nodes() {
                if nodes.insert(*id, node).is_some() {
                    return Err(ElaborationError::InvalidHir {
                        origin: *id,
                        reason: "node identity occurs in more than one module",
                    });
                }
            }
            for root in module.hir_roots() {
                roots.entry(root.owner()).or_default().push(root);
            }
            for (id, binder) in module.local_binders() {
                locals.insert(*id, binder);
            }
        }
        for values in roots.values_mut() {
            values.sort_by(|left, right| left.semantic_path().cmp(right.semantic_path()));
        }
        Ok(Self {
            declarations,
            nodes,
            roots,
            locals,
        })
    }

    fn node(&self, id: NodeId) -> Result<&'a HirNode, ElaborationError> {
        self.nodes
            .get(&id)
            .copied()
            .ok_or(ElaborationError::MissingNode(id))
    }

    fn roots(&self, owner: DefId) -> impl Iterator<Item = &'a HirRoot> + '_ {
        self.roots.get(&owner).into_iter().flatten().copied()
    }

    fn root_exact(
        &self,
        owner: DefId,
        segment: &SemanticPathSegment,
        role: &'static str,
    ) -> Result<&'a HirRoot, ElaborationError> {
        self.roots(owner)
            .find(|root| root.semantic_path().segments() == [segment.clone()])
            .ok_or(ElaborationError::MissingRoot { owner, role })
    }
}

#[derive(Clone)]
struct TermResult {
    node: CoreNodeId,
    ty: CoreType,
    derivation: DerivationNodeId,
}

struct Builder<'a> {
    index: Index<'a>,
    state_types: BTreeMap<DefId, CoreType>,
    local_types: BTreeMap<LocalId, CoreType>,
    terms: BTreeMap<CoreNodeId, CoreTerm>,
    derivations: BTreeMap<DerivationNodeId, DerivationNode>,
    obligations: BTreeMap<ObligationKey, DerivationNodeId>,
    required_roots: BTreeMap<ObligationKey, DerivationNodeId>,
    type_cache: BTreeMap<NodeId, (CoreType, DerivationNodeId)>,
    value_cache: BTreeMap<(NodeId, Option<CoreType>), TermResult>,
    formula_cache: BTreeMap<(NodeId, DefId), TermResult>,
    premise_edges: usize,
}

impl<'a> Builder<'a> {
    fn new(hir: &'a ResolvedProgram) -> Result<Self, ElaborationError> {
        Ok(Self {
            index: Index::new(hir)?,
            state_types: BTreeMap::new(),
            local_types: BTreeMap::new(),
            terms: BTreeMap::new(),
            derivations: BTreeMap::new(),
            obligations: BTreeMap::new(),
            required_roots: BTreeMap::new(),
            type_cache: BTreeMap::new(),
            value_cache: BTreeMap::new(),
            formula_cache: BTreeMap::new(),
            premise_edges: 0,
        })
    }

    fn add_derivation(
        &mut self,
        rule: ElaborationRule,
        obligation: ObligationKey,
        conclusion: DerivationConclusion,
        witness: DerivationWitness,
        premises: Vec<DerivationNodeId>,
    ) -> Result<DerivationNodeId, ElaborationError> {
        if premises.len() > MAX_PREMISES {
            return Err(ElaborationError::ResourceLimit {
                resource: "premises per derivation",
                actual: premises.len(),
                maximum: MAX_PREMISES,
            });
        }
        self.premise_edges = self.premise_edges.saturating_add(premises.len());
        if self.premise_edges > MAX_PREMISE_EDGES {
            return Err(ElaborationError::ResourceLimit {
                resource: "premise edges",
                actual: self.premise_edges,
                maximum: MAX_PREMISE_EDGES,
            });
        }
        if self.obligations.contains_key(&obligation) {
            return Err(ElaborationError::DuplicateObligation(obligation));
        }
        let node = make_derivation_node(rule, obligation, conclusion, witness, premises);
        let id = node.id();
        self.obligations.insert(obligation, id);
        self.derivations.insert(id, node);
        if self.derivations.len() > MAX_DERIVATIONS {
            return Err(ElaborationError::ResourceLimit {
                resource: "derivation nodes",
                actual: self.derivations.len(),
                maximum: MAX_DERIVATIONS,
            });
        }
        Ok(id)
    }

    fn require(
        &mut self,
        obligation: ObligationKey,
        derivation: DerivationNodeId,
    ) -> Result<(), ElaborationError> {
        if self.required_roots.insert(obligation, derivation).is_some() {
            return Err(ElaborationError::DuplicateObligation(obligation));
        }
        Ok(())
    }

    fn insert_term(&mut self, term: CoreTerm) -> Result<(), ElaborationError> {
        let id = term.id();
        if self.terms.insert(id, term).is_some() {
            return Err(ElaborationError::DuplicateCoreNode(id));
        }
        Ok(())
    }

    fn validate_artifact_shape(&self) -> Result<(), ElaborationError> {
        let expected_roots = self
            .index
            .roots
            .values()
            .flatten()
            .map(|root| {
                let judgment = match root.semantic_path().segments() {
                    [SemanticPathSegment::DeclaredType]
                    | [
                        SemanticPathSegment::ActionParameter(_),
                        SemanticPathSegment::DeclaredType,
                    ] => JudgmentKind::TypeFormation,
                    [SemanticPathSegment::Initializer]
                    | [SemanticPathSegment::Guard(_)]
                    | [SemanticPathSegment::UpdateValue(_)] => JudgmentKind::Check,
                    [SemanticPathSegment::UpdateTarget(_)] => JudgmentKind::UpdateTarget,
                    [SemanticPathSegment::Output(_)]
                    | [SemanticPathSegment::ObservationItem(_)] => JudgmentKind::Synthesize,
                    [SemanticPathSegment::Consume(_)] => JudgmentKind::CapabilityUse,
                    [SemanticPathSegment::PropertyBody] => JudgmentKind::Formula,
                    _ => {
                        return Err(ElaborationError::ArtifactInvariant(
                            "semantic root has no frozen elaboration judgment",
                        ));
                    }
                };
                Ok(ObligationKey::new(judgment, root.node()))
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let actual_roots = self.required_roots.keys().copied().collect::<BTreeSet<_>>();
        if expected_roots != actual_roots {
            return Err(ElaborationError::ArtifactInvariant(
                "required roots are not a bijection with resolved-HIR roots",
            ));
        }
        for (obligation, derivation) in &self.required_roots {
            let node =
                self.derivations
                    .get(derivation)
                    .ok_or(ElaborationError::ArtifactInvariant(
                        "required root names a missing derivation",
                    ))?;
            if node.obligation() != *obligation {
                return Err(ElaborationError::ArtifactInvariant(
                    "required root and derivation obligation disagree",
                ));
            }
        }

        let covered_origins = self
            .derivations
            .values()
            .map(|node| node.obligation().origin())
            .collect::<BTreeSet<_>>();
        let hir_origins = self.index.nodes.keys().copied().collect::<BTreeSet<_>>();
        if covered_origins != hir_origins {
            return Err(ElaborationError::ArtifactInvariant(
                "derivation subjects do not cover exactly the HIR node origins",
            ));
        }

        let mut reachable = BTreeSet::new();
        let mut pending = self.required_roots.values().copied().collect::<Vec<_>>();
        while let Some(id) = pending.pop() {
            if !reachable.insert(id) {
                continue;
            }
            let node = self
                .derivations
                .get(&id)
                .ok_or(ElaborationError::ArtifactInvariant(
                    "derivation premise is missing",
                ))?;
            pending.extend(node.premises().iter().copied());
        }
        if reachable.len() != self.derivations.len() {
            return Err(ElaborationError::ArtifactInvariant(
                "derivation DAG contains unreachable nodes",
            ));
        }
        Ok(())
    }

    fn type_from_node(
        &mut self,
        id: NodeId,
    ) -> Result<(CoreType, DerivationNodeId), ElaborationError> {
        if let Some(result) = self.type_cache.get(&id) {
            return Ok(result.clone());
        }
        let node = self.index.node(id)?.clone();
        let obligation = ObligationKey::new(JudgmentKind::TypeFormation, id);
        let (ty, rule, witness, premises) = match node.kind() {
            HirNodeKind::TypeBool => (
                CoreType::Bool,
                ElaborationRule::TypeBool,
                DerivationWitness::None,
                vec![],
            ),
            HirNodeKind::TypeNat => (
                CoreType::Nat,
                ElaborationRule::TypeNat,
                DerivationWitness::None,
                vec![],
            ),
            HirNodeKind::TypeInt => (
                CoreType::Int,
                ElaborationRule::TypeInt,
                DerivationWitness::None,
                vec![],
            ),
            HirNodeKind::TypeNamed(ResolvedRef::Definition(definition)) => {
                let meta = self
                    .index
                    .declarations
                    .get(definition)
                    .ok_or(ElaborationError::MissingDefinition(*definition))?;
                if meta.namespace != Namespace::Type {
                    return Err(ElaborationError::InvalidHir {
                        origin: id,
                        reason: "named type does not target an enumeration",
                    });
                }
                (
                    CoreType::Enum(*definition),
                    ElaborationRule::TypeEnum,
                    DerivationWitness::Definition(*definition),
                    vec![],
                )
            }
            HirNodeKind::TypeOnce { protocol } => {
                let protocol_node = self.index.node(*protocol)?;
                if !matches!(protocol_node.kind(), HirNodeKind::ProtocolTag { .. }) {
                    return Err(ElaborationError::InvalidHir {
                        origin: *protocol,
                        reason: "Once protocol is not a protocol tag",
                    });
                }
                let protocol_obligation =
                    ObligationKey::new(JudgmentKind::ProtocolFormation, *protocol);
                let protocol_derivation = self.add_derivation(
                    ElaborationRule::ProtocolTag,
                    protocol_obligation,
                    DerivationConclusion::Protocol(*protocol),
                    DerivationWitness::None,
                    vec![],
                )?;
                (
                    CoreType::Once {
                        protocol: *protocol,
                    },
                    ElaborationRule::TypeOnce,
                    DerivationWitness::None,
                    vec![protocol_derivation],
                )
            }
            _ => {
                return Err(ElaborationError::InvalidHir {
                    origin: id,
                    reason: "expected a type node",
                });
            }
        };
        let derivation = self.add_derivation(
            rule,
            obligation,
            DerivationConclusion::Type(ty.clone()),
            witness,
            premises,
        )?;
        self.type_cache.insert(id, (ty.clone(), derivation));
        Ok((ty, derivation))
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_value(
        &mut self,
        hir: &HirNode,
        expected: Option<CoreType>,
        actual: CoreType,
        kind: CoreTermKind,
        rule: ElaborationRule,
        witness: DerivationWitness,
        premises: Vec<DerivationNodeId>,
    ) -> Result<TermResult, ElaborationError> {
        if let Some(expected) = &expected {
            if *expected != actual {
                return Err(ElaborationError::TypeMismatch {
                    origin: hir.id(),
                    expected: expected.clone(),
                    actual,
                });
            }
        }
        let node = CoreNodeId::from_origin(hir.id(), &[])?;
        self.insert_term(CoreTerm::new(
            node,
            hir.id(),
            hir.owner(),
            actual.clone(),
            kind,
        ))?;
        let judgment = if expected.is_some() {
            JudgmentKind::Check
        } else {
            JudgmentKind::Synthesize
        };
        let derivation = self.add_derivation(
            rule,
            ObligationKey::new(judgment, hir.id()),
            DerivationConclusion::Term {
                node,
                ty: actual.clone(),
            },
            witness,
            premises,
        )?;
        Ok(TermResult {
            node,
            ty: actual,
            derivation,
        })
    }

    fn elab_value(
        &mut self,
        id: NodeId,
        expected: Option<CoreType>,
    ) -> Result<TermResult, ElaborationError> {
        if let Some(result) = self.value_cache.get(&(id, expected.clone())) {
            return Ok(result.clone());
        }
        if let Some(expected_ty) = expected.as_ref()
            && let Some(synthesized) = self.value_cache.get(&(id, None)).cloned()
        {
            if synthesized.ty != *expected_ty {
                return Err(ElaborationError::TypeMismatch {
                    origin: id,
                    expected: expected_ty.clone(),
                    actual: synthesized.ty,
                });
            }
            let derivation = self.add_derivation(
                ElaborationRule::CheckSynthesis,
                ObligationKey::new(JudgmentKind::Check, id),
                DerivationConclusion::Term {
                    node: synthesized.node,
                    ty: synthesized.ty.clone(),
                },
                DerivationWitness::None,
                vec![synthesized.derivation],
            )?;
            let result = TermResult {
                derivation,
                ..synthesized
            };
            self.value_cache
                .insert((id, expected.clone()), result.clone());
            return Ok(result);
        }
        let hir = self.index.node(id)?.clone();
        let result = match hir.kind().clone() {
            HirNodeKind::BoolLiteral(value) => self.finish_value(
                &hir,
                expected.clone(),
                CoreType::Bool,
                CoreTermKind::Bool(value),
                ElaborationRule::BoolLiteral,
                DerivationWitness::Boolean(value),
                vec![],
            )?,
            HirNodeKind::NaturalLiteral { magnitude } => {
                let ty = match expected.as_ref() {
                    None | Some(CoreType::Nat) => CoreType::Nat,
                    Some(CoreType::Int) => CoreType::Int,
                    Some(other) => {
                        return Err(ElaborationError::TypeMismatch {
                            origin: id,
                            expected: other.clone(),
                            actual: CoreType::Nat,
                        });
                    }
                };
                let (kind, rule) = if ty == CoreType::Nat {
                    (
                        CoreTermKind::Nat {
                            magnitude: magnitude.clone(),
                        },
                        ElaborationRule::NatLiteral,
                    )
                } else {
                    (
                        CoreTermKind::Int {
                            negative: false,
                            magnitude: magnitude.clone(),
                        },
                        ElaborationRule::IntLiteral,
                    )
                };
                self.finish_value(
                    &hir,
                    expected.clone(),
                    ty,
                    kind,
                    rule,
                    DerivationWitness::Magnitude {
                        negative: false,
                        bytes: magnitude,
                    },
                    vec![],
                )?
            }
            HirNodeKind::Reference(reference) => {
                let (ty, kind, rule, witness) = self.reference_term(id, &reference)?;
                self.finish_value(&hir, expected.clone(), ty, kind, rule, witness, vec![])?
            }
            HirNodeKind::Unary { operator, operand } => match operator {
                HirUnaryOp::Not => {
                    let operand = self.elab_value(operand, Some(CoreType::Bool))?;
                    self.finish_value(
                        &hir,
                        expected.clone(),
                        CoreType::Bool,
                        CoreTermKind::Unary {
                            operator: CoreUnaryOp::Not,
                            operand: operand.node,
                        },
                        ElaborationRule::Not,
                        DerivationWitness::None,
                        vec![operand.derivation],
                    )?
                }
                HirUnaryOp::Negate => {
                    if matches!(
                        self.index.node(operand)?.kind(),
                        HirNodeKind::NaturalLiteral { magnitude } if magnitude.is_empty()
                    ) {
                        return Err(ElaborationError::InvalidHir {
                            origin: id,
                            reason: "negative zero is forbidden",
                        });
                    }
                    let operand = self.elab_value(operand, Some(CoreType::Int))?;
                    self.finish_value(
                        &hir,
                        expected.clone(),
                        CoreType::Int,
                        CoreTermKind::Unary {
                            operator: CoreUnaryOp::Negate,
                            operand: operand.node,
                        },
                        ElaborationRule::Negate,
                        DerivationWitness::None,
                        vec![operand.derivation],
                    )?
                }
            },
            HirNodeKind::Binary {
                operator,
                left,
                right,
            } => self.elab_value_binary(&hir, expected.clone(), operator, left, right)?,
            HirNodeKind::IntFromNat { operand } => {
                let operand = self.elab_value(operand, Some(CoreType::Nat))?;
                self.finish_value(
                    &hir,
                    expected.clone(),
                    CoreType::Int,
                    CoreTermKind::IntFromNat {
                        operand: operand.node,
                    },
                    ElaborationRule::IntFromNat,
                    DerivationWitness::None,
                    vec![operand.derivation],
                )?
            }
            _ => {
                return Err(ElaborationError::InvalidHir {
                    origin: id,
                    reason: "temporal or type node used in a value judgment",
                });
            }
        };
        self.value_cache.insert((id, expected), result.clone());
        Ok(result)
    }

    fn reference_term(
        &self,
        origin: NodeId,
        reference: &ResolvedRef,
    ) -> Result<(CoreType, CoreTermKind, ElaborationRule, DerivationWitness), ElaborationError>
    {
        match reference {
            ResolvedRef::Local(local) => {
                let ty =
                    self.local_types
                        .get(local)
                        .cloned()
                        .ok_or(ElaborationError::InvalidHir {
                            origin,
                            reason: "local reference has no declared type",
                        })?;
                Ok((
                    ty,
                    CoreTermKind::Local(*local),
                    ElaborationRule::LocalReference,
                    DerivationWitness::None,
                ))
            }
            ResolvedRef::StateField { system, state } => {
                let ty =
                    self.state_types
                        .get(state)
                        .cloned()
                        .ok_or(ElaborationError::InvalidHir {
                            origin,
                            reason: "state reference has no declared type",
                        })?;
                Ok((
                    ty,
                    CoreTermKind::State {
                        system: *system,
                        state: *state,
                    },
                    ElaborationRule::StateReference,
                    DerivationWitness::SystemDefinition {
                        system: *system,
                        definition: *state,
                    },
                ))
            }
            ResolvedRef::Constructor {
                enumeration,
                constructor,
            } => Ok((
                CoreType::Enum(*enumeration),
                CoreTermKind::Constructor {
                    enumeration: *enumeration,
                    constructor: *constructor,
                },
                ElaborationRule::ConstructorReference,
                DerivationWitness::SystemDefinition {
                    system: *enumeration,
                    definition: *constructor,
                },
            )),
            ResolvedRef::Definition(_) | ResolvedRef::Capability { .. } => {
                Err(ElaborationError::InvalidHir {
                    origin,
                    reason: "definition kind is not a value",
                })
            }
        }
    }

    fn elab_value_binary(
        &mut self,
        hir: &HirNode,
        expected: Option<CoreType>,
        operator: HirBinaryOp,
        left: NodeId,
        right: NodeId,
    ) -> Result<TermResult, ElaborationError> {
        let (left, right, result_ty) = match operator {
            HirBinaryOp::Or | HirBinaryOp::And | HirBinaryOp::Implies => (
                self.elab_value(left, Some(CoreType::Bool))?,
                self.elab_value(right, Some(CoreType::Bool))?,
                CoreType::Bool,
            ),
            HirBinaryOp::Subtract => (
                self.elab_value(left, Some(CoreType::Int))?,
                self.elab_value(right, Some(CoreType::Int))?,
                CoreType::Int,
            ),
            HirBinaryOp::Add | HirBinaryOp::Multiply => {
                let operand_ty = match expected.as_ref() {
                    Some(CoreType::Nat) => CoreType::Nat,
                    Some(CoreType::Int) => CoreType::Int,
                    _ => self.infer_pair_type(left, right, true)?,
                };
                (
                    self.elab_value(left, Some(operand_ty.clone()))?,
                    self.elab_value(right, Some(operand_ty.clone()))?,
                    operand_ty,
                )
            }
            HirBinaryOp::Equal | HirBinaryOp::NotEqual => {
                let operand_ty = self.infer_pair_type(left, right, false)?;
                (
                    self.elab_value(left, Some(operand_ty.clone()))?,
                    self.elab_value(right, Some(operand_ty))?,
                    CoreType::Bool,
                )
            }
            HirBinaryOp::Less
            | HirBinaryOp::LessEqual
            | HirBinaryOp::Greater
            | HirBinaryOp::GreaterEqual => {
                let operand_ty = self.infer_pair_type(left, right, true)?;
                (
                    self.elab_value(left, Some(operand_ty.clone()))?,
                    self.elab_value(right, Some(operand_ty))?,
                    CoreType::Bool,
                )
            }
        };
        let core_operator = map_binary(operator);
        self.finish_value(
            hir,
            expected,
            result_ty,
            CoreTermKind::Binary {
                operator: core_operator,
                left: left.node,
                right: right.node,
            },
            rule_binary(operator),
            DerivationWitness::None,
            vec![left.derivation, right.derivation],
        )
    }

    fn infer_pair_type(
        &mut self,
        left: NodeId,
        right: NodeId,
        numeric: bool,
    ) -> Result<CoreType, ElaborationError> {
        let left_literal = matches!(
            self.index.node(left)?.kind(),
            HirNodeKind::NaturalLiteral { .. }
        );
        let right_literal = matches!(
            self.index.node(right)?.kind(),
            HirNodeKind::NaturalLiteral { .. }
        );
        let ty = if left_literal && !right_literal {
            self.elab_value(right, None)?.ty
        } else {
            self.elab_value(left, None)?.ty
        };
        let valid = if numeric {
            matches!(ty, CoreType::Nat | CoreType::Int)
        } else {
            ty.is_scalar()
        };
        if !valid {
            return Err(ElaborationError::InvalidHir {
                origin: left,
                reason: "binary operands have an unsupported type",
            });
        }
        Ok(ty)
    }

    fn elab_formula(&mut self, id: NodeId, system: DefId) -> Result<TermResult, ElaborationError> {
        if let Some(result) = self.formula_cache.get(&(id, system)) {
            return Ok(result.clone());
        }
        let hir = self.index.node(id)?.clone();
        let result = match hir.kind().clone() {
            HirNodeKind::Always { property }
            | HirNodeKind::Eventually { property }
            | HirNodeKind::Next { property } => {
                let property = self.elab_formula(property, system)?;
                let (kind, rule) = match hir.kind() {
                    HirNodeKind::Always { .. } => (
                        CoreTermKind::Always {
                            system,
                            property: property.node,
                        },
                        ElaborationRule::Always,
                    ),
                    HirNodeKind::Eventually { .. } => (
                        CoreTermKind::Eventually {
                            system,
                            property: property.node,
                        },
                        ElaborationRule::Eventually,
                    ),
                    HirNodeKind::Next { .. } => (
                        CoreTermKind::Next {
                            system,
                            property: property.node,
                        },
                        ElaborationRule::Next,
                    ),
                    _ => unreachable!(),
                };
                self.finish_formula_term(
                    &hir,
                    CoreType::TemporalProp { system },
                    kind,
                    rule,
                    vec![property.derivation],
                    DerivationWitness::None,
                )?
            }
            HirNodeKind::Until { left, right } => {
                let left = self.elab_formula(left, system)?;
                let right = self.elab_formula(right, system)?;
                self.finish_formula_term(
                    &hir,
                    CoreType::TemporalProp { system },
                    CoreTermKind::Until {
                        system,
                        left: left.node,
                        right: right.node,
                    },
                    ElaborationRule::Until,
                    vec![left.derivation, right.derivation],
                    DerivationWitness::None,
                )?
            }
            HirNodeKind::Enabled {
                action_origin,
                action,
            } => {
                let ResolvedRef::Definition(action) = action else {
                    return Err(ElaborationError::InvalidHir {
                        origin: id,
                        reason: "enabled target is not an action definition",
                    });
                };
                let meta = self
                    .index
                    .declarations
                    .get(&action)
                    .ok_or(ElaborationError::MissingDefinition(action))?;
                if meta.namespace != Namespace::Action || meta.parent != Some(system) {
                    return Err(ElaborationError::InvalidHir {
                        origin: id,
                        reason: "enabled target belongs to another system",
                    });
                }
                let action_derivation = self.definition_use(
                    action_origin,
                    JudgmentKind::ActionUse,
                    ElaborationRule::ActionUse,
                    action,
                )?;
                self.finish_formula_term(
                    &hir,
                    CoreType::StateProp { system },
                    CoreTermKind::Enabled { system, action },
                    ElaborationRule::Enabled,
                    vec![action_derivation],
                    DerivationWitness::SystemDefinition {
                        system,
                        definition: action,
                    },
                )?
            }
            HirNodeKind::Unary {
                operator: HirUnaryOp::Not,
                operand,
            } => {
                let operand = self.elab_formula(operand, system)?;
                self.finish_formula_term(
                    &hir,
                    operand.ty.clone(),
                    CoreTermKind::Unary {
                        operator: CoreUnaryOp::Not,
                        operand: operand.node,
                    },
                    ElaborationRule::Not,
                    vec![operand.derivation],
                    DerivationWitness::None,
                )?
            }
            HirNodeKind::Binary {
                operator: HirBinaryOp::Or | HirBinaryOp::And | HirBinaryOp::Implies,
                left,
                right,
            } => {
                let left = self.elab_formula(left, system)?;
                let right = self.elab_formula(right, system)?;
                let temporal = matches!(left.ty, CoreType::TemporalProp { .. })
                    || matches!(right.ty, CoreType::TemporalProp { .. });
                let ty = if temporal {
                    CoreType::TemporalProp { system }
                } else {
                    CoreType::StateProp { system }
                };
                let operator = match hir.kind() {
                    HirNodeKind::Binary { operator, .. } => *operator,
                    _ => unreachable!(),
                };
                self.finish_formula_term(
                    &hir,
                    ty,
                    CoreTermKind::Binary {
                        operator: map_binary(operator),
                        left: left.node,
                        right: right.node,
                    },
                    rule_binary(operator),
                    vec![left.derivation, right.derivation],
                    DerivationWitness::None,
                )?
            }
            _ => {
                let condition = self.elab_value(id, Some(CoreType::Bool))?;
                let core_node = CoreNodeId::from_origin(id, &[1])?;
                let ty = CoreType::StateProp { system };
                self.insert_term(CoreTerm::new(
                    core_node,
                    id,
                    hir.owner(),
                    ty.clone(),
                    CoreTermKind::StatePredicate {
                        system,
                        condition: condition.node,
                    },
                ))?;
                let derivation = self.add_derivation(
                    ElaborationRule::StatePredicate,
                    ObligationKey::new(JudgmentKind::Formula, id),
                    DerivationConclusion::Term {
                        node: core_node,
                        ty: ty.clone(),
                    },
                    DerivationWitness::None,
                    vec![condition.derivation],
                )?;
                TermResult {
                    node: core_node,
                    ty,
                    derivation,
                }
            }
        };
        self.formula_cache.insert((id, system), result.clone());
        Ok(result)
    }

    fn finish_formula_term(
        &mut self,
        hir: &HirNode,
        ty: CoreType,
        kind: CoreTermKind,
        rule: ElaborationRule,
        premises: Vec<DerivationNodeId>,
        witness: DerivationWitness,
    ) -> Result<TermResult, ElaborationError> {
        let node = CoreNodeId::from_origin(hir.id(), &[])?;
        self.insert_term(CoreTerm::new(node, hir.id(), hir.owner(), ty.clone(), kind))?;
        let derivation = self.add_derivation(
            rule,
            ObligationKey::new(JudgmentKind::Formula, hir.id()),
            DerivationConclusion::Term {
                node,
                ty: ty.clone(),
            },
            witness,
            premises,
        )?;
        Ok(TermResult {
            node,
            ty,
            derivation,
        })
    }

    fn definition_use(
        &mut self,
        origin: NodeId,
        judgment: JudgmentKind,
        rule: ElaborationRule,
        expected: DefId,
    ) -> Result<DerivationNodeId, ElaborationError> {
        let node = self.index.node(origin)?;
        let target = match node.kind() {
            HirNodeKind::Reference(reference) => reference.terminal_definition(),
            _ => None,
        };
        if target != Some(expected) {
            return Err(ElaborationError::InvalidHir {
                origin,
                reason: "reference root does not select the expected definition",
            });
        }
        self.add_derivation(
            rule,
            ObligationKey::new(judgment, origin),
            DerivationConclusion::Definition(expected),
            DerivationWitness::Definition(expected),
            vec![],
        )
    }
}

/// Elaborate a complete resolved HIR into typed core and a proof-relevant
/// derivation artifact. No kernel acceptance is implied.
pub fn elaborate(hir: &ResolvedProgram) -> Result<ElaborationArtifact, ElaborationError> {
    let mut builder = Builder::new(hir)?;

    // Type environments are established before any value judgment.
    let declarations = builder.index.declarations.clone();
    for (id, meta) in &declarations {
        if meta.namespace == Namespace::State || meta.namespace == Namespace::Capability {
            let root = builder.index.root_exact(
                *id,
                &SemanticPathSegment::DeclaredType,
                "declared type",
            )?;
            let (ty, derivation) = builder.type_from_node(root.node())?;
            builder.require(
                ObligationKey::new(JudgmentKind::TypeFormation, root.node()),
                derivation,
            )?;
            if meta.namespace == Namespace::State {
                builder.state_types.insert(*id, ty);
            }
        }
    }
    let locals = builder.index.locals.clone();
    for (id, binder) in locals {
        let (ty, derivation) = builder.type_from_node(binder.declared_type())?;
        builder.require(
            ObligationKey::new(JudgmentKind::TypeFormation, binder.declared_type()),
            derivation,
        )?;
        builder.local_types.insert(id, ty);
    }

    let mut core_modules = Vec::new();
    for module in hir.modules().values() {
        let module_declarations = declarations
            .iter()
            .filter(|(_, meta)| meta.module == module.id())
            .map(|(id, meta)| (*id, meta.clone()))
            .collect::<Vec<_>>();
        let mut enumerations = BTreeMap::new();
        for (id, meta) in &module_declarations {
            if meta.namespace == Namespace::Type {
                let constructors = module_declarations
                    .iter()
                    .filter(|(_, child)| {
                        child.namespace == Namespace::Constructor && child.parent == Some(*id)
                    })
                    .map(|(child, _)| *child)
                    .collect();
                enumerations.insert(*id, CoreEnum::new(*id, constructors));
            }
        }
        let mut systems = BTreeMap::new();
        for (system_id, meta) in &module_declarations {
            if meta.namespace != Namespace::System {
                continue;
            }
            let members = module_declarations
                .iter()
                .filter(|(_, member)| member.parent == Some(*system_id))
                .map(|(id, member)| (*id, member.clone()))
                .collect::<Vec<_>>();
            let mut states = BTreeMap::new();
            let all_state = members
                .iter()
                .filter(|(_, member)| member.namespace == Namespace::State)
                .map(|(id, _)| *id)
                .collect::<BTreeSet<_>>();
            for (state_id, state_meta) in &members {
                if state_meta.namespace != Namespace::State {
                    continue;
                }
                let ty = builder.state_types[state_id].clone();
                let root = builder.index.root_exact(
                    *state_id,
                    &SemanticPathSegment::Initializer,
                    "initializer",
                )?;
                let term = builder.elab_value(root.node(), Some(ty.clone()))?;
                builder.require(
                    ObligationKey::new(JudgmentKind::Check, root.node()),
                    term.derivation,
                )?;
                states.insert(*state_id, CoreStateField::new(*state_id, ty, term.node));
            }
            let mut capabilities = BTreeMap::new();
            for (capability_id, capability_meta) in &members {
                if capability_meta.namespace != Namespace::Capability {
                    continue;
                }
                let root = builder.index.root_exact(
                    *capability_id,
                    &SemanticPathSegment::DeclaredType,
                    "capability type",
                )?;
                let (CoreType::Once { protocol }, _) = builder.type_from_node(root.node())? else {
                    return Err(ElaborationError::InvalidHir {
                        origin: root.node(),
                        reason: "capability type must be Once<Protocol>",
                    });
                };
                capabilities.insert(
                    *capability_id,
                    CoreCapability::new(*capability_id, protocol),
                );
            }
            let mut actions = BTreeMap::new();
            for (action_id, action_meta) in &members {
                if action_meta.namespace != Namespace::Action {
                    continue;
                }
                let parameters = builder
                    .index
                    .locals
                    .values()
                    .filter(|binder| binder.owner() == *action_id)
                    .map(|binder| {
                        (
                            binder.id(),
                            CoreActionParameter::new(
                                binder.id(),
                                builder.local_types[&binder.id()].clone(),
                            ),
                        )
                    })
                    .collect();
                let action_roots = builder.index.roots(*action_id).cloned().collect::<Vec<_>>();
                let mut guards = Vec::new();
                let mut updates = BTreeMap::new();
                let mut outputs = Vec::new();
                let mut consumes = BTreeSet::new();
                for root in action_roots {
                    match root.semantic_path().segments() {
                        [SemanticPathSegment::Guard(_)] => {
                            let term = builder.elab_value(root.node(), Some(CoreType::Bool))?;
                            builder.require(
                                ObligationKey::new(JudgmentKind::Check, root.node()),
                                term.derivation,
                            )?;
                            guards.push(term.node);
                        }
                        [SemanticPathSegment::UpdateTarget(state)] => {
                            let derivation = builder.definition_use(
                                root.node(),
                                JudgmentKind::UpdateTarget,
                                ElaborationRule::UpdateTarget,
                                *state,
                            )?;
                            builder.require(
                                ObligationKey::new(JudgmentKind::UpdateTarget, root.node()),
                                derivation,
                            )?;
                        }
                        [SemanticPathSegment::UpdateValue(state)] => {
                            let ty = builder
                                .state_types
                                .get(state)
                                .cloned()
                                .ok_or(ElaborationError::MissingDefinition(*state))?;
                            let term = builder.elab_value(root.node(), Some(ty))?;
                            builder.require(
                                ObligationKey::new(JudgmentKind::Check, root.node()),
                                term.derivation,
                            )?;
                            updates.insert(*state, term.node);
                        }
                        [SemanticPathSegment::Output(_)] => {
                            let term = builder.elab_value(root.node(), None)?;
                            if !term.ty.is_scalar() {
                                return Err(ElaborationError::InvalidHir {
                                    origin: root.node(),
                                    reason: "action output is not scalar",
                                });
                            }
                            builder.require(
                                ObligationKey::new(JudgmentKind::Synthesize, root.node()),
                                term.derivation,
                            )?;
                            outputs.push(term.node);
                        }
                        [SemanticPathSegment::Consume(_)] => {
                            let node = builder.index.node(root.node())?;
                            let HirNodeKind::Reference(ResolvedRef::Capability {
                                system,
                                capability,
                            }) = node.kind()
                            else {
                                return Err(ElaborationError::InvalidHir {
                                    origin: root.node(),
                                    reason: "consume root is not a capability reference",
                                });
                            };
                            if *system != *system_id {
                                return Err(ElaborationError::InvalidHir {
                                    origin: root.node(),
                                    reason: "consumed capability belongs to another system",
                                });
                            }
                            let derivation = builder.definition_use(
                                root.node(),
                                JudgmentKind::CapabilityUse,
                                ElaborationRule::CapabilityUse,
                                *capability,
                            )?;
                            builder.require(
                                ObligationKey::new(JudgmentKind::CapabilityUse, root.node()),
                                derivation,
                            )?;
                            if !consumes.insert(*capability) {
                                return Err(ElaborationError::InvalidHir {
                                    origin: root.node(),
                                    reason: "affine capability consumed more than once",
                                });
                            }
                        }
                        [SemanticPathSegment::ActionParameter(_)]
                        | [
                            SemanticPathSegment::ActionParameter(_),
                            SemanticPathSegment::DeclaredType,
                        ] => {}
                        _ => {}
                    }
                }
                let frames = all_state
                    .difference(&updates.keys().copied().collect())
                    .copied()
                    .collect();
                actions.insert(
                    *action_id,
                    CoreAction::new(
                        *action_id, *system_id, parameters, guards, updates, frames, outputs,
                        consumes,
                    ),
                );
            }
            let mut properties = BTreeMap::new();
            for (property_id, property_meta) in &members {
                if property_meta.namespace != Namespace::Property {
                    continue;
                }
                let root = builder.index.root_exact(
                    *property_id,
                    &SemanticPathSegment::PropertyBody,
                    "property body",
                )?;
                let term = builder.elab_formula(root.node(), *system_id)?;
                if term.ty != (CoreType::TemporalProp { system: *system_id }) {
                    return Err(ElaborationError::TypeMismatch {
                        origin: root.node(),
                        expected: CoreType::TemporalProp { system: *system_id },
                        actual: term.ty,
                    });
                }
                builder.require(
                    ObligationKey::new(JudgmentKind::Formula, root.node()),
                    term.derivation,
                )?;
                let kind = match property_meta.flavor {
                    DeclarationFlavor::SafetyProperty => CorePropertyKind::Safety,
                    DeclarationFlavor::TemporalProperty => CorePropertyKind::Temporal,
                    DeclarationFlavor::Ordinary => {
                        return Err(ElaborationError::InvalidHir {
                            origin: root.node(),
                            reason: "property declaration lacks safety/temporal flavor",
                        });
                    }
                };
                properties.insert(
                    *property_id,
                    CoreProperty::new(*property_id, *system_id, kind, term.node),
                );
            }
            let observation_roots = builder
                .index
                .roots(*system_id)
                .filter(|root| {
                    matches!(
                        root.semantic_path().segments(),
                        [SemanticPathSegment::ObservationItem(_)]
                    )
                })
                .cloned()
                .collect::<Vec<_>>();
            let mut observations = Vec::new();
            if let Some(first) = observation_roots.first() {
                let mut items = Vec::new();
                for root in &observation_roots {
                    let term = builder.elab_value(root.node(), None)?;
                    if !term.ty.is_scalar() {
                        return Err(ElaborationError::InvalidHir {
                            origin: root.node(),
                            reason: "observation item is not scalar",
                        });
                    }
                    builder.require(
                        ObligationKey::new(JudgmentKind::Synthesize, root.node()),
                        term.derivation,
                    )?;
                    items.push(term.node);
                }
                observations.push(CoreObservation::new(*system_id, first.node(), items));
            }
            systems.insert(
                *system_id,
                CoreSystem::new(
                    *system_id,
                    states,
                    capabilities,
                    actions,
                    properties,
                    observations,
                ),
            );
        }
        let imports = module
            .imports()
            .iter()
            .map(|import| import.module_id())
            .collect();
        core_modules.push(CoreModule::new(module.id(), imports, enumerations, systems));
    }

    builder.validate_artifact_shape()?;
    let core_program = CoreProgram::new(
        hir.resolution_id(),
        core_modules,
        builder.terms.into_values(),
    )?;
    let ruleset_bundle_id = ruleset_bundle_id();
    let resource_policy_id = resource_policy_id();
    let (certificate_id, certificate_bytes) = certificate_id(
        hir.source_set_id(),
        hir.module_map_id(),
        hir.surface_program_id(),
        hir.resolution_id(),
        core_program.id(),
        ruleset_bundle_id,
        resource_policy_id,
        &builder.required_roots,
        &builder.derivations,
    );
    if certificate_bytes > MAX_CERTIFICATE_BYTES {
        return Err(ElaborationError::ResourceLimit {
            resource: "canonical certificate bytes",
            actual: certificate_bytes,
            maximum: MAX_CERTIFICATE_BYTES,
        });
    }
    Ok(ElaborationArtifact::new(
        hir.source_set_id(),
        hir.module_map_id(),
        hir.surface_program_id(),
        hir.resolution_id(),
        core_program,
        ruleset_bundle_id,
        resource_policy_id,
        builder.required_roots,
        builder.derivations,
        certificate_id,
    ))
}

const fn map_binary(operator: HirBinaryOp) -> CoreBinaryOp {
    match operator {
        HirBinaryOp::Or => CoreBinaryOp::Or,
        HirBinaryOp::And => CoreBinaryOp::And,
        HirBinaryOp::Implies => CoreBinaryOp::Implies,
        HirBinaryOp::Equal => CoreBinaryOp::Equal,
        HirBinaryOp::NotEqual => CoreBinaryOp::NotEqual,
        HirBinaryOp::Less => CoreBinaryOp::Less,
        HirBinaryOp::LessEqual => CoreBinaryOp::LessEqual,
        HirBinaryOp::Greater => CoreBinaryOp::Greater,
        HirBinaryOp::GreaterEqual => CoreBinaryOp::GreaterEqual,
        HirBinaryOp::Add => CoreBinaryOp::Add,
        HirBinaryOp::Subtract => CoreBinaryOp::Subtract,
        HirBinaryOp::Multiply => CoreBinaryOp::Multiply,
    }
}

const fn rule_binary(operator: HirBinaryOp) -> ElaborationRule {
    match operator {
        HirBinaryOp::Or => ElaborationRule::Or,
        HirBinaryOp::And => ElaborationRule::And,
        HirBinaryOp::Implies => ElaborationRule::Implies,
        HirBinaryOp::Equal => ElaborationRule::Equal,
        HirBinaryOp::NotEqual => ElaborationRule::NotEqual,
        HirBinaryOp::Less => ElaborationRule::Less,
        HirBinaryOp::LessEqual => ElaborationRule::LessEqual,
        HirBinaryOp::Greater => ElaborationRule::Greater,
        HirBinaryOp::GreaterEqual => ElaborationRule::GreaterEqual,
        HirBinaryOp::Add => ElaborationRule::Add,
        HirBinaryOp::Subtract => ElaborationRule::Subtract,
        HirBinaryOp::Multiply => ElaborationRule::Multiply,
    }
}

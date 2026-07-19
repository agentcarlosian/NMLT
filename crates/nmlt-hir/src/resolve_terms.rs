//! Source-derived local binding, term lowering, and reference-map construction.

use std::collections::{BTreeMap, BTreeSet};

use crate::hir::{
    HirBinaryOp, HirNode, HirNodeKind, HirRoot, HirUnaryOp, LocalBinder, ResolutionEntry,
    ResolutionMap, ResolvedRef, SemanticPath, SemanticPathSegment, encode_hir_node,
    encode_resolved_ref, push_bytes, push_count, push_text,
};
use crate::model::{
    DeclarationKey, DefPath, Namespace, ResolvedModule, ResolvedProgram, SourceSpan,
};
use crate::resolver::{ResolveError, ResourceDimension};
use crate::term::{
    LocalBinderInput, NameUse, ParsedBinaryOp, ParsedBuiltin, ParsedExpr, ParsedExprKind,
    ParsedType, ParsedTypeKind, ParsedUnaryOp, RawTermInput, RawTermInputKind, TermRootInput,
    parse_expression, parse_expression_list, parse_type,
};
use crate::{DefId, LocalId, NodeId};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PendingModuleTerms {
    pub local_binders: Vec<LocalBinderInput>,
    pub raw_terms: Vec<RawTermInput>,
}

#[derive(Default)]
struct ModuleTermOutput {
    local_binders: BTreeMap<LocalId, LocalBinder>,
    hir_roots: Vec<HirRoot>,
    hir_nodes: BTreeMap<NodeId, HirNode>,
}

#[derive(Clone, Copy)]
enum ReferenceClass {
    Type,
    Value,
    State,
    Capability,
    Action,
}

pub(crate) fn resolve_program_terms(
    pending: &BTreeMap<String, PendingModuleTerms>,
    modules: &mut BTreeMap<String, ResolvedModule>,
) -> Result<ResolutionMap, ResolveError> {
    let mut outputs = modules
        .keys()
        .map(|name| (name.clone(), ModuleTermOutput::default()))
        .collect::<BTreeMap<_, _>>();
    let mut resolution_map = ResolutionMap::default();
    let mut total_integer_payload = 0_u64;
    let empty_binders = BTreeMap::new();

    // Binder types cannot refer to action locals. Lower them first, then make
    // the complete binder table available to action expressions.
    for (logical_module, terms) in pending {
        let mut binders = terms.local_binders.iter().collect::<Vec<_>>();
        binders.sort_by(|left, right| {
            left.owner
                .cmp(&right.owner)
                .then_with(|| left.index.cmp(&right.index))
                .then_with(|| left.name_span.cmp(&right.name_span))
        });
        let mut seen = BTreeMap::<(DefPath, String), Vec<SourceSpan>>::new();
        for binder in binders {
            if !is_identifier(&binder.name) || is_reserved(&binder.name) {
                return Err(ResolveError::InvalidLocalBinder {
                    logical_module: logical_module.clone(),
                    owner: binder.owner.clone(),
                    name: binder.name.clone(),
                    span: binder.name_span,
                });
            }
            let spans = seen
                .entry((binder.owner.clone(), binder.name.clone()))
                .or_default();
            spans.push(binder.name_span);
            if spans.len() > 1 {
                return Err(ResolveError::DuplicateLocalBinder {
                    logical_module: logical_module.clone(),
                    owner: binder.owner.clone(),
                    name: binder.name.clone(),
                    spans: spans.clone(),
                });
            }

            let owner = owner_id(modules, logical_module, &binder.owner, binder.name_span)?;
            let path = SemanticPath::new([
                SemanticPathSegment::ActionParameter(binder.index),
                SemanticPathSegment::DeclaredType,
            ]);
            let output = outputs
                .get_mut(logical_module)
                .expect("every module has term output");
            let mut lowerer = Lowerer {
                logical_module,
                owner_path: &binder.owner,
                owner,
                modules,
                all_binders: &empty_binders,
                output,
                resolution_map: &mut resolution_map,
                integer_payload: &mut total_integer_payload,
            };
            let parsed = parse_type(&binder.declared_type)
                .map_err(|error| lowerer.syntax_error(error.span, error.message))?;
            let declared_type = lowerer.lower_type(&parsed, &path)?;
            lowerer.add_root(path, declared_type);
            let local = LocalBinder::new(
                owner,
                binder.index,
                binder.name.clone(),
                declared_type,
                binder.name_span,
            );
            lowerer.output.local_binders.insert(local.id(), local);
        }
    }

    let all_binders = outputs
        .iter()
        .map(|(module, output)| (module.clone(), output.local_binders.clone()))
        .collect::<BTreeMap<_, _>>();

    for (logical_module, terms) in pending {
        let mut raw_terms = terms.raw_terms.iter().collect::<Vec<_>>();
        raw_terms.sort_by(|left, right| {
            left.owner
                .cmp(&right.owner)
                .then_with(|| left.root.cmp(&right.root))
                .then_with(|| left.span.cmp(&right.span))
        });
        let mut update_targets = BTreeMap::<(DefId, u32), DefId>::new();
        let mut observation_indices = BTreeMap::<DefId, u32>::new();
        for input in raw_terms {
            let owner = owner_id(modules, logical_module, &input.owner, input.span)?;
            let output = outputs
                .get_mut(logical_module)
                .expect("every module has term output");
            let mut lowerer = Lowerer {
                logical_module,
                owner_path: &input.owner,
                owner,
                modules,
                all_binders: &all_binders,
                output,
                resolution_map: &mut resolution_map,
                integer_payload: &mut total_integer_payload,
            };
            match (&input.root, input.kind) {
                (TermRootInput::DeclaredType, RawTermInputKind::Type) => {
                    let path = SemanticPath::new([SemanticPathSegment::DeclaredType]);
                    let parsed = parse_type(input)
                        .map_err(|error| lowerer.syntax_error(error.span, error.message))?;
                    let root = lowerer.lower_type(&parsed, &path)?;
                    lowerer.add_root(path, root);
                }
                (TermRootInput::Initializer, RawTermInputKind::Expression) => {
                    lower_expression_root(
                        &mut lowerer,
                        input,
                        SemanticPath::new([SemanticPathSegment::Initializer]),
                    )?;
                }
                (TermRootInput::Guard(index), RawTermInputKind::Expression) => {
                    lower_expression_root(
                        &mut lowerer,
                        input,
                        SemanticPath::new([SemanticPathSegment::Guard(*index)]),
                    )?;
                }
                (TermRootInput::UpdateTarget(index), RawTermInputKind::UpdateTarget) => {
                    let parsed = parse_expression(input)
                        .map_err(|error| lowerer.syntax_error(error.span, error.message))?;
                    let ParsedExprKind::Name(name) = &parsed.kind else {
                        return Err(lowerer.invalid_form("update target", parsed.span));
                    };
                    let selected = lowerer.resolve_reference(name, ReferenceClass::State)?;
                    let target = selected
                        .terminal_definition()
                        .expect("state references have a terminal definition");
                    let path = SemanticPath::new([SemanticPathSegment::UpdateTarget(target)]);
                    let root =
                        lowerer.insert_reference_node(&parsed, name, path.clone(), selected)?;
                    lowerer.add_root(path, root);
                    if update_targets.insert((owner, *index), target).is_some() {
                        return Err(ResolveError::DuplicateHirOrigin {
                            logical_module: logical_module.clone(),
                            origin: root,
                        });
                    }
                }
                (TermRootInput::UpdateValue(index), RawTermInputKind::Expression) => {
                    let Some(target) = update_targets.get(&(owner, *index)).copied() else {
                        return Err(lowerer.invalid_form("update value without target", input.span));
                    };
                    lower_expression_root(
                        &mut lowerer,
                        input,
                        SemanticPath::new([SemanticPathSegment::UpdateValue(target)]),
                    )?;
                }
                (TermRootInput::Output(index), RawTermInputKind::Expression) => {
                    lower_expression_root(
                        &mut lowerer,
                        input,
                        SemanticPath::new([SemanticPathSegment::Output(*index)]),
                    )?;
                }
                (TermRootInput::Consume(index), RawTermInputKind::Consume) => {
                    let parsed = parse_expression(input)
                        .map_err(|error| lowerer.syntax_error(error.span, error.message))?;
                    let ParsedExprKind::Name(name) = &parsed.kind else {
                        return Err(lowerer.invalid_form("capability consumption", parsed.span));
                    };
                    let path = SemanticPath::new([SemanticPathSegment::Consume(*index)]);
                    let selected = lowerer.resolve_reference(name, ReferenceClass::Capability)?;
                    let root =
                        lowerer.insert_reference_node(&parsed, name, path.clone(), selected)?;
                    lowerer.add_root(path, root);
                }
                (TermRootInput::PropertyBody, RawTermInputKind::Expression) => {
                    lower_expression_root(
                        &mut lowerer,
                        input,
                        SemanticPath::new([SemanticPathSegment::PropertyBody]),
                    )?;
                }
                (TermRootInput::ObservationItems, RawTermInputKind::ExpressionList) => {
                    let parsed = parse_expression_list(input)
                        .map_err(|error| lowerer.syntax_error(error.span, error.message))?;
                    let next = observation_indices.entry(owner).or_default();
                    for expression in parsed {
                        let path = SemanticPath::new([SemanticPathSegment::ObservationItem(*next)]);
                        *next = next.checked_add(1).ok_or(ResolveError::ResourceLimit {
                            dimension: ResourceDimension::ContextEntries,
                            maximum: ResourceDimension::ContextEntries.maximum(),
                            actual: u64::MAX,
                        })?;
                        let root = lowerer.lower_expression(&expression, &path)?;
                        lowerer.add_root(path, root);
                    }
                }
                (TermRootInput::ActionParameterType(_), _) => {
                    return Err(lowerer.invalid_form(
                        "action parameter type duplicated in raw-term stream",
                        input.span,
                    ));
                }
                _ => {
                    return Err(lowerer.invalid_form("mismatched raw-term context", input.span));
                }
            }
        }
    }

    enforce_post_resolution_limits(&outputs, &resolution_map)?;
    for (logical_module, output) in outputs {
        let module = modules
            .get_mut(&logical_module)
            .expect("resolved module remains present");
        module.local_binders = output.local_binders;
        module.hir_roots = output.hir_roots;
        module.hir_nodes = output.hir_nodes;
    }
    Ok(resolution_map)
}

fn lower_expression_root(
    lowerer: &mut Lowerer<'_>,
    input: &RawTermInput,
    path: SemanticPath,
) -> Result<(), ResolveError> {
    let parsed =
        parse_expression(input).map_err(|error| lowerer.syntax_error(error.span, error.message))?;
    let root = lowerer.lower_expression(&parsed, &path)?;
    lowerer.add_root(path, root);
    Ok(())
}

struct Lowerer<'a> {
    logical_module: &'a str,
    owner_path: &'a DefPath,
    owner: DefId,
    modules: &'a BTreeMap<String, ResolvedModule>,
    all_binders: &'a BTreeMap<String, BTreeMap<LocalId, LocalBinder>>,
    output: &'a mut ModuleTermOutput,
    resolution_map: &'a mut ResolutionMap,
    integer_payload: &'a mut u64,
}

impl Lowerer<'_> {
    fn lower_type(
        &mut self,
        parsed: &ParsedType,
        path: &SemanticPath,
    ) -> Result<NodeId, ResolveError> {
        let kind = match &parsed.kind {
            ParsedTypeKind::Bool => HirNodeKind::TypeBool,
            ParsedTypeKind::Nat => HirNodeKind::TypeNat,
            ParsedTypeKind::Int => HirNodeKind::TypeInt,
            ParsedTypeKind::Named(name) => {
                let target = self.resolve_reference(name, ReferenceClass::Type)?;
                self.insert_resolution_entry(path, name, Namespace::Type, target.clone())?;
                HirNodeKind::TypeNamed(target)
            }
            ParsedTypeKind::Once {
                protocol,
                protocol_span,
            } => {
                if !is_identifier(protocol) || is_reserved(protocol) {
                    return Err(self.invalid_form("`Once` protocol tag", *protocol_span));
                }
                let protocol_path = path.child(SemanticPathSegment::CapabilityProtocol);
                let protocol_id = protocol_path.node_id(self.owner);
                self.insert_node(HirNode {
                    id: protocol_id,
                    owner: self.owner,
                    semantic_path: protocol_path,
                    span: *protocol_span,
                    kind: HirNodeKind::ProtocolTag {
                        spelling: protocol.clone(),
                    },
                })?;
                HirNodeKind::TypeOnce {
                    protocol: protocol_id,
                }
            }
        };
        let id = path.node_id(self.owner);
        self.insert_node(HirNode {
            id,
            owner: self.owner,
            semantic_path: path.clone(),
            span: parsed.span,
            kind,
        })?;
        Ok(id)
    }

    fn lower_expression(
        &mut self,
        parsed: &ParsedExpr,
        path: &SemanticPath,
    ) -> Result<NodeId, ResolveError> {
        let kind = match &parsed.kind {
            ParsedExprKind::Bool(value) => HirNodeKind::BoolLiteral(*value),
            ParsedExprKind::Natural(magnitude) => {
                *self.integer_payload = self
                    .integer_payload
                    .checked_add(magnitude.len() as u64)
                    .ok_or(ResolveError::ResourceLimit {
                        dimension: ResourceDimension::TotalIntegerPayload,
                        maximum: ResourceDimension::TotalIntegerPayload.maximum(),
                        actual: u64::MAX,
                    })?;
                if *self.integer_payload > ResourceDimension::TotalIntegerPayload.maximum() {
                    return Err(ResolveError::ResourceLimit {
                        dimension: ResourceDimension::TotalIntegerPayload,
                        maximum: ResourceDimension::TotalIntegerPayload.maximum(),
                        actual: *self.integer_payload,
                    });
                }
                HirNodeKind::NaturalLiteral {
                    magnitude: magnitude.clone(),
                }
            }
            ParsedExprKind::Name(name) => {
                let target = self.resolve_reference(name, ReferenceClass::Value)?;
                return self.insert_reference_node(parsed, name, path.clone(), target);
            }
            ParsedExprKind::Unary { operator, operand } => {
                let operand_path = path.child(SemanticPathSegment::Operand(0));
                let operand = self.lower_expression(operand, &operand_path)?;
                HirNodeKind::Unary {
                    operator: match operator {
                        ParsedUnaryOp::Not => HirUnaryOp::Not,
                        ParsedUnaryOp::Negate => HirUnaryOp::Negate,
                    },
                    operand,
                }
            }
            ParsedExprKind::Binary {
                operator,
                left,
                right,
            } => {
                let left =
                    self.lower_expression(left, &path.child(SemanticPathSegment::Operand(0)))?;
                let right =
                    self.lower_expression(right, &path.child(SemanticPathSegment::Operand(1)))?;
                HirNodeKind::Binary {
                    operator: map_binary(*operator),
                    left,
                    right,
                }
            }
            ParsedExprKind::Builtin { builtin, arguments } => match builtin {
                ParsedBuiltin::Enabled => {
                    let ParsedExprKind::Name(name) = &arguments[0].kind else {
                        return Err(
                            self.invalid_form("`enabled` action argument", arguments[0].span)
                        );
                    };
                    let argument_path = path.child(SemanticPathSegment::CallArgument(0));
                    let action = self.resolve_reference(name, ReferenceClass::Action)?;
                    let action_origin = self.insert_reference_node(
                        &arguments[0],
                        name,
                        argument_path,
                        action.clone(),
                    )?;
                    HirNodeKind::Enabled {
                        action_origin,
                        action,
                    }
                }
                ParsedBuiltin::Until => {
                    let left = self.lower_expression(
                        &arguments[0],
                        &path.child(SemanticPathSegment::CallArgument(0)),
                    )?;
                    let right = self.lower_expression(
                        &arguments[1],
                        &path.child(SemanticPathSegment::CallArgument(1)),
                    )?;
                    HirNodeKind::Until { left, right }
                }
                ParsedBuiltin::ToInt
                | ParsedBuiltin::Always
                | ParsedBuiltin::Eventually
                | ParsedBuiltin::Next => {
                    let argument = self.lower_expression(
                        &arguments[0],
                        &path.child(SemanticPathSegment::CallArgument(0)),
                    )?;
                    match builtin {
                        ParsedBuiltin::ToInt => HirNodeKind::IntFromNat { operand: argument },
                        ParsedBuiltin::Always => HirNodeKind::Always { property: argument },
                        ParsedBuiltin::Eventually => HirNodeKind::Eventually { property: argument },
                        ParsedBuiltin::Next => HirNodeKind::Next { property: argument },
                        ParsedBuiltin::Until | ParsedBuiltin::Enabled => unreachable!(),
                    }
                }
            },
        };
        let id = path.node_id(self.owner);
        self.insert_node(HirNode {
            id,
            owner: self.owner,
            semantic_path: path.clone(),
            span: parsed.span,
            kind,
        })?;
        Ok(id)
    }

    fn insert_reference_node(
        &mut self,
        parsed: &ParsedExpr,
        name: &NameUse,
        path: SemanticPath,
        target: ResolvedRef,
    ) -> Result<NodeId, ResolveError> {
        let namespace = namespace_of_target(&target, self.modules).unwrap_or(Namespace::Value);
        self.insert_resolution_entry(&path, name, namespace, target.clone())?;
        let id = path.node_id(self.owner);
        self.insert_node(HirNode {
            id,
            owner: self.owner,
            semantic_path: path,
            span: parsed.span,
            kind: HirNodeKind::Reference(target),
        })?;
        Ok(id)
    }

    fn insert_resolution_entry(
        &mut self,
        path: &SemanticPath,
        name: &NameUse,
        namespace: Namespace,
        target: ResolvedRef,
    ) -> Result<(), ResolveError> {
        let origin = path.node_id(self.owner);
        let entry = ResolutionEntry {
            origin,
            module: self.modules[self.logical_module].id,
            owner: self.owner,
            semantic_path: path.clone(),
            namespace,
            qualifier: name.qualifier.clone(),
            spelling: name.spelling.clone(),
            span: name.span,
            target,
        };
        if self.resolution_map.entries.insert(origin, entry).is_some() {
            return Err(ResolveError::DuplicateHirOrigin {
                logical_module: self.logical_module.to_owned(),
                origin,
            });
        }
        Ok(())
    }

    fn insert_node(&mut self, node: HirNode) -> Result<(), ResolveError> {
        let origin = node.id;
        if self.output.hir_nodes.insert(origin, node).is_some() {
            return Err(ResolveError::DuplicateHirOrigin {
                logical_module: self.logical_module.to_owned(),
                origin,
            });
        }
        Ok(())
    }

    fn add_root(&mut self, semantic_path: SemanticPath, node: NodeId) {
        self.output.hir_roots.push(HirRoot {
            owner: self.owner,
            semantic_path,
            node,
        });
    }

    fn resolve_reference(
        &self,
        name: &NameUse,
        class: ReferenceClass,
    ) -> Result<ResolvedRef, ResolveError> {
        let locals = self.locals_for_owner();
        let local = if matches!(class, ReferenceClass::Value) && name.qualifier.is_none() {
            locals.iter().find(|binder| binder.name == name.spelling)
        } else {
            None
        };
        let candidates = definition_candidates(
            self.modules,
            self.logical_module,
            self.owner_path,
            name,
            class,
        );
        if local.is_some() && !candidates.is_empty() {
            return Err(ResolveError::LocalShadowing {
                logical_module: self.logical_module.to_owned(),
                owner: self.owner_path.clone(),
                spelling: display_name(name),
                candidates: candidates
                    .iter()
                    .map(|(_, definition)| *definition)
                    .collect(),
                span: name.span,
            });
        }
        if let Some(local) = local {
            return Ok(ResolvedRef::Local(local.id));
        }
        match candidates.as_slice() {
            [] => Err(ResolveError::UnresolvedReference {
                logical_module: self.logical_module.to_owned(),
                owner: self.owner_path.clone(),
                spelling: display_name(name),
                span: name.span,
            }),
            [(_, definition)] => Ok(resolved_ref_for_definition(self.modules, *definition)),
            candidates => Err(ResolveError::AmbiguousReference {
                logical_module: self.logical_module.to_owned(),
                owner: self.owner_path.clone(),
                spelling: display_name(name),
                candidates: candidates
                    .iter()
                    .map(|(_, definition)| *definition)
                    .collect(),
                span: name.span,
            }),
        }
    }

    fn locals_for_owner(&self) -> Vec<&LocalBinder> {
        self.all_binders
            .get(self.logical_module)
            .into_iter()
            .flat_map(|binders| binders.values())
            .filter(|binder| binder.owner == self.owner)
            .collect()
    }

    fn syntax_error(&self, span: SourceSpan, message: String) -> ResolveError {
        ResolveError::TermSyntax {
            logical_module: self.logical_module.to_owned(),
            owner: self.owner_path.clone(),
            span,
            message,
        }
    }

    fn invalid_form(&self, context: &str, span: SourceSpan) -> ResolveError {
        ResolveError::InvalidReferenceForm {
            logical_module: self.logical_module.to_owned(),
            owner: self.owner_path.clone(),
            context: context.to_owned(),
            span,
        }
    }
}

fn owner_id(
    modules: &BTreeMap<String, ResolvedModule>,
    logical_module: &str,
    owner: &DefPath,
    span: SourceSpan,
) -> Result<DefId, ResolveError> {
    modules[logical_module]
        .declarations
        .get(&DeclarationKey::new(owner.clone()))
        .map(|declaration| declaration.id)
        .ok_or_else(|| ResolveError::MissingTermOwner {
            logical_module: logical_module.to_owned(),
            owner: owner.clone(),
            span,
        })
}

fn definition_candidates(
    modules: &BTreeMap<String, ResolvedModule>,
    logical_module: &str,
    owner: &DefPath,
    name: &NameUse,
    class: ReferenceClass,
) -> Vec<(Namespace, DefId)> {
    let origin = &modules[logical_module];
    let mut visible = if let Some(qualifier) = &name.qualifier {
        if qualifier != logical_module
            && !origin
                .imports
                .iter()
                .any(|import| import.logical_module == *qualifier)
        {
            return Vec::new();
        }
        vec![qualifier.as_str()]
    } else {
        let mut names = vec![logical_module];
        names.extend(
            origin
                .imports
                .iter()
                .map(|import| import.logical_module.as_str()),
        );
        names
    };
    visible.sort();
    visible.dedup();
    let current_system = owner.segments.first().and_then(|segment| {
        (segment.namespace == Namespace::System).then_some(segment.name.as_str())
    });
    let mut candidates = Vec::new();
    for module_name in visible {
        for declaration in modules[module_name].declarations.values() {
            let Some(last) = declaration.key.path.segments.last() else {
                continue;
            };
            if last.name != name.spelling || !class_allows(class, last.namespace) {
                continue;
            }
            if matches!(
                last.namespace,
                Namespace::State
                    | Namespace::SystemInput
                    | Namespace::Capability
                    | Namespace::Action
            ) {
                let same_system = module_name == logical_module
                    && declaration
                        .key
                        .path
                        .segments
                        .first()
                        .is_some_and(|segment| {
                            segment.namespace == Namespace::System
                                && Some(segment.name.as_str()) == current_system
                        });
                if !same_system {
                    continue;
                }
            }
            candidates.push((last.namespace, declaration.id));
        }
    }
    candidates.sort_by_key(|(_, definition)| *definition);
    candidates.dedup();
    candidates
}

const fn class_allows(class: ReferenceClass, namespace: Namespace) -> bool {
    match class {
        ReferenceClass::Type => matches!(namespace, Namespace::Type),
        ReferenceClass::Value => matches!(
            namespace,
            Namespace::Value | Namespace::State | Namespace::SystemInput | Namespace::Constructor
        ),
        ReferenceClass::State => matches!(namespace, Namespace::State),
        ReferenceClass::Capability => matches!(namespace, Namespace::Capability),
        ReferenceClass::Action => matches!(namespace, Namespace::Action),
    }
}

fn resolved_ref_for_definition(
    modules: &BTreeMap<String, ResolvedModule>,
    definition: DefId,
) -> ResolvedRef {
    for module in modules.values() {
        let Some(declaration) = module
            .declarations
            .values()
            .find(|candidate| candidate.id == definition)
        else {
            continue;
        };
        let namespace = declaration
            .key
            .namespace()
            .expect("resolved declarations have nonempty paths");
        let parent = || {
            let mut segments = declaration.key.path.segments.clone();
            segments.pop();
            module.declarations[&DeclarationKey::new(DefPath::new(segments))].id
        };
        return match namespace {
            Namespace::State => ResolvedRef::StateField {
                system: parent(),
                state: definition,
            },
            Namespace::Constructor => ResolvedRef::Constructor {
                enumeration: parent(),
                constructor: definition,
            },
            Namespace::Capability => ResolvedRef::Capability {
                system: parent(),
                capability: definition,
            },
            Namespace::Type
            | Namespace::Value
            | Namespace::System
            | Namespace::Action
            | Namespace::SystemInput
            | Namespace::Property
            | Namespace::Observation => ResolvedRef::Definition(definition),
        };
    }
    unreachable!("candidate definition comes from the resolved module table")
}

fn namespace_of_target(
    target: &ResolvedRef,
    modules: &BTreeMap<String, ResolvedModule>,
) -> Option<Namespace> {
    let definition = target.terminal_definition()?;
    modules.values().find_map(|module| {
        module
            .declarations
            .values()
            .find(|candidate| candidate.id == definition)
            .and_then(|candidate| candidate.key.namespace())
    })
}

const fn map_binary(operator: ParsedBinaryOp) -> HirBinaryOp {
    match operator {
        ParsedBinaryOp::Or => HirBinaryOp::Or,
        ParsedBinaryOp::And => HirBinaryOp::And,
        ParsedBinaryOp::Implies => HirBinaryOp::Implies,
        ParsedBinaryOp::Equal => HirBinaryOp::Equal,
        ParsedBinaryOp::NotEqual => HirBinaryOp::NotEqual,
        ParsedBinaryOp::Less => HirBinaryOp::Less,
        ParsedBinaryOp::LessEqual => HirBinaryOp::LessEqual,
        ParsedBinaryOp::Greater => HirBinaryOp::Greater,
        ParsedBinaryOp::GreaterEqual => HirBinaryOp::GreaterEqual,
        ParsedBinaryOp::Add => HirBinaryOp::Add,
        ParsedBinaryOp::Subtract => HirBinaryOp::Subtract,
        ParsedBinaryOp::Multiply => HirBinaryOp::Multiply,
    }
}

fn enforce_post_resolution_limits(
    outputs: &BTreeMap<String, ModuleTermOutput>,
    resolution_map: &ResolutionMap,
) -> Result<(), ResolveError> {
    let nodes = outputs
        .values()
        .try_fold(0_u64, |total, output| {
            total.checked_add(output.hir_nodes.len() as u64)
        })
        .unwrap_or(u64::MAX);
    if nodes > ResourceDimension::HirNodes.maximum() {
        return Err(ResolveError::ResourceLimit {
            dimension: ResourceDimension::HirNodes,
            maximum: ResourceDimension::HirNodes.maximum(),
            actual: nodes,
        });
    }
    let contexts = outputs
        .values()
        .map(|output| output.local_binders.len() as u64)
        .sum::<u64>()
        .saturating_add(resolution_map.entries.len() as u64);
    if contexts > ResourceDimension::ContextEntries.maximum() {
        return Err(ResolveError::ResourceLimit {
            dimension: ResourceDimension::ContextEntries,
            maximum: ResourceDimension::ContextEntries.maximum(),
            actual: contexts,
        });
    }
    Ok(())
}

pub(crate) fn canonical_hir_bytes(
    modules: &BTreeMap<String, ResolvedModule>,
    resolution_map: &ResolutionMap,
) -> Vec<u8> {
    let mut output = Vec::new();
    push_count(&mut output, modules.len());
    for module in modules.values() {
        push_text(&mut output, &module.logical_module);
        push_text(&mut output, &module.repository_path);
        output.extend_from_slice(module.source_id.digest());
        push_count(&mut output, module.imports.len());
        for import in &module.imports {
            push_text(&mut output, &import.logical_module);
        }
        push_count(&mut output, module.declarations.len());
        for declaration in module.declarations.values() {
            output.extend_from_slice(declaration.id.digest());
            output.push(declaration.flavor.wire_tag());
            push_count(&mut output, declaration.key.path.segments.len());
            for segment in &declaration.key.path.segments {
                output.push(segment.namespace.wire_tag());
                push_text(&mut output, &segment.name);
            }
        }
        push_count(&mut output, module.local_binders.len());
        for binder in module.local_binders.values() {
            output.extend_from_slice(binder.id.digest());
            output.extend_from_slice(binder.owner.digest());
            output.extend_from_slice(binder.binder_node.digest());
            push_text(&mut output, &binder.name);
            output.extend_from_slice(binder.declared_type.digest());
        }
        let mut roots = module.hir_roots.iter().collect::<Vec<_>>();
        roots.sort();
        push_count(&mut output, roots.len());
        for root in roots {
            output.extend_from_slice(root.owner.digest());
            push_bytes(&mut output, &root.semantic_path.encode());
            output.extend_from_slice(root.node.digest());
        }
        push_count(&mut output, module.hir_nodes.len());
        for node in module.hir_nodes.values() {
            encode_hir_node(&mut output, node);
        }
    }
    push_count(&mut output, resolution_map.entries.len());
    for entry in resolution_map.entries.values() {
        output.extend_from_slice(entry.origin.digest());
        output.extend_from_slice(entry.module.digest());
        output.extend_from_slice(entry.owner.digest());
        push_bytes(&mut output, &entry.semantic_path.encode());
        output.push(entry.namespace.wire_tag());
        match &entry.qualifier {
            Some(qualifier) => {
                output.push(1);
                push_text(&mut output, qualifier);
            }
            None => output.push(0),
        }
        push_text(&mut output, &entry.spelling);
        encode_resolved_ref(&mut output, &entry.target);
    }
    output
}

/// Independently read back HIR origins, exact source spellings, graph closure,
/// local scopes, and selected definition targets.
///
/// This check does not trust serialized contexts or rerun the primary lowering
/// routine. It reconstructs candidate sets from the sealed module/declaration
/// tables and requires a bijection between reference-shaped HIR nodes and the
/// canonical resolution map.
pub fn verify_resolution_readback(program: &ResolvedProgram) -> Result<(), ResolveError> {
    let mut expected_references = BTreeSet::new();
    for (logical_module, module) in &program.modules {
        let source = std::str::from_utf8(&module.exact_bytes)
            .expect("resolved modules already passed UTF-8 validation");
        let mut roots = BTreeSet::new();
        for root in &module.hir_roots {
            if root.semantic_path.node_id(root.owner) != root.node
                || !module.hir_nodes.contains_key(&root.node)
                || !roots.insert(root.node)
            {
                return Err(readback_error(
                    logical_module,
                    "invalid, missing, or duplicate HIR root",
                    None,
                ));
            }
        }

        for node in module.hir_nodes.values() {
            if node.semantic_path.node_id(node.owner) != node.id {
                return Err(readback_error(
                    logical_module,
                    "HIR NodeId does not match owner and semantic path",
                    Some(node.span),
                ));
            }
            match &node.kind {
                HirNodeKind::Reference(_) | HirNodeKind::TypeNamed(_) => {
                    expected_references.insert(node.id);
                }
                _ => {}
            }
            for child in hir_children(&node.kind) {
                if !module.hir_nodes.contains_key(&child) {
                    return Err(readback_error(
                        logical_module,
                        "HIR node refers to a missing child",
                        Some(node.span),
                    ));
                }
            }
        }

        let reachable = reachable_nodes(module, &roots)
            .map_err(|message| readback_error(logical_module, &message, None))?;
        if reachable.len() != module.hir_nodes.len() {
            return Err(readback_error(
                logical_module,
                "HIR contains a node unreachable from every semantic root",
                None,
            ));
        }

        for binder in module.local_binders.values() {
            let Some(type_node) = module.hir_nodes.get(&binder.declared_type) else {
                return Err(readback_error(
                    logical_module,
                    "local binder type node is missing",
                    Some(binder.span),
                ));
            };
            let [
                SemanticPathSegment::ActionParameter(index),
                SemanticPathSegment::DeclaredType,
            ] = type_node.semantic_path.segments()
            else {
                return Err(readback_error(
                    logical_module,
                    "local binder type has the wrong semantic path",
                    Some(binder.span),
                ));
            };
            let rebuilt = LocalBinder::new(
                binder.owner,
                *index,
                binder.name.clone(),
                binder.declared_type,
                binder.span,
            );
            if rebuilt.id != binder.id || rebuilt.binder_node != binder.binder_node {
                return Err(readback_error(
                    logical_module,
                    "local binder identity does not match its owner path",
                    Some(binder.span),
                ));
            }
        }

        for entry in program
            .resolution_map
            .entries
            .values()
            .filter(|entry| entry.module == module.id)
        {
            let Some(node) = module.hir_nodes.get(&entry.origin) else {
                return Err(readback_error(
                    logical_module,
                    "resolution-map origin is absent from HIR",
                    Some(entry.span),
                ));
            };
            let node_target = match &node.kind {
                HirNodeKind::Reference(target) | HirNodeKind::TypeNamed(target) => target,
                _ => {
                    return Err(readback_error(
                        logical_module,
                        "resolution-map origin is not a reference-shaped HIR node",
                        Some(entry.span),
                    ));
                }
            };
            if node_target != &entry.target
                || entry.semantic_path != node.semantic_path
                || entry.owner != node.owner
                || entry.origin != entry.semantic_path.node_id(entry.owner)
            {
                return Err(readback_error(
                    logical_module,
                    "resolution entry disagrees with its HIR node",
                    Some(entry.span),
                ));
            }
            let Some(slice) = source.get(entry.span.start..entry.span.end) else {
                return Err(readback_error(
                    logical_module,
                    "reference span is outside exact source",
                    Some(entry.span),
                ));
            };
            let significant = nmlt_core::lex_source(slice)
                .tokens
                .into_iter()
                .filter(|token| !token.kind.is_trivia())
                .map(|token| token.text(slice))
                .collect::<String>();
            let expected = entry.qualifier.as_ref().map_or_else(
                || entry.spelling.clone(),
                |qualifier| format!("{qualifier}.{}", entry.spelling),
            );
            if significant != expected {
                return Err(readback_error(
                    logical_module,
                    "reference spelling does not match exact source bytes",
                    Some(entry.span),
                ));
            }
            let owner_path = module
                .declarations
                .values()
                .find(|declaration| declaration.id == entry.owner)
                .map(|declaration| &declaration.key.path)
                .ok_or_else(|| {
                    readback_error(
                        logical_module,
                        "resolution entry owner is not declared",
                        Some(entry.span),
                    )
                })?;
            let name = NameUse {
                qualifier: entry.qualifier.clone(),
                spelling: entry.spelling.clone(),
                span: entry.span,
            };
            let selected = readback_select(program, logical_module, owner_path, &name, entry)?;
            if selected != entry.target {
                return Err(readback_error(
                    logical_module,
                    "independent candidate replay selected a different target",
                    Some(entry.span),
                ));
            }
        }
    }

    let actual_references = program
        .resolution_map
        .entries
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    if expected_references != actual_references {
        return Err(ResolveError::ResolutionReadback {
            logical_module: "<source-set>".to_owned(),
            message: "reference-node and ResolutionMap origin sets are not bijective".to_owned(),
            span: None,
        });
    }
    Ok(())
}

fn readback_select(
    program: &ResolvedProgram,
    logical_module: &str,
    owner_path: &DefPath,
    name: &NameUse,
    entry: &ResolutionEntry,
) -> Result<ResolvedRef, ResolveError> {
    let class = match entry.namespace {
        Namespace::Type => ReferenceClass::Type,
        Namespace::State => ReferenceClass::State,
        Namespace::Capability => ReferenceClass::Capability,
        Namespace::Action => ReferenceClass::Action,
        Namespace::Constructor | Namespace::Value | Namespace::SystemInput => ReferenceClass::Value,
        Namespace::System | Namespace::Property | Namespace::Observation => {
            return Err(readback_error(
                logical_module,
                "reference map contains a namespace outside the M9 term fragment",
                Some(entry.span),
            ));
        }
    };
    let module = &program.modules[logical_module];
    let local = if matches!(class, ReferenceClass::Value) && name.qualifier.is_none() {
        module
            .local_binders
            .values()
            .find(|binder| binder.owner == entry.owner && binder.name == name.spelling)
    } else {
        None
    };
    let candidates =
        definition_candidates(&program.modules, logical_module, owner_path, name, class);
    match (local, candidates.as_slice()) {
        (Some(_), [_, ..]) => Err(readback_error(
            logical_module,
            "local binder shadows a visible definition",
            Some(entry.span),
        )),
        (Some(local), []) => Ok(ResolvedRef::Local(local.id)),
        (None, [(_, definition)]) => Ok(resolved_ref_for_definition(&program.modules, *definition)),
        (None, []) => Err(readback_error(
            logical_module,
            "candidate replay found no target",
            Some(entry.span),
        )),
        (None, [_, _, ..]) => Err(readback_error(
            logical_module,
            "candidate replay remained ambiguous",
            Some(entry.span),
        )),
    }
}

fn reachable_nodes(
    module: &ResolvedModule,
    roots: &BTreeSet<NodeId>,
) -> Result<BTreeSet<NodeId>, String> {
    let mut reachable = BTreeSet::new();
    let mut active = BTreeSet::new();
    for root in roots {
        let mut stack = vec![(*root, false)];
        while let Some((node_id, leaving)) = stack.pop() {
            if leaving {
                active.remove(&node_id);
                reachable.insert(node_id);
                continue;
            }
            if reachable.contains(&node_id) {
                continue;
            }
            if !active.insert(node_id) {
                return Err("HIR child graph contains a cycle".to_owned());
            }
            stack.push((node_id, true));
            let node = &module.hir_nodes[&node_id];
            for child in hir_children(&node.kind).into_iter().rev() {
                stack.push((child, false));
            }
        }
    }
    Ok(reachable)
}

fn hir_children(kind: &HirNodeKind) -> Vec<NodeId> {
    match kind {
        HirNodeKind::TypeOnce { protocol } => vec![*protocol],
        HirNodeKind::Unary { operand, .. } | HirNodeKind::IntFromNat { operand } => vec![*operand],
        HirNodeKind::Binary { left, right, .. } | HirNodeKind::Until { left, right } => {
            vec![*left, *right]
        }
        HirNodeKind::Always { property }
        | HirNodeKind::Eventually { property }
        | HirNodeKind::Next { property } => vec![*property],
        HirNodeKind::Enabled { action_origin, .. } => vec![*action_origin],
        HirNodeKind::TypeBool
        | HirNodeKind::TypeNat
        | HirNodeKind::TypeInt
        | HirNodeKind::TypeNamed(_)
        | HirNodeKind::ProtocolTag { .. }
        | HirNodeKind::BoolLiteral(_)
        | HirNodeKind::NaturalLiteral { .. }
        | HirNodeKind::Reference(_) => Vec::new(),
    }
}

fn readback_error(logical_module: &str, message: &str, span: Option<SourceSpan>) -> ResolveError {
    ResolveError::ResolutionReadback {
        logical_module: logical_module.to_owned(),
        message: message.to_owned(),
        span,
    }
}

fn is_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_reserved(value: &str) -> bool {
    matches!(
        value,
        "true"
            | "false"
            | "not"
            | "and"
            | "or"
            | "implies"
            | "always"
            | "eventually"
            | "next"
            | "until"
            | "enabled"
            | "to_int"
            | "Bool"
            | "Nat"
            | "Int"
            | "Once"
    )
}

fn display_name(name: &NameUse) -> String {
    name.qualifier.as_ref().map_or_else(
        || name.spelling.clone(),
        |qualifier| format!("{qualifier}.{}", name.spelling),
    )
}

#[allow(dead_code)]
fn referenced_definitions(map: &ResolutionMap) -> BTreeSet<DefId> {
    map.entries
        .values()
        .filter_map(|entry| entry.target.terminal_definition())
        .collect()
}

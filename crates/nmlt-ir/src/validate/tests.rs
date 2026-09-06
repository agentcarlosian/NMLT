use std::collections::{BTreeMap, BTreeSet};

use nmlt_hir::{
    DefId, ModuleId, Namespace, NodeId, ResolutionId, SemanticPathSegment, project_source_module,
    resolve_modules,
};

use super::{
    CoreResourceDimension, CoreValidationError, MAX_TERM_DEPTH, resource, validate_graph_with_limit,
};
use crate::{
    CoreAction, CoreBinaryOp, CoreModule, CoreNodeId, CoreProgram, CoreStateField, CoreSystem,
    CoreTerm, CoreTermKind, CoreType, CoreUnaryOp,
};

struct Fixture {
    resolution: ResolutionId,
    module: ModuleId,
    system: DefId,
    state: DefId,
    action: DefId,
    initializer_origin: NodeId,
    guard_origin: NodeId,
}

impl Fixture {
    fn new() -> Self {
        let source = concat!(
            "system Tiny {\n",
            "  state ready: Bool = false\n",
            "  action check() {\n",
            "    require true\n",
            "    set ready = true\n",
            "  }\n",
            "}\n",
        );
        let resolved = resolve_modules(vec![project_source_module(
            "Tiny",
            "src/tiny.nmlt",
            source.as_bytes(),
        )])
        .unwrap();
        let module = resolved.module("Tiny").unwrap();
        let definition = |namespace, name: &str| {
            module
                .declarations()
                .values()
                .find(|declaration| {
                    declaration
                        .key()
                        .path
                        .segments
                        .last()
                        .is_some_and(|segment| {
                            segment.namespace == namespace && segment.name == name
                        })
                })
                .unwrap()
                .id()
        };
        let state = definition(Namespace::State, "ready");
        let action = definition(Namespace::Action, "check");
        let origin = |owner, segment| {
            module
                .hir_roots()
                .iter()
                .find(|root| {
                    root.owner() == owner
                        && root.semantic_path().segments() == std::slice::from_ref(&segment)
                })
                .unwrap()
                .node()
        };
        Self {
            resolution: resolved.resolution_id(),
            module: module.id(),
            system: definition(Namespace::System, "Tiny"),
            state,
            action,
            initializer_origin: origin(state, SemanticPathSegment::Initializer),
            guard_origin: origin(action, SemanticPathSegment::Guard(0)),
        }
    }

    fn id(&self, index: u32) -> CoreNodeId {
        CoreNodeId::from_origin(self.guard_origin, &[index]).unwrap()
    }

    fn term(&self, index: u32, kind: CoreTermKind) -> CoreTerm {
        CoreTerm::new(
            self.id(index),
            self.guard_origin,
            self.action,
            CoreType::Bool,
            kind,
        )
    }

    fn leaf(&self, index: u32) -> CoreTerm {
        self.term(index, CoreTermKind::Bool(true))
    }

    fn not(&self, index: u32, operand: u32) -> CoreTerm {
        self.term(
            index,
            CoreTermKind::Unary {
                operator: CoreUnaryOp::Not,
                operand: self.id(operand),
            },
        )
    }

    fn and(&self, index: u32, left: u32, right: u32) -> CoreTerm {
        self.term(
            index,
            CoreTermKind::Binary {
                operator: CoreBinaryOp::And,
                left: self.id(left),
                right: self.id(right),
            },
        )
    }

    fn program(&self, roots: &[u32], terms: Vec<CoreTerm>) -> CoreProgram {
        let initializer_id = CoreNodeId::from_origin(self.initializer_origin, &[]).unwrap();
        let initializer = CoreTerm::new(
            initializer_id,
            self.initializer_origin,
            self.state,
            CoreType::Bool,
            CoreTermKind::Bool(false),
        );
        let action = CoreAction::new(
            self.action,
            self.system,
            BTreeMap::new(),
            roots.iter().map(|index| self.id(*index)).collect(),
            BTreeMap::new(),
            BTreeSet::from([self.state]),
            Vec::new(),
            BTreeSet::new(),
        );
        let system = CoreSystem::new(
            self.system,
            BTreeMap::from([(
                self.state,
                CoreStateField::new(self.state, CoreType::Bool, initializer_id),
            )]),
            BTreeMap::new(),
            BTreeMap::from([(self.action, action)]),
            BTreeMap::new(),
            Vec::new(),
        );
        let module = CoreModule::new(
            self.module,
            BTreeSet::new(),
            BTreeMap::new(),
            BTreeMap::from([(self.system, system)]),
        );
        CoreProgram::new(
            self.resolution,
            [module],
            std::iter::once(initializer).chain(terms),
        )
        .unwrap()
    }
}

fn depth_error(actual: usize, maximum: usize) -> CoreValidationError {
    CoreValidationError::ResourceExceeded {
        dimension: CoreResourceDimension::TermDepth,
        actual,
        maximum,
    }
}

#[test]
fn shared_diamond_and_repeated_children_are_valid_at_their_small_limit() {
    let fixture = Fixture::new();
    // The shared conjunction repeats one leaf; both sides of the diamond use it.
    let program = fixture.program(
        &[4],
        vec![
            fixture.leaf(0),
            fixture.and(1, 0, 0),
            fixture.not(2, 1),
            fixture.not(3, 1),
            fixture.and(4, 2, 3),
        ],
    );
    assert_eq!(program.terms().len(), 6);
    assert_eq!(validate_graph_with_limit(&program, 4), Ok(()));
    assert_eq!(
        validate_graph_with_limit(&program, 3),
        Err(depth_error(4, 3))
    );
}

#[test]
fn shared_subtree_depth_is_independent_of_child_order() {
    let fixture = Fixture::new();
    for (left, right) in [(1, 2), (2, 1)] {
        let program = fixture.program(
            &[3],
            vec![
                fixture.leaf(0),
                fixture.not(1, 0),
                fixture.not(2, 1),
                fixture.and(3, left, right),
            ],
        );
        assert_eq!(validate_graph_with_limit(&program, 4), Ok(()));
        assert_eq!(
            validate_graph_with_limit(&program, 3),
            Err(depth_error(4, 3))
        );
    }
}

#[test]
fn shared_roots_obey_the_same_depth_limit_in_both_orders() {
    let fixture = Fixture::new();
    for roots in [[1, 3], [3, 1]] {
        let program = fixture.program(
            &roots,
            vec![
                fixture.leaf(0),
                fixture.not(1, 0),
                fixture.not(2, 1),
                fixture.not(3, 2),
            ],
        );
        assert_eq!(validate_graph_with_limit(&program, 4), Ok(()));
        assert_eq!(
            validate_graph_with_limit(&program, 3),
            Err(depth_error(4, 3))
        );
    }
}

#[test]
fn term_insertion_order_preserves_shared_graph_identity() {
    let fixture = Fixture::new();
    let terms = vec![
        fixture.leaf(0),
        fixture.not(1, 0),
        fixture.not(2, 1),
        fixture.and(3, 1, 2),
    ];
    let forward = fixture.program(&[1, 3], terms.clone());
    let reverse = fixture.program(&[1, 3], terms.into_iter().rev().collect());
    assert_eq!(forward.id(), reverse.id());
    assert_eq!(forward.terms(), reverse.terms());
    for program in [&forward, &reverse] {
        assert_eq!(validate_graph_with_limit(program, 4), Ok(()));
        assert_eq!(
            validate_graph_with_limit(program, 3),
            Err(depth_error(4, 3))
        );
    }
}

#[test]
fn empty_leaf_and_small_chain_depth_boundaries_are_inclusive() {
    let fixture = Fixture::new();
    let empty = CoreProgram::new(
        fixture.resolution,
        [CoreModule::new(
            fixture.module,
            BTreeSet::new(),
            BTreeMap::new(),
            BTreeMap::new(),
        )],
        [],
    )
    .unwrap();
    assert_eq!(validate_graph_with_limit(&empty, 0), Ok(()));

    let leaf = fixture.program(&[0], vec![fixture.leaf(0)]);
    assert_eq!(validate_graph_with_limit(&leaf, 0), Err(depth_error(1, 0)));
    assert_eq!(validate_graph_with_limit(&leaf, 1), Ok(()));

    let chain = fixture.program(
        &[2],
        vec![fixture.leaf(0), fixture.not(1, 0), fixture.not(2, 1)],
    );
    assert_eq!(validate_graph_with_limit(&chain, 2), Err(depth_error(3, 2)));
    assert_eq!(validate_graph_with_limit(&chain, 3), Ok(()));
}

#[test]
fn active_cycle_is_rejected_with_a_small_budget() {
    let fixture = Fixture::new();
    let mut program = fixture.program(&[1], vec![fixture.leaf(0), fixture.not(1, 0)]);
    program.terms.insert(fixture.id(0), fixture.not(0, 1));
    assert_eq!(
        validate_graph_with_limit(&program, 4),
        Err(CoreValidationError::Cycle(fixture.id(1)))
    );
}

#[test]
fn missing_child_is_rejected_with_a_small_budget() {
    let fixture = Fixture::new();
    let mut program = fixture.program(&[1], vec![fixture.leaf(0), fixture.not(1, 0)]);
    program.terms.remove(&fixture.id(0));
    assert_eq!(
        validate_graph_with_limit(&program, 4),
        Err(CoreValidationError::MissingTerm {
            context: "term graph",
            term: fixture.id(0),
        })
    );
}

#[test]
fn unreachable_term_is_rejected_with_a_small_budget() {
    let fixture = Fixture::new();
    let mut program = fixture.program(&[0], vec![fixture.leaf(0)]);
    program.terms.insert(fixture.id(1), fixture.leaf(1));
    assert_eq!(
        validate_graph_with_limit(&program, 4),
        Err(CoreValidationError::UnreachableTerm(fixture.id(1)))
    );
}

#[test]
fn production_term_depth_resource_boundary_remains_256() {
    assert_eq!(MAX_TERM_DEPTH, 256);
    assert_eq!(
        resource(CoreResourceDimension::TermDepth, 256, MAX_TERM_DEPTH),
        Ok(())
    );
    assert_eq!(
        resource(CoreResourceDimension::TermDepth, 257, MAX_TERM_DEPTH),
        Err(depth_error(257, 256))
    );
}

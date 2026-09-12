use super::*;
use serde_json::json;

struct Fixture {
    builder: Builder,
    names: BTreeMap<u64, String>,
    root: u64,
    target: u64,
}
impl Fixture {
    fn new() -> Self {
        let mut fixture = Self {
            builder: Builder::default(),
            names: BTreeMap::new(),
            root: 0,
            target: 0,
        };
        fixture.target = fixture.name("NMLTTask.target");
        fixture.root = fixture.name("NMLTChecked.result");
        fixture.expression(json!({"sort":0}));
        fixture
    }
    fn name(&mut self, name: &str) -> u64 {
        let id = self.names.len() as u64 + 1;
        self.names.insert(id, name.into());
        id
    }
    fn expression(&mut self, mut row: Value) -> usize {
        let id = self.builder.expressions.len();
        row["ie"] = json!(id);
        self.builder.observe(&row).unwrap();
        id
    }
    fn constant(&mut self, name: u64) -> usize {
        self.expression(json!({"const":{"name":name,"us":[]}}))
    }
    fn axiom(&mut self, name: &str) -> u64 {
        let id = self.name(name);
        self.builder
            .observe(&json!({"axiom":{"name":id,"type":0}}))
            .unwrap();
        id
    }
    fn finish(mut self, target_type: usize, proof: usize) -> Graph {
        self.builder
            .observe(&json!({"def":{"name":self.target,"type":0,"value":target_type}}))
            .unwrap();
        let fixed = self.constant(self.target);
        self.builder
            .observe(&json!({"thm":{"name":self.root,"type":fixed,"value":proof}}))
            .unwrap();
        self.builder.finish(&self.names, "a".repeat(64)).unwrap()
    }
}

#[test]
fn references_distinguish_the_type_and_value_without_binder_names() {
    // Structural export fixtures exercise explanation only, not proof validity.
    let mut fixture = Fixture::new();
    let a = fixture.axiom("A");
    let b = fixture.axiom("B");
    let binder = fixture.name("UnreferencedBinderName");
    let a = fixture.constant(a);
    let b = fixture.constant(b);
    let proof = fixture.expression(json!({"lam":{"name":binder,"type":a,"body":b}}));
    let graph = fixture.finish(a, proof);
    assert_eq!(
        graph.node("NMLTChecked.result").unwrap().type_references,
        ["NMLTTask.target"]
    );
    assert_eq!(
        graph.node("NMLTChecked.result").unwrap().value_references,
        ["A", "B"]
    );
    assert_eq!(
        graph.node("NMLTTask.target").unwrap().value_references,
        ["A"]
    );
    assert!(
        graph
            .nodes
            .iter()
            .all(|node| node.name != "UnreferencedBinderName")
    );
}

#[test]
fn expression_forms_include_projection_names_and_separate_literal_support() {
    let mut fixture = Fixture::new();
    let a = fixture.axiom("A");
    let structure = fixture.axiom("Structure");
    for support in ["Nat", "Char.ofNat", "String.ofList"] {
        fixture.axiom(support);
    }
    let a = fixture.constant(a);
    let nat = fixture.expression(json!({"natVal":"3"}));
    let string = fixture.expression(json!({"strVal":"text"}));
    let app = fixture.expression(json!({"app":{"fn":a,"arg":nat}}));
    let var = fixture.expression(json!({"bvar":0}));
    let projection =
        fixture.expression(json!({"proj":{"typeName":structure,"idx":0,"struct":var}}));
    let forall = fixture.expression(json!({"forallE":{"name":0,"type":string,"body":projection}}));
    let let_expr =
        fixture.expression(json!({"letE":{"name":0,"type":forall,"value":app,"body":var}}));
    let proof =
        fixture.expression(json!({"mdata":{"expr":let_expr,"data":{"ignored":"metadata"}}}));
    let graph = fixture.finish(0, proof);
    let root = graph.node("NMLTChecked.result").unwrap();
    assert_eq!(root.value_references, ["A"]);
    assert_eq!(root.projection_references, ["Structure"]);
    assert_eq!(root.literal_support, ["Char.ofNat", "Nat", "String.ofList"]);
    assert!(root.reduction_references.is_empty());
}

#[test]
fn inductive_groups_and_recursor_rules_preserve_cycles_and_reduction_references() {
    let mut fixture = Fixture::new();
    let first = fixture.name("First");
    let second = fixture.name("Second");
    let ctor = fixture.name("First.mk");
    let rec = fixture.name("First.rec");
    let extra = fixture.axiom("RuleDependency");
    let first_type = fixture.constant(first);
    let second_type = fixture.constant(second);
    let rhs = fixture.constant(extra);
    fixture
        .builder
        .observe(&json!({"inductive":{
            "types":[{"name":first,"type":second_type},{"name":second,"type":first_type}],
            "ctors":[{"name":ctor,"type":first_type,"induct":first}],
            "recs":[{"name":rec,"type":first_type,"rules":[{"ctor":ctor,"rhs":rhs}]}]
        }}))
        .unwrap();
    let graph = fixture.finish(first_type, first_type);
    assert_eq!(graph.node("First").unwrap().type_references, ["Second"]);
    assert_eq!(graph.node("Second").unwrap().type_references, ["First"]);
    assert_eq!(
        graph.node("First.rec").unwrap().reduction_references,
        ["First.mk", "RuleDependency"]
    );
    assert_eq!(
        graph.groups,
        [Group {
            kind: GroupKind::Inductive,
            members: vec![
                "First".into(),
                "First.mk".into(),
                "First.rec".into(),
                "Second".into()
            ]
        }]
    );
}

#[test]
fn quotient_group_is_separate_from_direct_references() {
    let mut fixture = Fixture::new();
    for name in ["Quot", "Quot.mk", "Quot.lift", "Quot.ind"] {
        let name = fixture.name(name);
        fixture
            .builder
            .observe(&json!({"quot":{"name":name,"type":0}}))
            .unwrap();
    }
    let graph = fixture.finish(0, 0);
    assert_eq!(graph.groups[0].kind, GroupKind::Quotient);
    assert_eq!(
        graph.groups[0].members,
        ["Quot", "Quot.ind", "Quot.lift", "Quot.mk"]
    );
    assert!(
        graph
            .nodes
            .iter()
            .all(|node| node.literal_support.is_empty())
    );
}

#[test]
fn unknown_ambiguous_duplicate_or_forward_expressions_are_rejected() {
    for row in [
        json!({"ie":0,"unknown":0}),
        json!({"ie":0,"bvar":0,"sort":0}),
        json!({"ie":1,"sort":0}),
        json!({"ie":0,"app":{"fn":0,"arg":0}}),
        json!({"ie":0,"const":{"name":-1}}),
    ] {
        assert!(Builder::default().observe(&row).is_err());
    }
    let mut builder = Builder::default();
    builder.observe(&json!({"ie":0,"sort":0})).unwrap();
    assert!(builder.observe(&json!({"ie":0,"sort":0})).is_err());
    assert!(
        builder
            .observe(&json!({"axiom":{"name":1,"type":1}}))
            .is_err()
    );
}

#[test]
fn deep_shared_expression_dags_are_walked_without_recursion_or_tree_expansion() {
    let mut fixture = Fixture::new();
    let name = fixture.axiom("Shared");
    let mut expression = fixture.constant(name);
    for _ in 0..10_000 {
        expression = fixture.expression(json!({"app":{"fn":expression,"arg":expression}}));
    }
    let graph = fixture.finish(expression, expression);
    assert_eq!(
        graph.node("NMLTChecked.result").unwrap().value_references,
        ["Shared"]
    );
    assert_eq!(
        graph.node("NMLTTask.target").unwrap().value_references,
        ["Shared"]
    );
}

#[test]
fn a_dense_graph_with_valid_endpoints_is_rejected_at_the_reference_bound() {
    let mut graph = Fixture::new().finish(0, 0);
    let prototype = graph.node("NMLTTask.target").unwrap().clone();
    let names = (0..1024)
        .map(|index| format!("X{index:04}"))
        .collect::<Vec<_>>();
    for (index, name) in names.iter().enumerate() {
        let mut node = prototype.clone();
        node.name = name.clone();
        if index < 257 {
            node.type_references = names.clone();
        }
        graph.nodes.push(node);
    }
    assert!(
        graph
            .validate()
            .unwrap_err()
            .contains("reference or name-byte bound")
    );
}

#[test]
fn malformed_graphs_and_misleading_markdown_names_do_not_pass_as_ordinary_references() {
    let mut fixture = Fixture::new();
    let hostile_label = fixture.axiom("A[link](outside)<script>`\nB");
    let proof = fixture.constant(hostile_label);
    let graph = fixture.finish(0, proof);
    let markdown = graph.markdown().unwrap();
    assert!(!markdown.contains("<script>"));
    assert!(!markdown.contains("[link](outside)"));
    assert!(markdown.contains("&#10;"));
    assert!(markdown.contains("#declaration-"));
    let roundtrip: Graph = serde_json::from_slice(&graph.json().unwrap()).unwrap();
    assert_eq!(roundtrip, graph);
    let mut missing = graph.clone();
    missing.nodes[0].type_references = vec!["Absent".into()];
    assert!(missing.validate().is_err());
    let mut duplicate = graph.clone();
    duplicate.nodes.push(duplicate.nodes[0].clone());
    assert!(duplicate.validate().is_err());
    let mut group = graph;
    group.groups.push(Group {
        kind: GroupKind::Inductive,
        members: vec!["Absent".into()],
    });
    assert!(group.validate().is_err());
}

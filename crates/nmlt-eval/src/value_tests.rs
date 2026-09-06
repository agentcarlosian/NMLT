use super::*;

fn mixed_program() -> BehaviorCoreProgram {
    BehaviorCoreProgram::from_canonical_json(include_str!(
        "../../../examples/pivot/canonical_terms_and_wiring.behavior-core-v1.json"
    ))
    .unwrap()
}

fn enumeration(type_name: &str, constructor: &str) -> CoreBehaviorTerm {
    CoreBehaviorTerm::Enum {
        r#type: type_name.to_owned(),
        constructor: constructor.to_owned(),
    }
}

fn enum_value(constructor: &str) -> EvalValue {
    EvalValue::Enum {
        type_name: "Phase".to_owned(),
        constructor: constructor.to_owned(),
    }
}

fn read(ty: &str, field: &str) -> CoreBehaviorTerm {
    CoreBehaviorTerm::Read {
        r#type: ty.to_owned(),
        field: field.to_owned(),
    }
}

fn equal(left: CoreBehaviorTerm, right: CoreBehaviorTerm) -> CoreBehaviorTerm {
    CoreBehaviorTerm::Equal {
        r#type: "Bool".to_owned(),
        left: Box::new(left),
        right: Box::new(right),
    }
}

#[test]
fn existing_mixed_fixture_explores_all_finite_values_and_connections() {
    let program = mixed_program();
    let result = explore(&program, "Network", ExploreConfig::default()).unwrap();
    assert!(!result.truncated);
    assert_eq!(result.states.len(), 3);
    assert_eq!(result.transitions.len(), 6);
    let initial = &result.states[0];
    assert_eq!(initial.values["Sender.marker"], EvalValue::Unit);
    assert_eq!(initial.values["Sender.phase"], enum_value("Idle"));
    assert_eq!(initial.values["Sender.ready"], EvalValue::Bool(true));
    assert_eq!(initial.values["Receiver.done"], EvalValue::Bool(false));
    let receive = result
        .transitions
        .iter()
        .find(|step| step.from == 0 && step.label == "Receiver.receive|Sender.send")
        .unwrap();
    let after = &result.states[receive.to];
    assert_eq!(after.values["Sender.phase"], enum_value("Ready"));
    assert_eq!(after.values["Sender.marker"], EvalValue::Unit);
    assert_eq!(after.values["Sender.ready"], EvalValue::Bool(false));
    assert_eq!(after.values["Receiver.done"], EvalValue::Bool(true));
    assert_eq!(initial.values["Sender.ready"].to_string(), "true");
    assert_eq!(initial.values["Sender.marker"].to_string(), "()");
    assert_eq!(initial.values["Sender.phase"].to_string(), "Phase.Idle");
}

#[test]
fn enum_and_unit_equality_enable_actions_and_updates_use_the_same_prestate() {
    let mut program = mixed_program();
    let sender = program.systems.get_mut("Sender").unwrap();
    let mut other = sender.state["phase"].clone();
    other.name = "other".to_owned();
    other.initial_ast = enumeration("Phase", "Ready");
    sender.state.insert("other".to_owned(), other);
    sender.actions.remove("ack");
    let action = sender.actions.get_mut("send").unwrap();
    action.guard_ast = vec![
        equal(read("Phase", "phase"), enumeration("Phase", "Idle")),
        equal(
            read("Unit", "marker"),
            CoreBehaviorTerm::Unit {
                r#type: "Unit".to_owned(),
            },
        ),
    ];
    action.update_ast = BTreeMap::from([
        ("phase".to_owned(), read("Phase", "other")),
        ("other".to_owned(), read("Phase", "phase")),
    ]);
    let result = explore(&program, "Sender", ExploreConfig::default()).unwrap();
    assert_eq!(result.states.len(), 2);
    assert_eq!(result.transitions.len(), 1);
    assert_eq!(result.states[1].values["phase"], enum_value("Ready"));
    assert_eq!(result.states[1].values["other"], enum_value("Idle"));
    assert_eq!(result.states[0].values["phase"], enum_value("Idle"));
}

#[test]
fn closed_initial_equality_evaluates_enum_and_unit_values() {
    let mut program = mixed_program();
    let sender = program.systems.get_mut("Sender").unwrap();
    sender.state.get_mut("ready").unwrap().initial_ast =
        equal(enumeration("Phase", "Idle"), enumeration("Phase", "Ready"));
    let result = explore(&program, "Sender", ExploreConfig::default()).unwrap();
    assert_eq!(result.states[0].values["ready"], EvalValue::Bool(false));

    let sender = program.systems.get_mut("Sender").unwrap();
    let unit = CoreBehaviorTerm::Unit {
        r#type: "Unit".to_owned(),
    };
    sender.state.get_mut("ready").unwrap().initial_ast = equal(unit.clone(), unit);
    let result = explore(&program, "Sender", ExploreConfig::default()).unwrap();
    assert_eq!(result.states[0].values["ready"], EvalValue::Bool(true));
}

#[test]
fn enum_literals_require_the_declared_type_and_constructor() {
    for (ty, constructor) in [("Phase", "Unknown"), ("Unknown", "Idle"), ("Bool", "Idle")] {
        let mut program = mixed_program();
        program
            .enums
            .insert("Bool".to_owned(), BTreeSet::from(["Idle".to_owned()]));
        let phase = program
            .systems
            .get_mut("Sender")
            .unwrap()
            .state
            .get_mut("phase")
            .unwrap();
        phase.ty = ty.to_owned();
        phase.initial_ast = enumeration(ty, constructor);
        let error = explore(&program, "Sender", ExploreConfig::default()).unwrap_err();
        assert!(error.to_string().contains("closed initializer"), "{error}");
    }
}

#[test]
fn mismatched_types_and_invalid_updates_are_errors_even_in_disabled_actions() {
    let cases = [
        enumeration("Phase", "Unknown"),
        CoreBehaviorTerm::Unit {
            r#type: "Unit".to_owned(),
        },
        read("Bool", "phase"),
    ];
    for term in cases {
        let mut program = mixed_program();
        let action = program
            .systems
            .get_mut("Sender")
            .unwrap()
            .actions
            .get_mut("send")
            .unwrap();
        action.guard_ast = vec![CoreBehaviorTerm::Bool {
            r#type: "Bool".to_owned(),
            value: false,
        }];
        action.update_ast.insert("phase".to_owned(), term);
        let error = explore(&program, "Sender", ExploreConfig::default()).unwrap_err();
        assert!(error.to_string().contains("invalid update AST"), "{error}");
    }
}

#[test]
fn equality_does_not_conflate_types_or_accept_non_boolean_guards() {
    for guard in [
        enumeration("Phase", "Idle"),
        equal(
            enumeration("Phase", "Idle"),
            CoreBehaviorTerm::Bool {
                r#type: "Bool".to_owned(),
                value: false,
            },
        ),
        equal(enumeration("Phase", "Idle"), enumeration("Other", "Idle")),
    ] {
        let mut program = mixed_program();
        program
            .enums
            .insert("Other".to_owned(), BTreeSet::from(["Idle".to_owned()]));
        let action = program
            .systems
            .get_mut("Sender")
            .unwrap()
            .actions
            .get_mut("send")
            .unwrap();
        action.guard_ast = vec![guard];
        let error = explore(&program, "Sender", ExploreConfig::default()).unwrap_err();
        assert!(
            error.to_string().contains("invalid Bool guard AST"),
            "{error}"
        );
    }
}

#[test]
fn finite_value_states_obey_the_exploration_limit() {
    let program = mixed_program();
    let result = explore(&program, "Network", ExploreConfig { max_states: 1 }).unwrap();
    assert!(result.truncated);
    assert_eq!(result.states.len(), 1);
    assert!(
        result
            .transitions
            .iter()
            .all(|step| step.from == 0 && step.to == 0)
    );
}

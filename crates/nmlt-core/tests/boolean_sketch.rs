//! Paper 1 boolean sketch: reachable Bool assignments, not source-to-LTS.

use std::collections::BTreeMap;

use nmlt_core::{SketchTransition, parse_cst, project_untyped, sketch_boolean_system};

fn paper1_source() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/paper1/hidden_ping_receive.nmlt"
    ))
    .expect("paper1 fixture")
}

fn sketch_named(source: &str, name: &str) -> nmlt_core::BooleanSketch {
    let projection = project_untyped(&parse_cst(source));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let system = projection
        .file
        .system_named(name)
        .unwrap_or_else(|| panic!("{name}"));
    sketch_boolean_system(system).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn state(field: &str, value: bool) -> BTreeMap<String, bool> {
    BTreeMap::from([(field.to_owned(), value)])
}

fn sketch_err(source: &str) -> nmlt_core::BooleanSketchError {
    let projection = project_untyped(&parse_cst(source));
    assert!(
        projection.is_structurally_complete(),
        "{:?}",
        projection.issues
    );
    let system = projection
        .file
        .systems()
        .into_iter()
        .next()
        .expect("system");
    sketch_boolean_system(system).expect_err("sketch must refuse")
}

#[test]
fn concrete_sender_is_one_state_ping_loop() {
    let sketch = sketch_named(&paper1_source(), "ConcreteSender");
    assert_eq!(sketch.fields, ["unit"]);
    assert_eq!(sketch.states, [state("unit", false)]);
    assert_eq!(sketch.initial, 0);
    assert_eq!(
        sketch.transitions,
        [SketchTransition {
            from: 0,
            action: "ping".to_owned(),
            to: 0,
        }]
    );
    assert_eq!(sketch.action_names, ["ping"]);
    assert_eq!(sketch.hidden_actions, ["ping"]);
    assert_eq!(sketch.observed_fields, ["unit"]);
}

#[test]
fn receiver_receive_only_from_false_to_true() {
    let sketch = sketch_named(&paper1_source(), "Receiver");
    assert_eq!(sketch.fields, ["bit"]);
    assert_eq!(sketch.states, [state("bit", false), state("bit", true)]);
    assert_eq!(sketch.initial, 0);
    assert_eq!(
        sketch.transitions,
        [SketchTransition {
            from: 0,
            action: "receive".to_owned(),
            to: 1,
        }]
    );
    assert_eq!(sketch.action_names, ["receive"]);
    assert!(sketch.hidden_actions.is_empty());
    assert_eq!(sketch.observed_fields, ["bit"]);
}

#[test]
fn abstract_sender_is_one_state_with_no_transitions() {
    let sketch = sketch_named(&paper1_source(), "AbstractSender");
    assert_eq!(sketch.fields, ["unit"]);
    assert_eq!(sketch.states, [state("unit", false)]);
    assert_eq!(sketch.initial, 0);
    assert!(sketch.transitions.is_empty());
    assert!(sketch.action_names.is_empty());
    assert!(sketch.hidden_actions.is_empty());
    assert_eq!(sketch.observed_fields, ["unit"]);
}

#[test]
fn rejects_nat_state_and_unsupported_statements() {
    let nat = sketch_err(concat!(
        "system BadNat {\n",
        "  state n: Nat = 0\n",
        "  observe n\n",
        "}\n",
    ));
    assert_eq!(nat.code, "NMLT-SKETCH-NAT");
    assert!(nat.to_string().contains("Nat"));

    let emit = sketch_err(concat!(
        "system BadEmit {\n",
        "  state bit: Bool = false\n",
        "  action go { emit bit }\n",
        "  observe bit\n",
        "}\n",
    ));
    assert_eq!(emit.code, "NMLT-SKETCH-UNSUPPORTED");
    assert!(emit.to_string().contains("emit"));

    let consume = sketch_err(concat!(
        "system BadConsume {\n",
        "  state bit: Bool = false\n",
        "  action go { consume bit }\n",
        "  observe bit\n",
        "}\n",
    ));
    assert_eq!(consume.code, "NMLT-SKETCH-UNSUPPORTED");
    assert!(consume.to_string().contains("consume"));
}

fn receptive_source() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/paper1/receptive_receiver.nmlt"
    ))
    .expect("receptive dual fixture")
}

#[test]
fn receptive_receiver_receive_from_false_and_true() {
    // Unguarded `set bit = true` is enabled at both reachable assignments.
    // This is the OpenSystem dual, not the Lean Receiver.
    let sketch = sketch_named(&receptive_source(), "ReceptiveReceiver");
    assert_eq!(sketch.fields, ["bit"]);
    assert_eq!(sketch.states, [state("bit", false), state("bit", true)]);
    assert_eq!(sketch.initial, 0);
    assert_eq!(
        sketch.transitions,
        [
            SketchTransition {
                from: 0,
                action: "receive".to_owned(),
                to: 1,
            },
            SketchTransition {
                from: 1,
                action: "receive".to_owned(),
                to: 1,
            },
        ]
    );
    assert_eq!(sketch.action_names, ["receive"]);
    assert!(sketch.hidden_actions.is_empty());
    assert_eq!(sketch.observed_fields, ["bit"]);
}

use std::collections::{BTreeMap, VecDeque};

use nmlt_compile::{compile_behavior_single, compile_behavior_v2};
use nmlt_eval::{
    EvalValue, ExploreConfig, MAX_RUN_STEPS, RunConfig, RunOutcome, Schedule, execute, explore,
};

const CONTINUATION: &str = include_str!("../../../examples/pivot/affine_continuation.nmlt");
const VALUES: &str = include_str!("../../../examples/pivot/finite_value_cycle.nmlt");
const RETRY: &str = include_str!("../../../examples/pivot/finite_retry.nmlt");

fn config(labels: &[&str]) -> RunConfig {
    RunConfig {
        max_steps: 32,
        schedule: Schedule::Actions {
            labels: labels.iter().map(|s| (*s).into()).collect(),
        },
    }
}

#[test]
fn run_handles_finite_failure_retries_once_and_reuses_a_value_without_authority() {
    for (reject, expected_steps, expected_work) in [(true, 5, 2), (false, 3, 1)] {
        let source = RETRY.replace(
            "reject_first: Bool = true",
            &format!("reject_first: Bool = {reject}"),
        );
        let program = compile_behavior_v2("retry.nmlt", source).unwrap();
        let trace = execute(
            &program,
            "Main",
            &RunConfig {
                max_steps: expected_steps,
                schedule: Schedule::FirstEnabled,
            },
        )
        .unwrap();
        assert_eq!(trace.outcome, RunOutcome::Quiescent);
        assert_eq!(trace.steps.len(), expected_steps);
        assert_eq!(
            trace.model_grade,
            BTreeMap::from([("work".into(), expected_work)])
        );
        let last = &trace.steps.last().unwrap().after;
        assert!(last.authority.is_empty());
        assert_eq!(last.values["Worker.copied_result"], EvalValue::Bool(true));
        assert_eq!(trace.steps.last().unwrap().label, "Worker.reuse");
    }
}

#[test]
fn received_authority_is_required_and_consumed_or_returned_once() {
    let program = compile_behavior_v2("continuation.nmlt", CONTINUATION).unwrap();
    for labels in [
        vec![
            "Receiver.receive|Sender.send",
            "Receiver.monitor",
            "Receiver.use",
        ],
        vec![
            "Receiver.receive|Sender.send",
            "Receiver.giveback|Sender.returned",
            "Sender.finish",
        ],
    ] {
        let trace = execute(&program, "Network", &config(&labels)).unwrap();
        assert_eq!(trace.outcome, RunOutcome::ScheduleComplete);
        assert_eq!(trace.initial.authority["permit"], "Sender");
        assert_eq!(trace.steps[0].after.authority["permit"], "Receiver");
        assert!(trace.steps.last().unwrap().after.authority.is_empty());
        let mut repeat = labels.clone();
        repeat.push(labels.last().unwrap());
        let rejected = execute(&program, "Network", &config(&repeat)).unwrap();
        assert!(matches!(
            rejected.outcome,
            RunOutcome::ActionUnavailable { .. }
        ));
        assert_eq!(rejected.steps, trace.steps);
    }
    let rejected = execute(&program, "Network", &config(&["Receiver.use"])).unwrap();
    assert!(rejected.steps.is_empty());
    assert!(matches!(
        rejected.outcome,
        RunOutcome::ActionUnavailable { .. }
    ));
    let rejected = execute(
        &program,
        "Network",
        &config(&["Receiver.receive|Sender.send", "Sender.finish"]),
    )
    .unwrap();
    assert_eq!(rejected.steps.len(), 1);
    assert!(matches!(
        rejected.outcome,
        RunOutcome::ActionUnavailable { .. }
    ));
}

#[test]
fn cycles_are_bounded_even_when_no_new_state_is_created() {
    let program = compile_behavior_v2("values.nmlt", VALUES).unwrap();
    let trace = execute(
        &program,
        "ValueCycle",
        &RunConfig {
            max_steps: 4,
            schedule: Schedule::FirstEnabled,
        },
    )
    .unwrap();
    assert_eq!(trace.outcome, RunOutcome::StepLimit);
    assert_eq!(trace.steps.len(), 4);
    assert_eq!(trace.steps[0].after, trace.steps[3].after);
    assert_eq!(trace.steps[1].label, "ValueCycle.hold");
    let mut bounded = config(&["ValueCycle.hold", "ValueCycle.hold"]);
    bounded.max_steps = 1;
    assert_eq!(
        execute(&program, "ValueCycle", &bounded).unwrap().outcome,
        RunOutcome::StepLimit
    );
    bounded.max_steps = 2;
    assert_eq!(
        execute(&program, "ValueCycle", &bounded).unwrap().outcome,
        RunOutcome::ScheduleComplete
    );
}

#[test]
fn no_enabled_step_and_requested_disabled_step_have_distinct_outcomes() {
    let program = compile_behavior_v2("continuation.nmlt", CONTINUATION).unwrap();
    let trace = execute(
        &program,
        "Sender",
        &RunConfig {
            max_steps: 1,
            schedule: Schedule::FirstEnabled,
        },
    )
    .unwrap();
    assert_eq!(trace.outcome, RunOutcome::Quiescent);
    assert!(trace.steps.is_empty());
    let trace = execute(&program, "Sender", &config(&["Sender.send"])).unwrap();
    assert_eq!(
        trace.outcome,
        RunOutcome::ActionUnavailable {
            requested: "Sender.send".into(),
            enabled: vec![]
        }
    );
    assert_eq!(
        execute(&program, "Sender", &config(&[])).unwrap().outcome,
        RunOutcome::ScheduleComplete
    );
}

#[test]
fn cumulative_grade_overflow_stops_before_changing_the_state() {
    let source = format!(
        "system Cost {{\n state bit: Bool = false\n action tick grade {{ work: {} }} {{\n set bit = ! bit\n }}\n observe bit\n }}",
        u64::MAX
    );
    let program = compile_behavior_v2("cost.nmlt", source).unwrap();
    let trace = execute(
        &program,
        "Cost",
        &RunConfig {
            max_steps: 3,
            schedule: Schedule::FirstEnabled,
        },
    )
    .unwrap();
    assert_eq!(
        trace.outcome,
        RunOutcome::GradeOverflow {
            atom: "work".into()
        }
    );
    assert_eq!(trace.steps.len(), 1);
    assert_eq!(trace.steps[0].after.values["bit"], EvalValue::Bool(true));
    assert_eq!(trace.model_grade["work"], u64::MAX);
}

#[test]
fn invalid_configuration_v1_and_forged_initial_authority_are_rejected() {
    let mut program = compile_behavior_v2("continuation.nmlt", CONTINUATION).unwrap();
    for max_steps in [0, MAX_RUN_STEPS + 1, usize::MAX] {
        assert!(
            execute(
                &program,
                "Network",
                &RunConfig {
                    max_steps,
                    schedule: Schedule::FirstEnabled
                }
            )
            .is_err()
        );
    }
    assert!(execute(&program, "Missing", &config(&[])).is_err());
    assert!(execute(&program, "Network", &config(&[""])).is_err());
    program
        .initial_authority
        .get_mut("Network")
        .unwrap()
        .insert("permit".into(), Some("Receiver".into()));
    assert!(execute(&program, "Network", &config(&[])).is_err());
    let v1 = compile_behavior_single("values.nmlt", VALUES).unwrap();
    assert!(execute(&v1, "ValueCycle", &config(&[])).is_err());
}

#[test]
fn every_frozen_graph_edge_can_be_executed_from_its_initial_path() {
    // Cover all value/resource edges, including hidden steps and self-loops.
    // The separate Rust/Lean gates give these same frozen graphs an external oracle.
    for (source, behavior, state_count, edge_count) in [
        (VALUES, "ValueCycle", 4, 10),
        (CONTINUATION, "Network", 8, 12),
    ] {
        let program = compile_behavior_v2("fixture.nmlt", source).unwrap();
        let graph = explore(&program, behavior, ExploreConfig { max_states: 32 }).unwrap();
        assert!(!graph.truncated);
        assert_eq!(
            (graph.states.len(), graph.transitions.len()),
            (state_count, edge_count)
        );
        let mut paths = BTreeMap::from([(0, Vec::<String>::new())]);
        let mut queue = VecDeque::from([0]);
        while let Some(from) = queue.pop_front() {
            for edge in graph.transitions.iter().filter(|edge| edge.from == from) {
                let mut labels = paths[&from].clone();
                labels.push(edge.label.clone());
                let trace = execute(
                    &program,
                    behavior,
                    &RunConfig {
                        max_steps: 32,
                        schedule: Schedule::Actions {
                            labels: labels.clone(),
                        },
                    },
                )
                .unwrap();
                assert_eq!(trace.initial, graph.states[0]);
                let step = trace.steps.last().unwrap();
                assert_eq!(step.after, graph.states[edge.to]);
                assert_eq!(step.model_grade, edge.grade);
                assert_eq!(step.transfers, edge.transfers);
                if let std::collections::btree_map::Entry::Vacant(entry) = paths.entry(edge.to) {
                    entry.insert(labels);
                    queue.push_back(edge.to);
                }
            }
        }
        assert_eq!(paths.len(), state_count);
    }
}

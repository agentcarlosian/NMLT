use std::collections::BTreeMap;

use nmlt_temporal::{
    ActionHiding, CheckOutcome, Fairness, FairnessSet, FiniteGraph, JournalAction, JournalRecord,
    JournalValue, ModelState, ObservationMap, RefinementChecker, RefinementSpec, RuntimeMapping,
    RuntimeTraceAdapter, RuntimeVerdict, TemporalChecker, Transition, Value,
};

fn state(done: bool, internal: i64) -> ModelState {
    BTreeMap::from([
        ("done".to_owned(), Value::Bool(done)),
        ("internal".to_owned(), Value::Int(internal)),
    ])
}

fn observations(done: bool) -> BTreeMap<String, JournalValue> {
    BTreeMap::from([("done".to_owned(), JournalValue::Known(Value::Bool(done)))])
}

fn provider_state(phase: &str, dispatch_enabled: bool) -> ModelState {
    BTreeMap::from([
        ("phase".to_owned(), Value::Text(phase.to_owned())),
        ("dispatch_enabled".to_owned(), Value::Bool(dispatch_enabled)),
    ])
}

fn no_blind_replay(value: &ModelState) -> bool {
    value.get("phase") != Some(&Value::Text("indeterminate".to_owned()))
        || value.get("dispatch_enabled") == Some(&Value::Bool(false))
}

fn assert_enabled_projection(graph: &FiniteGraph) {
    for (state, value) in graph.states().iter().enumerate() {
        assert_eq!(
            value.get("dispatch_enabled"),
            Some(&Value::Bool(graph.action_enabled(state, "dispatch"))),
            "provider projection must derive enabledness from outgoing actions"
        );
    }
}

fn main() {
    let progress_graph = FiniteGraph::new(
        vec![state(false, 0), state(true, 0)],
        vec![0],
        vec![
            Transition::action(0, "idle", 0),
            Transition::action(0, "work", 1),
        ],
    )
    .expect("phase 4 fixture graph");
    let goal = |value: &ModelState| value.get("done") == Some(&Value::Bool(true));
    let unfair = TemporalChecker::new(&progress_graph, FairnessSet::default()).eventually(goal);
    let fair = TemporalChecker::new(
        &progress_graph,
        FairnessSet::new(vec![Fairness::weak("work")]),
    )
    .eventually(goal);
    let CheckOutcome::Violated { witness, .. } = unfair else {
        panic!("identity stutter/idle must witness missing progress without fairness")
    };
    assert!(matches!(fair, CheckOutcome::Holds { .. }));

    let concrete = FiniteGraph::new(
        vec![state(false, 0), state(false, 1), state(true, 1)],
        vec![0],
        vec![
            Transition::action(0, "cache", 1),
            Transition::action(1, "publish", 2),
        ],
    )
    .expect("concrete graph");
    let abstract_graph = FiniteGraph::new(
        vec![state(false, 99), state(true, 99)],
        vec![0],
        vec![Transition::action(0, "commit", 1)],
    )
    .expect("abstract graph");
    let refinement = RefinementChecker::check(
        &concrete,
        &abstract_graph,
        &RefinementSpec {
            state_map: vec![0, 0, 1],
            concrete_observation: ObservationMap::identity(["done"]),
            abstract_observation: ObservationMap::identity(["done"]),
            actions: ActionHiding::new([("cache", None::<&str>), ("publish", Some("commit"))]),
        },
    );
    assert!(refinement.accepted);

    let mapping = RuntimeMapping::identity(["done"]);
    let adapter = RuntimeTraceAdapter::new(&progress_graph, &mapping);
    let accepted = adapter.check(&[
        JournalRecord {
            sequence: 40,
            action: JournalAction::Initial,
            observations: observations(false),
        },
        JournalRecord {
            sequence: 41,
            action: JournalAction::Action("work".to_owned()),
            observations: observations(true),
        },
    ]);
    let rejected = adapter.check(&[
        JournalRecord {
            sequence: 40,
            action: JournalAction::Initial,
            observations: observations(false),
        },
        JournalRecord {
            sequence: 41,
            action: JournalAction::Action("work".to_owned()),
            observations: observations(false),
        },
    ]);
    assert_eq!(accepted.verdict, RuntimeVerdict::Accepted);
    assert_eq!(rejected.verdict, RuntimeVerdict::Rejected);
    let issue = rejected.issue.expect("rejection is localized");

    // Property-relevant observation graph of the frozen provider reference.
    // States separated only by hidden pass/fail data retain distinct IDs so the
    // transition structure is preserved even when their observations coincide.
    let provider_reference = FiniteGraph::new(
        vec![
            provider_state("proposed", false),
            provider_state("authorized", false),
            provider_state("authorized", true),
            provider_state("dispatched", false),
            provider_state("responded", false),
            provider_state("indeterminate", false),
            provider_state("evaluated", false),
            provider_state("evaluated", false),
            provider_state("selected", false),
        ],
        vec![0],
        vec![
            Transition::action(0, "authorize", 1),
            Transition::action(1, "arm", 2),
            Transition::action(2, "dispatch", 3),
            Transition::action(3, "receive_response", 4),
            Transition::action(3, "lose_response", 5),
            Transition::action(4, "evaluate_fail", 7),
            Transition::action(4, "evaluate_pass", 6),
            Transition::action(6, "select", 8),
        ],
    )
    .expect("provider reference projection");
    assert_enabled_projection(&provider_reference);
    let provider_reference_outcome =
        TemporalChecker::new(&provider_reference, FairnessSet::default()).always(no_blind_replay);
    let CheckOutcome::Holds {
        explored_states: provider_reference_states,
    } = provider_reference_outcome
    else {
        panic!("reference provider must satisfy temporal NoBlindReplay")
    };

    // The blind mutant is quotiented by hiding its unbounded dispatch counter.
    // Dispatch remains enabled in the sole observation state, so the initial
    // violation extends through universal identity stutter to an infinite lasso.
    let provider_mutant = FiniteGraph::new(
        vec![provider_state("indeterminate", true)],
        vec![0],
        vec![Transition::action(0, "dispatch", 0)],
    )
    .expect("blind replay observation quotient");
    assert_enabled_projection(&provider_mutant);
    let CheckOutcome::Violated {
        witness: provider_witness,
        ..
    } = TemporalChecker::new(&provider_mutant, FairnessSet::default()).always(no_blind_replay)
    else {
        panic!("blind replay mutant must violate temporal NoBlindReplay")
    };

    println!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"1.0.0\",\n",
            "  \"fixture\": \"phase4-temporal-refinement-runtime-v1\",\n",
            "  \"graph_id\": \"nmlt-temporal-graph-v1:sha256:7cd9a36ee384964f76b4c3f8b9ea0ccad3016c9e178f864629a5ac7ed6dd849f\",\n",
            "  \"temporal\": {{\n",
            "    \"property\": \"eventually(done)\",\n",
            "    \"without_fairness\": {{ \"result\": \"refuted\", \"stem_states\": {:?}, \"stem_transitions\": {:?}, \"loop_states\": {:?}, \"loop_transitions\": {:?} }},\n",
            "    \"with_fairness\": {{ \"result\": \"model_checked\", \"assumptions\": [\"weak:work\"] }}\n",
            "  }},\n",
            "  \"refinement\": {{ \"result\": \"accepted\", \"hidden_actions\": [\"cache\"], \"state_map\": [0, 0, 1], \"checked_states\": {}, \"checked_transitions\": {} }},\n",
            "  \"runtime\": {{\n",
            "    \"case_id\": \"\",\n",
            "    \"mapping\": {{ \"done\": \"done\" }},\n",
            "    \"accepted_journal\": [{{ \"sequence\": 40, \"action\": \"initial\", \"observations\": {{ \"done\": false }} }}, {{ \"sequence\": 41, \"action\": \"work\", \"observations\": {{ \"done\": true }} }}],\n",
            "    \"contradictory_journal\": [{{ \"sequence\": 40, \"action\": \"initial\", \"observations\": {{ \"done\": false }} }}, {{ \"sequence\": 41, \"action\": \"work\", \"observations\": {{ \"done\": false }} }}],\n",
            "    \"accepted_trace\": \"accepted\", \"contradictory_trace\": \"rejected\", \"rejected_record_index\": {}, \"rejected_candidates_before\": {:?}, \"rejected_reason\": {:?}\n",
            "  }},\n",
            "  \"provider_temporal\": {{\n",
            "    \"semantics\": \"finite-observation-quotient+universal-identity-stutter-v1\",\n",
            "    \"reference\": {{ \"result\": \"model_checked\", \"explored_states\": {} }},\n",
            "    \"mutant\": {{ \"result\": \"refuted\", \"stem_states\": {:?}, \"stem_transitions\": {:?}, \"loop_states\": {:?}, \"loop_transitions\": {:?} }}\n",
            "  }},\n",
            "  \"residual_gaps\": [\"finite graph only\", \"provider source-to-observation-graph construction is manually audited, not compiler-verified\", \"fairness is an explicit assumption\", \"forward simulation does not prove liveness refinement\", \"journal authenticity is not established\"],\n",
            "  \"implementation\": {{}}\n",
            "}}"
        ),
        witness.stem_states,
        witness.stem_transitions,
        witness.loop_states,
        witness.loop_transitions,
        refinement.checked_states,
        refinement.checked_transitions,
        issue.record_index.expect("transition rejection has index"),
        issue.candidates_before,
        issue.message,
        provider_reference_states,
        provider_witness.stem_states,
        provider_witness.stem_transitions,
        provider_witness.loop_states,
        provider_witness.loop_transitions,
    );
}

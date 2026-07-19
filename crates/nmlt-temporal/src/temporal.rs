use crate::graph::{FiniteGraph, ModelState, StateId, TransitionId};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FairnessKind {
    Weak,
    Strong,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fairness {
    pub action: String,
    pub kind: FairnessKind,
}

impl Fairness {
    pub fn weak(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            kind: FairnessKind::Weak,
        }
    }

    pub fn strong(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            kind: FairnessKind::Strong,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FairnessSet(Vec<Fairness>);

impl FairnessSet {
    pub fn new(mut assumptions: Vec<Fairness>) -> Self {
        assumptions.sort();
        assumptions.dedup();
        Self(assumptions)
    }

    pub fn assumptions(&self) -> &[Fairness] {
        &self.0
    }
}

/// A deterministic ultimately-periodic counterexample.
///
/// Each transition vector is one shorter than its state vector. The final loop state
/// equals the first loop state. The final stem state equals the first loop state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lasso {
    pub stem_states: Vec<StateId>,
    pub stem_transitions: Vec<TransitionId>,
    pub loop_states: Vec<StateId>,
    pub loop_transitions: Vec<TransitionId>,
}

impl Lasso {
    pub fn is_well_formed(&self, graph: &FiniteGraph) -> bool {
        if self.stem_states.is_empty()
            || self.loop_states.len() < 2
            || self.stem_states.len() != self.stem_transitions.len() + 1
            || self.loop_states.len() != self.loop_transitions.len() + 1
            || self.stem_states.last() != self.loop_states.first()
            || self.loop_states.first() != self.loop_states.last()
        {
            return false;
        }
        path_matches(graph, &self.stem_states, &self.stem_transitions)
            && path_matches(graph, &self.loop_states, &self.loop_transitions)
    }

    pub fn loop_actions<'a>(&self, graph: &'a FiniteGraph) -> Vec<Option<&'a str>> {
        self.loop_transitions
            .iter()
            .map(|&transition| graph.transition(transition).kind.action())
            .collect()
    }
}

fn path_matches(graph: &FiniteGraph, states: &[StateId], transitions: &[TransitionId]) -> bool {
    transitions.iter().enumerate().all(|(index, &transition)| {
        graph
            .transitions()
            .get(transition)
            .is_some_and(|edge| edge.from == states[index] && edge.to == states[index + 1])
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckOutcome {
    Holds {
        explored_states: usize,
    },
    Violated {
        explored_states: usize,
        /// Present for `leads_to`; the state at which the antecedent became true.
        trigger: Option<StateId>,
        witness: Lasso,
    },
}

impl CheckOutcome {
    pub fn holds(&self) -> bool {
        matches!(self, Self::Holds { .. })
    }
}

/// Explicit finite-state temporal checking with identity-stutter closure at every state.
pub struct TemporalChecker {
    graph: FiniteGraph,
    fairness: FairnessSet,
}

impl TemporalChecker {
    pub fn new(graph: &FiniteGraph, fairness: FairnessSet) -> Self {
        Self {
            graph: graph.stutter_closed(),
            fairness,
        }
    }

    pub fn graph(&self) -> &FiniteGraph {
        &self.graph
    }

    /// Checks `always predicate` on every infinite behavior.
    ///
    /// Universal identity-stutter closure means any reachable violating state
    /// extends to a deterministic infinite lasso, including an initial-state
    /// violation with a zero-transition stem.
    pub fn always<P>(&self, predicate: P) -> CheckOutcome
    where
        P: Fn(&ModelState) -> bool,
    {
        let reachable = self.graph.reachable_states();
        let explored_states = reachable.len();
        let Some(violating) = reachable
            .iter()
            .copied()
            .find(|&state| !predicate(self.graph.state(state)))
        else {
            return CheckOutcome::Holds { explored_states };
        };
        let (stem_states, stem_transitions) = self
            .graph
            .shortest_path(self.graph.initial_states(), violating, &reachable)
            .expect("reachable violating state has a path from an initial state");
        let stutter = self
            .graph
            .outgoing_ids(violating)
            .iter()
            .copied()
            .find(|&transition| {
                let edge = self.graph.transition(transition);
                edge.from == violating && edge.to == violating && edge.kind.action().is_none()
            })
            .expect("temporal checker graph is universally stutter closed");
        let witness = Lasso {
            stem_states,
            stem_transitions,
            loop_states: vec![violating, violating],
            loop_transitions: vec![stutter],
        };
        debug_assert!(witness.is_well_formed(&self.graph));
        CheckOutcome::Violated {
            explored_states,
            trigger: Some(violating),
            witness,
        }
    }

    /// Checks `eventually goal` on every fair infinite behavior.
    pub fn eventually<P>(&self, goal: P) -> CheckOutcome
    where
        P: Fn(&ModelState) -> bool,
    {
        let allowed: BTreeSet<_> = (0..self.graph.states().len())
            .filter(|&state| !goal(self.graph.state(state)))
            .collect();
        let reachable = reachable_within(&self.graph, self.graph.initial_states(), &allowed);
        let explored_states = reachable.len();

        if let Some(component) = self.first_fair_component(&reachable)
            && let Some(witness) =
                self.build_lasso(self.graph.initial_states(), &reachable, &component)
        {
            return CheckOutcome::Violated {
                explored_states,
                trigger: None,
                witness,
            };
        }

        CheckOutcome::Holds { explored_states }
    }

    /// Checks `always (antecedent implies eventually consequent)` on every fair behavior.
    pub fn leads_to<P, Q>(&self, antecedent: P, consequent: Q) -> CheckOutcome
    where
        P: Fn(&ModelState) -> bool,
        Q: Fn(&ModelState) -> bool,
    {
        let globally_reachable = self.graph.reachable_states();
        let explored_states = globally_reachable.len();
        let not_consequent: BTreeSet<_> = (0..self.graph.states().len())
            .filter(|&state| !consequent(self.graph.state(state)))
            .collect();

        for trigger in globally_reachable.iter().copied().filter(|&state| {
            antecedent(self.graph.state(state)) && !consequent(self.graph.state(state))
        }) {
            let suffix_reachable = reachable_within(&self.graph, &[trigger], &not_consequent);
            let Some(component) = self.first_fair_component(&suffix_reachable) else {
                continue;
            };
            let Some(suffix) = self.build_lasso(&[trigger], &suffix_reachable, &component) else {
                continue;
            };
            let Some((prefix_states, prefix_transitions)) =
                self.graph
                    .shortest_path(self.graph.initial_states(), trigger, &globally_reachable)
            else {
                continue;
            };

            let mut stem_states = prefix_states;
            stem_states.pop();
            stem_states.extend(suffix.stem_states);
            let mut stem_transitions = prefix_transitions;
            stem_transitions.extend(suffix.stem_transitions);
            let witness = Lasso {
                stem_states,
                stem_transitions,
                loop_states: suffix.loop_states,
                loop_transitions: suffix.loop_transitions,
            };
            debug_assert!(witness.is_well_formed(&self.graph));
            return CheckOutcome::Violated {
                explored_states,
                trigger: Some(trigger),
                witness,
            };
        }

        CheckOutcome::Holds { explored_states }
    }

    fn first_fair_component(&self, allowed: &BTreeSet<StateId>) -> Option<BTreeSet<StateId>> {
        let mut candidates = Vec::new();
        for component in strongly_connected_components(&self.graph, allowed) {
            self.refine_for_strong_fairness(component, &mut candidates);
        }
        candidates.sort_by_key(|component| component.first().copied().unwrap_or(usize::MAX));
        candidates.into_iter().next()
    }

    fn refine_for_strong_fairness(
        &self,
        component: BTreeSet<StateId>,
        output: &mut Vec<BTreeSet<StateId>>,
    ) {
        if !is_cyclic(&self.graph, &component) {
            return;
        }

        // If a strongly fair action is enabled somewhere forever but cannot occur in
        // this component, any fair suffix must eventually avoid all states enabling it.
        // Remove those states and recurse into the remaining SCCs.
        for fairness in self.fairness.assumptions() {
            if fairness.kind != FairnessKind::Strong {
                continue;
            }
            let enabled: BTreeSet<_> = component
                .iter()
                .copied()
                .filter(|&state| self.graph.action_enabled(state, &fairness.action))
                .collect();
            if !enabled.is_empty()
                && internal_action_edges(&self.graph, &component, &fairness.action).is_empty()
            {
                let remaining: BTreeSet<_> = component.difference(&enabled).copied().collect();
                for subcomponent in strongly_connected_components(&self.graph, &remaining) {
                    self.refine_for_strong_fairness(subcomponent, output);
                }
                return;
            }
        }

        // Weak fairness requires occurrence only when the action is enabled at every
        // state visited infinitely often. An SCC with a disabling state can construct
        // a fair closed walk through that state.
        for fairness in self.fairness.assumptions() {
            if fairness.kind != FairnessKind::Weak {
                continue;
            }
            let enabled_everywhere = component
                .iter()
                .all(|&state| self.graph.action_enabled(state, &fairness.action));
            if enabled_everywhere
                && internal_action_edges(&self.graph, &component, &fairness.action).is_empty()
            {
                return;
            }
        }

        output.push(component);
    }

    fn build_lasso(
        &self,
        starts: &[StateId],
        stem_allowed: &BTreeSet<StateId>,
        component: &BTreeSet<StateId>,
    ) -> Option<Lasso> {
        let representative = component.first().copied()?;
        let (stem_states, stem_transitions) =
            self.graph
                .shortest_path(starts, representative, stem_allowed)?;
        let (loop_states, loop_transitions) = self.build_fair_loop(component, representative)?;
        let witness = Lasso {
            stem_states,
            stem_transitions,
            loop_states,
            loop_transitions,
        };
        witness.is_well_formed(&self.graph).then_some(witness)
    }

    fn build_fair_loop(
        &self,
        component: &BTreeSet<StateId>,
        representative: StateId,
    ) -> Option<(Vec<StateId>, Vec<TransitionId>)> {
        let mut required_edges = BTreeSet::new();
        let mut required_states = BTreeSet::new();

        for fairness in self.fairness.assumptions() {
            let enabled: Vec<_> = component
                .iter()
                .copied()
                .filter(|&state| self.graph.action_enabled(state, &fairness.action))
                .collect();
            let edges = internal_action_edges(&self.graph, component, &fairness.action);
            match fairness.kind {
                FairnessKind::Strong if !enabled.is_empty() => {
                    required_edges.insert(*edges.first()?);
                }
                FairnessKind::Weak if enabled.len() == component.len() => {
                    required_edges.insert(*edges.first()?);
                }
                FairnessKind::Weak => {
                    if let Some(disabled) = component
                        .iter()
                        .copied()
                        .find(|&state| !self.graph.action_enabled(state, &fairness.action))
                    {
                        required_states.insert(disabled);
                    }
                }
                FairnessKind::Strong => {}
            }
        }

        let mut states = vec![representative];
        let mut transitions = Vec::new();
        let mut current = representative;

        for transition in required_edges {
            let edge = self.graph.transition(transition);
            append_path(
                &self.graph,
                component,
                &mut current,
                edge.from,
                &mut states,
                &mut transitions,
            )?;
            transitions.push(transition);
            states.push(edge.to);
            current = edge.to;
        }

        for state in required_states {
            append_path(
                &self.graph,
                component,
                &mut current,
                state,
                &mut states,
                &mut transitions,
            )?;
        }

        if transitions.is_empty() {
            let transition = component
                .iter()
                .flat_map(|&state| self.graph.outgoing_ids(state).iter().copied())
                .find(|&transition| component.contains(&self.graph.transition(transition).to))?;
            let edge = self.graph.transition(transition);
            append_path(
                &self.graph,
                component,
                &mut current,
                edge.from,
                &mut states,
                &mut transitions,
            )?;
            transitions.push(transition);
            states.push(edge.to);
            current = edge.to;
        }

        append_path(
            &self.graph,
            component,
            &mut current,
            representative,
            &mut states,
            &mut transitions,
        )?;
        Some((states, transitions))
    }
}

fn append_path(
    graph: &FiniteGraph,
    allowed: &BTreeSet<StateId>,
    current: &mut StateId,
    target: StateId,
    states: &mut Vec<StateId>,
    transitions: &mut Vec<TransitionId>,
) -> Option<()> {
    let (path_states, path_transitions) = graph.shortest_path(&[*current], target, allowed)?;
    states.extend(path_states.into_iter().skip(1));
    transitions.extend(path_transitions);
    *current = target;
    Some(())
}

fn reachable_within(
    graph: &FiniteGraph,
    starts: &[StateId],
    allowed: &BTreeSet<StateId>,
) -> BTreeSet<StateId> {
    let mut reached = BTreeSet::new();
    let mut queue = VecDeque::new();
    for &start in starts {
        if allowed.contains(&start) && reached.insert(start) {
            queue.push_back(start);
        }
    }
    while let Some(state) = queue.pop_front() {
        for &transition in graph.outgoing_ids(state) {
            let target = graph.transition(transition).to;
            if allowed.contains(&target) && reached.insert(target) {
                queue.push_back(target);
            }
        }
    }
    reached
}

fn internal_action_edges(
    graph: &FiniteGraph,
    component: &BTreeSet<StateId>,
    action: &str,
) -> Vec<TransitionId> {
    component
        .iter()
        .flat_map(|&state| graph.outgoing_ids(state).iter().copied())
        .filter(|&transition| {
            let edge = graph.transition(transition);
            component.contains(&edge.to) && edge.kind.action() == Some(action)
        })
        .collect()
}

fn is_cyclic(graph: &FiniteGraph, component: &BTreeSet<StateId>) -> bool {
    component.len() > 1
        || component.iter().next().is_some_and(|&state| {
            graph
                .outgoing_ids(state)
                .iter()
                .any(|&transition| graph.transition(transition).to == state)
        })
}

fn strongly_connected_components(
    graph: &FiniteGraph,
    allowed: &BTreeSet<StateId>,
) -> Vec<BTreeSet<StateId>> {
    struct Tarjan<'a> {
        graph: &'a FiniteGraph,
        allowed: &'a BTreeSet<StateId>,
        next_index: usize,
        indices: Vec<Option<usize>>,
        lowlink: Vec<usize>,
        stack: Vec<StateId>,
        on_stack: Vec<bool>,
        output: Vec<BTreeSet<StateId>>,
    }

    impl Tarjan<'_> {
        fn visit(&mut self, state: StateId) {
            let index = self.next_index;
            self.next_index += 1;
            self.indices[state] = Some(index);
            self.lowlink[state] = index;
            self.stack.push(state);
            self.on_stack[state] = true;

            for &transition in self.graph.outgoing_ids(state) {
                let target = self.graph.transition(transition).to;
                if !self.allowed.contains(&target) {
                    continue;
                }
                if self.indices[target].is_none() {
                    self.visit(target);
                    self.lowlink[state] = self.lowlink[state].min(self.lowlink[target]);
                } else if self.on_stack[target] {
                    self.lowlink[state] = self.lowlink[state]
                        .min(self.indices[target].expect("visited target has an index"));
                }
            }

            if self.lowlink[state] == index {
                let mut component = BTreeSet::new();
                loop {
                    let member = self.stack.pop().expect("root state remains on the stack");
                    self.on_stack[member] = false;
                    component.insert(member);
                    if member == state {
                        break;
                    }
                }
                self.output.push(component);
            }
        }
    }

    let count = graph.states().len();
    let mut tarjan = Tarjan {
        graph,
        allowed,
        next_index: 0,
        indices: vec![None; count],
        lowlink: vec![0; count],
        stack: Vec::new(),
        on_stack: vec![false; count],
        output: Vec::new(),
    };
    for &state in allowed {
        if tarjan.indices[state].is_none() {
            tarjan.visit(state);
        }
    }
    tarjan
        .output
        .sort_by_key(|component| component.first().copied().unwrap_or(usize::MAX));
    tarjan.output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ModelState, Transition, Value};
    use std::collections::BTreeMap;

    fn state(done: bool, trigger: bool) -> ModelState {
        BTreeMap::from([
            ("done".to_owned(), Value::Bool(done)),
            ("trigger".to_owned(), Value::Bool(trigger)),
        ])
    }

    fn is_done(state: &ModelState) -> bool {
        state.get("done") == Some(&Value::Bool(true))
    }

    #[test]
    fn eventuality_returns_a_deterministic_lasso() {
        let graph = FiniteGraph::new(
            vec![state(false, false), state(false, false)],
            vec![0],
            vec![
                Transition::action(0, "enter", 1),
                Transition::action(1, "idle", 1),
            ],
        )
        .unwrap();
        let checker = TemporalChecker::new(&graph, FairnessSet::new(vec![Fairness::weak("enter")]));
        let CheckOutcome::Violated { witness, .. } = checker.eventually(is_done) else {
            panic!("expected an eventuality counterexample")
        };

        assert!(witness.is_well_formed(checker.graph()));
        assert_eq!(witness.stem_states, vec![0, 1]);
        assert_eq!(witness.loop_states, vec![1, 1]);
    }

    #[test]
    fn always_returns_an_initial_zero_step_stutter_lasso() {
        let graph = FiniteGraph::new(
            vec![state(false, true)],
            vec![0],
            vec![Transition::action(0, "retry", 0)],
        )
        .unwrap();
        let checker = TemporalChecker::new(&graph, FairnessSet::default());
        let CheckOutcome::Violated {
            trigger, witness, ..
        } = checker.always(|state| state.get("trigger") == Some(&Value::Bool(false)))
        else {
            panic!("initial safety violation must produce a temporal lasso")
        };
        assert_eq!(trigger, Some(0));
        assert_eq!(witness.stem_states, vec![0]);
        assert!(witness.stem_transitions.is_empty());
        assert_eq!(witness.loop_states, vec![0, 0]);
        assert_eq!(witness.loop_actions(checker.graph()), vec![None]);
        assert!(witness.is_well_formed(checker.graph()));
    }

    #[test]
    fn weak_fairness_excludes_a_continuously_enabled_idle_loop() {
        let graph = FiniteGraph::new(
            vec![state(false, false), state(true, false)],
            vec![0],
            vec![
                Transition::action(0, "idle", 0),
                Transition::action(0, "work", 1),
            ],
        )
        .unwrap();

        assert!(
            !TemporalChecker::new(&graph, FairnessSet::default())
                .eventually(is_done)
                .holds()
        );
        assert!(
            TemporalChecker::new(&graph, FairnessSet::new(vec![Fairness::weak("work")]))
                .eventually(is_done)
                .holds()
        );
    }

    #[test]
    fn strong_and_weak_fairness_are_distinct() {
        let graph = FiniteGraph::new(
            vec![state(false, false), state(false, false), state(true, false)],
            vec![0],
            vec![
                Transition::action(0, "tick", 1),
                Transition::action(1, "tick", 0),
                Transition::action(0, "work", 2),
            ],
        )
        .unwrap();

        let weak = TemporalChecker::new(
            &graph,
            FairnessSet::new(vec![Fairness::weak("tick"), Fairness::weak("work")]),
        );
        let strong = TemporalChecker::new(
            &graph,
            FairnessSet::new(vec![Fairness::weak("tick"), Fairness::strong("work")]),
        );
        assert!(!weak.eventually(is_done).holds());
        assert!(strong.eventually(is_done).holds());
    }

    #[test]
    fn fair_occurrence_can_still_witness_a_violation() {
        let graph = FiniteGraph::new(
            vec![state(false, false)],
            vec![0],
            vec![Transition::action(0, "work", 0)],
        )
        .unwrap();
        let checker =
            TemporalChecker::new(&graph, FairnessSet::new(vec![Fairness::strong("work")]));
        let CheckOutcome::Violated { witness, .. } = checker.eventually(is_done) else {
            panic!("the fair work loop never reaches the goal")
        };
        assert_eq!(witness.loop_actions(checker.graph()), vec![Some("work")]);
    }

    #[test]
    fn leads_to_reports_the_trigger_and_avoiding_suffix() {
        let graph = FiniteGraph::new(
            vec![state(false, false), state(false, true), state(false, true)],
            vec![0],
            vec![
                Transition::action(0, "request", 1),
                Transition::action(1, "lose", 2),
                Transition::action(2, "idle", 2),
            ],
        )
        .unwrap();
        let checker = TemporalChecker::new(&graph, FairnessSet::new(vec![Fairness::weak("lose")]));
        let outcome = checker.leads_to(
            |state| state.get("trigger") == Some(&Value::Bool(true)),
            is_done,
        );
        let CheckOutcome::Violated {
            trigger, witness, ..
        } = outcome
        else {
            panic!("expected leads-to violation")
        };
        assert_eq!(trigger, Some(1));
        assert_eq!(witness.stem_states, vec![0, 1, 2]);
        assert!(witness.is_well_formed(checker.graph()));
    }

    #[test]
    fn progress_requires_fairness_when_identity_stutter_is_available() {
        let graph = FiniteGraph::new(
            vec![state(false, false), state(true, false)],
            vec![0],
            vec![Transition::action(0, "finish", 1)],
        )
        .unwrap();
        assert!(
            !TemporalChecker::new(&graph, FairnessSet::default())
                .eventually(is_done)
                .holds()
        );
        assert!(
            TemporalChecker::new(&graph, FairnessSet::new(vec![Fairness::weak("finish")]),)
                .eventually(is_done)
                .holds()
        );
    }
}

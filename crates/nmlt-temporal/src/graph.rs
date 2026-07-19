use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

pub type StateId = usize;
pub type TransitionId = usize;

/// A deliberately small, total value domain shared by model states and journals.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Text(String),
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

pub type ModelState = BTreeMap<String, Value>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransitionKind {
    /// A declared action, including an internal or refinement-hidden action.
    Action(String),
    /// The distinguished temporal closure step. It must be a self-loop and has no action.
    IdentityStutter,
}

impl TransitionKind {
    pub fn action(&self) -> Option<&str> {
        match self {
            Self::Action(action) => Some(action),
            Self::IdentityStutter => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Transition {
    pub from: StateId,
    pub to: StateId,
    pub kind: TransitionKind,
}

impl Transition {
    pub fn action(from: StateId, action: impl Into<String>, to: StateId) -> Self {
        Self {
            from,
            to,
            kind: TransitionKind::Action(action.into()),
        }
    }

    pub fn identity_stutter(state: StateId) -> Self {
        Self {
            from: state,
            to: state,
            kind: TransitionKind::IdentityStutter,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphError {
    EmptyStateSpace,
    NoInitialStates,
    StateOutOfRange {
        role: &'static str,
        state: StateId,
        state_count: usize,
    },
    InvalidIdentityStutter {
        from: StateId,
        to: StateId,
    },
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyStateSpace => write!(f, "a finite graph must contain at least one state"),
            Self::NoInitialStates => write!(f, "a finite graph must contain an initial state"),
            Self::StateOutOfRange {
                role,
                state,
                state_count,
            } => write!(
                f,
                "{role} state {state} is outside the state range 0..{state_count}"
            ),
            Self::InvalidIdentityStutter { from, to } => write!(
                f,
                "identity stutter must be a self-loop, but transition is {from} -> {to}"
            ),
        }
    }
}

impl std::error::Error for GraphError {}

/// A canonical finite graph.
///
/// Initial states and transitions are sorted and deduplicated on construction, so all
/// traversals and witnesses are reproducible for the same logical input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiniteGraph {
    states: Vec<ModelState>,
    initial: Vec<StateId>,
    transitions: Vec<Transition>,
    outgoing: Vec<Vec<TransitionId>>,
}

impl FiniteGraph {
    pub fn new(
        states: Vec<ModelState>,
        mut initial: Vec<StateId>,
        mut transitions: Vec<Transition>,
    ) -> Result<Self, GraphError> {
        if states.is_empty() {
            return Err(GraphError::EmptyStateSpace);
        }
        if initial.is_empty() {
            return Err(GraphError::NoInitialStates);
        }
        let state_count = states.len();
        for &state in &initial {
            if state >= state_count {
                return Err(GraphError::StateOutOfRange {
                    role: "initial",
                    state,
                    state_count,
                });
            }
        }
        for transition in &transitions {
            if transition.from >= state_count {
                return Err(GraphError::StateOutOfRange {
                    role: "transition source",
                    state: transition.from,
                    state_count,
                });
            }
            if transition.to >= state_count {
                return Err(GraphError::StateOutOfRange {
                    role: "transition target",
                    state: transition.to,
                    state_count,
                });
            }
            if matches!(transition.kind, TransitionKind::IdentityStutter)
                && transition.from != transition.to
            {
                return Err(GraphError::InvalidIdentityStutter {
                    from: transition.from,
                    to: transition.to,
                });
            }
        }

        initial.sort_unstable();
        initial.dedup();
        transitions.sort();
        transitions.dedup();

        let mut outgoing = vec![Vec::new(); state_count];
        for (index, transition) in transitions.iter().enumerate() {
            outgoing[transition.from].push(index);
        }

        Ok(Self {
            states,
            initial,
            transitions,
            outgoing,
        })
    }

    pub fn states(&self) -> &[ModelState] {
        &self.states
    }

    pub fn state(&self, state: StateId) -> &ModelState {
        &self.states[state]
    }

    pub fn initial_states(&self) -> &[StateId] {
        &self.initial
    }

    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    pub fn transition(&self, transition: TransitionId) -> &Transition {
        &self.transitions[transition]
    }

    pub fn outgoing_ids(&self, state: StateId) -> &[TransitionId] {
        &self.outgoing[state]
    }

    pub fn action_enabled(&self, state: StateId, action: &str) -> bool {
        self.outgoing_ids(state)
            .iter()
            .any(|&transition| self.transition(transition).kind.action() == Some(action))
    }

    /// Closes every state under the distinguished action-free identity stutter.
    pub fn stutter_closed(&self) -> Self {
        let mut transitions = self.transitions.clone();
        for state in 0..self.states.len() {
            transitions.push(Transition::identity_stutter(state));
        }
        Self::new(self.states.clone(), self.initial.clone(), transitions)
            .expect("stutter closure preserves graph invariants")
    }

    pub fn reachable_states(&self) -> BTreeSet<StateId> {
        let mut reached = BTreeSet::new();
        let mut queue = VecDeque::new();
        for &initial in &self.initial {
            if reached.insert(initial) {
                queue.push_back(initial);
            }
        }
        while let Some(state) = queue.pop_front() {
            for &transition in self.outgoing_ids(state) {
                let target = self.transition(transition).to;
                if reached.insert(target) {
                    queue.push_back(target);
                }
            }
        }
        reached
    }

    pub(crate) fn shortest_path(
        &self,
        starts: &[StateId],
        target: StateId,
        allowed: &BTreeSet<StateId>,
    ) -> Option<(Vec<StateId>, Vec<TransitionId>)> {
        let mut parent: BTreeMap<StateId, (StateId, TransitionId)> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::new();
        for &start in starts {
            if allowed.contains(&start) && seen.insert(start) {
                queue.push_back(start);
            }
        }

        while let Some(state) = queue.pop_front() {
            if state == target {
                let mut states = vec![target];
                let mut transitions = Vec::new();
                let mut cursor = target;
                while let Some(&(previous, transition)) = parent.get(&cursor) {
                    states.push(previous);
                    transitions.push(transition);
                    cursor = previous;
                }
                states.reverse();
                transitions.reverse();
                return Some((states, transitions));
            }
            for &transition in self.outgoing_ids(state) {
                let next = self.transition(transition).to;
                if allowed.contains(&next) && seen.insert(next) {
                    parent.insert(next, (state, transition));
                    queue.push_back(next);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_state() -> ModelState {
        BTreeMap::new()
    }

    #[test]
    fn canonicalizes_transitions_and_initial_states() {
        let graph = FiniteGraph::new(
            vec![empty_state(), empty_state()],
            vec![1, 0, 1],
            vec![
                Transition::action(1, "z", 0),
                Transition::action(0, "a", 1),
                Transition::action(0, "a", 1),
            ],
        )
        .unwrap();

        assert_eq!(graph.initial_states(), &[0, 1]);
        assert_eq!(graph.transitions().len(), 2);
        assert_eq!(graph.transition(0), &Transition::action(0, "a", 1));
    }

    #[test]
    fn stutter_closure_adds_identity_at_every_state() {
        let graph = FiniteGraph::new(
            vec![empty_state(), empty_state()],
            vec![0],
            vec![Transition::action(0, "go", 1)],
        )
        .unwrap()
        .stutter_closed();

        assert_eq!(graph.transitions().len(), 3);
        assert!(
            graph
                .transitions()
                .contains(&Transition::identity_stutter(0))
        );
        assert!(
            graph
                .transitions()
                .contains(&Transition::identity_stutter(1))
        );
    }

    #[test]
    fn rejects_non_identity_stutter() {
        let error = FiniteGraph::new(
            vec![empty_state(), empty_state()],
            vec![0],
            vec![Transition {
                from: 0,
                to: 1,
                kind: TransitionKind::IdentityStutter,
            }],
        )
        .unwrap_err();

        assert!(matches!(error, GraphError::InvalidIdentityStutter { .. }));
    }
}

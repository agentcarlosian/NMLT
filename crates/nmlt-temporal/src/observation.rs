use crate::graph::{ModelState, StateId};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationError {
    DuplicateOutputField(String),
    MissingSourceField(String),
    StateOutOfRange(StateId),
}

impl fmt::Display for ObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateOutputField(field) => {
                write!(f, "observation field {field:?} has more than one source")
            }
            Self::MissingSourceField(field) => {
                write!(f, "state does not contain observed source field {field:?}")
            }
            Self::StateOutOfRange(state) => write!(f, "trace refers to missing state {state}"),
        }
    }
}

impl std::error::Error for ObservationError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionProjectionError {
    pub action_index: usize,
    pub action: String,
}

impl fmt::Display for ActionProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "action {} at trace position {} has no hiding/renaming mapping",
            self.action, self.action_index
        )
    }
}

impl std::error::Error for ActionProjectionError {}

/// A total renaming projection from model-state fields to public observation fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationMap {
    fields: BTreeMap<String, String>,
}

impl ObservationMap {
    pub fn new<I, S, T>(fields: I) -> Result<Self, ObservationError>
    where
        I: IntoIterator<Item = (S, T)>,
        S: Into<String>,
        T: Into<String>,
    {
        let fields: BTreeMap<String, String> = fields
            .into_iter()
            .map(|(source, output)| (source.into(), output.into()))
            .collect();
        let mut outputs = BTreeSet::new();
        for output in fields.values() {
            if !outputs.insert(output.clone()) {
                return Err(ObservationError::DuplicateOutputField(output.clone()));
            }
        }
        Ok(Self { fields })
    }

    pub fn identity<'a, I>(fields: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        Self::new(fields.into_iter().map(|field| (field, field)))
            .expect("an identity observation map has unique outputs")
    }

    pub fn fields(&self) -> &BTreeMap<String, String> {
        &self.fields
    }

    pub fn observe(&self, state: &ModelState) -> Result<ModelState, ObservationError> {
        self.fields
            .iter()
            .map(|(source, output)| {
                state
                    .get(source)
                    .cloned()
                    .map(|value| (output.clone(), value))
                    .ok_or_else(|| ObservationError::MissingSourceField(source.clone()))
            })
            .collect()
    }

    pub fn project_trace(
        &self,
        states: &[ModelState],
        trace: &[StateId],
    ) -> Result<Vec<ModelState>, ObservationError> {
        trace
            .iter()
            .map(|&state| {
                states
                    .get(state)
                    .ok_or(ObservationError::StateOutOfRange(state))
                    .and_then(|state| self.observe(state))
            })
            .collect()
    }
}

/// Maps concrete action names either to an abstract action or to a hidden step.
/// Absence from the map means "not specified", not "hidden".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionHiding {
    actions: BTreeMap<String, Option<String>>,
}

impl ActionHiding {
    pub fn new<I, S, T>(actions: I) -> Self
    where
        I: IntoIterator<Item = (S, Option<T>)>,
        S: Into<String>,
        T: Into<String>,
    {
        Self {
            actions: actions
                .into_iter()
                .map(|(concrete, abstract_action)| {
                    (concrete.into(), abstract_action.map(Into::into))
                })
                .collect(),
        }
    }

    pub fn get(&self, concrete_action: &str) -> Option<Option<&str>> {
        self.actions
            .get(concrete_action)
            .map(|mapped| mapped.as_deref())
    }

    pub fn mappings(&self) -> &BTreeMap<String, Option<String>> {
        &self.actions
    }

    /// Drops explicitly hidden actions and renames visible actions. Unmapped actions
    /// are errors so trace projection cannot silently hide a new or misspelled action.
    pub fn project_visible<'a, I>(&self, actions: I) -> Result<Vec<String>, ActionProjectionError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut projected = Vec::new();
        for (action_index, action) in actions.into_iter().enumerate() {
            match self.get(action) {
                Some(Some(visible)) => projected.push(visible.to_owned()),
                Some(None) => {}
                None => {
                    return Err(ActionProjectionError {
                        action_index,
                        action: action.to_owned(),
                    });
                }
            }
        }
        Ok(projected)
    }
}

/// Removes only adjacent duplicate observations. It does not erase action, event,
/// capability, or fairness metadata from an intensional trace.
pub fn stutter_project<T: Clone + Eq>(word: &[T]) -> Vec<T> {
    let mut projected = Vec::new();
    for item in word {
        if projected.last() != Some(item) {
            projected.push(item.clone());
        }
    }
    projected
}

/// Finite-prefix stutter equivalence, suitable for deterministic trace comparison.
/// Infinite temporal transport still needs the divergence obligations documented by
/// RFC 0009.
pub fn stutter_equivalent<T: Clone + Eq>(left: &[T], right: &[T]) -> bool {
    stutter_project(left) == stutter_project(right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Value;

    fn state(public: bool, secret: i64) -> ModelState {
        BTreeMap::from([
            ("public".to_owned(), Value::Bool(public)),
            ("secret".to_owned(), Value::Int(secret)),
        ])
    }

    #[test]
    fn observation_map_hides_and_renames_fields() {
        let map = ObservationMap::new([("public", "ready")]).unwrap();
        assert_eq!(
            map.observe(&state(true, 99)).unwrap(),
            BTreeMap::from([("ready".to_owned(), Value::Bool(true))])
        );
    }

    #[test]
    fn stutter_projection_collapses_only_adjacent_equal_observations() {
        let word = [false, false, true, true, false];
        assert_eq!(stutter_project(&word), vec![false, true, false]);
        assert!(stutter_equivalent(&word, &[false, true, false, false]));
    }

    #[test]
    fn hidden_is_distinct_from_unmapped() {
        let map = ActionHiding::new([("cache", None::<&str>), ("send", Some("deliver"))]);
        assert_eq!(map.get("cache"), Some(None));
        assert_eq!(map.get("send"), Some(Some("deliver")));
        assert_eq!(map.get("typo"), None);
        assert_eq!(
            map.project_visible(["cache", "send"]).unwrap(),
            vec!["deliver"]
        );
        assert_eq!(
            map.project_visible(["typo"]).unwrap_err(),
            ActionProjectionError {
                action_index: 0,
                action: "typo".to_owned(),
            }
        );
    }
}

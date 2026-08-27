//! Adapter from `nmlt-core`'s Paper 1 boolean sketch to `nmlt-temporal`
//! [`FiniteGraph`] / [`OpenSystem`].
//!
//! This crate depends on both `nmlt-core` and `nmlt-temporal` so those crates
//! stay independent (`nmlt-temporal` must not depend on `nmlt-core`). It is a
//! sketch fragment plus a finite instance check: not a verified compiler, not
//! source-to-LTS in general, and not an M9 elaborator.

use std::fmt;

use nmlt_core::{
    BooleanSketch, BooleanSketchError, SurfacePolarity, UntypedSystem, declared_polarity,
    sketch_boolean_system,
};
use nmlt_temporal::{
    ActionPolarity, ActionSignature, FiniteContract, FiniteContractError, FiniteGraph, GraphError,
    InterfaceBuildError, ModelState, OpenSystem, OpenSystemIssue, PayloadPredicate, PayloadType,
    Transition, Value,
};

/// Channel used for Paper 1 boundary actions. Matches the existing
/// `HiddenConnectedAction` fixtures in `nmlt-temporal`; not a surface channel.
pub const PAPER1_CHANNEL: &str = "bus";

/// Fail-closed adapter error. Not a compiler diagnostic class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SketchAdapterError {
    Sketch(BooleanSketchError),
    Graph(GraphError),
    Interface(InterfaceBuildError),
    Contract(FiniteContractError),
    Open(Vec<OpenSystemIssue>),
}

impl fmt::Display for SketchAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sketch(error) => write!(formatter, "{error}"),
            Self::Graph(error) => write!(formatter, "{error}"),
            Self::Interface(error) => write!(formatter, "{error}"),
            Self::Contract(error) => write!(formatter, "{error}"),
            Self::Open(issues) => {
                write!(formatter, "open-system adapter rejected sketch")?;
                for issue in issues {
                    write!(formatter, "; {issue}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for SketchAdapterError {}

impl From<BooleanSketchError> for SketchAdapterError {
    fn from(error: BooleanSketchError) -> Self {
        Self::Sketch(error)
    }
}

impl From<GraphError> for SketchAdapterError {
    fn from(error: GraphError) -> Self {
        Self::Graph(error)
    }
}

impl From<InterfaceBuildError> for SketchAdapterError {
    fn from(error: InterfaceBuildError) -> Self {
        Self::Interface(error)
    }
}

impl From<FiniteContractError> for SketchAdapterError {
    fn from(error: FiniteContractError) -> Self {
        Self::Contract(error)
    }
}

/// Polarities: surface if present, else Paper 1 defaults (`ping` Output,
/// `receive` Input). Anything else is Internal (total interface, not inferred
/// from action bodies).
#[must_use]
pub fn paper1_action_polarity(action: &str, surface: Option<SurfacePolarity>) -> ActionPolarity {
    match surface {
        Some(SurfacePolarity::Input) => ActionPolarity::Input,
        Some(SurfacePolarity::Output) => ActionPolarity::Output,
        None => match action {
            "ping" => ActionPolarity::Output,
            "receive" => ActionPolarity::Input,
            _ => ActionPolarity::Internal,
        },
    }
}

/// Bool assignments become [`Value::Bool`]; action names are unchanged.
pub fn finite_graph_from_boolean_sketch(sketch: &BooleanSketch) -> Result<FiniteGraph, GraphError> {
    let states = sketch
        .states
        .iter()
        .map(|assignment| {
            assignment
                .iter()
                .map(|(field, value)| (field.clone(), Value::Bool(*value)))
                .collect::<ModelState>()
        })
        .collect();
    let transitions = sketch
        .transitions
        .iter()
        .map(|transition| {
            Transition::action(transition.from, transition.action.clone(), transition.to)
        })
        .collect();
    FiniteGraph::new(states, vec![sketch.initial], transitions)
}

/// [`BooleanSketch`] → [`OpenSystem`]: bool values, action names, polarities
/// from `surface_polarity` if present else Paper 1 defaults.
///
/// Boundary actions get unit payload on [`PAPER1_CHANNEL`]. Hidden names stay
/// on the sketch / [`nmlt_core::BooleanSketch::hidden_actions`]; hiding is
/// applied later via `ActionHiding::from_hide_actions`. Not source-to-LTS.
pub fn open_system_from_boolean_sketch(
    sketch: &BooleanSketch,
    mut surface_polarity: impl FnMut(&str) -> Option<SurfacePolarity>,
) -> Result<OpenSystem, SketchAdapterError> {
    let graph = finite_graph_from_boolean_sketch(sketch)?;
    let unit = PayloadType::unit();
    let mut actions = Vec::new();
    let mut assumptions = Vec::new();
    let mut guarantees = Vec::new();
    for name in &sketch.action_names {
        let polarity = paper1_action_polarity(name, surface_polarity(name));
        match polarity {
            ActionPolarity::Input => {
                actions.push((
                    name.clone(),
                    ActionSignature::input(PAPER1_CHANNEL, unit.clone()),
                ));
                assumptions.push((name.clone(), PayloadPredicate::all(&unit)));
            }
            ActionPolarity::Output => {
                actions.push((
                    name.clone(),
                    ActionSignature::output(PAPER1_CHANNEL, unit.clone()),
                ));
                guarantees.push((name.clone(), PayloadPredicate::all(&unit)));
            }
            ActionPolarity::Internal => {
                actions.push((name.clone(), ActionSignature::internal()));
            }
        }
    }
    let interface = nmlt_temporal::OpenInterface::new(actions)?;
    let contract = FiniteContract::new(assumptions, guarantees)?;
    OpenSystem::new(graph, interface, contract).map_err(SketchAdapterError::Open)
}

/// Sketch one untyped system and adapt it, using [`declared_polarity`] then
/// Paper 1 defaults.
pub fn open_system_from_untyped(
    system: &UntypedSystem,
) -> Result<(BooleanSketch, OpenSystem), SketchAdapterError> {
    let sketch = sketch_boolean_system(system)?;
    let open =
        open_system_from_boolean_sketch(&sketch, |action| declared_polarity(system, action))?;
    Ok((sketch, open))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nmlt_core::{parse_cst, project_untyped};

    fn sketch_source(source: &str) -> (BooleanSketch, OpenSystem) {
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
        open_system_from_untyped(system).expect("adapter")
    }

    #[test]
    fn bool_values_and_action_names_survive_graph_adapter() {
        let (sketch, open) = sketch_source(concat!(
            "system ConcreteSender {\n",
            "  state unit: Bool = false\n",
            "  action output ping { set unit = unit }\n",
            "  observe unit\n",
            "  hide action ping\n",
            "}\n",
        ));
        assert_eq!(sketch.action_names, ["ping"]);
        assert_eq!(sketch.hidden_actions, ["ping"]);
        let graph = open.graph();
        assert_eq!(graph.states().len(), 1);
        assert_eq!(graph.states()[0].get("unit"), Some(&Value::Bool(false)));
        assert_eq!(graph.transitions()[0].kind.action(), Some("ping"));
        assert_eq!(
            open.interface()
                .get("ping")
                .map(|signature| signature.polarity),
            Some(ActionPolarity::Output)
        );
    }

    #[test]
    fn paper1_defaults_apply_when_surface_has_no_polarity() {
        let ping = paper1_action_polarity("ping", None);
        let receive = paper1_action_polarity("receive", None);
        assert_eq!(ping, ActionPolarity::Output);
        assert_eq!(receive, ActionPolarity::Input);
        assert_eq!(
            paper1_action_polarity("other", None),
            ActionPolarity::Internal
        );

        let (_, open) = sketch_source(concat!(
            "system BarePing {\n",
            "  state unit: Bool = false\n",
            "  action ping { set unit = unit }\n",
            "  observe unit\n",
            "}\n",
        ));
        assert_eq!(
            open.interface()
                .get("ping")
                .map(|signature| signature.polarity),
            Some(ActionPolarity::Output)
        );
    }
}

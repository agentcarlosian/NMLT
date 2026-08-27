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
    ActionHiding, ActionPolarity, ActionSignature, FiniteContract, FiniteContractError,
    FiniteGraph, GraphError, InterfaceBuildError, ModelState, ObservationMap, OpenSystem,
    OpenSystemIssue, PayloadPredicate, PayloadType, RefinementSpec, Transition, Value,
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
    /// Left and right sketch states share a field name; the one-wire product
    /// merges fields without namespacing.
    ProductFieldCollision(String),
    ProductStateOverflow,
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
            Self::ProductFieldCollision(field) => {
                write!(
                    formatter,
                    "Paper 1 sync product collides on state field {field}"
                )
            }
            Self::ProductStateOverflow => {
                write!(formatter, "Paper 1 sync product state index overflow")
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

/// [`ObservationMap::identity`] on the sketch's surface `observe` fields.
#[must_use]
pub fn observation_map_from_sketch(sketch: &BooleanSketch) -> ObservationMap {
    ObservationMap::identity(sketch.observed_fields.iter().map(String::as_str))
}

/// Concatenate left and right surface `observe` fields as an identity map.
///
/// Paper 1 senders observe `unit` and the receiver observes `bit`; those names
/// do not collide, so the one-wire product can keep them un-namespaced.
#[must_use]
pub fn product_observation_map(left: &BooleanSketch, right: &BooleanSketch) -> ObservationMap {
    let fields = left
        .observed_fields
        .iter()
        .chain(right.observed_fields.iter())
        .map(String::as_str);
    ObservationMap::identity(fields)
}

/// Identity-state local refinement using sketch `observe` fields and a given
/// action map (`ping` → `ping` for VisibleSync; hide `ping` for InvalidHiddenPing).
#[must_use]
pub fn local_refinement_spec(
    concrete: &OpenSystem,
    concrete_sketch: &BooleanSketch,
    abstract_sketch: &BooleanSketch,
    actions: ActionHiding,
) -> RefinementSpec {
    RefinementSpec {
        state_map: (0..concrete.graph().states().len()).collect(),
        concrete_observation: observation_map_from_sketch(concrete_sketch),
        abstract_observation: observation_map_from_sketch(abstract_sketch),
        actions,
    }
}

fn product_index(
    left_state: usize,
    right_state: usize,
    right_count: usize,
) -> Result<usize, SketchAdapterError> {
    left_state
        .checked_mul(right_count)
        .and_then(|base| base.checked_add(right_state))
        .ok_or(SketchAdapterError::ProductStateOverflow)
}

fn merge_product_state(
    left: &ModelState,
    right: &ModelState,
) -> Result<ModelState, SketchAdapterError> {
    let mut merged = left.clone();
    for (field, value) in right {
        if merged.contains_key(field) {
            return Err(SketchAdapterError::ProductFieldCollision(field.clone()));
        }
        merged.insert(field.clone(), value.clone());
    }
    Ok(merged)
}

/// One-wire cartesian product of sketched graphs. Not `nmlt-temporal` `compose`:
/// no receptiveness, contracts, or interfaces. Mirrors Lean `parallel` for the
/// Paper 1 VisibleSync finite instance (`ping` || `receive` → sync). Connected
/// actions fire only together; unconnected actions stay independent.
pub fn paper1_sync_product_graph(
    left: &FiniteGraph,
    right: &FiniteGraph,
    left_action: &str,
    right_action: &str,
    sync: &str,
) -> Result<FiniteGraph, SketchAdapterError> {
    let left_count = left.states().len();
    let right_count = right.states().len();
    let mut states = Vec::with_capacity(
        left_count
            .checked_mul(right_count)
            .ok_or(SketchAdapterError::ProductStateOverflow)?,
    );
    for left_state in left.states() {
        for right_state in right.states() {
            states.push(merge_product_state(left_state, right_state)?);
        }
    }
    let mut initial = Vec::new();
    for &left_state in left.initial_states() {
        for &right_state in right.initial_states() {
            initial.push(product_index(left_state, right_state, right_count)?);
        }
    }
    let mut transitions = Vec::new();
    for left_from in 0..left_count {
        for right_from in 0..right_count {
            let from = product_index(left_from, right_from, right_count)?;
            for transition in left.transitions() {
                if transition.from != left_from {
                    continue;
                }
                let Some(action) = transition.kind.action() else {
                    continue;
                };
                if action == left_action {
                    continue;
                }
                let to = product_index(transition.to, right_from, right_count)?;
                transitions.push(Transition::action(from, action, to));
            }
            for transition in right.transitions() {
                if transition.from != right_from {
                    continue;
                }
                let Some(action) = transition.kind.action() else {
                    continue;
                };
                if action == right_action {
                    continue;
                }
                let to = product_index(left_from, transition.to, right_count)?;
                transitions.push(Transition::action(from, action, to));
            }
            for left_transition in left.transitions() {
                if left_transition.from != left_from
                    || left_transition.kind.action() != Some(left_action)
                {
                    continue;
                }
                for right_transition in right.transitions() {
                    if right_transition.from != right_from
                        || right_transition.kind.action() != Some(right_action)
                    {
                        continue;
                    }
                    let to = product_index(left_transition.to, right_transition.to, right_count)?;
                    transitions.push(Transition::action(from, sync, to));
                }
            }
        }
    }
    Ok(FiniteGraph::new(states, initial, transitions)?)
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

    #[test]
    fn observation_map_uses_surface_observe_fields() {
        let (sketch, _) = sketch_source(concat!(
            "system VisibleAbstractSender {\n",
            "  state unit: Bool = false\n",
            "  action output ping { set unit = unit }\n",
            "  observe unit\n",
            "}\n",
        ));
        assert_eq!(sketch.observed_fields, ["unit"]);
        assert_eq!(
            observation_map_from_sketch(&sketch),
            ObservationMap::identity(["unit"])
        );
    }

    #[test]
    fn one_wire_product_merges_unit_and_bit_and_syncs() {
        let (left_sketch, left) = sketch_source(concat!(
            "system VisibleAbstractSender {\n",
            "  state unit: Bool = false\n",
            "  action output ping { set unit = unit }\n",
            "  observe unit\n",
            "}\n",
        ));
        let (right_sketch, right) = sketch_source(concat!(
            "system Receiver {\n",
            "  state bit: Bool = false\n",
            "  action input receive {\n",
            "    require bit == false\n",
            "    set bit = true\n",
            "  }\n",
            "  observe bit\n",
            "}\n",
        ));
        let product = paper1_sync_product_graph(
            left.graph(),
            right.graph(),
            "ping",
            "receive",
            "VisibleSync",
        )
        .expect("product");
        assert_eq!(product.states().len(), 2);
        assert_eq!(product.states()[0].get("unit"), Some(&Value::Bool(false)));
        assert_eq!(product.states()[0].get("bit"), Some(&Value::Bool(false)));
        assert_eq!(product.states()[1].get("bit"), Some(&Value::Bool(true)));
        assert_eq!(product.transitions().len(), 1);
        assert_eq!(product.transitions()[0].from, 0);
        assert_eq!(product.transitions()[0].to, 1);
        assert_eq!(product.transitions()[0].kind.action(), Some("VisibleSync"));
        assert_eq!(
            product_observation_map(&left_sketch, &right_sketch),
            ObservationMap::identity(["unit", "bit"])
        );
    }
}

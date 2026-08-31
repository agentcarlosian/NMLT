//! The inspectable, finite behavioral core shared by the language frontend,
//! the reference evaluator, and Lean's normative artifact checker.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::{Deserialize, Deserializer};

pub const BEHAVIOR_CORE_SCHEMA: &str = "behavior-core-v1";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum CorePortDirection {
    Input,
    Output,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CoreBehaviorTerm {
    Bool {
        r#type: String,
        value: bool,
    },
    Unit {
        r#type: String,
    },
    Enum {
        r#type: String,
        constructor: String,
    },
    Read {
        r#type: String,
        field: String,
    },
    Not {
        r#type: String,
        value: Box<CoreBehaviorTerm>,
    },
    Equal {
        r#type: String,
        left: Box<CoreBehaviorTerm>,
        right: Box<CoreBehaviorTerm>,
    },
}

impl CoreBehaviorTerm {
    #[must_use]
    pub fn ty(&self) -> &str {
        match self {
            Self::Bool { r#type, .. }
            | Self::Unit { r#type }
            | Self::Enum { r#type, .. }
            | Self::Read { r#type, .. }
            | Self::Not { r#type, .. }
            | Self::Equal { r#type, .. } => r#type,
        }
    }
}

impl CorePortDirection {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
        }
    }

    #[must_use]
    pub const fn complements(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Input, Self::Output) | (Self::Output, Self::Input)
        )
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CoreBehaviorState {
    #[serde(default, skip)]
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(rename = "initial")]
    pub initializer: String,
    pub initial_ast: CoreBehaviorTerm,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CorePort {
    #[serde(default, skip)]
    pub name: String,
    pub direction: CorePortDirection,
    #[serde(rename = "payload")]
    pub payload_type: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CoreBehaviorBinding {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

/// Resources are part of an action's meaning. Hiding never removes this data.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct CoreResourceProfile {
    pub requires: BTreeSet<String>,
    pub consumes: BTreeSet<String>,
    pub transfers: BTreeSet<String>,
    pub receives: BTreeSet<String>,
    pub grade: BTreeMap<String, u64>,
    pub relies: BTreeSet<String>,
    pub guarantees: BTreeSet<String>,
}

impl CoreResourceProfile {
    #[must_use]
    pub fn is_stutter_compatible(&self) -> bool {
        self.requires.is_empty()
            && self.consumes.is_empty()
            && self.transfers.is_empty()
            && self.receives.is_empty()
            && self.grade.values().all(|value| *value == 0)
            && self.relies.is_empty()
    }

    #[must_use]
    pub fn parallel(&self, other: &Self) -> Self {
        let mut grade = self.grade.clone();
        for (atom, value) in &other.grade {
            let entry = grade.entry(atom.clone()).or_default();
            *entry = entry.saturating_add(*value);
        }
        Self {
            requires: self.requires.union(&other.requires).cloned().collect(),
            consumes: self.consumes.union(&other.consumes).cloned().collect(),
            transfers: BTreeSet::new(),
            receives: BTreeSet::new(),
            grade,
            relies: self
                .relies
                .difference(&other.guarantees)
                .chain(other.relies.difference(&self.guarantees))
                .cloned()
                .collect(),
            guarantees: self.guarantees.union(&other.guarantees).cloned().collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CoreBehaviorAction {
    #[serde(default, skip)]
    pub name: String,
    #[serde(deserialize_with = "deserialize_direction")]
    pub direction: Option<CorePortDirection>,
    pub parameters: Vec<CoreBehaviorBinding>,
    pub guards: Vec<String>,
    pub guard_ast: Vec<CoreBehaviorTerm>,
    pub updates: BTreeMap<String, String>,
    pub update_ast: BTreeMap<String, CoreBehaviorTerm>,
    pub outputs: Vec<String>,
    pub hidden: bool,
    pub resources: CoreResourceProfile,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CoreBehaviorSystem {
    #[serde(default, skip)]
    pub name: String,
    pub state: BTreeMap<String, CoreBehaviorState>,
    pub capabilities: BTreeMap<String, String>,
    pub ports: BTreeMap<String, CorePort>,
    pub actions: BTreeMap<String, CoreBehaviorAction>,
    #[serde(rename = "observe")]
    pub observations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CoreConnection {
    pub left_system: String,
    pub left_action: String,
    pub right_system: String,
    pub right_action: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CoreComposition {
    #[serde(default, skip)]
    pub name: String,
    #[serde(rename = "left")]
    pub left_system: String,
    #[serde(rename = "right")]
    pub right_system: String,
    pub connections: Vec<CoreConnection>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CoreRefinement {
    #[serde(rename = "concrete")]
    pub concrete_system: String,
    #[serde(rename = "abstract")]
    pub abstract_system: String,
    pub state_map: BTreeMap<String, String>,
    pub hidden_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct BehaviorCoreProgram {
    pub schema: String,
    pub source_path: String,
    pub source_sha256: String,
    pub enums: BTreeMap<String, BTreeSet<String>>,
    pub systems: BTreeMap<String, CoreBehaviorSystem>,
    pub compositions: BTreeMap<String, CoreComposition>,
    pub refinements: Vec<CoreRefinement>,
}

impl BehaviorCoreProgram {
    /// Decode only the canonical representation produced by `to_json_pretty`.
    /// Lean remains the semantic acceptance authority.
    pub fn from_canonical_json(input: &str) -> Result<Self, String> {
        let mut program: Self = serde_json::from_str(input)
            .map_err(|error| format!("malformed behavior core: {error}"))?;
        for (system_name, system) in &mut program.systems {
            system.name.clone_from(system_name);
            for (state_name, state) in &mut system.state {
                state.name.clone_from(state_name);
            }
            for (port_name, port) in &mut system.ports {
                port.name.clone_from(port_name);
            }
            for (action_name, action) in &mut system.actions {
                action.name.clone_from(action_name);
            }
        }
        for (composition_name, composition) in &mut program.compositions {
            composition.name.clone_from(composition_name);
        }
        if program.schema != BEHAVIOR_CORE_SCHEMA {
            return Err(format!("unsupported behavior schema '{}'", program.schema));
        }
        if program.to_json_pretty() != input {
            return Err("behavior artifact is not in canonical ordering or formatting".to_owned());
        }
        Ok(program)
    }

    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n  \"schema\": ");
        push_json_string(&mut out, &self.schema);
        out.push_str(",\n  \"source_path\": ");
        push_json_string(&mut out, &self.source_path);
        out.push_str(",\n  \"source_sha256\": ");
        push_json_string(&mut out, &self.source_sha256);
        out.push_str(",\n  \"enums\": {");
        push_map(&mut out, &self.enums, 4, |out, variants, indent| {
            push_string_set(out, variants, indent);
        });
        out.push_str("\n  },\n  \"systems\": {");
        push_map(&mut out, &self.systems, 4, push_system);
        out.push_str("\n  },\n  \"compositions\": {");
        push_map(&mut out, &self.compositions, 4, push_composition);
        out.push_str("\n  },\n  \"refinements\": [");
        for (index, refinement) in self.refinements.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("\n    ");
            push_refinement(&mut out, refinement, 4);
        }
        if !self.refinements.is_empty() {
            out.push('\n');
        }
        out.push_str("  ]\n}\n");
        out
    }
}

fn deserialize_direction<'de, D>(deserializer: D) -> Result<Option<CorePortDirection>, D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = String::deserialize(deserializer)?;
    match encoded.as_str() {
        "internal" => Ok(None),
        "input" => Ok(Some(CorePortDirection::Input)),
        "output" => Ok(Some(CorePortDirection::Output)),
        _ => Err(serde::de::Error::custom(format!(
            "unknown action direction '{encoded}'"
        ))),
    }
}

fn push_map<V>(
    out: &mut String,
    values: &BTreeMap<String, V>,
    indent: usize,
    mut push_value: impl FnMut(&mut String, &V, usize),
) {
    for (index, (key, value)) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('\n');
        out.push_str(&" ".repeat(indent));
        push_json_string(out, key);
        out.push_str(": ");
        push_value(out, value, indent);
    }
}

fn push_system(out: &mut String, system: &CoreBehaviorSystem, indent: usize) {
    out.push('{');
    out.push('\n');
    out.push_str(&" ".repeat(indent + 2));
    out.push_str("\"state\": {");
    push_map(out, &system.state, indent + 4, |out, state, _| {
        out.push_str("{\"type\": ");
        push_json_string(out, &state.ty);
        out.push_str(", \"initial\": ");
        push_json_string(out, &state.initializer);
        out.push_str(", \"initial_ast\": ");
        push_term(out, &state.initial_ast);
        out.push('}');
    });
    out.push('\n');
    out.push_str(&" ".repeat(indent + 2));
    out.push_str("},\n");
    out.push_str(&" ".repeat(indent + 2));
    out.push_str("\"capabilities\": {");
    push_map(out, &system.capabilities, indent + 4, |out, ty, _| {
        push_json_string(out, ty);
    });
    out.push('\n');
    out.push_str(&" ".repeat(indent + 2));
    out.push_str("},\n");
    out.push_str(&" ".repeat(indent + 2));
    out.push_str("\"ports\": {");
    push_map(out, &system.ports, indent + 4, |out, port, _| {
        out.push_str("{\"direction\": ");
        push_json_string(out, port.direction.as_str());
        out.push_str(", \"payload\": ");
        push_json_string(out, &port.payload_type);
        out.push('}');
    });
    out.push('\n');
    out.push_str(&" ".repeat(indent + 2));
    out.push_str("},\n");
    out.push_str(&" ".repeat(indent + 2));
    out.push_str("\"actions\": {");
    push_map(out, &system.actions, indent + 4, push_action);
    out.push('\n');
    out.push_str(&" ".repeat(indent + 2));
    out.push_str("},\n");
    out.push_str(&" ".repeat(indent + 2));
    out.push_str("\"observe\": ");
    push_string_slice(out, &system.observations);
    out.push('\n');
    out.push_str(&" ".repeat(indent));
    out.push('}');
}

fn push_action(out: &mut String, action: &CoreBehaviorAction, _indent: usize) {
    out.push_str("{\"direction\": ");
    match action.direction {
        Some(direction) => push_json_string(out, direction.as_str()),
        None => out.push_str("\"internal\""),
    }
    out.push_str(", \"hidden\": ");
    out.push_str(if action.hidden { "true" } else { "false" });
    out.push_str(", \"parameters\": [");
    for (index, parameter) in action.parameters.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str("{\"name\": ");
        push_json_string(out, &parameter.name);
        out.push_str(", \"type\": ");
        push_json_string(out, &parameter.ty);
        out.push('}');
    }
    out.push_str("], \"guards\": ");
    push_string_slice(out, &action.guards);
    out.push_str(", \"guard_ast\": [");
    for (index, term) in action.guard_ast.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        push_term(out, term);
    }
    out.push(']');
    out.push_str(", \"updates\": {");
    for (index, (field, value)) in action.updates.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        push_json_string(out, field);
        out.push_str(": ");
        push_json_string(out, value);
    }
    out.push_str("}, \"update_ast\": {");
    for (index, (field, term)) in action.update_ast.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        push_json_string(out, field);
        out.push_str(": ");
        push_term(out, term);
    }
    out.push_str("}, \"outputs\": ");
    push_string_slice(out, &action.outputs);
    out.push_str(", \"resources\": ");
    push_resources(out, &action.resources);
    out.push('}');
}

fn push_term(out: &mut String, term: &CoreBehaviorTerm) {
    match term {
        CoreBehaviorTerm::Bool { r#type, value } => {
            out.push_str("{\"kind\": \"bool\", \"type\": ");
            push_json_string(out, r#type);
            out.push_str(", \"value\": ");
            out.push_str(if *value { "true" } else { "false" });
            out.push('}');
        }
        CoreBehaviorTerm::Unit { r#type } => {
            out.push_str("{\"kind\": \"unit\", \"type\": ");
            push_json_string(out, r#type);
            out.push('}');
        }
        CoreBehaviorTerm::Enum {
            r#type,
            constructor,
        } => {
            out.push_str("{\"kind\": \"enum\", \"type\": ");
            push_json_string(out, r#type);
            out.push_str(", \"constructor\": ");
            push_json_string(out, constructor);
            out.push('}');
        }
        CoreBehaviorTerm::Read { r#type, field } => {
            out.push_str("{\"kind\": \"read\", \"type\": ");
            push_json_string(out, r#type);
            out.push_str(", \"field\": ");
            push_json_string(out, field);
            out.push('}');
        }
        CoreBehaviorTerm::Not { r#type, value } => {
            out.push_str("{\"kind\": \"not\", \"type\": ");
            push_json_string(out, r#type);
            out.push_str(", \"value\": ");
            push_term(out, value);
            out.push('}');
        }
        CoreBehaviorTerm::Equal {
            r#type,
            left,
            right,
        } => {
            out.push_str("{\"kind\": \"equal\", \"type\": ");
            push_json_string(out, r#type);
            out.push_str(", \"left\": ");
            push_term(out, left);
            out.push_str(", \"right\": ");
            push_term(out, right);
            out.push('}');
        }
    }
}

fn push_resources(out: &mut String, resources: &CoreResourceProfile) {
    out.push_str("{\"requires\": ");
    push_string_set(out, &resources.requires, 0);
    out.push_str(", \"consumes\": ");
    push_string_set(out, &resources.consumes, 0);
    out.push_str(", \"transfers\": ");
    push_string_set(out, &resources.transfers, 0);
    out.push_str(", \"receives\": ");
    push_string_set(out, &resources.receives, 0);
    out.push_str(", \"grade\": {");
    for (index, (atom, value)) in resources.grade.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        push_json_string(out, atom);
        write!(out, ": {value}").expect("writing to a String cannot fail");
    }
    out.push_str("}, \"relies\": ");
    push_string_set(out, &resources.relies, 0);
    out.push_str(", \"guarantees\": ");
    push_string_set(out, &resources.guarantees, 0);
    out.push('}');
}

fn push_composition(out: &mut String, composition: &CoreComposition, _indent: usize) {
    out.push_str("{\"left\": ");
    push_json_string(out, &composition.left_system);
    out.push_str(", \"right\": ");
    push_json_string(out, &composition.right_system);
    out.push_str(", \"connections\": [");
    for (index, connection) in composition.connections.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str("{\"left_system\": ");
        push_json_string(out, &connection.left_system);
        out.push_str(", \"left_action\": ");
        push_json_string(out, &connection.left_action);
        out.push_str(", \"right_system\": ");
        push_json_string(out, &connection.right_system);
        out.push_str(", \"right_action\": ");
        push_json_string(out, &connection.right_action);
        out.push('}');
    }
    out.push_str("]}");
}

fn push_refinement(out: &mut String, refinement: &CoreRefinement, _indent: usize) {
    out.push_str("{\"concrete\": ");
    push_json_string(out, &refinement.concrete_system);
    out.push_str(", \"abstract\": ");
    push_json_string(out, &refinement.abstract_system);
    out.push_str(", \"state_map\": {");
    for (index, (concrete, abstract_name)) in refinement.state_map.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        push_json_string(out, concrete);
        out.push_str(": ");
        push_json_string(out, abstract_name);
    }
    out.push_str("}, \"hidden_actions\": ");
    push_string_set(out, &refinement.hidden_actions, 0);
    out.push('}');
}

fn push_string_set(out: &mut String, values: &BTreeSet<String>, _indent: usize) {
    let values = values.iter().cloned().collect::<Vec<_>>();
    push_string_slice(out, &values);
}

fn push_string_slice(out: &mut String, values: &[String]) {
    out.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        push_json_string(out, value);
    }
    out.push(']');
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character.is_control() => {
                write!(out, "\\u{:04x}", u32::from(character))
                    .expect("writing to a String cannot fail");
            }
            character => out.push(character),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_profiles_add_grades_and_discharge_relies() {
        let left = CoreResourceProfile {
            grade: BTreeMap::from([("work".to_owned(), 1)]),
            relies: BTreeSet::from(["Ready".to_owned()]),
            guarantees: BTreeSet::from(["Authorized".to_owned()]),
            ..CoreResourceProfile::default()
        };
        let right = CoreResourceProfile {
            grade: BTreeMap::from([("work".to_owned(), 2)]),
            relies: BTreeSet::from(["Authorized".to_owned()]),
            guarantees: BTreeSet::from(["Ready".to_owned()]),
            ..CoreResourceProfile::default()
        };
        let product = left.parallel(&right);
        assert_eq!(product.grade["work"], 3);
        assert!(product.relies.is_empty());
    }

    #[test]
    fn canonical_decoder_rejects_reformatting() {
        let program = BehaviorCoreProgram {
            schema: BEHAVIOR_CORE_SCHEMA.to_owned(),
            source_path: "example.nmlt".to_owned(),
            source_sha256: "0".repeat(64),
            enums: BTreeMap::new(),
            systems: BTreeMap::from([(
                "Empty".to_owned(),
                CoreBehaviorSystem {
                    name: "Empty".to_owned(),
                    state: BTreeMap::new(),
                    capabilities: BTreeMap::new(),
                    ports: BTreeMap::new(),
                    actions: BTreeMap::new(),
                    observations: Vec::new(),
                },
            )]),
            compositions: BTreeMap::new(),
            refinements: Vec::new(),
        };
        let canonical = program.to_json_pretty();
        assert_eq!(
            BehaviorCoreProgram::from_canonical_json(&canonical).expect("canonical artifact"),
            program
        );
        assert!(BehaviorCoreProgram::from_canonical_json(canonical.trim()).is_err());
    }

    #[test]
    fn canonical_decoder_rejects_duplicate_identities_and_resource_entries() {
        let canonical =
            include_str!("../../../examples/pivot/visible_resource_sync.behavior-core-v1.json");

        let duplicate_schema =
            canonical.replacen("{\n", "{\n  \"schema\": \"behavior-core-v1\",\n", 1);
        assert!(BehaviorCoreProgram::from_canonical_json(&duplicate_schema).is_err());

        let duplicate_transfer = canonical.replacen(
            "\"transfers\": [\"permit\"]",
            "\"transfers\": [\"permit\", \"permit\"]",
            1,
        );
        assert!(BehaviorCoreProgram::from_canonical_json(&duplicate_transfer).is_err());
    }
}

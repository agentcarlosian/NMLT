//! Versioned finite-execution data. Decoding here is not Lean acceptance.
use crate::{BehaviorCoreProgram, CorePortDirection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionState {
    pub left: usize,
    pub right: usize,
    pub authority: BTreeMap<String, Option<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionAction {
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPath {
    pub schema: String,
    pub artifact_sha256: String,
    pub behavior: String,
    pub states: Vec<ExecutionState>,
    pub actions: Vec<ExecutionAction>,
}

impl ExecutionPath {
    pub fn to_json_pretty(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map(|text| text + "\n")
            .map_err(|e| e.to_string())
    }
}

impl BehaviorCoreProgram {
    pub fn populate_execution_maps(&mut self) -> Result<(), String> {
        let mut known = BTreeMap::new();
        let mut universe = BTreeMap::new();
        for (name, system) in &self.systems {
            let mut local = system.capabilities.clone();
            for action in system
                .actions
                .values()
                .filter(|a| a.direction == Some(CorePortDirection::Input))
            {
                for binding in &action.parameters {
                    if !binding.ty.starts_with("Once<") {
                        continue;
                    }
                    if local
                        .insert(binding.name.clone(), binding.ty.clone())
                        .is_some_and(|old| old != binding.ty)
                    {
                        return Err(format!(
                            "inconsistent capability type for {name}.{}",
                            binding.name
                        ));
                    }
                }
            }
            for (cap, ty) in &local {
                if universe
                    .insert(cap.clone(), ty.clone())
                    .is_some_and(|old| old != *ty)
                {
                    return Err(format!("inconsistent nominal capability type for '{cap}'"));
                }
            }
            known.insert(name.clone(), local);
        }
        let mut initial = BTreeMap::new();
        let targets = self.systems.keys().map(|name| (name, vec![name])).chain(
            self.compositions
                .iter()
                .map(|(name, c)| (name, vec![&c.left_system, &c.right_system])),
        );
        for (name, leaves) in targets {
            let mut world: BTreeMap<_, _> =
                universe.keys().map(|cap| (cap.clone(), None)).collect();
            if leaves.len() == 2 && leaves[0] == leaves[1] {
                return Err("repeated leaf instance".into());
            }
            for leaf in leaves {
                let system = self
                    .systems
                    .get(leaf)
                    .ok_or_else(|| format!("unknown leaf '{leaf}'"))?;
                for cap in system.capabilities.keys() {
                    if world
                        .insert(cap.clone(), Some(leaf.clone()))
                        .flatten()
                        .is_some()
                    {
                        return Err(format!(
                            "duplicate initial ownership of '{cap}' in '{name}'"
                        ));
                    }
                }
            }
            if initial.insert(name.clone(), world).is_some() {
                return Err(format!("ambiguous behavior '{name}'"));
            }
        }
        self.known_capabilities = known;
        self.initial_authority = initial;
        Ok(())
    }

    pub fn validate_execution_maps(&self) -> Result<(), String> {
        let mut expected = self.clone();
        expected.populate_execution_maps()?;
        if expected.known_capabilities != self.known_capabilities
            || expected.initial_authority != self.initial_authority
        {
            return Err(
                "v2 capability namespace or initial authority disagrees with declarations".into(),
            );
        }
        Ok(())
    }
}

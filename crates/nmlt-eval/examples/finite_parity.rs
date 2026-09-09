//! Test-only adapter for the frozen, resource-free finite-value corpus.
//! The line format is scoped to its delimiter-free names, not a public wire API.

use std::collections::BTreeSet;

use nmlt_eval::{EvalState, ExploreConfig, explore};
use nmlt_ir::BehaviorCoreProgram;

fn state_key(state: &EvalState) -> String {
    assert!(state.authority.is_empty(), "fixture must be resource-free");
    state
        .values
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn main() -> Result<(), String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [path, system] = arguments.as_slice() else {
        return Err("usage: finite_parity <canonical-artifact> <system>".to_owned());
    };
    let input = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let program = BehaviorCoreProgram::from_canonical_json(&input)?;
    let result = explore(&program, system, ExploreConfig { max_states: 16 })
        .map_err(|error| error.to_string())?;
    if result.truncated {
        return Err("frozen parity corpus exceeded its 16-state bound".to_owned());
    }
    let keys = result.states.iter().map(state_key).collect::<Vec<_>>();
    let initial = keys.first().ok_or("no initial state")?;
    println!("I|{initial}");
    let mut lines = keys
        .iter()
        .map(|state| format!("S|{state}"))
        .collect::<BTreeSet<_>>();
    for edge in &result.transitions {
        lines.insert(format!(
            "T|{}|{}|{}",
            keys[edge.from], edge.label, keys[edge.to]
        ));
    }
    for line in lines {
        println!("{line}");
    }
    Ok(())
}

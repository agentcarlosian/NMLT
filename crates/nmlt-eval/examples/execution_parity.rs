use nmlt_eval::{ExploreConfig, execution_path, explore};
use nmlt_ir::{BehaviorCoreProgram, ExecutionState};
use std::{collections::BTreeSet, env, fs};

fn key(s: &ExecutionState) -> String {
    format!(
        "{}|{}|{}",
        s.left,
        s.right,
        s.authority
            .iter()
            .map(|(cap, owner)| format!("{cap}={}", owner.as_deref().unwrap_or("vacant")))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [path, behavior] = args.as_slice() else {
        return Err("expected artifact and behavior".into());
    };
    let p = BehaviorCoreProgram::from_canonical_json_v2(&fs::read_to_string(path)?)?;
    let graph = explore(&p, behavior, ExploreConfig { max_states: 32 })?;
    if graph.truncated {
        return Err("comparison was truncated".into());
    }
    let mut paths: Vec<Option<Vec<String>>> = vec![None; graph.states.len()];
    paths[0] = Some(vec![]);
    let mut keys = vec![];
    for i in 0..graph.states.len() {
        let labels = paths[i].clone().ok_or("state lacks a path")?;
        let witness = execution_path(&p, &graph, "0".repeat(64), &labels)?;
        keys.push(key(witness.states.last().ok_or("empty path")?));
        for edge in graph.transitions.iter().filter(|e| e.from == i) {
            if paths[edge.to].is_none() {
                let mut next = labels.clone();
                next.push(edge.label.clone());
                paths[edge.to] = Some(next);
            }
        }
    }
    let mut rows = BTreeSet::new();
    rows.insert(format!("I|{}", keys[0]));
    for key in &keys {
        rows.insert(format!("S|{key}"));
    }
    for edge in &graph.transitions {
        rows.insert(format!(
            "T|{}|{}|{}",
            keys[edge.from], edge.label, keys[edge.to]
        ));
    }
    if rows.len() != 1 + graph.states.len() + graph.transitions.len() {
        return Err("duplicate snapshot rows".into());
    }
    for row in rows {
        println!("{row}");
    }
    Ok(())
}

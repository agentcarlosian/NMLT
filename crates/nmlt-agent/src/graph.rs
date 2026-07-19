use std::collections::{BTreeMap, BTreeSet};

use crate::artifact::ArtifactRole;
use crate::feedback::ResultClass;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactNode {
    pub id: String,
    pub role: ArtifactRole,
    pub digest: String,
    pub summary: String,
    pub result_class: Option<ResultClass>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Edge {
    pub from: String,
    pub relation: String,
    pub to: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ArtifactGraph {
    pub graph_id: String,
    pub nodes: BTreeMap<String, ArtifactNode>,
    pub edges: BTreeSet<Edge>,
}

impl ArtifactGraph {
    #[must_use]
    pub fn new(graph_id: impl Into<String>) -> Self {
        Self {
            graph_id: graph_id.into(),
            nodes: BTreeMap::new(),
            edges: BTreeSet::new(),
        }
    }

    pub fn add_node(&mut self, node: ArtifactNode) -> Result<(), String> {
        if node.id.is_empty() || node.digest.is_empty() {
            return Err("artifact nodes require identity and digest".into());
        }
        if self.nodes.insert(node.id.clone(), node).is_some() {
            return Err("duplicate artifact node".into());
        }
        Ok(())
    }

    pub fn add_edge(&mut self, edge: Edge) -> Result<(), String> {
        if !self.nodes.contains_key(&edge.from) || !self.nodes.contains_key(&edge.to) {
            return Err("artifact edge endpoint is missing".into());
        }
        if edge.relation.is_empty() {
            return Err("artifact edge relation is empty".into());
        }
        self.edges.insert(edge);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.graph_id.is_empty() || self.nodes.is_empty() {
            return Err("artifact graph is incomplete".into());
        }
        for (id, node) in &self.nodes {
            if id != &node.id {
                return Err("artifact node map key mismatch".into());
            }
            if !valid_digest(&node.digest) {
                return Err(format!(
                    "artifact node {} has a non-canonical digest",
                    node.id
                ));
            }
            if let Some(class) = node.result_class
                && matches!(
                    node.role,
                    ArtifactRole::Intent | ArtifactRole::Property | ArtifactRole::Oracle
                )
            {
                return Err(format!(
                    "trusted artifact {} cannot assert checker result {}",
                    node.id,
                    class.as_str()
                ));
            }
        }
        for edge in &self.edges {
            if !self.nodes.contains_key(&edge.from) || !self.nodes.contains_key(&edge.to) {
                return Err("artifact edge endpoint is missing".into());
            }
        }
        Ok(())
    }

    /// Dependency-free, deterministic wire representation used for readback.
    #[must_use]
    pub fn to_wire(&self) -> String {
        let mut output = format!(
            "NMLT-ARTIFACT-GRAPH\t1\t{}\n",
            hex(self.graph_id.as_bytes())
        );
        for node in self.nodes.values() {
            let class = node.result_class.map_or("-", ResultClass::as_str);
            output.push_str(&format!(
                "N\t{}\t{}\t{}\t{}\t{}\n",
                hex(node.id.as_bytes()),
                node.role.as_str(),
                hex(node.digest.as_bytes()),
                hex(node.summary.as_bytes()),
                class
            ));
        }
        for edge in &self.edges {
            output.push_str(&format!(
                "E\t{}\t{}\t{}\n",
                hex(edge.from.as_bytes()),
                hex(edge.relation.as_bytes()),
                hex(edge.to.as_bytes())
            ));
        }
        output
    }

    pub fn from_wire(wire: &str) -> Result<Self, String> {
        let mut lines = wire.lines();
        let header = lines.next().ok_or("missing graph header")?;
        let header = header.split('\t').collect::<Vec<_>>();
        if header.len() != 3 || header[0] != "NMLT-ARTIFACT-GRAPH" || header[1] != "1" {
            return Err("unsupported artifact graph header".into());
        }
        let mut graph = Self::new(unhex_string(header[2])?);
        let mut deferred_edges = Vec::new();
        for line in lines {
            let fields = line.split('\t').collect::<Vec<_>>();
            match fields.first().copied() {
                Some("N") if fields.len() == 6 => {
                    let role = parse_role(fields[2])?;
                    let result_class = if fields[5] == "-" {
                        None
                    } else {
                        Some(parse_result_class(fields[5])?)
                    };
                    graph.add_node(ArtifactNode {
                        id: unhex_string(fields[1])?,
                        role,
                        digest: unhex_string(fields[3])?,
                        summary: unhex_string(fields[4])?,
                        result_class,
                    })?;
                }
                Some("E") if fields.len() == 4 => deferred_edges.push(Edge {
                    from: unhex_string(fields[1])?,
                    relation: unhex_string(fields[2])?,
                    to: unhex_string(fields[3])?,
                }),
                _ => return Err("malformed artifact graph record".into()),
            }
        }
        for edge in deferred_edges {
            graph.add_edge(edge)?;
        }
        graph.validate()?;
        Ok(graph)
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        let nodes = self
            .nodes
            .values()
            .map(|node| {
                let class = node.result_class.map_or_else(
                    || "null".to_owned(),
                    |class| format!("\"{}\"", class.as_str()),
                );
                format!(
                    "{{\"id\":\"{}\",\"role\":\"{}\",\"digest\":\"{}\",\"summary\":\"{}\",\"result_class\":{class}}}",
                    json_escape(&node.id),
                    node.role.as_str(),
                    json_escape(&node.digest),
                    json_escape(&node.summary)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let edges = self
            .edges
            .iter()
            .map(|edge| {
                format!(
                    "{{\"from\":\"{}\",\"relation\":\"{}\",\"to\":\"{}\"}}",
                    json_escape(&edge.from),
                    json_escape(&edge.relation),
                    json_escape(&edge.to)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"schema_version\":\"1.0.0\",\"graph_id\":\"{}\",\"nodes\":[{nodes}],\"edges\":[{edges}]}}",
            json_escape(&self.graph_id)
        )
    }
}

fn parse_role(value: &str) -> Result<ArtifactRole, String> {
    match value {
        "intent" => Ok(ArtifactRole::Intent),
        "property" => Ok(ArtifactRole::Property),
        "oracle" => Ok(ArtifactRole::Oracle),
        "candidate" => Ok(ArtifactRole::Candidate),
        "feedback" => Ok(ArtifactRole::Feedback),
        "proposal" => Ok(ArtifactRole::Proposal),
        "evaluation" => Ok(ArtifactRole::Evaluation),
        _ => Err("unknown artifact role".into()),
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_result_class(value: &str) -> Result<ResultClass, String> {
    match value {
        "syntax_accepted" => Ok(ResultClass::SyntaxAccepted),
        "type_accepted" => Ok(ResultClass::TypeAccepted),
        "model_checked" => Ok(ResultClass::ModelChecked),
        "refuted" => Ok(ResultClass::Refuted),
        "unknown" => Ok(ResultClass::Unknown),
        "conflict" => Ok(ResultClass::Conflict),
        _ => Err("unknown result class".into()),
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

fn unhex_string(value: &str) -> Result<String, String> {
    if value.len() % 2 != 0 {
        return Err("invalid hexadecimal field".into());
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;
        bytes.push((high << 4) | low);
    }
    String::from_utf8(bytes).map_err(|_| "graph field is not UTF-8".into())
}

fn hex_digit(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err("invalid hexadecimal field".into()),
    }
}

fn json_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", u32::from(character)));
            }
            character => output.push(character),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{ArtifactGraph, ArtifactNode, Edge};
    use crate::artifact::ArtifactRole;
    use crate::feedback::ResultClass;

    #[test]
    fn wire_round_trip_preserves_graph_exactly() {
        let mut graph = ArtifactGraph::new("graph:one");
        graph
            .add_node(ArtifactNode {
                id: "intent:one".into(),
                role: ArtifactRole::Intent,
                digest: format!("sha256:{}", "a".repeat(64)),
                summary: "human intent\nwith delimiter: tabs\t".into(),
                result_class: None,
            })
            .unwrap();
        graph
            .add_node(ArtifactNode {
                id: "evaluation:one".into(),
                role: ArtifactRole::Evaluation,
                digest: format!("sha256:{}", "b".repeat(64)),
                summary: "bounded checker result".into(),
                result_class: Some(ResultClass::ModelChecked),
            })
            .unwrap();
        graph
            .add_edge(Edge {
                from: "intent:one".into(),
                relation: "constrains".into(),
                to: "evaluation:one".into(),
            })
            .unwrap();
        let decoded = ArtifactGraph::from_wire(&graph.to_wire()).unwrap();
        assert_eq!(decoded, graph);
    }
}

//! Direct declaration references derived from the pinned NDJSON export.
//! This is a bounded explanation of exported syntax, not another proof checker.
use super::{Result, err};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) const MAX_GRAPH_BYTES: usize = 16 * 1024 * 1024;
const MAX_NODES: usize = 16_384;
const MAX_REFERENCES: usize = 262_144;
const MAX_NAME_BYTES: usize = 8 * 1024 * 1024;
const MAX_VISITS: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Graph {
    pub schema: String,
    pub root: String,
    pub target: String,
    pub export_sha256: String,
    pub nodes: Vec<Node>,
    pub groups: Vec<Group>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Node {
    pub name: String,
    pub kind: Kind,
    /// Explicit constants occurring in the declaration type.
    pub type_references: Vec<String>,
    /// Explicit constants occurring in the stored value.
    pub value_references: Vec<String>,
    /// Recursor rule constructors and explicit constants in rule right sides.
    pub reduction_references: Vec<String>,
    /// Projection type names in the type, value or recursor rule right sides.
    pub projection_references: Vec<String>,
    /// Support declarations retained by the exporter for primitive literals.
    pub literal_support: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Kind {
    Axiom,
    Definition,
    Theorem,
    Opaque,
    Quotient,
    Inductive,
    Constructor,
    Recursor,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Group {
    pub kind: GroupKind,
    pub members: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum GroupKind {
    Inductive,
    Quotient,
}

#[derive(Default)]
struct Expression {
    children: Vec<usize>,
    reference: Option<u64>,
    projection_reference: Option<u64>,
    literal_support: Vec<&'static str>,
}

struct Declaration {
    name: u64,
    kind: Kind,
    type_expr: usize,
    value_expr: Option<usize>,
    rules: Vec<(u64, usize)>,
}

#[derive(Default)]
pub(super) struct Builder {
    expressions: Vec<Expression>,
    declarations: Vec<Declaration>,
    groups: Vec<(GroupKind, Vec<u64>)>,
    quotients: Vec<u64>,
}

fn number(value: &Value) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| "invalid dependency reference".into())
}

fn expression_id(value: &Value, length: usize) -> Result<usize> {
    let id = usize::try_from(number(value)?).map_err(err)?;
    if id >= length {
        return Err("undefined or forward dependency expression".into());
    }
    Ok(id)
}

impl Builder {
    pub fn observe(&mut self, row: &Value) -> Result<()> {
        if let Some(id) = row.get("ie") {
            if number(id)? != self.expressions.len() as u64 {
                return Err("dependency expressions must have consecutive unique indices".into());
            }
            let kinds = [
                "bvar", "sort", "const", "app", "lam", "forallE", "letE", "proj", "natVal",
                "strVal", "mdata",
            ];
            let mut forms = kinds.into_iter().filter(|kind| row.get(kind).is_some());
            let kind = forms.next().ok_or("unknown dependency expression")?;
            if forms.next().is_some() {
                return Err("ambiguous dependency expression".into());
            }
            let form = &row[kind];
            let mut expression = Expression::default();
            let children: &[&str] = match kind {
                "app" => &["fn", "arg"],
                "lam" | "forallE" => &["type", "body"],
                "letE" => &["type", "value", "body"],
                "proj" => &["struct"],
                "mdata" => &["expr"],
                _ => &[],
            };
            for child in children {
                expression
                    .children
                    .push(expression_id(&form[child], self.expressions.len())?);
            }
            expression.reference = match kind {
                "const" => Some(number(&form["name"])?),
                _ => None,
            };
            if kind == "proj" {
                expression.projection_reference = Some(number(&form["typeName"])?);
            }
            expression.literal_support = match kind {
                "natVal" => vec!["Nat"],
                "strVal" => vec!["Char.ofNat", "String.ofList"],
                _ => vec![],
            };
            self.expressions.push(expression);
        }
        for (field, kind) in [
            ("axiom", Kind::Axiom),
            ("def", Kind::Definition),
            ("thm", Kind::Theorem),
            ("opaque", Kind::Opaque),
            ("quot", Kind::Quotient),
        ] {
            if let Some(declaration) = row.get(field) {
                let name = self.declaration(declaration, kind)?;
                if kind == Kind::Quotient {
                    self.quotients.push(name);
                }
            }
        }
        if let Some(block) = row.get("inductive") {
            let mut members = vec![];
            for (field, kind) in [
                ("types", Kind::Inductive),
                ("ctors", Kind::Constructor),
                ("recs", Kind::Recursor),
            ] {
                for declaration in block[field]
                    .as_array()
                    .ok_or("invalid dependency inductive block")?
                {
                    members.push(self.declaration(declaration, kind)?);
                }
            }
            self.groups.push((GroupKind::Inductive, members));
        }
        Ok(())
    }

    fn declaration(&mut self, value: &Value, kind: Kind) -> Result<u64> {
        if self.declarations.len() >= MAX_NODES {
            return Err("proof dependency graph exceeds declaration bound".into());
        }
        let name = number(&value["name"])?;
        let type_expr = expression_id(&value["type"], self.expressions.len())?;
        let value_expr = if matches!(kind, Kind::Definition | Kind::Theorem | Kind::Opaque) {
            Some(expression_id(&value["value"], self.expressions.len())?)
        } else {
            None
        };
        let mut rules = vec![];
        if kind == Kind::Recursor {
            for rule in value["rules"].as_array().ok_or("missing recursor rules")? {
                rules.push((
                    number(&rule["ctor"])?,
                    expression_id(&rule["rhs"], self.expressions.len())?,
                ));
            }
        }
        self.declarations.push(Declaration {
            name,
            kind,
            type_expr,
            value_expr,
            rules,
        });
        Ok(name)
    }

    pub fn finish(self, names: &BTreeMap<u64, String>, export_sha256: String) -> Result<Graph> {
        let mut nodes = vec![];
        let mut budget = MAX_VISITS;
        let mut marks = vec![0; self.expressions.len()];
        let mut ticket = 0;
        let mut name_bytes = MAX_NAME_BYTES;
        let mut references = MAX_REFERENCES;
        for declaration in &self.declarations {
            let mut literals = BTreeSet::new();
            let mut projections = BTreeSet::new();
            let mut collect = |roots: &[usize]| {
                ticket += 1;
                let mut found = BTreeSet::new();
                let mut stack = roots.to_vec();
                while let Some(id) = stack.pop() {
                    if marks[id] == ticket {
                        continue;
                    }
                    marks[id] = ticket;
                    budget = budget
                        .checked_sub(1)
                        .ok_or("proof dependency traversal exceeds its work bound")?;
                    let expression = &self.expressions[id];
                    stack.extend(&expression.children);
                    found.extend(expression.reference);
                    projections.extend(expression.projection_reference);
                    literals.extend(expression.literal_support.iter().copied());
                }
                Ok::<_, String>(found)
            };
            let type_ids = collect(&[declaration.type_expr])?;
            let value_ids = collect(&declaration.value_expr.into_iter().collect::<Vec<_>>())?;
            let mut reduction_ids = collect(
                &declaration
                    .rules
                    .iter()
                    .map(|(_, rhs)| *rhs)
                    .collect::<Vec<_>>(),
            )?;
            reduction_ids.extend(declaration.rules.iter().map(|(ctor, _)| *ctor));
            let mut resolve = |ids: BTreeSet<u64>| -> Result<Vec<String>> {
                let mut found = BTreeSet::new();
                for id in ids {
                    let name = names.get(&id).ok_or("undefined dependency name")?;
                    references = references
                        .checked_sub(1)
                        .ok_or("too many dependency references")?;
                    name_bytes = name_bytes
                        .checked_sub(name.len())
                        .ok_or("dependency names exceed byte bound")?;
                    found.insert(name.clone());
                }
                Ok(found.into_iter().collect())
            };
            let name = names
                .get(&declaration.name)
                .ok_or("undefined graph declaration")?;
            nodes.push(Node {
                name: name.clone(),
                kind: declaration.kind,
                type_references: resolve(type_ids)?,
                value_references: resolve(value_ids)?,
                reduction_references: resolve(reduction_ids)?,
                projection_references: resolve(projections)?,
                literal_support: literals.into_iter().map(str::to_owned).collect(),
            });
        }
        nodes.sort_by(|a, b| a.name.cmp(&b.name));
        let mut groups = self.groups;
        if !self.quotients.is_empty() {
            groups.push((GroupKind::Quotient, self.quotients));
        }
        let mut groups = groups
            .into_iter()
            .map(|(kind, members)| {
                let mut members = members
                    .into_iter()
                    .map(|id| {
                        names
                            .get(&id)
                            .cloned()
                            .ok_or_else(|| "undefined group member".to_owned())
                    })
                    .collect::<Result<Vec<_>>>()?;
                members.sort();
                Ok(Group { kind, members })
            })
            .collect::<Result<Vec<_>>>()?;
        groups.sort_by(|a, b| a.members.cmp(&b.members));
        let graph = Graph {
            schema: "nmlt-lean-proof-dependencies-v1".into(),
            root: "NMLTChecked.result".into(),
            target: "NMLTTask.target".into(),
            export_sha256,
            nodes,
            groups,
        };
        graph.validate()?;
        Ok(graph)
    }
}

impl Graph {
    pub fn validate(&self) -> Result<()> {
        if self.schema != "nmlt-lean-proof-dependencies-v1"
            || self.root != "NMLTChecked.result"
            || self.target != "NMLTTask.target"
            || self.nodes.is_empty()
            || self.nodes.len() > MAX_NODES
            || self
                .nodes
                .windows(2)
                .any(|pair| pair[0].name >= pair[1].name)
        {
            return Err("invalid proof dependency graph identity or node sequence".into());
        }
        let names = self
            .nodes
            .iter()
            .map(|node| node.name.as_str())
            .collect::<BTreeSet<_>>();
        if self.node(&self.root)?.kind != Kind::Theorem
            || self.node(&self.target)?.kind != Kind::Definition
        {
            return Err("wrong proof dependency root or target kind".into());
        }
        let mut references = 0;
        let mut name_bytes = 0;
        for node in &self.nodes {
            name_bytes += node.name.len();
            for list in [
                &node.type_references,
                &node.value_references,
                &node.reduction_references,
                &node.projection_references,
                &node.literal_support,
            ] {
                references += list.len();
                name_bytes += list.iter().map(String::len).sum::<usize>();
                if list.windows(2).any(|pair| pair[0] >= pair[1])
                    || list.iter().any(|name| !names.contains(name.as_str()))
                {
                    return Err("unsorted, duplicate or missing proof dependency".into());
                }
            }
        }
        let mut grouped = BTreeSet::new();
        for group in &self.groups {
            if group.members.is_empty() || group.members.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err("invalid proof dependency group".into());
            }
            for member in &group.members {
                references += 1;
                name_bytes += member.len();
                if !names.contains(member.as_str()) || !grouped.insert(member) {
                    return Err("missing or repeated proof dependency group member".into());
                }
            }
        }
        if references > MAX_REFERENCES || name_bytes > MAX_NAME_BYTES {
            return Err("proof dependency graph exceeds reference or name-byte bound".into());
        }
        Ok(())
    }

    pub fn node(&self, name: &str) -> Result<&Node> {
        self.nodes
            .binary_search_by(|node| node.name.as_str().cmp(name))
            .map(|index| &self.nodes[index])
            .map_err(|_| "missing proof dependency node".into())
    }

    pub fn json(&self) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec_pretty(self).map_err(err)?;
        if bytes.len() > MAX_GRAPH_BYTES {
            return Err("proof dependency JSON exceeds 16 MiB".into());
        }
        Ok(bytes)
    }

    pub fn markdown(&self) -> Result<String> {
        let mut text = format!(
            "# Proof dependencies\n\nExport SHA-256: `{}`.\n\nThis report describes the syntax of the exported declarations checked by NanoDA. It is not a separate proof check or a draft plan. Type and value references contain explicit constants. Reduction references include recursor rule constructors and constants in rule right sides. Projection type names, literal support and export groups are recorded separately; the export need not be a minimal dependency set. Groups and recursive declarations can contain cycles.\n\n",
            self.export_sha256
        );
        let links = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                (
                    node.name.as_str(),
                    format!("[{}](#declaration-{index})", label(&node.name)),
                )
            })
            .collect::<BTreeMap<_, _>>();
        text.push_str(&format!(
            "Root: {}. Target binding: {}.\n\n",
            links[self.root.as_str()],
            links[self.target.as_str()]
        ));
        for (index, node) in self.nodes.iter().enumerate() {
            text.push_str(&format!(
                "<a id=\"declaration-{index}\"></a>\n\n## {}\n\nKind: `{:?}`.\n\n",
                label(&node.name),
                node.kind
            ));
            for (title, references) in [
                ("Type", &node.type_references),
                ("Value", &node.value_references),
                ("Reduction rules", &node.reduction_references),
                ("Projection types", &node.projection_references),
                ("Literal support", &node.literal_support),
            ] {
                if !references.is_empty() {
                    text.push_str(&format!(
                        "- {title}: {}\n",
                        references
                            .iter()
                            .map(|name| links[name.as_str()].as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            text.push('\n');
            if text.len() > MAX_GRAPH_BYTES {
                return Err("proof dependency report exceeds 16 MiB".into());
            }
        }
        text.push_str("## Export groups\n\nThese declarations are exported together; group membership is not a direct type/value reference.\n\n");
        for group in &self.groups {
            text.push_str(&format!(
                "- `{:?}`: {}\n",
                group.kind,
                group
                    .members
                    .iter()
                    .map(|name| links[name.as_str()].as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if text.len() > MAX_GRAPH_BYTES {
            return Err("proof dependency report exceeds 16 MiB".into());
        }
        Ok(text)
    }
}

fn label(name: &str) -> String {
    // Numeric anchors are independent of declaration text. Escape all Markdown
    // punctuation in labels, including unusual quoted Lean names.
    let escaped = name
        .chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || c == ' ' {
                c.to_string().chars().collect::<Vec<_>>()
            } else {
                format!("&#{};", u32::from(c)).chars().collect()
            }
        })
        .collect::<String>();
    format!("<code>{escaped}</code>")
}

#[cfg(test)]
mod tests;

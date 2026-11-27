//! Bounded native Lean tactics and guarded Lean 4 tactic scripts, retained as candidate data.
use super::{Result, err, name};
use nmlt_runtime::lean;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

// The context-only Lake editor retains all tactic support. Acceptance uses only
// the validated candidate's fixed imports under its unchanged process policy.
pub(super) const EDITOR_IMPORTS: &str = "import Lean.Elab.Tactic\n";

const MAX_SCRIPT_BYTES: usize = 32_768;
const MAX_SCRIPT_LINES: usize = 512;

/// Identifiers forbidden anywhere outside Lean comments/strings in guarded `v3` scripts.
const BANNED_IDENTIFIERS: &[&str] = &[
    // Soundness / axiom / unsafe escapes
    "sorry",
    "admit",
    "sorryAx",
    "native_decide",
    "unsafe",
    "implemented_by",
    "extern",
    "partial",
    // Top-level commands and declarations
    "import",
    "axiom",
    "constant",
    "opaque",
    "theorem",
    "lemma",
    "def",
    "abbrev",
    "structure",
    "class",
    "inductive",
    "instance",
    "example",
    "namespace",
    "section",
    "end",
    "open",
    "variable",
    "universe",
    "attribute",
    "set_option",
    "export",
    "mutual",
    "noncomputable",
    "private",
    "protected",
    // Metaprogramming, macros, syntax extensions, and IO hooks
    "macro",
    "macro_rules",
    "syntax",
    "declare_syntax_cat",
    "elab",
    "elab_rules",
    "notation",
    "infix",
    "infixl",
    "infixr",
    "prefix",
    "postfix",
    "initialize",
    "builtin_initialize",
    "run_cmd",
    "run_meta",
    "run_elab",
    "run_tac",
    "dbg_trace",
    "IO",
    "System",
];

const BANNED_PREFIXES: &[&str] = &[
    "Lean.Elab",
    "Lean.Meta",
    "Lean.Core",
    "Lean.Environment",
    "Lean.addDecl",
    "Lean.trustCompiler",
    "IO.",
    "System.",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "schema", deny_unknown_fields)]
pub(super) enum Candidate {
    #[serde(rename = "nmlt-lean-proof-candidate-v1")]
    Term { task_sha256: String, proof: String },
    #[serde(rename = "nmlt-lean-proof-candidate-v2")]
    Automation {
        task_sha256: String,
        steps: Vec<Step>,
    },
    #[serde(rename = "nmlt-lean-proof-candidate-v3")]
    Script { task_sha256: String, script: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tactic", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Step {
    Intro { names: Vec<String> },
    Exact { term: String },
    Apply { term: String },
    SimpOnly { lemmas: Vec<String> },
    SimpaOnly { lemmas: Vec<String> },
    Assumption {},
    Rfl {},
    Constructor {},
    Omega {},
    Grind {},
    Decide {},
}

fn names(values: &[String], locals: bool) -> Result<String> {
    if values.is_empty() || values.len() > 32 {
        return Err("tactic name list requires 1..32 names".into());
    }
    for value in values {
        if !name(value) || (locals && value.contains('.')) {
            return Err("tactic expects ordinary Lean names".into());
        }
        // Use the closed term parser's reserved-word rules as well.
        lean::render_proof_term(value).map_err(err)?;
    }
    Ok(values.join(if locals { " " } else { ", " }))
}

impl Step {
    fn module(&self) -> Option<&'static str> {
        match self {
            // These elaborators are supplied by the pinned Lean frontend.
            // Importing their implementation modules into the proof unnecessarily
            // retains those modules for independent export as well.
            Self::Intro { .. }
            | Self::Assumption {}
            | Self::Rfl {}
            | Self::Exact { .. }
            | Self::Apply { .. }
            | Self::Constructor {}
            | Self::Decide {} => None,
            Self::SimpOnly { .. } => Some("Lean.Elab.Tactic.Simp"),
            Self::SimpaOnly { .. } => Some("Lean.Elab.Tactic.Simpa"),
            Self::Omega {} => Some("Lean.Elab.Tactic.Omega"),
            Self::Grind {} => Some("Lean.Elab.Tactic.Grind"),
        }
    }

    fn render(&self) -> Result<String> {
        Ok(match self {
            Self::Intro { names: values } => format!("intro {}", names(values, true)?),
            Self::Exact { term } => {
                format!("exact {}", lean::render_proof_term(term).map_err(err)?)
            }
            Self::Apply { term } => {
                format!("apply {}", lean::render_proof_term(term).map_err(err)?)
            }
            Self::SimpOnly { lemmas } => format!("simp only [{}]", names(lemmas, false)?),
            Self::SimpaOnly { lemmas } => format!("simpa only [{}]", names(lemmas, false)?),
            Self::Assumption {} => "assumption".into(),
            Self::Rfl {} => "rfl".into(),
            Self::Constructor {} => "constructor".into(),
            Self::Omega {} => "omega".into(),
            Self::Grind {} => "grind".into(),
            Self::Decide {} => "decide".into(),
        })
    }

    fn parse(line: &str) -> Result<Self> {
        let (head, rest) = line.split_once(' ').unwrap_or((line, ""));
        let rest = rest.trim();
        let fixed = match head {
            "assumption" => Some(Self::Assumption {}),
            "rfl" => Some(Self::Rfl {}),
            "constructor" => Some(Self::Constructor {}),
            "omega" => Some(Self::Omega {}),
            "grind" => Some(Self::Grind {}),
            "decide" => Some(Self::Decide {}),
            _ => None,
        };
        if let Some(step) = fixed {
            return if rest.is_empty() {
                Ok(step)
            } else {
                Err("fixed tactic does not accept extra syntax".into())
            };
        }
        Ok(match head {
            "intro" => Self::Intro {
                names: rest.split_whitespace().map(str::to_owned).collect(),
            },
            "exact" => Self::Exact { term: rest.into() },
            "apply" => Self::Apply { term: rest.into() },
            "simp" | "simpa" => {
                let body = rest
                    .strip_prefix("only [")
                    .and_then(|s| s.strip_suffix(']'))
                    .ok_or("simplification requires an explicit 'only [lemma, ...]' list")?;
                let lemmas = body.split(',').map(|s| s.trim().to_owned()).collect();
                if head == "simp" {
                    Self::SimpOnly { lemmas }
                } else {
                    Self::SimpaOnly { lemmas }
                }
            }
            _ => {
                return Err("unsupported candidate tactic; use intro, exact, apply, simp only, simpa only, assumption, rfl, constructor, omega, grind or decide".into());
            }
        })
    }
}

/// Lexer-aware static guard for ordinary multi-line Lean 4 tactic scripts (`v3`).
/// Returns the set of identifier tokens observed outside comments and string/char literals.
fn scan_guarded_script(raw: &str) -> Result<(String, BTreeSet<String>)> {
    if raw.is_empty() || raw.len() > MAX_SCRIPT_BYTES {
        return Err("guarded tactic script requires 1..32768 UTF-8 bytes".into());
    }
    if raw
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err("guarded tactic script contains disallowed control characters".into());
    }
    let normalized = raw.replace("\r\n", "\n");
    let script = normalized.trim_end_matches([' ', '\t', '\n']).to_owned();
    let trimmed_start = script.trim_start_matches('\n');
    let lines: Vec<&str> = trimmed_start.lines().collect();
    if lines.is_empty() || lines.len() > MAX_SCRIPT_LINES {
        return Err("guarded tactic script requires 1..512 lines".into());
    }
    let first = lines[0].trim_start();
    if !(first == "by" || first.starts_with("by ") || first.starts_with("by\t")) {
        return Err("guarded tactic script must begin with 'by'".into());
    }
    // Enforce Lean 4 layout containment: every non-empty, non-comment line after
    // the opening `by` line must be indented so the script cannot terminate
    // `theorem NMLTChecked.result` and start a new top-level command at column 0.
    let mut block_comment_depth: usize = 0;
    for (line_idx, line) in lines.iter().enumerate() {
        let mut chars = line.chars().peekable();
        if line_idx > 0 && block_comment_depth == 0 {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") && !line.starts_with([' ', '\t']) {
                return Err(
                    "guarded tactic script lines after 'by' must be indented to stay inside the proof block"
                        .into(),
                );
            }
        }
        // Track nested block-comment state across lines for accurate indentation check.
        let bytes = line.as_bytes();
        let mut i = 0;
        let mut in_string = false;
        while i < bytes.len() {
            if block_comment_depth > 0 {
                if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'-' {
                    block_comment_depth += 1;
                    i += 2;
                } else if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'/' {
                    block_comment_depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            } else if in_string {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                } else if bytes[i] == b'"' {
                    in_string = false;
                    i += 1;
                } else {
                    i += 1;
                }
            } else if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'-' {
                break;
            } else if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'-' {
                block_comment_depth += 1;
                i += 2;
            } else if bytes[i] == b'"' {
                in_string = true;
                i += 1;
            } else {
                i += 1;
            }
        }
        let _ = chars.peek();
    }

    // Full lexer scan across the script for banned tokens, command prefixes,
    // raw strings, char literals, nested block comments, and balanced delimiters.
    let src = trimmed_start;
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut delimiters: Vec<char> = Vec::new();
    let mut identifiers = BTreeSet::new();
    let mut has_tactic_token = false;

    while i < bytes.len() {
        // Line comment `-- ...`
        if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // Nested block comment `/- ... -/`
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'-' {
            let mut depth = 1usize;
            i += 2;
            while i < bytes.len() && depth > 0 {
                if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'-' {
                    depth += 1;
                    i += 2;
                } else if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'/' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if depth != 0 {
                return Err("unterminated block comment in tactic script".into());
            }
            continue;
        }
        // Raw string `r#"..."#`
        if bytes[i] == b'r'
            && (i == 0
                || !(bytes[i - 1].is_ascii_alphanumeric()
                    || bytes[i - 1] == b'_'
                    || bytes[i - 1] == b'\''))
            && let Some(end) = match_raw_string(bytes, i)
        {
            i = end;
            has_tactic_token = true;
            continue;
        }
        // Ordinary string `"..."`
        if bytes[i] == b'"' {
            i += 1;
            let mut closed = false;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2;
                } else if bytes[i] == b'"' {
                    i += 1;
                    closed = true;
                    break;
                } else {
                    i += 1;
                }
            }
            if !closed {
                return Err("unterminated string literal in tactic script".into());
            }
            has_tactic_token = true;
            continue;
        }
        // Character literal `'a'`, `'\''`, `'\n'`, `'\x41'`, `'\u0041'`
        if bytes[i] == b'\''
            && let Some(end) = match_char_literal(src, i)
        {
            i = end;
            has_tactic_token = true;
            continue;
        }
        // Reject `#` commands (`#eval`, `#check`, `#print`, `#guard`, etc.)
        if bytes[i] == b'#' {
            return Err("guarded tactic script forbids '#' commands".into());
        }
        // Reject backtick syntax quotations and name literals that could feed metaprograms
        if bytes[i] == b'`' {
            return Err("guarded tactic script forbids backtick quotations".into());
        }
        // Reject `+native` (e.g. `decide +native`)
        if bytes[i] == b'+' && bytes[i + 1..].starts_with(b"native") {
            let after = i + 7;
            if after == bytes.len()
                || !(bytes[after].is_ascii_alphanumeric() || bytes[after] == b'_')
            {
                return Err("guarded tactic script forbids '+native'".into());
            }
        }
        // Balanced delimiters (ASCII + Unicode angle brackets `⟨` / `⟩`)
        let ch = src[i..].chars().next().unwrap();
        match ch {
            '(' | '[' | '{' | '⟨' => {
                delimiters.push(ch);
                i += ch.len_utf8();
                continue;
            }
            ')' | ']' | '}' | '⟩' => {
                let expected = match ch {
                    ')' => '(',
                    ']' => '[',
                    '}' => '{',
                    '⟩' => '⟨',
                    _ => unreachable!(),
                };
                if delimiters.pop() != Some(expected) {
                    return Err("unbalanced delimiters in guarded tactic script".into());
                }
                i += ch.len_utf8();
                continue;
            }
            _ => {}
        }
        // Identifier / dotted-name token
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            i += ch.len_utf8();
            while i < bytes.len() {
                let c = src[i..].chars().next().unwrap();
                if c.is_ascii_alphanumeric() || matches!(c, '_' | '\'' | '.') {
                    // Stop before `..` range or trailing `.`
                    if c == '.'
                        && (i + 1 >= bytes.len()
                            || !(bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_'))
                    {
                        break;
                    }
                    i += c.len_utf8();
                } else {
                    break;
                }
            }
            let ident = &src[start..i];
            if BANNED_IDENTIFIERS.contains(&ident)
                || ident
                    .split('.')
                    .any(|part| BANNED_IDENTIFIERS.contains(&part))
                || BANNED_PREFIXES
                    .iter()
                    .any(|prefix| ident.starts_with(prefix))
            {
                return Err(format!(
                    "guarded tactic script forbids '{ident}' inside candidate proofs"
                ));
            }
            if ident != "by" {
                has_tactic_token = true;
            }
            identifiers.insert(ident.to_owned());
            continue;
        }
        i += ch.len_utf8();
    }

    if !delimiters.is_empty() {
        return Err("unbalanced delimiters in guarded tactic script".into());
    }
    if !has_tactic_token {
        return Err("guarded tactic script requires at least one tactic".into());
    }
    let mut rendered = trimmed_start.to_owned();
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    Ok((rendered, identifiers))
}

fn match_raw_string(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'r') {
        return None;
    }
    let mut j = start + 1;
    let mut hashes = 0usize;
    while bytes.get(j) == Some(&b'#') {
        hashes += 1;
        j += 1;
    }
    if hashes == 0 || bytes.get(j) != Some(&b'"') {
        return None;
    }
    j += 1;
    while j < bytes.len() {
        if bytes[j] == b'"' {
            let mut k = j + 1;
            let mut closing = 0usize;
            while closing < hashes && bytes.get(k) == Some(&b'#') {
                closing += 1;
                k += 1;
            }
            if closing == hashes {
                return Some(k);
            }
        }
        j += 1;
    }
    None
}

fn match_char_literal(src: &str, start: usize) -> Option<usize> {
    let rest = src.get(start + 1..)?;
    if let Some(after_slash) = rest.strip_prefix('\\') {
        let mut chars = after_slash.char_indices();
        let (_, esc) = chars.next()?;
        let after_esc = match esc {
            'x' => {
                let mut end = 2;
                for _ in 0..2 {
                    let (idx, c) = chars.next()?;
                    if !c.is_ascii_hexdigit() {
                        return None;
                    }
                    end = 1 + idx + c.len_utf8();
                }
                end
            }
            'u' => {
                let mut end = 2;
                for _ in 0..4 {
                    let (idx, c) = chars.next()?;
                    if !c.is_ascii_hexdigit() {
                        return None;
                    }
                    end = 1 + idx + c.len_utf8();
                }
                end
            }
            '\\' | '"' | '\'' | 'n' | 't' | 'r' | '0' => 2,
            _ => return None,
        };
        if rest.as_bytes().get(after_esc) == Some(&b'\'') {
            return Some(start + 1 + after_esc + 1);
        }
        return None;
    }
    let ch = rest.chars().next()?;
    if ch == '\'' || ch == '\n' || ch == '\r' {
        return None;
    }
    let end = ch.len_utf8();
    if rest.as_bytes().get(end) == Some(&b'\'') {
        Some(start + 1 + end + 1)
    } else {
        None
    }
}

impl Candidate {
    pub fn imports(&self) -> String {
        match self {
            Self::Term { .. } => String::new(),
            Self::Automation { steps, .. } => steps
                .iter()
                .filter_map(Step::module)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(|module| format!("import {module}\n"))
                .collect(),
            Self::Script { script, .. } => {
                let Ok((_, idents)) = scan_guarded_script(script) else {
                    return String::new();
                };
                let mut modules = BTreeSet::new();
                if idents.contains("simp")
                    || idents.contains("dsimp")
                    || idents.contains("simp_all")
                {
                    modules.insert("Lean.Elab.Tactic.Simp");
                }
                if idents.contains("simpa") {
                    modules.insert("Lean.Elab.Tactic.Simpa");
                }
                if idents.contains("omega") {
                    modules.insert("Lean.Elab.Tactic.Omega");
                }
                if idents.contains("grind") {
                    modules.insert("Lean.Elab.Tactic.Grind");
                }
                modules
                    .into_iter()
                    .map(|module| format!("import {module}\n"))
                    .collect()
            }
        }
    }

    pub fn from_file(digest: &str, source: &str, target: &super::Target) -> Result<Self> {
        let normalized;
        let source = if source.contains("\r\n") {
            normalized = source.replace("\r\n", "\n");
            normalized.as_str()
        } else {
            source
        };
        if source.starts_with("import ") {
            // Accept a familiar full Lean file only when every byte outside its
            // proof body still matches the task's generated checking wrapper.
            const MARKER: &str = "NMLTCandidateBodyPlaceholder";
            let template = super::source::proof(target, MARKER, "")?;
            let (before, after) = template.split_once(MARKER).ok_or("missing proof marker")?;
            const ROOT: &str = "import NMLTTask\n";
            let wrapper = before.strip_prefix(ROOT).ok_or("missing root import")?;
            if let Some((imports, rest)) = source
                .strip_prefix(ROOT)
                .and_then(|s| s.split_once(wrapper))
                && let Some(body) = rest.strip_suffix(after)
            {
                let candidate = Self::from_proof(digest, body)?;
                // Only adapter-generated wrappers are accepted: a term
                // wrapper, the unchanged editor draft, or this candidate's
                // exact fixed imports. No user-selected module is retained.
                if imports.is_empty() || imports == EDITOR_IMPORTS || imports == candidate.imports()
                {
                    return Ok(candidate);
                }
            }
            return Err("edited Lean file changed the task wrapper; revise the task explicitly or supply only the proof body".into());
        }
        Self::from_proof(digest, source)
    }

    pub fn from_proof(digest: &str, source: &str) -> Result<Self> {
        if source.is_empty() || source.len() > MAX_SCRIPT_BYTES {
            return Err("candidate proof file requires 1..32768 UTF-8 bytes".into());
        }
        let trimmed = source.trim();
        let is_tactic_block = trimmed == "by"
            || trimmed.starts_with("by\n")
            || trimmed.starts_with("by\r\n")
            || trimmed.starts_with("by ")
            || trimmed.starts_with("by\t");
        if is_tactic_block {
            // Preserve v2 structured automation whenever the proof fits the 11-tactic
            // ASCII grammar so existing task hashes, schemas, and minimal imports stay exact.
            if source.is_ascii()
                && source.len() <= 8192
                && (trimmed == "by" || trimmed.starts_with("by\n") || trimmed.starts_with("by\r\n"))
                && trimmed
                    .lines()
                    .skip(1)
                    .all(|line| line.trim().is_empty() || line.starts_with([' ', '\t']))
                && let Ok(steps) = trimmed
                    .lines()
                    .skip(1)
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(Step::parse)
                    .collect::<Result<Vec<_>>>()
            {
                let automation = Self::Automation {
                    task_sha256: digest.into(),
                    steps,
                };
                if automation.validate(digest).is_ok() {
                    return Ok(automation);
                }
            }
            let (script, _) = scan_guarded_script(source)?;
            let candidate = Self::Script {
                task_sha256: digest.into(),
                script,
            };
            candidate.validate(digest)?;
            return Ok(candidate);
        }
        if source.len() > 8192 || !source.is_ascii() {
            return Err("candidate proof term requires 1..8192 ASCII bytes".into());
        }
        let candidate = Self::Term {
            task_sha256: digest.into(),
            proof: trimmed.into(),
        };
        candidate.validate(digest)?;
        Ok(candidate)
    }

    pub fn validate(&self, digest: &str) -> Result<String> {
        let (Self::Term { task_sha256, .. }
        | Self::Automation { task_sha256, .. }
        | Self::Script { task_sha256, .. }) = self;
        if task_sha256 != digest {
            return Err("candidate does not refer to the selected task hash".into());
        }
        match self {
            Self::Term { proof, .. } => lean::render_proof_term(proof).map_err(err),
            Self::Automation { steps, .. } => {
                if steps.is_empty() || steps.len() > 32 {
                    return Err("candidate automation requires 1..32 steps".into());
                }
                let mut source = String::from("by\n");
                for step in steps {
                    source.push_str("  ");
                    source.push_str(&step.render()?);
                    source.push('\n');
                }
                if source.len() > 8192 {
                    return Err("candidate automation exceeds 8192 rendered bytes".into());
                }
                scan_guarded_script(&source)?;
                Ok(source)
            }
            Self::Script { script, .. } => Ok(scan_guarded_script(script)?.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_automation_is_explicit_bounded_and_cannot_change_commands() {
        let candidate =
            Candidate::from_proof("pin", "by\n  intro n\n  simpa only [Nat.zero_add]\n").unwrap();
        assert!(matches!(candidate, Candidate::Automation { .. }));
        assert_eq!(
            candidate.validate("pin").unwrap(),
            "by\n  intro n\n  simpa only [Nat.zero_add]\n"
        );
        assert!(candidate.validate("other").is_err());
        for source in [
            "by\n sorry",
            "by\n run_tac trivial",
            "by\n rfl; sorry",
            "by\n exact True.intro\naxiom bad : False",
            "by\n simp only [x]\nset_option maxRecDepth 1000",
            "by\n exact (by sorry)",
        ] {
            assert!(Candidate::from_proof("pin", source).is_err(), "{source}");
        }
        // Unindented lines after `by` cannot bypass v2's 32-step cap into v3.
        assert!(Candidate::from_proof("pin", &format!("by\n{}", "rfl\n".repeat(33))).is_err());
        assert!(serde_json::from_str::<Candidate>(r#"{"schema":"nmlt-lean-proof-candidate-v2","task_sha256":"pin","steps":[{"tactic":"rfl","statement":"True"}]}"#).is_err());
        assert!(serde_json::from_str::<Candidate>(r#"{"schema":"nmlt-lean-proof-candidate-v2","task_sha256":"pin","steps":[],"statement":"True"}"#).is_err());
        // Direct v2 step limit remains enforced on deserialized Automation candidates.
        let oversized_v2 = Candidate::Automation {
            task_sha256: "pin".into(),
            steps: vec![Step::Rfl {}; 33],
        };
        assert!(oversized_v2.validate("pin").is_err());
    }

    #[test]
    fn guarded_tactic_scripts_allow_real_lean_proofs_and_block_escapes() {
        let script = "\
by
  intro n m h
  -- Handle inductive proof with Unicode bullet and angle brackets
  induction n with
  | zero =>
    have h0 : 0 + m = m := Nat.zero_add m
    rw [h0]
    exact ⟨rfl, h⟩
  | succ k ih =>
    /- nested /- comment with sorry inside -/ still comment -/
    simp [Nat.succ_add, ih]
";
        let candidate = Candidate::from_proof("pin", script).unwrap();
        assert!(matches!(candidate, Candidate::Script { .. }));
        assert_eq!(candidate.imports(), "import Lean.Elab.Tactic.Simp\n");
        let rendered = candidate.validate("pin").unwrap();
        assert!(rendered.starts_with("by\n"));

        // Tactics that need no umbrella import emit empty imports.
        let rw_only = Candidate::from_proof("pin", "by\n  intro n\n  rw [Nat.add_zero]\n").unwrap();
        assert!(matches!(rw_only, Candidate::Script { .. }));
        assert_eq!(rw_only.imports(), "");

        // Adversarial negative controls must all be rejected.
        for bad in [
            "by\n  sorry\n",
            "by\n  admit\n",
            "by\n  native_decide\n",
            "by\n  decide +native\n",
            "by\n  #eval IO.println 1\n",
            "by\n  run_tac pure ()\n",
            "by\n  exact Lean.addDecl\n",
            "by\n  exact IO.println\n",
            "by\n  exact `(rfl)\n",
            "by\n  rfl\naxiom sneaky : False\n",
            "by\n  rfl\nend\n",
            "by\n  rfl\nrfl\n",
            "by\n  have h := (rfl\n",
            "by\n  /- unterminated comment\n  rfl\n",
            "by\n  have s := \"unterminated\n",
            "by\n  exact '\"'; sorry\n",
            "by\n  have x := r#\"ok\"#; sorry\n",
            "by\n  -- only comment\n",
        ] {
            assert!(
                Candidate::from_proof("pin", bad).is_err(),
                "expected rejection for: {bad}"
            );
        }
        // Char-literal containing `"` must NOT swallow a following `sorry`.
        assert!(Candidate::from_proof("pin", "by\n  have c := '\"'\n  sorry\n").is_err());
        // Legitimate char literal `'"'` without banned tokens is allowed.
        assert!(Candidate::from_proof("pin", "by\n  have c := '\"'\n  rfl\n").is_ok());
    }

    #[test]
    fn imports_are_fixed_by_steps_and_full_wrappers_cannot_add_modules() {
        let target = super::super::Target {
            declaration: "Example.goal".into(),
            module: "Example".into(),
            type_repr: "True".into(),
            type_lean: "True".into(),
            level_params: vec![],
            type_references: vec![],
        };
        for body in [
            "by\n intro n\n constructor\n exact True.intro\n exact True.intro\n",
            "by\n grind\n",
            "by\n decide\n",
            "by\n simpa only [Nat.zero_add]\n",
            "by\n  intro n\n  cases n with\n  | zero => rfl\n  | succ k => simp\n",
        ] {
            let candidate = Candidate::from_proof("pin", body).unwrap();
            let imports = candidate.imports();
            assert!(
                !imports
                    .lines()
                    .any(|line| line == "import Lean.Elab.Tactic")
            );
            let rendered = candidate.validate("pin").unwrap();
            for header in [imports.as_str(), EDITOR_IMPORTS, ""] {
                let source = super::super::source::proof(&target, &rendered, header).unwrap();
                let extracted = Candidate::from_file("pin", &source, &target).unwrap();
                assert_eq!(extracted.validate("pin").unwrap(), rendered);
                assert_eq!(extracted.imports(), imports);
                let crlf_source = source.replace('\n', "\r\n");
                let extracted_crlf = Candidate::from_file("pin", &crlf_source, &target).unwrap();
                assert_eq!(extracted_crlf.validate("pin").unwrap(), rendered);
                assert_eq!(extracted_crlf.imports(), imports);
                assert!(
                    Candidate::from_file(
                        "pin",
                        &crlf_source
                            .replace("import NMLTTask\r\n", "import NMLTTask\r\nimport Evil\r\n"),
                        &target
                    )
                    .is_err()
                );
                assert!(
                    Candidate::from_file(
                        "pin",
                        &source.replace("NMLTTask.target", "True"),
                        &target
                    )
                    .is_err()
                );
                assert!(
                    Candidate::from_file(
                        "pin",
                        &source.replace("import NMLTTask\n", "import NMLTTask\nimport Evil\n"),
                        &target
                    )
                    .is_err()
                );
            }
        }
        assert!(Candidate::from_proof("pin", "by\n decide +native\n").is_err());
        let exact = Candidate::from_proof("pin", "by\n exact True.intro\n").unwrap();
        assert_eq!(exact.imports(), "");
        assert_eq!(
            Candidate::from_proof("pin", "True.intro")
                .unwrap()
                .imports(),
            ""
        );
    }
}

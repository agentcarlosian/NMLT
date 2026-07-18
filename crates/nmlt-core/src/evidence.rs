/// The assurance class of an evidence result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceResult {
    Proved,
    ModelChecked,
    Tested,
    Monitored,
    Refuted,
    Unknown,
    Indeterminate,
}

impl EvidenceResult {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proved => "proved",
            Self::ModelChecked => "model_checked",
            Self::Tested => "tested",
            Self::Monitored => "monitored",
            Self::Refuted => "refuted",
            Self::Unknown => "unknown",
            Self::Indeterminate => "indeterminate",
        }
    }
}

/// A deliberately small evidence representation for the structural frontend.
/// The stable contract is the JSON Schema, not this pre-alpha Rust type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceManifest {
    pub manifest_id: String,
    pub artifact_path: String,
    pub claim_id: String,
    pub claim_kind: String,
    pub result: EvidenceResult,
    pub method_kind: String,
    pub assumptions: Vec<String>,
    pub negative_controls: Vec<String>,
    pub residual_gaps: Vec<String>,
}

impl EvidenceManifest {
    #[must_use]
    pub fn structural_unknown(path: impl Into<String>) -> Self {
        let artifact_path = path.into();
        Self {
            manifest_id: format!("structural:{artifact_path}"),
            artifact_path,
            claim_id: "source-structure".to_owned(),
            claim_kind: "well_formedness".to_owned(),
            result: EvidenceResult::Unknown,
            method_kind: "structural_check".to_owned(),
            assumptions: Vec::new(),
            negative_controls: Vec::new(),
            residual_gaps: vec![
                "Only structural parsing ran.".to_owned(),
                "No name resolution, type checking, or semantic verification ran.".to_owned(),
            ],
        }
    }

    /// Serialize the pre-alpha manifest without introducing a dependency before
    /// the canonicalization and hashing rules have been decided.
    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"schema_version\": \"0.1.0\",\n",
                "  \"manifest_id\": {},\n",
                "  \"artifact\": {{ \"path\": {} }},\n",
                "  \"claim\": {{ \"id\": {}, \"kind\": {} }},\n",
                "  \"result\": {},\n",
                "  \"method\": {{ \"kind\": {}, \"engine\": \"nmlt-cli\", ",
                "\"engine_version\": {} }},\n",
                "  \"assumptions\": {},\n",
                "  \"negative_controls\": {},\n",
                "  \"residual_gaps\": {}\n",
                "}}"
            ),
            json_string(&self.manifest_id),
            json_string(&self.artifact_path),
            json_string(&self.claim_id),
            json_string(&self.claim_kind),
            json_string(self.result.as_str()),
            json_string(&self.method_kind),
            json_string(env!("CARGO_PKG_VERSION")),
            json_array(&self.assumptions),
            json_array(&self.negative_controls),
            json_array(&self.residual_gaps),
        )
    }
}

fn json_array(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{values}]")
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                use std::fmt::Write;
                write!(output, "\\u{:04x}", u32::from(character))
                    .expect("writing to a String cannot fail");
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use super::{EvidenceManifest, EvidenceResult, json_string};

    #[test]
    fn structural_manifest_is_explicitly_unknown() {
        let manifest = EvidenceManifest::structural_unknown("example.nmlt");
        assert_eq!(manifest.result, EvidenceResult::Unknown);
        assert!(
            manifest
                .to_json_pretty()
                .contains("\"result\": \"unknown\"")
        );
        assert!(
            manifest
                .to_json_pretty()
                .contains("No name resolution, type checking, or semantic verification ran.")
        );
    }

    #[test]
    fn escapes_json_strings() {
        assert_eq!(json_string("a\n\"b\\c"), "\"a\\n\\\"b\\\\c\"");
    }

    #[test]
    fn names_every_result_class() {
        let results = [
            EvidenceResult::Proved,
            EvidenceResult::ModelChecked,
            EvidenceResult::Tested,
            EvidenceResult::Monitored,
            EvidenceResult::Refuted,
            EvidenceResult::Unknown,
            EvidenceResult::Indeterminate,
        ];
        assert!(results.iter().all(|result| !result.as_str().is_empty()));
    }
}

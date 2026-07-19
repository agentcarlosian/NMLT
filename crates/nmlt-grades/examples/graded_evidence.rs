use std::convert::Infallible;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use nmlt_grades::{
    BudgetDecision, Grade, GradeAlgebra, ProductGradeAlgebra, check_budget, check_laws,
    parse_program,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Word(Vec<u8>);

struct NonCommutativeWords;

impl GradeAlgebra for NonCommutativeWords {
    type Element = Word;
    type Error = Infallible;

    fn zero(&self) -> Self::Element {
        Word(Vec::new())
    }

    fn sequential(
        &self,
        left: &Self::Element,
        right: &Self::Element,
    ) -> Result<Self::Element, Self::Error> {
        let mut word = left.0.clone();
        word.extend_from_slice(&right.0);
        Ok(Word(word))
    }

    fn choice(
        &self,
        left: &Self::Element,
        right: &Self::Element,
    ) -> Result<Self::Element, Self::Error> {
        Ok(if left.0 >= right.0 {
            left.clone()
        } else {
            right.clone()
        })
    }

    fn parallel(
        &self,
        left: &Self::Element,
        right: &Self::Element,
    ) -> Result<Self::Element, Self::Error> {
        self.sequential(left, right)
    }

    fn leq(&self, left: &Self::Element, right: &Self::Element) -> bool {
        left.0 <= right.0
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 4 {
        return Err(
            "usage: graded_evidence REFERENCE BUDGET_CONTROL UNKNOWN_CONTROL INVALID_CONTROL"
                .to_owned(),
        );
    }
    let source_id = env::var("NMLT_GRADED_REFERENCE_SOURCE_ID")
        .map_err(|_| "NMLT_GRADED_REFERENCE_SOURCE_ID is required".to_owned())?;
    let claim_spec_source_id = env::var("NMLT_GRADED_CLAIM_SPEC_SOURCE_ID")
        .map_err(|_| "NMLT_GRADED_CLAIM_SPEC_SOURCE_ID is required".to_owned())?;
    let claim_id = env::var("NMLT_GRADED_CLAIM_ID")
        .map_err(|_| "NMLT_GRADED_CLAIM_ID is required".to_owned())?;

    let reference = parse_file(&arguments[0])?;
    let (usage, budget) = match check_budget(&reference) {
        BudgetDecision::WithinBudget { usage, budget } => (usage, budget),
        outcome => return Err(format!("reference did not pass: {outcome:?}")),
    };
    let expected_usage =
        Grade::checked(66, 500_000, 155, 47_000).map_err(|error| error.to_string())?;
    if usage != expected_usage {
        return Err(format!(
            "reference grade changed: expected {expected_usage:?}, found {usage:?}"
        ));
    }

    let budget_control = parse_file(&arguments[1])?;
    let (budget_actual, budget_limit, budget_dimensions) = match check_budget(&budget_control) {
        BudgetDecision::Exceeded { violations, .. } => {
            let dimensions = violations
                .iter()
                .map(|violation| violation.dimension.as_str())
                .collect::<Vec<_>>();
            if dimensions != ["privacy_micro_epsilon"] {
                return Err(format!(
                    "budget control violated unexpected dimensions: {dimensions:?}"
                ));
            }
            (violations[0].actual, violations[0].limit, dimensions)
        }
        outcome => return Err(format!("budget control was not exceeded: {outcome:?}")),
    };

    let unknown_control = parse_file(&arguments[2])?;
    let unknown_code = match check_budget(&unknown_control) {
        BudgetDecision::Unknown { diagnostics } if !diagnostics.is_empty() => diagnostics[0].code,
        outcome => return Err(format!("unknown control did not fail closed: {outcome:?}")),
    };

    let invalid_source = read(&arguments[3])?;
    let invalid_code = match parse_program(&invalid_source) {
        Err(diagnostics) if !diagnostics.is_empty() => diagnostics[0].code,
        outcome => return Err(format!("invalid control parsed unexpectedly: {outcome:?}")),
    };

    let samples = [
        Grade::ZERO,
        Grade::checked(1, 2, 3, 4).map_err(|error| error.to_string())?,
        Grade::checked(7, 5, 3, 900_000).map_err(|error| error.to_string())?,
        Grade::checked(11, 13, 17, 200_000).map_err(|error| error.to_string())?,
    ];
    let product_violations = check_laws(&ProductGradeAlgebra, &samples);
    if !product_violations.is_empty() {
        return Err(format!(
            "product algebra failed its finite regression laws: {product_violations:?}"
        ));
    }
    let noncommutative_violations = check_laws(
        &NonCommutativeWords,
        &[Word(Vec::new()), Word(vec![1]), Word(vec![2])],
    );
    let noncommutative_law = noncommutative_violations
        .iter()
        .find(|violation| violation.law == "sequence_commutative")
        .ok_or_else(|| "noncommutative control was not detected".to_owned())?
        .law;

    let dimensions = budget_dimensions
        .iter()
        .map(|dimension| json_string(dimension))
        .collect::<Vec<_>>()
        .join(",");
    let output = format!(
        concat!(
            "{{",
            "\"schema_version\":\"1.0.0\",",
            "\"fixture\":\"phase7-graded-resources-v1\",",
            "\"reference\":{{",
            "\"source_id\":{},",
            "\"claim_spec_source_id\":{},",
            "\"claim_id\":{},",
            "\"claim_handle\":\"Grades.ProviderPipeline.WithinBudget\",",
            "\"claim\":\"Under the declared annotations and bounded composition rules, the provider pipeline is componentwise within its declared product budget.\",",
            "\"decision\":\"within_budget\",",
            "\"usage\":{},",
            "\"budget\":{}",
            "}},",
            "\"algebra\":{{",
            "\"carrier\":\"Nat64 x Nat64 x Nat64 x Ppm\",",
            "\"sequence\":\"componentwise_checked_add_with_saturated_uncertainty\",",
            "\"choice\":\"componentwise_max\",",
            "\"parallel\":\"componentwise_checked_add_without_disjointness_evidence\",",
            "\"sample_count\":4,",
            "\"sampled_law_result\":\"passed\",",
            "\"sampled_laws\":[",
            "\"identity\",\"commutativity\",\"associativity\",",
            "\"choice_idempotence\",\"zero_bottom\",",
            "\"sequence_distributes_over_choice\"",
            "]",
            "}},",
            "\"bounds\":{{",
            "\"integer_representation\":\"u64_checked\",",
            "\"uncertainty_scale_ppm\":1000000,",
            "\"reference_iteration_bounds\":\"finite_explicit\",",
            "\"overflow_result\":\"unknown\"",
            "}},",
            "\"negative_controls\":[",
            "{{\"name\":\"privacy_budget_violation\",\"expected\":\"exceeded\",\"observed\":\"exceeded\",\"dimensions\":[{}],\"actual\":{},\"limit\":{}}},",
            "{{\"name\":\"unknown_iteration\",\"expected\":\"unknown\",\"observed\":\"unknown\",\"code\":{}}},",
            "{{\"name\":\"invalid_uncertainty\",\"expected\":\"parse_rejected\",\"observed\":\"parse_rejected\",\"code\":{}}},",
            "{{\"name\":\"noncommutative_word_algebra\",\"expected\":\"law_rejected\",\"observed\":\"law_rejected\",\"law\":{}}}",
            "],",
            "\"assumptions\":[",
            "\"Every atom annotation is a trusted upper bound in the declared unit.\",",
            "\"Choice executes at most one branch and uses a componentwise worst case.\",",
            "\"Parallel work is conservatively additive; no data-disjoint privacy theorem is invoked.\",",
            "\"Uncertainty annotations support only the stated saturated union-bound abstraction.\"",
            "],",
            "\"residual_gaps\":[",
            "\"No operational semantics connects annotations to measured cost or energy.\",",
            "\"No differential-privacy mechanism or sensitivity proof is checked.\",",
            "\"The Lean algebra proof is not verified Rust extraction or compiler correctness.\",",
            "\"No grade inference, symbolic bounds, or certified compiler is implemented.\"",
            "],",
            "\"implementation\":{{}}",
            "}}"
        ),
        json_string(&source_id),
        json_string(&claim_spec_source_id),
        json_string(&claim_id),
        grade_json(usage),
        grade_json(budget),
        dimensions,
        budget_actual,
        budget_limit,
        json_string(unknown_code),
        json_string(invalid_code),
        json_string(noncommutative_law),
    );
    println!("{output}");
    Ok(())
}

fn parse_file(path: &str) -> Result<nmlt_grades::Program, String> {
    let source = read(path)?;
    parse_program(&source).map_err(|diagnostics| format!("{path}: {diagnostics:?}"))
}

fn read(path: &str) -> Result<String, String> {
    fs::read_to_string(Path::new(path)).map_err(|error| format!("{path}: {error}"))
}

fn grade_json(grade: Grade) -> String {
    format!(
        concat!(
            "{{\"cost_ticks\":{},",
            "\"privacy_micro_epsilon\":{},",
            "\"energy_microjoules\":{},",
            "\"uncertainty_ppm\":{}}}"
        ),
        grade.cost_ticks(),
        grade.privacy_micro_epsilon(),
        grade.energy_microjoules(),
        grade.uncertainty_ppm(),
    )
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
            character if character <= '\u{1f}' => {
                write!(output, "\\u{:04x}", character as u32).expect("write to String");
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

use crate::{Dimension, Grade, GradeError};

/// A finite or explicitly unknown iteration count.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IterationBound {
    Exact(u64),
    Unknown,
}

/// The deliberately small graded-resource plan language.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Plan {
    Atom {
        name: String,
        grade: Grade,
    },
    Sequence(Vec<Self>),
    Choice(Vec<Self>),
    Parallel(Vec<Self>),
    Repeat {
        count: IterationBound,
        body: Box<Self>,
    },
}

/// A parsed benchmark program with one product budget.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub name: String,
    pub budget: Grade,
    pub plan: Plan,
}

/// A stable fail-closed analysis diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub path: String,
    pub message: String,
}

/// Resource analysis is three-valued: an exact annotated bound or unknown.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Analysis {
    Exact(Grade),
    Unknown(Vec<Diagnostic>),
}

/// A coordinate that exceeds its declared upper budget.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Violation {
    pub dimension: Dimension,
    pub actual: u64,
    pub limit: u64,
}

/// The checker never turns an unknown analysis into a successful budget claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BudgetDecision {
    WithinBudget {
        usage: Grade,
        budget: Grade,
    },
    Exceeded {
        usage: Grade,
        budget: Grade,
        violations: Vec<Violation>,
    },
    Unknown {
        diagnostics: Vec<Diagnostic>,
    },
}

#[must_use]
pub fn analyze(plan: &Plan) -> Analysis {
    analyze_at(plan, "$plan")
}

#[must_use]
pub fn check_budget(program: &Program) -> BudgetDecision {
    match analyze(&program.plan) {
        Analysis::Unknown(diagnostics) => BudgetDecision::Unknown { diagnostics },
        Analysis::Exact(usage) => {
            let mut violations = Vec::new();
            for dimension in [
                Dimension::CostTicks,
                Dimension::PrivacyMicroEpsilon,
                Dimension::EnergyMicrojoules,
                Dimension::UncertaintyUpperBoundPpm,
            ] {
                let actual = usage.coordinate(dimension);
                let limit = program.budget.coordinate(dimension);
                if actual > limit {
                    violations.push(Violation {
                        dimension,
                        actual,
                        limit,
                    });
                }
            }
            if violations.is_empty() {
                BudgetDecision::WithinBudget {
                    usage,
                    budget: program.budget,
                }
            } else {
                BudgetDecision::Exceeded {
                    usage,
                    budget: program.budget,
                    violations,
                }
            }
        }
    }
}

fn analyze_at(plan: &Plan, path: &str) -> Analysis {
    match plan {
        Plan::Atom { grade, .. } => Analysis::Exact(*grade),
        Plan::Sequence(children) => fold_children(children, path, "sequence", Grade::sequential),
        Plan::Parallel(children) => fold_children(children, path, "parallel", Grade::parallel),
        Plan::Choice(children) if children.is_empty() => Analysis::Unknown(vec![Diagnostic {
            code: "NMLT-GRADE-EMPTY-CHOICE",
            path: path.to_owned(),
            message: "a worst-case choice requires at least one alternative".to_owned(),
        }]),
        Plan::Choice(children) => {
            let mut exact = Grade::ZERO;
            let mut diagnostics = Vec::new();
            for (index, child) in children.iter().enumerate() {
                let child_path = format!("{path}.choice[{index}]");
                match analyze_at(child, &child_path) {
                    Analysis::Exact(grade) => match exact.choice(grade) {
                        Ok(combined) => exact = combined,
                        Err(error) => diagnostics.push(error_diagnostic(error, path)),
                    },
                    Analysis::Unknown(mut child_diagnostics) => {
                        diagnostics.append(&mut child_diagnostics);
                    }
                }
            }
            if diagnostics.is_empty() {
                Analysis::Exact(exact)
            } else {
                Analysis::Unknown(diagnostics)
            }
        }
        Plan::Repeat { count, body } => {
            let body_path = format!("{path}.repeat.body");
            let body_analysis = analyze_at(body, &body_path);
            match (count, body_analysis) {
                (_, Analysis::Unknown(diagnostics)) => Analysis::Unknown(diagnostics),
                (IterationBound::Unknown, Analysis::Exact(_)) => {
                    Analysis::Unknown(vec![Diagnostic {
                        code: "NMLT-GRADE-UNKNOWN-ITERATION",
                        path: format!("{path}.repeat.count"),
                        message: "resource use is unknown without a finite iteration bound"
                            .to_owned(),
                    }])
                }
                (IterationBound::Exact(count), Analysis::Exact(grade)) => {
                    repeat_grade(grade, *count, path)
                }
            }
        }
    }
}

fn fold_children(
    children: &[Plan],
    path: &str,
    field: &str,
    operation: fn(Grade, Grade) -> Result<Grade, GradeError>,
) -> Analysis {
    let mut exact = Grade::ZERO;
    let mut diagnostics = Vec::new();
    for (index, child) in children.iter().enumerate() {
        let child_path = format!("{path}.{field}[{index}]");
        match analyze_at(child, &child_path) {
            Analysis::Exact(grade) => match operation(exact, grade) {
                Ok(combined) => exact = combined,
                Err(error) => diagnostics.push(error_diagnostic(error, path)),
            },
            Analysis::Unknown(mut child_diagnostics) => {
                diagnostics.append(&mut child_diagnostics);
            }
        }
    }
    if diagnostics.is_empty() {
        Analysis::Exact(exact)
    } else {
        Analysis::Unknown(diagnostics)
    }
}

fn repeat_grade(grade: Grade, mut count: u64, path: &str) -> Analysis {
    let mut result = Grade::ZERO;
    let mut power = grade;
    while count > 0 {
        if count & 1 == 1 {
            result = match result.sequential(power) {
                Ok(value) => value,
                Err(error) => return Analysis::Unknown(vec![error_diagnostic(error, path)]),
            };
        }
        count >>= 1;
        if count > 0 {
            power = match power.sequential(power) {
                Ok(value) => value,
                Err(error) => return Analysis::Unknown(vec![error_diagnostic(error, path)]),
            };
        }
    }
    Analysis::Exact(result)
}

fn error_diagnostic(error: GradeError, path: &str) -> Diagnostic {
    Diagnostic {
        code: match error {
            GradeError::InvalidUncertainty { .. } => "NMLT-GRADE-INVALID-UNCERTAINTY",
            GradeError::ArithmeticOverflow { .. } => "NMLT-GRADE-ARITHMETIC-OVERFLOW",
            GradeError::IncompatibleUncertaintyFamilies { .. } => {
                "NMLT-GRADE-INCOMPATIBLE-UNCERTAINTY-FAMILIES"
            }
            GradeError::InvalidUncertaintyProfile { .. } => {
                "NMLT-GRADE-INVALID-UNCERTAINTY-PROFILE"
            }
            GradeError::IncompatibleUncertaintyProfiles { .. } => {
                "NMLT-GRADE-INCOMPATIBLE-UNCERTAINTY-PROFILES"
            }
        },
        path: path.to_owned(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grade(cost: u64, privacy: u64, energy: u64, uncertainty: u32) -> Grade {
        Grade::checked(
            cost,
            privacy,
            energy,
            crate::UncertaintyCertificate::checked_upper_bound(
                crate::UncertaintyFamily::Declared,
                crate::UncertaintyFamily::Declared.profile_id(),
                uncertainty,
            )
            .expect("valid uncertainty"),
        )
        .expect("valid test grade")
    }

    fn atom(name: &str, grade: Grade) -> Plan {
        Plan::Atom {
            name: name.to_owned(),
            grade,
        }
    }

    #[test]
    fn composition_is_deterministic_and_compositional() {
        let plan = Plan::Sequence(vec![
            atom("start", grade(12, 100_000, 30, 10_000)),
            Plan::Choice(vec![
                atom("cache", grade(4, 0, 8, 2_000)),
                Plan::Sequence(vec![
                    atom("fetch", grade(25, 300_000, 70, 25_000)),
                    atom("validate", grade(9, 0, 15, 5_000)),
                ]),
            ]),
            Plan::Parallel(vec![
                atom("audit", grade(8, 50_000, 20, 3_000)),
                atom("metrics", grade(6, 0, 12, 2_000)),
            ]),
            Plan::Repeat {
                count: IterationBound::Exact(2),
                body: Box::new(atom("retry_guard", grade(3, 25_000, 4, 1_000))),
            },
        ]);

        assert_eq!(
            analyze(&plan),
            Analysis::Exact(grade(66, 500_000, 155, 47_000))
        );
        assert_eq!(analyze(&plan), analyze(&plan));
    }

    #[test]
    fn budget_violation_names_every_exceeded_coordinate() {
        let program = Program {
            name: "control".to_owned(),
            budget: grade(9, 5, 7, 100),
            plan: atom("too_large", grade(10, 6, 7, 101)),
        };
        let BudgetDecision::Exceeded { violations, .. } = check_budget(&program) else {
            panic!("control must exceed its budget");
        };
        assert_eq!(
            violations
                .iter()
                .map(|violation| violation.dimension)
                .collect::<Vec<_>>(),
            [
                Dimension::CostTicks,
                Dimension::PrivacyMicroEpsilon,
                Dimension::UncertaintyUpperBoundPpm
            ]
        );
    }

    #[test]
    fn unknown_iteration_fails_closed() {
        let plan = Plan::Repeat {
            count: IterationBound::Unknown,
            body: Box::new(atom("work", grade(1, 1, 1, 1))),
        };
        let Analysis::Unknown(diagnostics) = analyze(&plan) else {
            panic!("unbounded repeat must be unknown");
        };
        assert_eq!(diagnostics[0].code, "NMLT-GRADE-UNKNOWN-ITERATION");
    }

    #[test]
    fn overflow_fails_closed() {
        let plan = Plan::Sequence(vec![
            atom("maximum", grade(u64::MAX, 0, 0, 0)),
            atom("one_more", grade(1, 0, 0, 0)),
        ]);
        let Analysis::Unknown(diagnostics) = analyze(&plan) else {
            panic!("overflow must be unknown");
        };
        assert_eq!(diagnostics[0].code, "NMLT-GRADE-ARITHMETIC-OVERFLOW");
    }

    #[test]
    fn empty_choice_is_not_zero_cost_success() {
        assert!(matches!(
            analyze(&Plan::Choice(Vec::new())),
            Analysis::Unknown(_)
        ));
    }
}

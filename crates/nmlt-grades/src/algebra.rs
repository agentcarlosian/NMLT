use std::fmt::{self, Debug, Display};

/// One whole unit of uncertainty, represented as parts per million.
pub const UNCERTAINTY_SCALE_PPM: u32 = 1_000_000;

/// The independent coordinates of the prototype product grade.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Dimension {
    CostTicks,
    PrivacyMicroEpsilon,
    EnergyMicrojoules,
    UncertaintyPpm,
}

impl Dimension {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CostTicks => "cost_ticks",
            Self::PrivacyMicroEpsilon => "privacy_micro_epsilon",
            Self::EnergyMicrojoules => "energy_microjoules",
            Self::UncertaintyPpm => "uncertainty_ppm",
        }
    }
}

impl Display for Dimension {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A checked failure of a grade operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GradeError {
    InvalidUncertainty {
        found: u32,
        maximum: u32,
    },
    ArithmeticOverflow {
        dimension: Dimension,
        operation: &'static str,
    },
}

impl Display for GradeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUncertainty { found, maximum } => write!(
                formatter,
                "uncertainty {found} ppm exceeds the scale maximum {maximum} ppm"
            ),
            Self::ArithmeticOverflow {
                dimension,
                operation,
            } => write!(formatter, "{operation} overflows dimension {dimension}"),
        }
    }
}

impl std::error::Error for GradeError {}

/// A product upper bound over four deliberately integer-valued dimensions.
///
/// `uncertainty_ppm` is an abstract upper bound in `[0, 1]`, encoded without
/// floating point. It is not automatically a probability: that interpretation
/// requires a model connecting annotations to events.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Grade {
    cost_ticks: u64,
    privacy_micro_epsilon: u64,
    energy_microjoules: u64,
    uncertainty_ppm: u32,
}

impl Grade {
    pub const ZERO: Self = Self {
        cost_ticks: 0,
        privacy_micro_epsilon: 0,
        energy_microjoules: 0,
        uncertainty_ppm: 0,
    };

    pub fn checked(
        cost_ticks: u64,
        privacy_micro_epsilon: u64,
        energy_microjoules: u64,
        uncertainty_ppm: u32,
    ) -> Result<Self, GradeError> {
        if uncertainty_ppm > UNCERTAINTY_SCALE_PPM {
            return Err(GradeError::InvalidUncertainty {
                found: uncertainty_ppm,
                maximum: UNCERTAINTY_SCALE_PPM,
            });
        }
        Ok(Self {
            cost_ticks,
            privacy_micro_epsilon,
            energy_microjoules,
            uncertainty_ppm,
        })
    }

    #[must_use]
    pub const fn cost_ticks(self) -> u64 {
        self.cost_ticks
    }

    #[must_use]
    pub const fn privacy_micro_epsilon(self) -> u64 {
        self.privacy_micro_epsilon
    }

    #[must_use]
    pub const fn energy_microjoules(self) -> u64 {
        self.energy_microjoules
    }

    #[must_use]
    pub const fn uncertainty_ppm(self) -> u32 {
        self.uncertainty_ppm
    }

    /// Conservative sequential composition.
    pub fn sequential(self, other: Self) -> Result<Self, GradeError> {
        Ok(Self {
            cost_ticks: checked_add(
                self.cost_ticks,
                other.cost_ticks,
                Dimension::CostTicks,
                "sequential composition",
            )?,
            privacy_micro_epsilon: checked_add(
                self.privacy_micro_epsilon,
                other.privacy_micro_epsilon,
                Dimension::PrivacyMicroEpsilon,
                "sequential composition",
            )?,
            energy_microjoules: checked_add(
                self.energy_microjoules,
                other.energy_microjoules,
                Dimension::EnergyMicrojoules,
                "sequential composition",
            )?,
            uncertainty_ppm: self
                .uncertainty_ppm
                .saturating_add(other.uncertainty_ppm)
                .min(UNCERTAINTY_SCALE_PPM),
        })
    }

    /// Worst-case alternative: take the componentwise upper envelope.
    #[must_use]
    pub const fn choice(self, other: Self) -> Self {
        Self {
            cost_ticks: max_u64(self.cost_ticks, other.cost_ticks),
            privacy_micro_epsilon: max_u64(self.privacy_micro_epsilon, other.privacy_micro_epsilon),
            energy_microjoules: max_u64(self.energy_microjoules, other.energy_microjoules),
            uncertainty_ppm: max_u32(self.uncertainty_ppm, other.uncertainty_ppm),
        }
    }

    /// Conservative parallel composition.
    ///
    /// This deliberately uses addition. In particular, privacy is not reduced
    /// to a maximum without separately checked evidence that data domains are
    /// disjoint.
    pub fn parallel(self, other: Self) -> Result<Self, GradeError> {
        self.sequential(other)
    }

    #[must_use]
    pub const fn componentwise_le(self, other: Self) -> bool {
        self.cost_ticks <= other.cost_ticks
            && self.privacy_micro_epsilon <= other.privacy_micro_epsilon
            && self.energy_microjoules <= other.energy_microjoules
            && self.uncertainty_ppm <= other.uncertainty_ppm
    }

    #[must_use]
    pub const fn coordinate(self, dimension: Dimension) -> u64 {
        match dimension {
            Dimension::CostTicks => self.cost_ticks,
            Dimension::PrivacyMicroEpsilon => self.privacy_micro_epsilon,
            Dimension::EnergyMicrojoules => self.energy_microjoules,
            Dimension::UncertaintyPpm => self.uncertainty_ppm as u64,
        }
    }
}

const fn max_u64(left: u64, right: u64) -> u64 {
    if left >= right { left } else { right }
}

const fn max_u32(left: u32, right: u32) -> u32 {
    if left >= right { left } else { right }
}

fn checked_add(
    left: u64,
    right: u64,
    dimension: Dimension,
    operation: &'static str,
) -> Result<u64, GradeError> {
    left.checked_add(right)
        .ok_or(GradeError::ArithmeticOverflow {
            dimension,
            operation,
        })
}

/// Operations required by this extension's declared commutative product
/// profile. Other NMLT extensions may legitimately choose a noncommutative
/// effect quantale and therefore a different law profile.
pub trait GradeAlgebra {
    type Element: Clone + Debug + Eq;
    type Error: Display;

    fn zero(&self) -> Self::Element;
    fn sequential(
        &self,
        left: &Self::Element,
        right: &Self::Element,
    ) -> Result<Self::Element, Self::Error>;
    fn choice(
        &self,
        left: &Self::Element,
        right: &Self::Element,
    ) -> Result<Self::Element, Self::Error>;
    fn parallel(
        &self,
        left: &Self::Element,
        right: &Self::Element,
    ) -> Result<Self::Element, Self::Error>;
    fn leq(&self, left: &Self::Element, right: &Self::Element) -> bool;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProductGradeAlgebra;

impl GradeAlgebra for ProductGradeAlgebra {
    type Element = Grade;
    type Error = GradeError;

    fn zero(&self) -> Self::Element {
        Grade::ZERO
    }

    fn sequential(
        &self,
        left: &Self::Element,
        right: &Self::Element,
    ) -> Result<Self::Element, Self::Error> {
        left.sequential(*right)
    }

    fn choice(
        &self,
        left: &Self::Element,
        right: &Self::Element,
    ) -> Result<Self::Element, Self::Error> {
        Ok(left.choice(*right))
    }

    fn parallel(
        &self,
        left: &Self::Element,
        right: &Self::Element,
    ) -> Result<Self::Element, Self::Error> {
        left.parallel(*right)
    }

    fn leq(&self, left: &Self::Element, right: &Self::Element) -> bool {
        left.componentwise_le(*right)
    }
}

/// A concrete counterexample produced by finite law checking.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LawViolation {
    pub law: &'static str,
    pub witness: String,
}

/// Check the selected algebra profile over an explicit finite sample set.
///
/// Passing this function is regression evidence, not a universal proof. The
/// RFC separately supplies pen-and-paper proofs for the concrete operations.
#[must_use]
pub fn check_laws<A: GradeAlgebra>(algebra: &A, samples: &[A::Element]) -> Vec<LawViolation> {
    let mut violations = Vec::new();
    let zero = algebra.zero();

    for (index, value) in samples.iter().enumerate() {
        check_equal(
            &mut violations,
            "sequence_left_identity",
            algebra.sequential(&zero, value),
            Ok(value.clone()),
            format!("sample={index}, value={value:?}"),
        );
        check_equal(
            &mut violations,
            "sequence_right_identity",
            algebra.sequential(value, &zero),
            Ok(value.clone()),
            format!("sample={index}, value={value:?}"),
        );
        check_equal(
            &mut violations,
            "parallel_left_identity",
            algebra.parallel(&zero, value),
            Ok(value.clone()),
            format!("sample={index}, value={value:?}"),
        );
        check_equal(
            &mut violations,
            "choice_zero_identity",
            algebra.choice(&zero, value),
            Ok(value.clone()),
            format!("sample={index}, value={value:?}"),
        );
        check_equal(
            &mut violations,
            "choice_idempotent",
            algebra.choice(value, value),
            Ok(value.clone()),
            format!("sample={index}, value={value:?}"),
        );
        if !algebra.leq(&zero, value) {
            violations.push(LawViolation {
                law: "zero_is_bottom",
                witness: format!("sample={index}, value={value:?}"),
            });
        }
    }

    for (left_index, left) in samples.iter().enumerate() {
        for (right_index, right) in samples.iter().enumerate() {
            let witness = format!("left[{left_index}]={left:?}, right[{right_index}]={right:?}");
            check_equal(
                &mut violations,
                "sequence_commutative",
                algebra.sequential(left, right),
                algebra.sequential(right, left),
                witness.clone(),
            );
            check_equal(
                &mut violations,
                "parallel_commutative",
                algebra.parallel(left, right),
                algebra.parallel(right, left),
                witness.clone(),
            );
            check_equal(
                &mut violations,
                "choice_commutative",
                algebra.choice(left, right),
                algebra.choice(right, left),
                witness,
            );
        }
    }

    for (a_index, a) in samples.iter().enumerate() {
        for (b_index, b) in samples.iter().enumerate() {
            for (c_index, c) in samples.iter().enumerate() {
                let witness = format!("a[{a_index}]={a:?}, b[{b_index}]={b:?}, c[{c_index}]={c:?}");
                check_associative(
                    &mut violations,
                    "sequence_associative",
                    algebra,
                    a,
                    b,
                    c,
                    Operation::Sequential,
                    &witness,
                );
                check_associative(
                    &mut violations,
                    "parallel_associative",
                    algebra,
                    a,
                    b,
                    c,
                    Operation::Parallel,
                    &witness,
                );
                check_associative(
                    &mut violations,
                    "choice_associative",
                    algebra,
                    a,
                    b,
                    c,
                    Operation::Choice,
                    &witness,
                );

                let left_distributes = algebra
                    .choice(b, c)
                    .and_then(|choice| algebra.sequential(a, &choice));
                let right_distributes = algebra.sequential(a, b).and_then(|ab| {
                    algebra
                        .sequential(a, c)
                        .and_then(|ac| algebra.choice(&ab, &ac))
                });
                check_equal(
                    &mut violations,
                    "sequence_distributes_over_choice",
                    left_distributes,
                    right_distributes,
                    witness,
                );
            }
        }
    }

    violations
}

#[derive(Clone, Copy)]
enum Operation {
    Sequential,
    Choice,
    Parallel,
}

fn apply<A: GradeAlgebra>(
    algebra: &A,
    operation: Operation,
    left: &A::Element,
    right: &A::Element,
) -> Result<A::Element, A::Error> {
    match operation {
        Operation::Sequential => algebra.sequential(left, right),
        Operation::Choice => algebra.choice(left, right),
        Operation::Parallel => algebra.parallel(left, right),
    }
}

#[allow(clippy::too_many_arguments)]
fn check_associative<A: GradeAlgebra>(
    violations: &mut Vec<LawViolation>,
    law: &'static str,
    algebra: &A,
    a: &A::Element,
    b: &A::Element,
    c: &A::Element,
    operation: Operation,
    witness: &str,
) {
    let left = apply(algebra, operation, a, b).and_then(|ab| apply(algebra, operation, &ab, c));
    let right = apply(algebra, operation, b, c).and_then(|bc| apply(algebra, operation, a, &bc));
    check_equal(violations, law, left, right, witness.to_owned());
}

fn check_equal<T, E>(
    violations: &mut Vec<LawViolation>,
    law: &'static str,
    left: Result<T, E>,
    right: Result<T, E>,
    witness: String,
) where
    T: Debug + Eq,
    E: Display,
{
    match (left, right) {
        (Ok(left), Ok(right)) if left == right => {}
        (Ok(left), Ok(right)) => violations.push(LawViolation {
            law,
            witness: format!("{witness}; left={left:?}, right={right:?}"),
        }),
        (Err(left), Err(right)) if left.to_string() == right.to_string() => {}
        (Err(left), Err(right)) => violations.push(LawViolation {
            law,
            witness: format!("{witness}; left-error={left}, right-error={right}"),
        }),
        (Err(error), Ok(value)) => violations.push(LawViolation {
            law,
            witness: format!("{witness}; left-error={error}, right={value:?}"),
        }),
        (Ok(value), Err(error)) => violations.push(LawViolation {
            law,
            witness: format!("{witness}; left={value:?}, right-error={error}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grade(cost: u64, privacy: u64, energy: u64, uncertainty: u32) -> Grade {
        Grade::checked(cost, privacy, energy, uncertainty).expect("valid test grade")
    }

    #[test]
    fn operations_have_the_declared_meaning() {
        let left = grade(2, 10, 5, 600_000);
        let right = grade(3, 7, 9, 700_000);

        assert_eq!(left.sequential(right), Ok(grade(5, 17, 14, 1_000_000)));
        assert_eq!(left.choice(right), grade(3, 10, 9, 700_000));
        assert_eq!(left.parallel(right), left.sequential(right));
    }

    #[test]
    fn invalid_uncertainty_is_rejected() {
        assert_eq!(
            Grade::checked(0, 0, 0, 1_000_001),
            Err(GradeError::InvalidUncertainty {
                found: 1_000_001,
                maximum: 1_000_000
            })
        );
    }

    #[test]
    fn overflow_is_not_saturated_or_wrapped() {
        let maximum = grade(u64::MAX, 0, 0, 0);
        let one = grade(1, 0, 0, 0);
        assert_eq!(
            maximum.sequential(one),
            Err(GradeError::ArithmeticOverflow {
                dimension: Dimension::CostTicks,
                operation: "sequential composition"
            })
        );
    }

    #[test]
    fn finite_samples_satisfy_the_declared_profile() {
        let samples = [
            Grade::ZERO,
            grade(1, 2, 3, 4),
            grade(7, 5, 3, 900_000),
            grade(11, 13, 17, 200_000),
        ];
        assert_eq!(check_laws(&ProductGradeAlgebra, &samples), []);
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Word(Vec<u8>);

    struct NonCommutativeWords;

    impl GradeAlgebra for NonCommutativeWords {
        type Element = Word;
        type Error = std::convert::Infallible;

        fn zero(&self) -> Self::Element {
            Word(Vec::new())
        }

        fn sequential(
            &self,
            left: &Self::Element,
            right: &Self::Element,
        ) -> Result<Self::Element, Self::Error> {
            let mut value = left.0.clone();
            value.extend_from_slice(&right.0);
            Ok(Word(value))
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

    #[test]
    fn noncommutative_control_is_detected() {
        let violations = check_laws(
            &NonCommutativeWords,
            &[Word(Vec::new()), Word(vec![1]), Word(vec![2])],
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.law == "sequence_commutative")
        );
    }
}

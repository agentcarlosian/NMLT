use std::fmt::{self, Debug, Display};

/// One whole unit of uncertainty, represented as parts per million.
pub const UNCERTAINTY_SCALE_PPM: u32 = 1_000_000;

/// Canonical identity of the family-specific evidence contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CertificateProfileId([u8; 32]);

impl CertificateProfileId {
    pub const DECLARED_V1: Self = Self([
        0xb3, 0x5a, 0x82, 0xf6, 0xb9, 0x3e, 0xf4, 0x9d, 0x59, 0x8e, 0x4a, 0x29, 0xbd, 0xd3, 0x1d,
        0x01, 0xc0, 0x56, 0x99, 0xfc, 0x8d, 0x51, 0x62, 0x65, 0xbf, 0x24, 0xdf, 0x62, 0xf4, 0x3c,
        0x63, 0xb9,
    ]);
    pub const HOEFFDING_V1: Self = Self([
        0xc4, 0x59, 0x33, 0xa7, 0xb2, 0x72, 0x42, 0xcb, 0xef, 0xb8, 0x3d, 0x6b, 0xf0, 0x27, 0x8b,
        0xcd, 0x38, 0x50, 0x76, 0x32, 0x8a, 0xd3, 0x20, 0x78, 0xfc, 0x4c, 0xbc, 0xb5, 0xe5, 0x8f,
        0x44, 0x49,
    ]);
    pub const CONFORMAL_V1: Self = Self([
        0x5e, 0xba, 0x88, 0x65, 0xb2, 0xf7, 0xf7, 0x8f, 0x1e, 0xb9, 0x1f, 0xbf, 0xd8, 0xaf, 0x61,
        0x84, 0x00, 0x84, 0x65, 0x2e, 0xf9, 0x48, 0xb0, 0x27, 0x15, 0xb9, 0xe6, 0x5f, 0x41, 0xc2,
        0x54, 0x34,
    ]);

    pub fn from_hex(value: &str) -> Option<Self> {
        if value.len() != 64 {
            return None;
        }
        let mut digest = [0_u8; 32];
        for (index, slot) in digest.iter_mut().enumerate() {
            *slot = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
        }
        Some(Self(digest))
    }

    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

impl Display for CertificateProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// The proof obligation represented by an uncertainty upper bound.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UncertaintyFamily {
    /// A bound supplied directly as an explicitly trusted annotation.
    Declared,
    /// A bound justified by a Hoeffding-style concentration certificate.
    Hoeffding,
    /// A bound justified by a conformal-coverage certificate.
    Conformal,
}

impl UncertaintyFamily {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Hoeffding => "hoeffding",
            Self::Conformal => "conformal",
        }
    }

    #[must_use]
    pub const fn profile_id(self) -> CertificateProfileId {
        match self {
            Self::Declared => CertificateProfileId::DECLARED_V1,
            Self::Hoeffding => CertificateProfileId::HOEFFDING_V1,
            Self::Conformal => CertificateProfileId::CONFORMAL_V1,
        }
    }
}

/// A typed uncertainty certificate summary.
///
/// The family tag prevents composition from laundering unlike statistical
/// claims into one undifferentiated number. The profile identity binds the
/// summary to one exact family contract. Binding and checking a concrete
/// dataset or proof artifact remains a later profile-payload extension.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UncertaintyCertificate {
    Certain,
    UpperBound {
        family: UncertaintyFamily,
        profile_id: CertificateProfileId,
        upper_bound_ppm: u32,
    },
}

impl UncertaintyCertificate {
    pub fn checked_upper_bound(
        family: UncertaintyFamily,
        profile_id: CertificateProfileId,
        upper_bound_ppm: u32,
    ) -> Result<Self, GradeError> {
        if profile_id != family.profile_id() {
            return Err(GradeError::InvalidUncertaintyProfile {
                family,
                expected: family.profile_id(),
                found: profile_id,
            });
        }
        if upper_bound_ppm > UNCERTAINTY_SCALE_PPM {
            return Err(GradeError::InvalidUncertainty {
                found: upper_bound_ppm,
                maximum: UNCERTAINTY_SCALE_PPM,
            });
        }
        if upper_bound_ppm == 0 {
            Ok(Self::Certain)
        } else {
            Ok(Self::UpperBound {
                family,
                profile_id,
                upper_bound_ppm,
            })
        }
    }

    #[must_use]
    pub const fn upper_bound_ppm(self) -> u32 {
        match self {
            Self::Certain => 0,
            Self::UpperBound {
                upper_bound_ppm, ..
            } => upper_bound_ppm,
        }
    }

    #[must_use]
    pub const fn family(self) -> Option<UncertaintyFamily> {
        match self {
            Self::Certain => None,
            Self::UpperBound { family, .. } => Some(family),
        }
    }

    #[must_use]
    pub const fn profile_id(self) -> Option<CertificateProfileId> {
        match self {
            Self::Certain => None,
            Self::UpperBound { profile_id, .. } => Some(profile_id),
        }
    }

    fn combine(self, other: Self, choice: bool) -> Result<Self, GradeError> {
        match (self, other) {
            (Self::Certain, value) | (value, Self::Certain) => Ok(value),
            (
                Self::UpperBound {
                    family: left,
                    profile_id: left_profile,
                    upper_bound_ppm: left_bound,
                },
                Self::UpperBound {
                    family: right,
                    profile_id: right_profile,
                    upper_bound_ppm: right_bound,
                },
            ) if left == right && left_profile == right_profile => Self::checked_upper_bound(
                left,
                left_profile,
                if choice {
                    max_u32(left_bound, right_bound)
                } else {
                    left_bound
                        .saturating_add(right_bound)
                        .min(UNCERTAINTY_SCALE_PPM)
                },
            ),
            (
                Self::UpperBound {
                    family: left,
                    profile_id: left_profile,
                    ..
                },
                Self::UpperBound {
                    family: right,
                    profile_id: right_profile,
                    ..
                },
            ) if left == right => Err(GradeError::IncompatibleUncertaintyProfiles {
                left: left_profile,
                right: right_profile,
            }),
            (Self::UpperBound { family: left, .. }, Self::UpperBound { family: right, .. }) => {
                Err(GradeError::IncompatibleUncertaintyFamilies { left, right })
            }
        }
    }

    #[must_use]
    pub fn leq(self, other: Self) -> bool {
        match (self, other) {
            (Self::Certain, _) => true,
            (Self::UpperBound { .. }, Self::Certain) => false,
            (
                Self::UpperBound {
                    family: left,
                    profile_id: left_profile,
                    upper_bound_ppm: left_bound,
                },
                Self::UpperBound {
                    family: right,
                    profile_id: right_profile,
                    upper_bound_ppm: right_bound,
                },
            ) => {
                same_family(left, right)
                    && left_profile == right_profile
                    && left_bound <= right_bound
            }
        }
    }
}

/// The independent coordinates of the prototype product grade.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Dimension {
    CostTicks,
    PrivacyMicroEpsilon,
    EnergyMicrojoules,
    UncertaintyUpperBoundPpm,
}

impl Dimension {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CostTicks => "cost_ticks",
            Self::PrivacyMicroEpsilon => "privacy_micro_epsilon",
            Self::EnergyMicrojoules => "energy_microjoules",
            Self::UncertaintyUpperBoundPpm => "uncertainty_upper_bound_ppm",
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
    IncompatibleUncertaintyFamilies {
        left: UncertaintyFamily,
        right: UncertaintyFamily,
    },
    InvalidUncertaintyProfile {
        family: UncertaintyFamily,
        expected: CertificateProfileId,
        found: CertificateProfileId,
    },
    IncompatibleUncertaintyProfiles {
        left: CertificateProfileId,
        right: CertificateProfileId,
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
            Self::IncompatibleUncertaintyFamilies { left, right } => write!(
                formatter,
                "cannot compose uncertainty certificate families {} and {}",
                left.as_str(),
                right.as_str()
            ),
            Self::InvalidUncertaintyProfile {
                family,
                expected,
                found,
            } => write!(
                formatter,
                "uncertainty family {} requires profile {expected}, found {found}",
                family.as_str()
            ),
            Self::IncompatibleUncertaintyProfiles { left, right } => write!(
                formatter,
                "cannot compose uncertainty certificate profiles {left} and {right}"
            ),
        }
    }
}

impl std::error::Error for GradeError {}

/// A product upper bound over four deliberately integer-valued dimensions.
///
/// The uncertainty coordinate is a typed certificate summary, not a generic
/// scalar. Its interpretation still requires the family-specific assumptions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Grade {
    cost_ticks: u64,
    privacy_micro_epsilon: u64,
    energy_microjoules: u64,
    uncertainty: UncertaintyCertificate,
}

impl Grade {
    pub const ZERO: Self = Self {
        cost_ticks: 0,
        privacy_micro_epsilon: 0,
        energy_microjoules: 0,
        uncertainty: UncertaintyCertificate::Certain,
    };

    pub fn checked(
        cost_ticks: u64,
        privacy_micro_epsilon: u64,
        energy_microjoules: u64,
        uncertainty: UncertaintyCertificate,
    ) -> Result<Self, GradeError> {
        Ok(Self {
            cost_ticks,
            privacy_micro_epsilon,
            energy_microjoules,
            uncertainty,
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
    pub const fn uncertainty(self) -> UncertaintyCertificate {
        self.uncertainty
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
            uncertainty: self.uncertainty.combine(other.uncertainty, false)?,
        })
    }

    /// Worst-case alternative: take the componentwise upper envelope.
    pub fn choice(self, other: Self) -> Result<Self, GradeError> {
        Ok(Self {
            cost_ticks: max_u64(self.cost_ticks, other.cost_ticks),
            privacy_micro_epsilon: max_u64(self.privacy_micro_epsilon, other.privacy_micro_epsilon),
            energy_microjoules: max_u64(self.energy_microjoules, other.energy_microjoules),
            uncertainty: self.uncertainty.combine(other.uncertainty, true)?,
        })
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
    pub fn componentwise_le(self, other: Self) -> bool {
        self.cost_ticks <= other.cost_ticks
            && self.privacy_micro_epsilon <= other.privacy_micro_epsilon
            && self.energy_microjoules <= other.energy_microjoules
            && self.uncertainty.leq(other.uncertainty)
    }

    #[must_use]
    pub const fn coordinate(self, dimension: Dimension) -> u64 {
        match dimension {
            Dimension::CostTicks => self.cost_ticks,
            Dimension::PrivacyMicroEpsilon => self.privacy_micro_epsilon,
            Dimension::EnergyMicrojoules => self.energy_microjoules,
            Dimension::UncertaintyUpperBoundPpm => self.uncertainty.upper_bound_ppm() as u64,
        }
    }
}

const fn max_u64(left: u64, right: u64) -> u64 {
    if left >= right { left } else { right }
}

const fn max_u32(left: u32, right: u32) -> u32 {
    if left >= right { left } else { right }
}

const fn same_family(left: UncertaintyFamily, right: UncertaintyFamily) -> bool {
    matches!(
        (left, right),
        (UncertaintyFamily::Declared, UncertaintyFamily::Declared)
            | (UncertaintyFamily::Hoeffding, UncertaintyFamily::Hoeffding)
            | (UncertaintyFamily::Conformal, UncertaintyFamily::Conformal)
    )
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
        left.choice(*right)
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
        Grade::checked(
            cost,
            privacy,
            energy,
            UncertaintyCertificate::checked_upper_bound(
                UncertaintyFamily::Declared,
                UncertaintyFamily::Declared.profile_id(),
                uncertainty,
            )
            .expect("valid uncertainty"),
        )
        .expect("valid test grade")
    }

    #[test]
    fn operations_have_the_declared_meaning() {
        let left = grade(2, 10, 5, 600_000);
        let right = grade(3, 7, 9, 700_000);

        assert_eq!(left.sequential(right), Ok(grade(5, 17, 14, 1_000_000)));
        assert_eq!(left.choice(right), Ok(grade(3, 10, 9, 700_000)));
        assert_eq!(left.parallel(right), left.sequential(right));
    }

    #[test]
    fn invalid_uncertainty_is_rejected() {
        assert_eq!(
            UncertaintyCertificate::checked_upper_bound(
                UncertaintyFamily::Declared,
                UncertaintyFamily::Declared.profile_id(),
                1_000_001,
            ),
            Err(GradeError::InvalidUncertainty {
                found: 1_000_001,
                maximum: 1_000_000
            })
        );
    }

    #[test]
    fn unlike_certificate_families_do_not_compose() {
        let declared = Grade::checked(
            0,
            0,
            0,
            UncertaintyCertificate::checked_upper_bound(
                UncertaintyFamily::Declared,
                UncertaintyFamily::Declared.profile_id(),
                10,
            )
            .unwrap(),
        )
        .unwrap();
        let hoeffding = Grade::checked(
            0,
            0,
            0,
            UncertaintyCertificate::checked_upper_bound(
                UncertaintyFamily::Hoeffding,
                UncertaintyFamily::Hoeffding.profile_id(),
                10,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            declared.sequential(hoeffding),
            Err(GradeError::IncompatibleUncertaintyFamilies {
                left: UncertaintyFamily::Declared,
                right: UncertaintyFamily::Hoeffding,
            })
        );
        assert!(!declared.componentwise_le(hoeffding));
    }

    #[test]
    fn family_profile_substitution_is_rejected() {
        assert!(matches!(
            UncertaintyCertificate::checked_upper_bound(
                UncertaintyFamily::Declared,
                UncertaintyFamily::Hoeffding.profile_id(),
                10,
            ),
            Err(GradeError::InvalidUncertaintyProfile { .. })
        ));
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

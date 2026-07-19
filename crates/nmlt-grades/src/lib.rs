//! A small, deterministic prototype for NMLT's graded-resource extension.
//!
//! The checker is intentionally narrow. It computes upper bounds supplied by
//! annotations; it does not infer operational cost, differential privacy, or
//! physical energy from arbitrary programs.

mod algebra;
mod checker;
mod parser;

pub use algebra::{
    CertificateProfileId, Dimension, Grade, GradeAlgebra, GradeError, LawViolation,
    ProductGradeAlgebra, UNCERTAINTY_SCALE_PPM, UncertaintyCertificate, UncertaintyFamily,
    check_laws,
};
pub use checker::{
    Analysis, BudgetDecision, Diagnostic, IterationBound, Plan, Program, Violation, analyze,
    check_budget,
};
pub use parser::{ParseDiagnostic, parse_program};

//! Independent validation of NMLT typed-elaboration certificates.
//!
//! This crate does not call the producer. It reconstructs the frozen M9-v1
//! judgments from exact resolved HIR, typed core, and untrusted certificate
//! data. Only successful checking can construct [`CheckedProgram`]. It is not
//! the behavioral semantics or a semantic prover; those live in Lean.

#![forbid(unsafe_code)]

mod check;
mod identity;
mod input;
mod wire;

pub use check::{CheckedProgram, KernelCode, KernelDiagnostic, check};
pub use identity::KernelProfileId;
pub use input::{RawCertificate, RawDerivationNode, RawObligation, RawRequiredRoot};
pub use wire::CertificateDecodeError;

//! Independent checking of NMLT elaboration certificates.
//!
//! This crate does not call the producer. It reconstructs the frozen M9-v1
//! judgments from exact resolved HIR, typed core, and untrusted certificate
//! data. Only successful checking can construct [`CheckedProgram`].

#![forbid(unsafe_code)]

mod check;
mod identity;
mod input;

pub use check::{CheckedProgram, KernelCode, KernelDiagnostic, check};
pub use identity::KernelProfileId;
pub use input::{RawCertificate, RawDerivationNode, RawObligation, RawRequiredRoot};

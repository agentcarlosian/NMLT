# Security Policy

NMLT is pre-alpha research software with no supported production release. Do
not use it to authorize safety-critical, financial, security-critical, or
irreversible effects.

## Reporting

Do not disclose an exploitable vulnerability in a public issue. Use GitHub's
private security-advisory channel when available. Include the affected
revision, impact, reproduction conditions, relevant trust boundary, and
whether the result is proven, observed, suspected, or indeterminate. Never
include credentials, private data, or destructive proof-of-concept payloads.

## Current boundary

The active trusted-component inventory is
[`security/trusted-components.toml`](security/trusted-components.toml), with
attacker stories in [`docs/threat-model.md`](docs/threat-model.md).

Rust parses, resolves, types, and emits deterministic artifacts, but it is not
the semantic prover. The retained `nmlt-kernel` checks ordinary typed
elaboration only. Lean defines the current static behavior semantics and a
separate affine authority-world layer with conditional full product-step
lifting. No theorem here establishes step existence, reachability, or an
equivalence between those two semantic layers. The Rust evaluator makes no
proof, model-checking, evidence, runtime-authority, or production-safety claim.

Source digests establish byte identity only. They do not establish authorship,
freshness, provenance, or correctness of Rust elaboration.

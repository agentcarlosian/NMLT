# Security Policy

NMLT is pre-alpha and has no supported production release. Do not use it to
authorize safety-critical, security-critical, financial, or irreversible
effects.

## Reporting

Do not disclose an exploitable vulnerability in a public issue. If a hosted
repository supports private security advisories, use that channel. Until a
private reporting channel is configured, contact a maintainer through a
previously agreed private channel.

Include the affected revision, impact, reproduction conditions, trust boundary,
and whether the result is proven, observed, suspected, or indeterminate. Never
include credentials, private data, or destructive proof-of-concept payloads.

## Trusted computing base

The pre-alpha, claim-specific trusted computing base is defined in
[`security/trusted-components.toml`](security/trusted-components.toml), and its
boundaries and attacker stories are documented in
[`docs/threat-model.md`](docs/threat-model.md). Current code performs only
lossless lexing, structural parsing, diagnostics, and unknown evidence
scaffolding. The manifest's `result_ceiling` is authoritative.

Future semantic releases must replace provisional component identities with
exact RFC 0004 identities and obtain the reviews required by `GOVERNANCE.md`.

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
[`docs/threat-model.md`](docs/threat-model.md). Current code includes the
lossless frontend, a typed/bounded provider engine, finite temporal and
refinement checks, a finite VC/certificate checker, an authority-bounded
deterministic repair evaluation, and a graded-resource prototype. Each has a
separate claim profile and `result_ceiling`; none authorizes external effects.

Current assurance artifacts bind exact source/source-set, engine, executable,
toolchain, configuration, and content-addressed trusted-component identities
where their profile requires them. These digests establish byte identity, not
authorship or freshness: NMLT has no signing, transparency-log, reproducible
bootstrap, or runtime-attestation claim. Semantic or release changes must
update the claim-specific inventory and obtain the reviews required by
`GOVERNANCE.md`.

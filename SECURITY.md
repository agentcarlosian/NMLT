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

The trusted computing base is not yet frozen. Current code performs only
structural parsing and evidence scaffolding. Future releases must publish an
explicit trusted-component manifest before making verification claims.

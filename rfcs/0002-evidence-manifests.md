# RFC 0002: Evidence manifests

- Status: Draft
- Authors: Carlosian <carlosian@agentmail.to>
- Created: 2026-07-18

## Summary

Define a canonical manifest that binds a claim to its exact artifact, method,
scope, assumptions, result class, supporting certificate or witness, negative
controls, and residual gaps.

## Motivation

The word “verified” hides crucial differences between proof, bounded model
checking, testing, monitoring, and unverifiable judgment. Machine-readable
evidence must preserve those distinctions across CLI output, CI, agents, and
runtime tooling.

## Result classes

```text
proved | model_checked | tested | monitored |
refuted | unknown | indeterminate
```

## Rules

- `model_checked` requires explicit bounds.
- `refuted` requires a witness or a reason explaining why no structured witness
  is available.
- `proved` identifies the proof checker and certificate or checked proof unit.
- `unknown` never satisfies a required promotion dimension.
- `indeterminate` prohibits blind replay of a possibly completed external
  verification effect.
- A structural parse or type check may produce evidence about well-formedness,
  but never evidence for unrelated semantic claims.

## Schema

The draft machine contract is `schemas/evidence-manifest.schema.json`.
Canonical identity and serialization are governed by accepted RFC 0004 and
`docs/artifact-identity.md`. Schema-valid legacy `structural:*` manifests are
explicitly noncanonical and have a result ceiling of `unknown`.

Every semantic result must bind a canonical source or source-set ID, claim ID,
configuration ID, engine executable digest, and trusted-component identities.
References to witnesses and certificates are content-addressed; paths are
display metadata only.

## Negative controls

- Omitted bounds on `model_checked` must fail validation.
- A missing claim identity must fail validation.
- Unknown results presented as passing must fail promotion.
- A source hash mismatch must fail binding.
- Conflicting results for one manifest identity must fail closed.

## Open questions

- Signature and transparency-log support.
- Composition rules for evidence covering multiple claims and components.
- Which checked certificate formats are accepted for each proof backend.

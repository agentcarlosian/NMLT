# RFC 0002: Evidence manifests

- Status: Draft
- Authors: NMLT project
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

## Negative controls

- Omitted bounds on `model_checked` must fail validation.
- A missing claim identity must fail validation.
- Unknown results presented as passing must fail promotion.
- A source hash mismatch must fail binding.
- Conflicting results for one manifest identity must fail closed.

## Open questions

- Canonical serialization and hashing rules.
- Certificate storage versus content-addressed references.
- Signature and transparency-log support.
- Composition rules for evidence covering multiple claims and components.

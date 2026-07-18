# Evidence model

NMLT treats verification results as typed artifacts rather than presentation
badges.

## Result classes

- `proved`: an accepted proof object covers the stated unbounded claim.
- `model_checked`: the property held within recorded finite bounds.
- `tested`: recorded concrete executions passed a stated test strategy.
- `monitored`: observed traces satisfied a monitor over a stated interval.
- `refuted`: a witness violates the claim.
- `unknown`: no accepted method has established or refuted the claim.
- `indeterminate`: verification or an external effect may have occurred, but
  authoritative terminal evidence is unavailable.

## Required fields

An evidence manifest records:

- schema version and manifest identity;
- artifact and claim identity;
- result class and verification method;
- source and engine identity when available;
- scope and finite bounds;
- assumptions and trusted components;
- structured witness or proof-certificate references;
- negative controls exercised;
- residual gaps.

The normative machine contract is `schemas/evidence-manifest.schema.json`.

## Promotion

Promotion is a vector of required evidence dimensions, not an average score.
If a required dimension is unknown, blocked, refuted, or indeterminate, the
artifact does not receive that promotion level.

## Structural evidence scaffolds

The current CLI can emit a manifest after parsing a file. Its result is always
`unknown`, its method is `structural_check`, and its residual gaps explicitly
state that semantic verification has not run.

# RFC 0027: Affine job transfer through functions

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-08

R2's scoped controls need to compose across reusable components. A direct
`Job<T>` value may move into a new binding, a function parameter, a function
result, or a bounded fold accumulator. Reading a handle as such an argument or
result consumes the old binding. Poll/cancel still borrow a named local;
collection consumes it. Every normal path must collect or transfer each handle.

Functions are checked independently with Job parameters initially available.
Branches agree on consumed outer handles. Each fold iteration consumes or
transfers its incoming Job accumulator while preserving other outer authority;
zero iterations return the initial accumulator unchanged. Normal function and
scope exits cannot silently discard a handle. Recursive function cycles remain
unsupported.

Handles still cannot enter serializable values, records, lists, outcomes,
comparisons, or externally supplied inputs/results. A function with a direct Job
parameter or result is usable as an internal component, not a CLI entry. Entry
validation rejects it before host state is created. This preserves the boundary
between reusable data/artifacts and affine control.

The evaluator has a private move-only result carrier and takes handle IDs from
their previous binding. Host sessions retain the actual non-cloneable control;
this source move does not create a new attempt or change its charged reservation.
Source replay follows the same deterministic transfers around captured host
events. This is executable-only ownership checking, not a Lean theorem about
external effects or transfer between authenticated OS principals.

Validation covers imported launch/collect components, moved local bindings,
returned handles, branch transfers, repeated and empty folds, stale references,
double argument use, unconsumed parameters, missing collection, and external
handle forgery. Existing data-only evaluation and scoped control accounting
remain regression gates. Workflow typecheck output advances to version 6.

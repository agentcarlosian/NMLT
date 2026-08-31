# NMLT RFCs

RFCs are the normative path for language, semantics, evidence, trusted-core,
and compatibility decisions.

The 2026-08-31 public pivot is recorded in
[`docs/decisions/0004-language-mathematics-pivot.md`](../docs/decisions/0004-language-mathematics-pivot.md).
Pre-pivot RFCs remain useful research records, but only Accepted proposals that
are compatible with that decision describe the active architecture.

## Statuses

- Draft
- Under review
- Accepted
- Rejected
- Postponed
- Withdrawn
- Superseded

Accepted RFCs define intended behavior but do not by themselves prove that an
implementation conforms. Conformance requires tests and appropriate formal
evidence.

## Process

1. Copy `0000-template.md`.
2. Obtain the next identifier without renumbering existing proposals.
3. Open discussion focused on the problem, semantics, alternatives, and
   evidence plan.
4. Revise the RFC until major objections are resolved or explicitly recorded.
5. A maintainer records the disposition and, for accepted RFCs, creates any
   needed decision records and implementation issues.

## Index

| RFC | Title | Status |
|---|---|---|
| 0001 | Behavior types | Under review |
| 0002 | Evidence manifests | Superseded by pivot |
| 0003 | Lexical grammar v1 | Accepted |
| 0004 | Canonical artifact identity | Partially superseded; language identities retained |
| 0005 | State and action typing v1 | Under review |
| 0006 | Explicit affine capabilities v1 | Under review |
| 0007 | Observation and stuttering semantics v1 | Under review |
| 0008 | Lean mechanization and compositional refinement | Superseded; rewrite required |
| 0009 | Finite temporal, refinement, and runtime semantics | Postponed |
| 0010 | Multiple verification engines and checked evidence composition | Withdrawn from active architecture |
| 0011 | Authority-bounded agentic formalization and repair | Postponed |
| 0012 | Conservative graded-resource modalities | Postponed experiment |
| 0013 | Integrated source-to-typed-core contract | Accepted |

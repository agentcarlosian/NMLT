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
| [0014](0014-executable-workflow-profile.md) | Executable workflow profile | Draft |
| [0015](0015-unified-resource-bearing-behavior.md) | Unified resource-bearing behavior | Under review |
| [0016](0016-decoded-finite-execution.md) | Decoded finite execution | Under review |
| [0017](0017-finite-local-run-and-replay.md) | Finite local run and replay | Under review |
| [0018](0018-bounded-local-job-lifecycle.md) | Bounded local job lifecycle and recovery | Under review |
| [0019](0019-pure-workflow-source.md) | Pure workflow source, entry points, and replay | Under review |
| [0020](0020-workflow-records-and-collections.md) | Workflow records and bounded collections | Under review |
| [0021](0021-workflow-source-packages.md) | Workflow source packages and import-bound replay | Under review |
| [0022](0022-source-local-job-effects.md) | Source-level bounded local job effects | Under review |
| [0023](0023-asynchronous-host-and-lean-adapter.md) | Asynchronous host controls and the first Lean adapter | Under review |
| [0024](0024-scoped-source-job-controls.md) | Scoped asynchronous source job controls | Under review |
| [0025](0025-local-projects-and-dependency-locks.md) | Local projects and dependency locks | Under review |
| [0026](0026-finite-source-safety-invariants.md) | Finite source safety invariants | Under review |
| [0027](0027-affine-job-transfer.md) | Affine job transfer through functions | Under review |
| [0028](0028-contained-process-lifecycle.md) | Contained local process lifecycle | Under review |
| [0029](0029-pinned-init-proof-terms.md) | Pinned Init proof terms | Under review |
| [0030](0030-durable-source-resumption.md) | Durable source resumption | Under review |
| [0031](0031-bound-lean-project-tasks.md) | Bound Lean project tasks | Under review |
| [0032](0032-lean-source-import-discovery.md) | Lean source import discovery | Under review |

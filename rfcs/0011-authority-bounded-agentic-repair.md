# RFC 0011: Authority-bounded agentic formalization and repair

- Status: Under review
- Authors: NMLT project
- Created: 2026-07-18

## Summary

An NMLT repair agent is a search component, never a claim authority. Each task
separates immutable intent/property artifacts from editable implementation
artifacts, gives the agent structured diagnostics or counterexamples, accepts
only localized source-span edits, and rechecks the complete bound claim and
its semantic negative controls. A proposal that changes a trusted digest,
widens its editable path set, removes a negative control, or converts
`unknown`/`indeterminate` into success is rejected before execution.

## Motivation

Compiler and verifier feedback can make generated repairs more useful, but a
system rewarded only for a green checker can weaken the checker input. NMLT's
initial benchmark already distinguishes three failure classes:

1. syntax or type failure;
2. a well-typed semantic counterexample;
3. a vacuous or weakened property that appears to pass.

The repair protocol must preserve those distinctions and keep the trusted
claim outside the proposal's edit authority.

## Artifact roles

Every repair task binds these roles by canonical digest:

```text
intent       trusted, review-required to change
property     trusted, review-required to change
oracle       trusted negative controls and expected witnesses
candidate    editable only within declared paths and source spans
feedback     checker-produced, read-only
proposal     untrusted span edits plus rationale
evaluation   checker-produced result graph
```

The proposal never contains a replacement intent, property, oracle, result,
or evidence manifest. Those artifacts can change only through a separate
human-reviewed task with a new identity.

## Repair protocol

```text
freeze(task identities and edit policy)
  -> check baseline
  -> emit structured diagnostic/subgoal/witness
  -> propose bounded edits
  -> reject authority violations
  -> apply in an isolated worktree or temporary directory
  -> rerun syntax, typing, semantics, and negative controls
  -> compare trusted identities
  -> emit outcome without promotion beyond the checker result
```

An edit is localized when its repository-relative path is allowlisted and its
half-open byte span lies outside every protected span. Insertions at a protected
boundary are rejected unless that boundary is explicitly listed. Path globs,
symlink escapes, generated changes to the benchmark oracle, and whole-program
replacement are forbidden in v1.

## Structured feedback

Feedback has one of these typed forms:

```text
ParseDiagnostic(code, primary_span, related_spans)
TypeDiagnostic(code, declaration, primary_span, expected, actual)
Counterexample(property_id, ordered_steps, violated_at)
Unknown(reason, bounds_or_backend)
Conflict(raw_backend_results)
```

Only the first three can directly seed repair search. `Unknown` means acquire
more evidence or change an explicitly reviewed configuration. `Conflict`
means investigate backend disagreement. Neither is a weak success signal.

## Semantic mutations

Mutation generation is property-directed. Operators are declared and named,
for example:

- delete one guard;
- replace a state update target;
- duplicate an affine capability use;
- remove a response-binding conjunct;
- enable an action in an ambiguity state.

Mutations may alter only candidate implementations. Property mutation is a
separate integrity control and its expected outcome is rejection. Mutation
score is reported per frozen property and operator; equivalent, malformed, and
out-of-scope mutants are separate categories rather than denominator tricks.

## Evaluation

The Phase 6 pilot uses held-out tasks whose trusted capsules are hidden from
the repair implementation except for the explicit structured interface. It
reports:

- baseline syntax, typing, and semantic completion separately;
- completion after each feedback round;
- localized edit count and byte extent;
- trusted-artifact modification attempts rejected;
- negative controls retained and killed;
- remaining `unknown`, conflicts, and human intent disagreements.

The first deterministic repair assistant is a protocol conformance baseline,
not evidence about general LLM capability. A future model-based agent must run
the same tasks and compare against that baseline without receiving expected
patches.

## Evidence and security requirements

- Evaluation binds the exact agent/policy/checker identities and task split.
- Checker output is untrusted until normal evidence readback succeeds.
- Source text, paper content, diagnostics, and model output are treated as data;
  embedded instructions cannot widen authority.
- A property identity change invalidates all prior repair evaluations.
- No repair is applied to production or an irreversible external effect.
- Human agreement is reported separately from machine-checker acceptance.

## Research basis

The local archive search on 2026-07-18 returned no close lexical match; that is
not novelty evidence. New/current primary leads shaped the protocol:

- [Specification-Guided Repair of Arithmetic Errors in Dafny Programs using
  LLMs](https://arxiv.org/abs/2507.03659) uses formal specifications for fault
  localization and repair validation. NMLT adopts verifier-local feedback but
  makes the paper's specification-correctness assumption an explicit trusted
  artifact boundary.
- [Property-Based Mutation Testing](https://arxiv.org/abs/2301.13615) argues
  that mutants should be judged relative to a named requirement. NMLT freezes
  property-linked mutation operators and witnesses rather than reporting one
  undifferentiated mutation score.
- [A Case Study of LLM for Automated Vulnerability Repair](https://arxiv.org/abs/2405.15690)
  evaluates reasoning plus validation feedback. NMLT retains structured tool
  feedback but does not trust a plausible patch until all claim-bound checks
  and controls rerun.
- [Verified VCG and Verified Compiler for Dafny](https://arxiv.org/abs/2512.05262)
  demonstrates why a verifier implementation itself cannot be treated as an
  axiom. NMLT's agent remains outside the kernel and its strongest results are
  limited by the checked backend/certificate path.

These works support ingredients, not NMLT's soundness or novelty. The central
research question is whether strict authority separation preserves useful
repair rates while preventing specification gaming.

## Negative controls

- A proposal that changes one property byte is rejected before checking.
- A proposal that removes a negative-control reference is rejected.
- A syntactically repaired but semantically refuted candidate remains refuted.
- A vacuous property pass fails integrity review.
- An exceeded bound remains unknown.
- A backend disagreement remains a conflict with both raw results.

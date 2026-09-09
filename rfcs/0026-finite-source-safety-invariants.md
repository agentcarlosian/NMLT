# RFC 0026: Finite source safety invariants

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-07

## Requirement and scope

R2 requires an executable user-defined safety property, including its exact
predicate, initialization and preservation obligations, and Lean-checked evidence
or a counterexample. This RFC gives `safety Name = always(predicate)` that meaning
for the existing finite, binary behavior profile. Predicates use Boolean fields
and literals, finite Bool/Unit/enum equality, negation, conjunction, disjunction,
implication, and Boolean parentheses. Unsupported temporal/resource properties,
unbounded domains, and cross-leaf field references remain explicit errors.

`check-invariant` compiles the ordinary v2 model and a separate versioned property
artifact. The existing v1/v2 artifact formats and commands retain their contracts.
The new compiler accepts safety declarations explicitly rather than dropping
them on an ordinary behavior compilation route. The property artifact preserves
the original expression bytes and byte span, its declared system/name, and the
typed predicate. The Lean checker checks that source slice and independently
parses the supported predicate grammar before comparing the supplied typed tree.
The broader source-to-model translation remains trusted Rust elaboration, as in R1.

## Evidence construction

Rust uses the same finite initializer and successors as execution/exploration.
If it finds a violation, it supplies an initialized execution path. Lean checks
that path against the actual unified step relation and checks that the final
state falsifies the selected predicate. This is a reachable counterexample, not
an arbitrary noninductive state.

Otherwise Rust supplies a candidate reached-state set. Lean independently
enumerates the complete finite control/authority universe and action universe,
including the existing sentinels. Generic completeness theorems justify those
enumerations. It checks that every initial state belongs to the candidate set
and satisfies the predicate, and that every actual step from that set remains
in it and satisfies the predicate. The checked initialization/preservation
conditions imply the predicate for every reachable state through the existing
unified path relation. Rust graph completeness is not an assumption of that proof.

Formation and at least one admitted initial state are required. Bounds on the
entire semantic universe and total obligation work are checked before enumeration.
Truncated exploration with no violation yields an explicit incomplete result,
never a safety certificate. Neither fairness nor liveness is claimed.

## Trust, compatibility, and validation

The compiled Lean checker and its installation remain a runtime trust boundary;
the theorem definitions and proofs are included in the existing axiom audit and
pinned NanoDA gate. No axiom policy is weakened. Output identities bind the exact
source, core, predicate, witness, checker executable, and observed checker result.
A finite model invariant does not establish the same property for external
workers without the stated abstraction/simulation obligation.

The bounded native checker uses the same explicit 64 MiB Lean thread stacks
and allocator arena reservation as the Lean job adapter. This permits startup
under the existing Linux data-segment limit; process bounds and invariant
acceptance checks remain unchanged.

Positive tests include Boolean/enum predicates, a nontrivial preserved property,
and an invariant requiring the reached-set strengthening. Negative tests include
an initial violation, a reachable later violation, omitted states, altered steps,
predicate/type/source mutations, formation errors, unsupported syntax, and
exhausted bounds. Rechecking the retained inputs must reproduce the Lean verdict.

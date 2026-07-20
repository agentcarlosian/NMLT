# Roadmap

Dates are records or planning ranges, not release commitments. The detailed
gates and residual gaps live in [`Plan.md`](../Plan.md). “Complete” below means
complete only at the explicitly bounded research scope; it never means a
stable, general-purpose, verified language.

## Phase 0 — Foundation and research contract (complete 2026-07-18)

Apache-2.0 contribution terms, ten identity-frozen canonical examples,
NMLT/TLA+/Quint/P comparison fixtures, candidate behavior rules, a
claim-specific TCB threat model, and source/evidence identity requirements.

## Phase 1 — Syntax and semantic skeleton (complete 2026-07-18)

Lossless tokens and immutable CST, deterministic recovery, stable spans,
preservation formatting, declaration/action shells, and a partial untyped-core
projection. Raw expressions and surface-only declarations retain no inferred
semantic assurance.

## Phase 2 — Typed behavioral core (complete for the provider slice 2026-07-18)

Contextual provider elaboration, typed state/actions, explicit frames, affine
provider capability, and a Rust/Lean correspondence vector with checked kernel
theorems. Full surface-to-kernel compiler correctness is not claimed.

## Phase 3 — Behavior execution (bounded implementation complete 2026-07-18)

Deterministic bounded BFS accepts one frozen reference and refutes four
semantic mutants with source-bound structured traces. Suite v2 corrects
`NoBlindReplay` to current-state enabledness and freezes a distinguishing
one-shot regression. A fresh clone of
`e3f7ec6ae2d14ade78183ff78d58f7198cb76858` reproduced the complete bounded
gate; this closes Phase 3 at its stated scope, not as unbounded proof.

## Phase 4 — Temporal properties and refinement (finite-fixture milestone complete 2026-07-18)

Finite fair-lasso checking, weak/strong fairness, stuttering, hiding,
one-step forward simulation, and accepted/rejected/unknown runtime-journal
conformance with independent evidence replay on a canonical finite fixture.
Provider `NoBlindReplay` is also checked with finite `always` semantics over a
nine-state reference observation graph; the blind mutant has a zero-step stem
and identity-stutter infinite lasso that the Python harness independently
replays. The source-to-observation-graph projection is manually audited, not
compiler-derived. No general temporal language, infinite-state, or
liveness-refinement proof is claimed.

## Phase 5 — Multiple verification engines (complete for one finite VC 2026-07-18)

One manual two-observable provider VC is checked by deterministic reachability
and separate finite inductiveness enumeration. Certificates are checked and
disagreement fails closed. The `proved` result applies only to that exact VC,
not to the full NMLT source translation.

## Phase 6 — Runtime and agentic workflow (complete as a deterministic baseline 2026-07-18)

Protected edit authority, structured feedback, localized repair, held-out
fixtures, negative controls, and a source-bound artifact graph linked to a
synthetic drift event. The three-task result is protocol-conformance evidence,
not an LLM capability or production-runtime claim.

## Phase 7 — Independent research extensions (one prototype track complete 2026-07-18)

The first track checks a product of declared cost, privacy, energy, and
uncertainty upper bounds in a separate annotated-plan language. Trusted
annotations, no privacy/physical model, and no verified correspondence between
the Rust analyzer and its kernel-checked Lean algebra keep its promotion gate
closed. Cubical, hybrid, probabilistic, alternative-grade, and open-system
tracks remain future work.

## M8 integration and release hardening (complete 2026-07-18)

- The full bounded gate reproduced from an independent clean clone.
- The final TCB and evidence identities passed adversarial readback.
- Every research slice retains a distinct assurance subject and ceiling.
- The `0.1.0` decision is “not yet”; no tag was created because P has not been
  rerun after correction and source-to-graph/VC/compiler correspondences remain
  manual or unverified.

## M9 — Integrated source-to-typed-core contract (complete 2026-07-19)

M9 replaces the current split frontend/provider parsing paths with one
identity-bound route from exact source bytes through lossless syntax, resolved
HIR, bidirectional elaboration, explicit typed core, and independent kernel
validation. The bounded provider engine will consume only checked core.

The first vertical slice covers explicit acyclic imports, closed enums,
primitive scalar types, system state, action inputs, guards, simultaneous
updates, `Once<T>` capability use, observations, and safety/temporal property
ASTs. Unsupported constructs must fail explicitly. Promotion requires complete
translation coverage, deterministic resolution and identities, forged/stale
certificate rejection, two-way action semantic correspondence, affine and
property-index preservation, and an identity-bound Rust/Lean bridge.

The governing proposal is [RFC 0013](../rfcs/0013-source-to-typed-core.md).
RFC acceptance and complete surface projection are implemented as M9-001 and
M9-002. M9-003 now produces a deterministic all-reference HIR, stable local
binders, and an exact-source replayed `ResolutionMap`. M9-004 defines a
span-free, identity-bound explicit typed core with structural validation and
resource ceilings. Neither result establishes HIR-to-core correspondence;
M9-005 emits identity-bound bidirectional derivations and M9-006 independently
replays them into opaque `CheckedProgram` values. M9-007 removes the second
engine parser, M9-008 binds and reads back the complete semantic identity
chain, M9-009 checks the extrinsic Lean correspondence model and shared
Rust/Lean vectors, and M9-010 freezes the canonical outcomes and reproduction
gate. Resolver or elaborator success alone is never a `CheckedProgram`.
Once this semantic spine is trustworthy, work proceeds to deeper temporal,
refinement, compositional, and quantitative mathematics and to broader
independently run comparison models without weakening existing evidence
classes.

## M10 — Behavior refinement and typed certificates (bounded seed complete 2026-07-19)

M10 establishes behavior-indexed temporal typing, constructive and classical
property families, finite-stuttering refinement laws, a checked Rzk seed,
untrusted certificate simplification with proof-DAG metrics, typed uncertainty
families with exact profile identities, finite coinductive/up-to certificates,
and a narrow checked-core-to-behavior bridge. Each result retains its own
subject identity and evidence ceiling.

This milestone does not establish fairness-complete liveness, validate
statistical side conditions, place Rzk in the TCB, admit arbitrary cyclic
certificates, or prove full source-to-behavior preservation.

## M11 — Open composition and evidence-bearing mathematics (active 2026-07-19)

M11-001a is complete at a finite exact-action safety scope. Rust checks global
input receptiveness, strict symbolic discharge, explicit one-to-one
synchronization, boundary/refinement compatibility, and bounded product
construction. Lean separately proves axiom-free one-sided structural product
congruence, composability preservation, and product receptiveness under its
arbitrary exact-wiring model. No Rust/Lean correspondence theorem is claimed.

M11-001b is complete at a finite contract-sound scope: canonical nominal
payload identities, finite assumption/guarantee predicates, exact-type
noncircular discharge, complete injective label renaming, contravariant
assumptions, covariant guarantees, and axiom-free identity/composition laws.
The [claim-specific manifest](../benchmarks/results/open-refinement/m11-001b-evidence.json)
binds the exact Rust controls, Lean declarations, sources, and limitations.
M11-001c's finite core now checks two-sided product lifting, remaining product
contracts, exact edge-bijective wiring, and finite invariant transport. Lean
separately proves the label-mapped two-sided core without axioms, including
complete boundary bijections, mapped wiring, contract variance, and a positive
instance with distinct concrete/abstract port types. The
[M11-001c manifest](../benchmarks/results/open-congruence/m11-001c-evidence.json)
now binds the resource-aware Rust checker, eleven controls, the Lean all-case
resource theorem, and an executable finite-table certificate checker. Accepted tables carry
typed `Fin` maps and exact nominal payload identity; Rust rejects nonuniform
payload universes, hidden maps, non-surjective state tables, shared authority,
nonmonotone grades, undischarged relies, and mutated canonical maps. Rust
revalidates the isolated certificate, while Lean proves a quantified semantic
contract for every accepted certificate and bundles behavioral, contract, and
resource refinement across all eight product-action constructors. The full
substage stays open because the Rust encoder and validator are not verified
extraction. Source/behavior preservation,
constructive temporal evidence, artifact-bound
uncertainty, fairness, theorem-bound up-to closures, certificate cost evidence,
the proof-relevant Segal experiment, and provenance synchronization remain
later M11 gates. See the
[reboot handoff](reboot-handoff-2026-07-19.md) for the prior continuation
state. M11-001c remains the active objective.

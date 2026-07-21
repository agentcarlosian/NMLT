# M10 behavior, refinement, and certificate research

Date: 2026-07-19

This note records the evidence used to begin the seven post-M9 research
threads. It distinguishes results recovered from Carlosian's local archive
from sources consulted during this work; neither category is itself proof of
an NMLT claim.

## Archive upgrade and archive findings

The original `~/research-archive/math_frontier_offline_index.json` contained
2,334 recent paper records but was not directly compatible with the archive
collector's expected top-level `papers` shape and lacked explicit quality and
provenance fields. The reproducible v2 builder now combines that immutable
input with a reviewable curated-source manifest and an eight-loop research
manifest. It emits `math_frontier_search_index.json` with all original records,
ten foundational additions, eight reviewed overlays, canonical work/version
identity, exact input hashes, gap records, cross-reference facets, conservative
screening flags, and explicit live-source failures. The builder has a
deterministic `--check` mode. It does not invent citations, peer-review status,
or withdrawal conclusions.

Focused archive searches surfaced:

- [Rzk: A Proof Assistant for Synthetic ∞-Categories](https://arxiv.org/abs/2607.12207),
  which motivates an isolated directed-refinement experiment;
- [Finding Simple Proofs](https://arxiv.org/abs/2607.08753), which motivates
  proof-DAG size/depth metrics and untrusted simplification;
- [Intuitionistic Dynamic Logic](https://arxiv.org/abs/2607.13528), which
  reinforces the need to separate constructive evidence from classically
  interpreted temporal propositions;
- [Contraction Certification](https://arxiv.org/abs/2607.11982) and
  [Proof-Carrying Covering Codes](https://arxiv.org/abs/2606.09600), which
  support keeping search/simplification outside a small independent checker.

The live arXiv collector was rate-limited or timed out on several focused
queries. Those failures are recorded as missing current-search coverage, not
as negative evidence.

## Current primary sources consulted separately

The current [Rzk reference](https://rzk-lang.github.io/rzk/en/latest/reference/introduction.rzk/)
documents `rzk-1`, its three universe layers, and its experimental status. It
also explicitly states that the implementation currently assumes `U : U` and
is unsound. Consequently, Rzk v0.10.0 is used only as an external research
instrument; the experiment is not in NMLT's trusted computing base.

For coinductive certificates, Bonchi, Petrişan, Pous, and Rot's
[fibrational account of coinduction up to](https://arxiv.org/abs/1401.6675)
and Madiot, Pous, and Sangiorgi's
[compatible-functions treatment](https://arxiv.org/abs/2001.07063) motivate
the explicit monotonicity and compatibility obligations in the Lean seed.
NMLT additionally requires extensiveness and idempotence-below for the exact
closure theorem currently mechanized. This is a deliberately sufficient
interface, not a completeness claim about all sound up-to techniques.

## Implemented consequences

1. Behavior identity is a type index in Lean; typed actions carry their grade
   in each admitted step and traces preserve exact identity stutter.
2. Classical temporal truth and constructive positive evidence use separate
   syntax. Constructive implication and negation remain open rather than
   silently invoking choice or excluded middle.
3. Directed forward refinements compose and preserve reachability and state
   invariants. Liveness transport is withheld until fairness and divergence
   obligations exist.
4. The Rzk seed checks one-way refinement maps and pointwise category laws on
   v0.10.0. A Segal-type model of proof-relevant simulations remains open.
5. `nmlt-certificate-tools` prunes unreachable raw proof-DAG records and
   reports roots, nodes, edges, maximum depth, fan-in, and canonical bytes.
   Its result remains untrusted until `nmlt-kernel::check` accepts it.
6. Uncertainty is no longer one untyped ppm scalar. Each nonzero bound names a
   `declared`, `hoeffding`, or `conformal` certificate family and its exact v1
   profile identity. Unlike or substituted profiles cannot parse, compose, or
   compare.
7. The Lean coinductive seed defines a one-sided finite-stuttering simulation
   generator and proves compatible up-to certificates sound without added
   axioms. The Rust temporal checker now accepts only canonical subject-bound
   finite post-fixed relations and rejects forged, stale, noncanonical, open,
   and semantically invalid witnesses.
8. The M9-to-M10 Lean bridge embeds checked core action selection into the
   behavior-indexed model and proves the selected action preserves its indexed
   state property.
9. The Rzk experiment now packages a directed map with a proof-relevant
   observation witness and checks identity and composition with path
   concatenation. It remains outside the TCB.

## Eight research loops

The dated loop manifest in `~/research-archive/math_frontier_research_loops.json`
records queries, archive results, new primary sources, gaps, implications, and
priority separately. Its strongest conclusions are:

1. Constructive temporal evidence still lacks a single mechanized account of
   implication, negation, until, and infinite-trace productivity suitable for
   NMLT. Positive evidence must remain distinct from classical truth.
2. Compositional open-system refinement is the sharpest semantic gap: NMLT
   needs environment assumptions, compatibility, receptiveness, and a
   composition congruence theorem.
3. Directed type theory supports proof-relevant composition as an experiment,
   but Rzk's documented `U : U` assumption keeps it outside the TCB.
4. Certificate simplification should report before/after DAG size, depth,
   checker time, memory, and acceptance equivalence while remaining untrusted.
5. Current mathlib contains machine-checked Hoeffding and Azuma-Hoeffding
   infrastructure. NMLT still needs profile payloads for the exact probabilistic
   premises; conformal profiles additionally need identity-bound calibration
   data, split, alpha, and coverage semantics.
6. Up-to closures should enter executable certificates only when a kernel
   theorem is named and bound to that exact closure identity. Basic finite
   simulation stability comes first.
7. Verified-compiler work reinforces making source-to-core preservation a
   universal theorem over the supported fragment, not a set of examples alone.
8. W3C PROV, DataCite version relations, and Crossref update metadata motivate
   separate work, version, retrieval, curation, and generated-artifact
   identities. Missing status metadata is never evidence of current validity.

The bundled live arXiv collector returned only HTTP 429 errors or timeouts for
all eight focused loop queries on 2026-07-19. The manifest records this as
missing live-API coverage. Primary arXiv pages and official documentation were
consulted separately; API failure is not treated as negative research evidence.

## Research limits

- The uncertainty profile identity names one exact proof contract but does not
  yet bind a certificate artifact, dataset, calibration split, or validate
  Hoeffding/conformal side conditions.
- The mathematical grade algebra models a fixed-family slice; verified
  correspondence to the Rust partial cross-family algebra is still open.
- The Rzk witness carries map and observation evidence, not Lean's
  proof-relevant step simulation or a contractible Segal composition space.
- The executable coinductive relation is an in-memory temporal checker input,
  not yet canonical evidence wire syntax accepted by `nmlt-kernel`; it proves
  neither fairness nor absence of hidden divergence.

## M10 gate decision

M10's bounded seed gate is complete. This closure means the four promotion
obligations—M9 behavior linkage, exact uncertainty-profile binding,
subject-bound executable finite coinductive checks, and proof-relevant Rzk
witness composition—have positive and negative controls at their stated
scope. It does not raise the language assurance ceiling. The research-loop
gaps above are promoted to M11 rather than being silently counted as M10
successes.

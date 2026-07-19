# M10 behavior, refinement, and certificate research

Date: 2026-07-19

This note records the evidence used to begin the seven post-M9 research
threads. It distinguishes results recovered from Carlosian's local archive
from sources consulted during this work; neither category is itself proof of
an NMLT claim.

## Archive upgrade and archive findings

The original `/home/carlos/Math/math_frontier_offline_index.json` contained
2,334 paper records but was not directly compatible with the archive
collector's expected top-level `papers` shape and lacked explicit quality and
provenance fields. The adjacent reproducible builder now emits
`math_frontier_search_index.json` with all original records, two curated
formal-methods records absent from the input, lexical topic tags, version and
discovery metadata, conservative screening flags, and an explicit statement
that no authoritative citation graph, venue status, or withdrawal registry is
present. The builder has a deterministic `--check` mode. It enriches retrieval
metadata; it does not invent citations or peer-review status.

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
   `declared`, `hoeffding`, or `conformal` certificate family, and unlike
   families cannot compose or compare.
7. The Lean coinductive seed defines a one-sided finite-stuttering simulation
   generator and proves compatible up-to certificates sound without added
   axioms. Full bisimulation, fairness, divergence, and executable certificate
   encoding remain future work.

## Research limits

- The typed uncertainty family tag names a proof obligation but does not yet
  bind a certificate artifact identity or validate Hoeffding/conformal side
  conditions.
- The mathematical grade algebra models a fixed-family slice; verified
  correspondence to the Rust partial cross-family algebra is still open.
- The Rzk function model does not yet encode Lean's proof-relevant step
  witnesses or contractible composition spaces.
- Coinductive relations are a metatheory experiment, not yet certificate wire
  syntax accepted by the independent Rust kernel.

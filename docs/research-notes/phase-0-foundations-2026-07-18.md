# Research note: Phase 0 foundations

- Search date: 2026-07-18
- Method: local research archive plus current arXiv and primary project/RFC
  documentation

## In the archive

The broad behavior-type and proof-certificate queries produced little exact
local recall. The strongest surfaced item was
[AxDafny: Agentic Verified Code Generation in Dafny](https://arxiv.org/abs/2606.32007),
which treats generated executable code and proof artifacts as distinct outputs
checked by Dafny. Its implication for NMLT is operational rather than a novelty
claim: agent output stays outside trust until a named checker accepts an exact
artifact.

The absence of close archive matches is recorded rather than interpreted as
evidence of novelty.

## New/current leads

- [Asynchronous Composition of LTL Properties over Infinite and Finite
  Traces](https://arxiv.org/abs/2312.14831) emphasizes explicit projection and
  stutter-invariance conditions for composing local temporal properties. RFC
  0001 therefore does not assume arbitrary liveness preservation.
- [Formally Verified Liveness with Multiparty Session Types in
  Rocq](https://arxiv.org/abs/2605.23633) is a 2026 preprint showing the value of
  mechanizing projection, subtyping, operational correspondence, safety, and
  liveness rather than leaving behavioral typing at paper notation.
- [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785.html) supplies independent
  canonical JSON rules; its verified negative-zero erratum is adopted as a
  rejection rule for evidence identity.
- The [Quint language manual](https://quint.sh/docs/lang) makes action/state/
  temporal modes and consistent update effects explicit. That directly shaped
  the comparative model's frame conditions.
- The [P state-machine](https://p-org.github.io/P/manual/statemachines/) and
  [monitor](https://p-org.github.io/P/manual/monitors/) manuals distinguish
  executable queued actors from synchronously composed safety/liveness
  observers, which shaped the P comparison and NMLT observation boundary.
- The local TLA+ Hyperbook extraction supplied the one-bit clock, Euclid,
  mutual-exclusion, bounded-channel, fairness, stuttering, and refinement
  teaching spine. The local Technicusverus model supplied authorization,
  dispatch, ambiguity, and replay constraints.

## Implication for NMLT

The defensible thesis is not “these ingredients have never existed.” It is
that NMLT will test whether behavior-indexed temporal properties, explicit
linear/graded authority, proof-relevant refinement, canonical evidence, and
runtime conformance can share one small mechanized core and usable language.
Phase 0 freezes the tests of that thesis; Phase 1 begins implementation without
promoting the hypothesis to a result.

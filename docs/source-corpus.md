# Source corpus and provenance

The project began from three local collections supplied by the project owner.
They are mutable research inputs, not vendored dependencies or authorities for
an NMLT result. This record distinguishes a point-in-time local observation
from the content-addressed artifacts that may support a released claim.

## TLA+ Hyperbook extraction

Origin at project creation:

```text
/home/carlos/Downloads/hyperbook
```

Primary themes: state-based reasoning, actions and behaviors, safety and
liveness, fairness, stuttering, refinement mappings, mathematical and
executable definitions, and structured proof.

This directory is not a Git worktree. On 2026-07-18, a GNU-tooling inventory
observed 1,090 regular files totaling 29,912,671 bytes. The inventory digest
was:

```text
sha256:6654461051e12a17ef0667abd92458087e36cfaa674e7736fbe4257d9caf9876
```

It is the SHA-256 of the concatenated, path-bearing `sha256sum` records emitted
by this exact command from the corpus root:

```bash
find . \
  \( -type d \( -name .git -o -name .cache -o -name __pycache__ \
    -o -name target -o -name .venv -o -name venv \) -prune \) \
  -o -type f -print0 \
  | sort -z \
  | xargs -0 sha256sum \
  | sha256sum
```

The digest is an inventory observation, not a canonical snapshot identity: the
directory can change, GNU path serialization is part of the method, and no
corpus-wide license conclusion is asserted here.

## Agentic formal-methods corpus

Origin at project creation:

```text
/home/carlos/Documents/Code/agentic-formal-methods-papers
```

Primary themes: oracle routing, structured counterexamples, specification
strength, drift taxonomy, trust chains, reusable contracts, model checking,
SMT and proof sidecars, and human intent checkpoints.

This directory is also a mutable, non-Git collection. It includes large
derived and cached material, so NMLT does not claim a corpus-wide snapshot
identity for it. No files or source text from this collection were copied into
NMLT. Where an idea from a paper matters to a claim, the NMLT document cites
the paper or primary project page directly instead of treating the local
collection path as evidence.

## Technicusverus

Origin at project creation:

```text
/home/carlos/Documents/Code/technicusverus
```

Primary themes: durable effect boundaries, authorization before dispatch,
ambiguity without blind replay, evidence binding, negative controls, runtime
trace refinement, assurance vectors, and explicit residual gaps.

The local Git worktree was clean at revision
`0ee33a2cd62d8e179b9ef3d3cfd7547529a68284` when observed on 2026-07-18. Its
README declares `MIT OR Apache-2.0`; the only checked-in `LICENSE` artifact is
the MIT text with SHA-256
`078e565e57c7ede91a9b49c69ab4efee5d06d0cd9aa0f303d58a700c73c8bce2`.
Accordingly, the independently re-encoded provider-attempt benchmark binds the
MIT artifact, not an inferred Apache license choice. Its release-relevant
source paths, hashes, license, and older frozen upstream revision
`a6802d9e13500113d096f4f66f806d0dc26248fc` are recorded in
[`benchmarks/provider-attempt/provenance.json`](../benchmarks/provider-attempt/provenance.json).
The mutable checkout's current revision does not silently replace that frozen
benchmark identity.

## Identity boundary

- A local path identifies where research was consulted; it does not identify
  immutable content.
- A Git revision identifies committed repository state, but a benchmark must
  additionally bind the exact imported paths and license artifact.
- A directory inventory digest is reproducible only with its stated file
  selection and serialization method. It is not promoted to evidence identity.
- Direct links to primary external papers, specifications, and tool
  documentation are used for citations. Archive membership is labeled as
  local research context rather than evidence that a current external source
  still says the same thing.

## Import policy

- Do not copy third-party or source-corpus content without recording its
  license and provenance.
- Prefer small original NMLT examples derived from concepts rather than copied
  text or source.
- Every imported benchmark records origin, transformation, license, and exact
  revision when available.
- Historical evidence artifacts remain in their source repository and are not
  rewritten inside NMLT.
- A content-addressed NMLT result binds its canonical NMLT source, property,
  engine/source set, bounds, and witness or certificate separately from these
  research-corpus observations.

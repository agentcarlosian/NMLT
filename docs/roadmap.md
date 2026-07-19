# Roadmap

Dates are planning ranges, not commitments. Advancement depends on exit gates.

## Phase 0: charter and corpus (complete 2026-07-18)

- Freeze design principles and non-goals.
- Freeze ten canonical examples for v1; expand only through a new corpus
  version.
- Establish RFC, decision, evidence, and benchmark formats.
- Compare representative encodings with TLA+, Quint, and P.

Exit: the proposed language has a clear negative space and testable thesis.

## Phase 1: syntax and semantic skeleton (started 2026-07-18)

- Implement the accepted lossless lexical contract and green-tree design.
- Define static, operational, and trace semantics from RFC 0001.
- Select resource and capability algebra interfaces.
- Define and mechanize composition and refinement judgments.

Exit: the kernel fragment has no known unsound placeholders.

## Phase 2: executable alpha

- Build a lossless parser, resolver, type checker, formatter, and interpreter.
- Stabilize diagnostics and source mapping.
- Execute the initial example corpus.

Exit: deterministic execution and reproducible frontend behavior.

## Phase 3: verification

- Implement explicit-state exploration and structured counterexamples.
- Add symbolic and proof backend protocols.
- Bind results into evidence manifests.
- Exercise seeded negative controls.

Exit: all required provider-attempt mutants are detected and classified.

## Phase 4: refinement and runtime

- Add trace adapters for concrete runtimes.
- Generate model-based tests and runtime monitors.
- Detect typed specification drift.

Exit: a concrete durable controller demonstrably refines its NMLT model.

## Phase 5: agentic workflow

- Add progressive formalization and compiler-guided repair.
- Evaluate semantic mutation and intent review.
- Enforce edit authority between trusted specifications and generated code.

Exit: agent assistance improves measured semantic outcomes without weakening
trusted claims.

## Phase 6: research extensions

Evaluate cubical equality, hybrid dynamics, probability, and additional
quantitative modalities as independent extensions.

Exit: each accepted extension has metatheory, implementation, backend support,
negative controls, and representative benchmarks.

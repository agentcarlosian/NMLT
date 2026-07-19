# M9 bidirectional elaboration research note

Date: 2026-07-19
Scope: M9-005 bidirectional elaboration and proof-relevant certificate output

## Question

What is the smallest useful elaborator boundary that can translate NMLT's
resolved supported fragment into explicit typed core without confusing a
producer's success with independently checked evidence?

## Archive and current-paper search

The `search-the-archives` workflow searched the local archive, Hugging Face
Papers, and arXiv for `bidirectional typing elaboration certificate`,
`proof producing type checker derivation DAG`, `elaboration correctness typed
intermediate language`, and `proof relevant typing derivations`.

The local archive returned no strong lexical match for these focused queries.
That absence is recorded rather than treated as evidence against the design.
Current arXiv retrieval supplied the relevant leads below; results such as
Generative Compilation were screened out because they did not establish the
receiver-checkable typing boundary needed here.

## Evidence used

- Dunfield and Krishnaswami's survey, [Bidirectional Typing](https://arxiv.org/abs/1908.05839),
  motivates separating synthesis from checking and making expected types an
  explicit input. NMLT follows this for contextual literals and term roots.
- Eisenberg's [Dependent Types in Haskell: Theory and Practice](https://arxiv.org/abs/1610.07978v2)
  reinforces the architectural value of elaborating richer source constructs
  into a smaller explicit typed language. NMLT adopts the boundary, not its
  dependent-type feature set.
- [Proof Relevant Corecursive Resolution](https://arxiv.org/abs/1511.09394v1)
  treats resolution evidence as structured proof terms. It supports retaining
  rule-local witnesses and ordered dependencies rather than recording only a
  Boolean success result; NMLT does not adopt its corecursive resolution logic.
- [Circular (Yet Sound) Proofs](https://arxiv.org/abs/1802.05266v2) is useful
  mainly as a boundary warning: cyclic proof structures require an explicit
  soundness discipline. M9-005 emits an acyclic, fully reachable derivation DAG
  and leaves independent cycle/readback rejection to M9-006.

These papers motivate design structure. They do not prove the Rust
implementation, the NMLT typing rules, or source-to-core semantic preservation.

## M9-005 decision

1. Use bidirectional judgments: declaration roots provide checking contexts;
   outputs and observations synthesize scalar types; bare nonnegative literals
   synthesize `Nat` but may check directly as `Nat` or `Int`.
2. Keep all conversion explicit. The only v1 numeric conversion node is
   `IntFromNat`; contextual literal selection is a distinct typing rule rather
   than an implicit coercion.
3. Separate scalar `Bool`, `StateProp(system)`, and `TemporalProp(system)`.
   Temporal operators and enabledness are dedicated core nodes. A Boolean
   state condition used as a formula receives a deterministic inserted
   `StatePredicate` node.
4. Reconstruct actions from resolved roots: typed parameters, Boolean guards,
   target-checked simultaneous updates, the exact complement frame, scalar
   outputs, and affine capability consumption.
5. Emit one canonical certificate DAG whose node IDs bind numeric rule tags,
   obligation keys, conclusions, minimal witnesses, and ordered premises. The
   certificate additionally binds source, module map, canonical surface, HIR,
   core, ruleset, and resource-policy identities.
6. Before returning, require exact root-obligation coverage, exact coverage of
   HIR node origins, existing premises, and full reachability from required
   roots. These producer checks prevent accidental incomplete artifacts but do
   not replace the independent M9-006 checker.

## Assurance boundary

M9-005 establishes that the current trusted Rust elaborator produced a
structurally valid typed core and a reconstructible derivation artifact for the
supported fragment. The elaborator, HIR resolver, typed-core validator,
identity encoders, Rust toolchain, SHA-256 implementation, and host remain in
this claim profile's TCB.

It does not establish kernel acceptance, semantic preservation, temporal
satisfaction, execution safety, proof, or a verified compiler theorem.
`CheckedProgram` remains unavailable until M9-006 independently replays the
certificate under checker-selected rules and limits.

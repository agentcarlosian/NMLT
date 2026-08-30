# NMLT execution plan

- Status: pre-alpha language and mathematics research
- Active architecture: Rust frontend, canonical behavioral core, Lean semantics
- Current milestone: finite resource-aware open composition and refinement
- Next milestone: behavior-indexed fairness after the safety/resource core is stable
- Updated: 2026-08-30

## Current milestone

The first vertical slice is:

```text
.nmlt source
  → resolved and typed behavioral core
  → deterministic behavior-core-v1 JSON
  → Lean decoding and conditional composition/refinement theorem
  → non-verifying Rust exploration
```

Its language boundary includes finite Bool/Unit/enum state, explicit
observations, typed input/output ports, affine capabilities, additive named
grades, nominal rely/guarantee facts, binary compositions, explicit
connections, and total refinement state maps.

Its mathematical boundary includes the actual resource-bearing step relation,
formed binary products, resource-aware weak refinement, and the conditional
`liftParallel` theorem. Permanent controls isolate the need for wiring,
hidden-boundary, ownership, transfer, grade, contract, and hidden-resource
premises.

The milestone gate requires:

- exact positive source/artifact reproduction;
- distinct compiler failures for every negative fixture;
- canonical artifact decoding and stale/tampered rejection;
- non-verifying exploration of state change, transfer, and grade addition;
- Lean build with no `sorry` and an explicit axiom audit; and
- formatting, warnings-denied Clippy, workspace tests, and clean-package build.

## Trust boundary

Rust parsing and elaboration are not verified. The exact source digest, typed
AST, deterministic artifact snapshot, and Lean decoder make the translation
auditable. They do not constitute a compiler-correctness theorem.

The retained `nmlt-kernel` validates only the ordinary typed-elaboration
certificate. It is not the behavioral prover. `nmlt-eval` has no proof or
model-check authority.

## Deferred work

The next milestone starts from the quarantined hidden-divergence idea and adds
behavior-indexed fairness only after a new threat model and semantic design.
Until then NMLT transports no liveness property.

Also deferred:

- infinite traces and infinite state;
- probabilistic and hybrid behavior;
- general and n-ary composition;
- user-defined grade algebras;
- higher-order or partial state maps;
- verified elaboration or code generation; and
- runtime attestation.

## Historical boundary

The former contest verifier is available only from the immutable
`build-week-judge-demo-2026` tag at `0417f6e`. The reviewed post-event resource
patch is recoverable from `codex/quarantine-grok-resource-pack` and is not part
of the active theorem or compiler path.

# Architecture

## Current checkpoint

NMLT develops the language and its mathematics together. This stack layer has
one promoted finite path:

```text
exact .nmlt bytes
  → lossless CST and surface projection
  → name resolution and typed elaboration
  → behavior-core-v1 JSON
  → Lean decoding and artifact-derived theorem instance
  → non-authoritative Rust exploration
```

Rust owns the language frontend, typed IR, artifact production, and reference
explorer. Lean owns the current behavioral definitions and theorem statements.

## Rust boundary

- `nmlt-core` preserves exact syntax and projects explicit surface structure.
- `nmlt-hir`, `nmlt-elaborate`, and `nmlt-ir` resolve and type the supported
  language fragment.
- `nmlt-kernel` remains an ordinary typed-elaboration validator; it does not
  prove behavioral claims.
- `nmlt-compile` attaches complete resource profiles to typed actions and emits
  canonical `behavior-core-v1` data.
- `nmlt-eval` explores finite artifacts with `assurance: none`.
- `nmlt-cli` exposes structural checks, typing, elaboration, and exploration.

## Lean boundary

`NMLT.Behavior.ResourceBehavior` defines finite behaviors, action profiles,
binary product steps, resource-aware weak refinement, product formation, and a
conditional static lifting theorem.

`NMLT.Artifact.BehaviorCore` decodes canonical artifact structure and compares
the declared source digest with supplied source bytes.
`NMLT.Artifact.SemanticClosure` reconstructs the finite behaviors,
compositions, state maps, and refinement witness carried by the primary
artifact, then instantiates the static lifting theorem. This is semantic
validation of decoded artifact contents, not a verified Rust elaboration
theorem.

## Deliberate limits

This checkpoint makes no compiler-correctness, reachability, infinite-trace,
fairness, liveness, runtime-attestation, or production-authorization claim.
Dynamic authority worlds are not part of this static layer, and no dynamic
semantics-to-static-semantics correspondence is claimed.

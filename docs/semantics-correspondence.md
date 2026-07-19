# Rust–Lean semantics correspondence

- Status: checked vectors, not a compiler-correctness theorem
- Rust implementation: `crates/nmlt-engine`
- Lean definition: `mechanization/lean/NMLT/Core/TypedCore.lean`

The Phase 2 implementation and mechanization share the following executable
kernel contract:

| Contract | Rust | Lean |
|---|---|---|
| Expressions read one frozen pre-state | `model::evaluate` before installation | `Expr.eval pre` |
| Updates are simultaneous | collect values, then install | every clause evaluates with `pre` |
| Omitted fields are frames | cloned state plus named updates | `applyUpdates_frame` |
| Guard false is blocked | action not enabled | `BlockReason.falseGuard` |
| Missing affine authority is blocked | consumed capability absent | `missingCapability` |
| Consumption cannot recreate authority | removal from ordered set | `remove_no_fabrication` |
| Properties are system-indexed | `property_behavior` index | dependent `Property system signature` |

The frozen provider correspondence vector starts with `authorized = true`,
`dispatched = false`, and one `attempt` capability. Its dispatch transition
sets `dispatched = true`, preserves `authorized`, and removes `attempt`. Lean
checks those facts in `Core/Provider.lean`; the Rust test
`lean_provider_vector_matches_the_rust_fragment` independently reproduces the
same vector and exhaustively checks its safety property.

This vector detects obvious semantic drift, including sequential instead of
simultaneous updates, implicit frame changes, or capability duplication. It
does not prove a translation theorem. Important non-corresponding surface area
remains explicit:

- Lean's mechanized value fragment is intrinsic `Bool`/unbounded `Nat`; Rust
  also has open nominal symbols and checked `i64` arithmetic.
- Lean's initial action object has one affine effect; Rust can consume several
  distinct declared capabilities.
- Rust parses source text and contextually elaborates constructor names; Lean's
  term is constructed directly and therefore does not verify that parser or
  elaborator.
- The Rust explorer, evidence serializer, and lasso/refinement engines are not
  covered by the Phase 2 preservation theorem.

Consequently the Lean results are `proved` only for the encoded kernel
theorems. Rust typechecking and model checking retain their implementation and
bounded result classes until a verified compiler or translation-validation
certificate connects complete elaborated IR to the Lean definitions.

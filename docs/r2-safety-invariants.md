# User-defined finite safety invariants

The [permit example](../examples/pivot/safety_invariant.nmlt) declares:

```nmlt
safety UseAfterReceive = always(used implies received)
```

This is checked through the explicit invariant route under
[RFC 0026](../rfcs/0026-finite-source-safety-invariants.md). Build the pinned Lean
package, then run from the repository root:

```bash
cd mechanization/lean && lake build && cd ../..
cargo run -p nmlt-cli -- check-invariant examples/pivot/safety_invariant.nmlt \
  --behavior Network --property Receiver.UseAfterReceive --max-states 32 \
  --checker mechanization/lean/.lake/build/bin/nmlt-invariant-check \
  --emit-evidence target/first-safety-check
```

On native Windows the checker filename ends in `.exe`. Evidence directories
must be new. `make r2-invariants` runs the example and independent controls.

The example's reachable-state set satisfies the property even though the bare
predicate is not inductive over arbitrary unreachable states. Lean checks a
reached-set strengthening: every admitted initial state is in the supplied set
and satisfies the predicate; every actual unified step from the set stays in
the set and satisfies the predicate. Complete state/action enumeration theorems
justify those checks. The resulting theorem covers every reachable state of the
decoded finite binary model; Rust exploration completeness is not assumed.

Select `Receiver.NeverUsed` to obtain a checked two-step counterexample, or
`Receiver.AlreadyReceived` for an initial-state counterexample. A refutation
returns a nonzero CLI exit code and retains its checked evidence. An exhausted
exploration or checker bound also returns nonzero, with no safety acceptance.

Supported predicate syntax is `always(...)` around Boolean fields/literals,
finite Bool/Unit/enum equality (`==`, `!=`), `not`/`!`, `and`/`&&`, `or`/`||`,
`implies`, and Boolean parentheses. Enum constructors are qualified, such as
`Phase.Ready`; use `unit` for the Unit value. Equality has priority over Boolean
connectives; negation precedes conjunction, disjunction, and right-associative
implication. Fields belong to the declaration's system. Unsupported temporal,
resource, cross-leaf, numeric, or host predicates are errors.

The source bound is 128 KiB, with 1–64 safety declarations. Expressions have at
most 4,096 bytes and 256 tokens; nesting is bounded. Checking supports at most
512 states in the complete semantic universe (including sentinels), 256 actions,
256 candidate reached states, four named capabilities, and one million
preservation checks. The checker has a 30-second process deadline. Truncation
never becomes a positive proof result.

Evidence contains the exact source, unchanged-format `behavior-core-v2`,
[`behavior-invariant-v1`](../schemas/behavior-invariant-v1.schema.json),
[`behavior-invariant-witness-v1`](../schemas/behavior-invariant-witness-v1.schema.json),
checker observations, and a result with input/executable identities. The predicate
artifact retains exact expression/declaration byte spans and a typed predicate.
Lean checks those source slices and independently parses the expression before
comparing its typed tree. Original behavioral compilation still rejects property
declarations; it does not silently discard them.

Recheck retained files directly using the same checker:

```bash
mechanization/lean/.lake/build/bin/nmlt-invariant-check \
  target/first-safety-check/core.json target/first-safety-check/source.nmlt \
  target/first-safety-check/invariant.json target/first-safety-check/witness.json
```

Both accepted invariants and checked counterexamples give exit zero from the
independent checker; its JSON verdict distinguishes them. The NMLT frontend
returns nonzero for the counterexample because the requested property is false.

`lean_checked_finite_model` is limited to the supplied decoded model and predicate.
System/behavior elaboration and its correspondence to source remain trusted Rust
translation. This does not establish the property for external worker execution
without a separate abstraction/simulation argument. Compiled Lean checking,
filesystem identity, and the selected checker executable remain runtime trust
dependencies. The source snapshots and hashes provide identification, not store
authentication or a verified compiler.

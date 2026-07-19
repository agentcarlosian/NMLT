# Typed executable core

- Status: implemented research fragment
- Rust boundary: `crates/nmlt-engine`
- Mathematical contract: RFCs 0005–0007

The first executable NMLT core is deliberately a vertical slice, not the full
surface language. It parses one `system`, resolves its state/action/property
names, contextually distinguishes nominal constructors from state names,
checks state and property expressions, derives complete frame sets, checks
affine capability consumption, and elaborates actions into simultaneous
pre-state-to-post-state transitions.

## Implemented judgments

For a system state signature `Sigma`, action-name set `A`, and affine
capability set `Delta`, the implementation checks the following finite
fragment of RFC 0005:

```text
Sigma |- initializer(x) : Sigma(x)
Sigma; A |- guard : Bool
Sigma; A |- update(x,e) : Sigma(x)
Delta |- consumes(action) unique and declared
Sigma; A |- property : Bool @ CurrentSystem
frame(action) = dom(Sigma) minus writes(action)
```

Every update expression is evaluated against the frozen pre-state. Updates
are installed together after all right-hand sides succeed. Every field in the
derived frame retains byte-for-byte equal runtime value. A capability is
available initially, can be consumed by one enabled transition, and is absent
from every successor thereafter. Reusing it disables that action rather than
recreating authority.

The checker reports blocked source actions exactly when a guard is false or a
required capability is absent. The Phase 3 execution profile evaluates `next`
over declared action successors. If no source action is enabled, it contributes
one identity-stutter successor so the relation remains total; stutter changes
neither state nor capability availability. It does not add identity-stutter
successors at nonterminal states. This profile is intentionally narrower than
RFC 0007 and Phase 4 universal identity-stutter closure, and a property
containing `next` must not be transported between the profiles.

## Executable boundary

The current engine supports:

- Boolean, signed-machine-integer, natural-number, and open nominal values;
- pure names, Boolean connectives, equality, comparison, addition, and
  subtraction;
- state initializers, guards, simultaneous whole-field updates, and affine
  `consume` statements;
- `always`, transition-local `next`, and `enabled(action)`;
- safety and transition-safety properties indexed by their containing system;
- declared observation field names.

The lossless Phase 1 frontend recognizes a larger syntax. The executable
parser rejects unsupported statements and indexed updates rather than giving
them approximate semantics. Ports, external inputs, resource grades, emits,
records, total functions, imports, and general temporal eventuality are not
yet executable through this crate.

Open nominal types are intentional in this provider benchmark revision. A
constructor used where a nominal value is expected is elaborated to an
explicit symbol even if an ordinary state field has the same spelling. This
handles the `phase = dispatched` constructor alongside the Boolean
`dispatched` field without a runtime namespace guess. A later closed algebraic
data resolver must reject misspelled constructors.

## Mechanization and assurance boundary

The Lean project under `mechanization/lean` is the proof boundary. Rust unit
and mutation tests demonstrate implementation behavior but do not turn the
RFC theorem statements into `proved` evidence. Until a checked correspondence
between Rust elaboration and Lean definitions exists:

- successful type checking supports the implementation's `type_checked`
  classification only;
- exhaustive finite exploration supports `model_checked`, never `proved`;
- arithmetic overflow, unsupported syntax, and an exceeded exploration bound
  fail without promotion;
- the exact Rust source set, toolchain, executable digest, bounds, assumptions,
  and witnesses remain part of persisted benchmark results.

Four type-level negative fixtures cover an undeclared update, an invalid
initializer, duplicate affine consumption, and a cross-system/unknown action
reference. The four semantic provider mutants all survive typing and are
therefore tested by explicit-state exploration rather than being misreported
as static errors.

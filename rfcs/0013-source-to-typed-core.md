# RFC 0013: Integrated source-to-typed-core contract

- Status: Accepted
- Authors: Carlosian <carlosian@agentmail.to>
- Created: 2026-07-19
- Accepted: 2026-07-19
- Milestone: M9

## Summary

Define one authoritative, identity-bound pipeline from an exact NMLT source
set to an independently checked typed `CoreProgram`. The lossless frontend
produces complete surface structure; resolution assigns stable, scoped IDs;
bidirectional elaboration makes types, frames, capability transitions, and
behavior indices explicit; and a small kernel validates an inspectable
derivation certificate before any execution or verification engine accepts the
program.

This RFC replaces the current provider engine's second source parser and
manual source-to-engine construction for the promoted slice. It does not claim
a verified compiler merely because the pipeline and certificate formats exist.

## Motivation

NMLT currently has three useful but still disconnected accomplishments:

- `nmlt-core` preserves tokens, trivia, recovery nodes, declarations, and
  stable spans in a lossless tree, then completely projects semantic CST
  origins into ordered untyped surface nodes with explicit recovery and
  unsupported cases;
- `nmlt-hir` seals that projection to exact source bytes and implements the
  closed module/import and named-declaration portion of resolution, while raw
  type/expression references and local binders still await complete map
  coverage and independent readback;
- `nmlt-engine` reparses a narrow provider fragment, resolves some names
  contextually, and constructs its own typed executable representation.

The split this RFC is closing permits semantic omissions. An unsupported declaration can remain
in the CST while the engine never sees it; an unknown name can be treated as an
open symbol; numeric types can be conflated; a temporal-looking call can be
handled as an ordinary Boolean expression; and the finite graph or VC can be
manually constructed without a compiler-checked link to source. Existing
results correctly disclose these gaps, but they cannot be promoted into a
flagship-language claim.

Recent work on correct-by-construction bidirectional elaboration, verified
source semantics and compilation, compiler trusted-computing-base analysis,
and receiver-checked proof-carrying code supports a narrow contract with an
explicit derivation and independent checker. The research basis and its
archive/current distinction are recorded in the
[initial M9 research note](../docs/research-notes/source-to-typed-core-and-project-identity-2026-07-19.md)
and the focused
[contract/resolution follow-up](../docs/research-notes/m9-contract-resolution-2026-07-19.md).

## Goals

- Make every semantically accepted source construct flow through one frontend.
- Resolve imports and names deterministically, with exact source spans and
  stable IDs.
- Elaborate the supported surface fragment into an explicit typed core.
- Emit a derivation certificate that a separate kernel can validate without
  trusting the elaborator's success bit.
- Preserve initialization, guards, action inputs, simultaneous updates,
  frames, affine capability use, observations, and property/system indexing.
- Bind source, HIR, core, certificate, elaborator, and kernel identities into
  downstream evidence.
- Give execution and verification adapters a `CheckedProgram`, never an
  unchecked `CoreProgram`.

## Non-goals

- A general-purpose dependent type theory or implicit proof search.
- Records, maps, indexed types, unrestricted functions, recursion, or effects
  in the first M9 slice.
- Compiling refinement, liveness proof, grades, open ports, code generation, or
  foreign functions in the first slice.
- Treating parser recovery as semantic acceptance.
- Proving the Rust implementation correct solely by testing certificates.
- Replacing exact source identity with a meaning-equivalence claim.

## Guide-level explanation

For a supported source set, compilation has one visible sequence:

```text
SourceSet
  -> LosslessModuleSet
  -> SurfaceProgram
  -> ResolvedProgram
  -> Elaborated { core, certificate }
  -> kernel::check(core, certificate)
  -> CheckedProgram
```

`SourceSet` binds exact bytes, logical module names, and import membership.
`LosslessModuleSet` retains all tokens, trivia, error nodes, and spans.
`SurfaceProgram` contains every declaration exactly once; nodes that cannot be
projected are explicit errors, not absent data. `ResolvedProgram` replaces
textual references with stable IDs. Elaboration checks or synthesizes types and
emits all semantic information that an engine would otherwise infer. The
kernel replays the certificate against the explicit core and accepted rules.

Illustratively:

```nmlt
system Toggle {
  state on: Bool = false

  action flip {
    set on = not on
  }

  safety IsBoolean = always(on == true or on == false)
}
```

The core does not retain `set on = not on` as a string or implicitly mutable
command. It contains a resolved state ID, an expression typed in the frozen
pre-state, an explicit write set `{on}`, a complete post-state constructor,
and a frame proof for every non-written field. `always` is a property form
indexed by `Toggle`, not an ordinary function returning `Bool`.

If source contains an unsupported record, unresolved name, recovery-dependent
expression, or cyclic import, M9 compilation fails with a stable diagnostic.
It may still be parsed and formatted; it cannot reach `CheckedProgram`.

## Initial supported fragment

The first complete vertical slice contains:

- simple ASCII logical module names, explicit acyclic imports, and an entire
  import closure present in the exact source set;
- closed enumeration declarations;
- `Bool`, arbitrary-precision `Nat`, and arbitrary-precision `Int`;
- non-parameterized system declarations with scalar state and total
  initializers;
- action inputs and total pure expressions;
- guards and simultaneous whole-state-field explicit updates;
- the `Once<T>` capability protocol used by provider-attempt;
- action emits/consumes and observation declarations;
- safety and temporal property syntax trees with explicit system indices.

The first slice rejects system-level constants/inputs, system parameters,
data/record/function declarations, ports, action grades, field/index selected
updates, resource properties, and hiding before resolution. Some of these
features remain losslessly parseable; that does not make them M9-eligible.

The implementation may land this fragment incrementally, but no partial
substage may be presented as the completed M9 pipeline.

## Reference representation

### Source and module identity

RFC 0004 remains authoritative for exact source and source-set identity:

```text
SourceEntry = (portable_repository_path, exact_bytes, SourceId)
SourceSetId = RFC0004(entries sorted by portable_repository_path UTF-8 bytes)
```

Logical module assignment is additional semantic input and therefore receives
a separate identity. For v1, a logical module is one ASCII NMLT identifier:
`[A-Za-z_][A-Za-z0-9_]*`. The module map is a bijection over the M9 source set.
Let `lp(x) = u64be(len(x)) || x`, and let `raw(ID)` be the 32 digest bytes from
a validated versioned ID:

```text
ModuleMapDigest = SHA256(
  "NMLT-MODULE-MAP\0v1\0" || raw(SourceSetId) || u64be(entry_count) ||
  concat(entries sorted by logical-module ASCII bytes)
)
entry = lp(logical_module) || lp(portable_repository_path)
ModuleMapId = "nmlt-module-map-v1:sha256:" || hex(ModuleMapDigest)
```

Resolution rejects duplicate logical modules, duplicate paths, a map that is
not a bijection over the source set, imports outside the closed map,
symlink/path-policy violations, and cycles. Import order does not affect
resolution. Source membership affects `SourceSetId`; changing only a logical
assignment affects `ModuleMapId` and every downstream semantic identity.

### Stable semantic IDs

IDs are derived from the module-map identity and complete typed declaration
paths, never allocation order:

```text
ModuleDigest = SHA256(
  "NMLT-MODULE\0v1\0" || raw(ModuleMapId) || lp(logical_module)
)
DefDigest = SHA256(
  "NMLT-DEF\0v1\0" || raw(ModuleId) || lp(DefPathEncoding)
)
NodeDigest = SHA256(
  "NMLT-NODE\0v1\0" || raw(DefId) || lp(SemanticPathEncoding)
)
```

The text prefixes are `nmlt-module-v1:sha256:`,
`nmlt-def-v1:sha256:`, and `nmlt-node-v1:sha256:`. A `DefPath` is
`u64be(segment_count)` followed by `(u8 kind_tag, lp(ASCII name))*`. Accepted
kind tags are:

| Tag | Definition kind |
|---:|---|
| `01` | type |
| `02` | constructor |
| `03` | constant/value |
| `04` | system |
| `05` | state field |
| `06` | action |
| `07` | system input |
| `08` | capability |
| `09` | property |
| `0a` | observation contract |

The full parent path is present, so `Left.x` and `Right.x` cannot collide.
Action parameters and other local binders receive owner-`NodeId`-derived
`LocalId`s rather than `DefId`s.

`SemanticPath` is an unhashed, allocation- and span-independent locator within
one definition. Named roles include initializer, guard, update target/RHS,
output, property body, observation item, operand, and call argument. Named
simultaneous updates key their paths by the target `DefPath`; repeated anonymous
siblings use a checked `u32` role index. The canonical segment encoding and its
golden vectors are maintained with the `nmlt-hir` identity implementation; a
change requires a new identity version.

M9-003b freezes the v1 segment tags: declared type `01`, action parameter
`02 || u32be(index)`, initializer `03`, guard `04 || u32be(index)`, update
target `05 || raw(target DefId)`, update value `06 || raw(target DefId)`, output
`07 || u32be(index)`, property body `08`, observation item
`09 || u32be(index)`, operand `0a || u32be(index)`, call argument
`0b || u32be(index)`, consume `0c || u32be(index)`, and capability protocol
`0d`. The path begins with `u64be(segment_count)`. `LocalId` hashes the owning
parameter node under `NMLT-LOCAL\0v1\0`. The canonical projection uses
`nmlt-surface-program-v1:sha256:` and binds source-set identity, module-map
identity, imports, declaration flavors, binders, and raw terms. The
all-reference resolved artifact uses `nmlt-hir-resolution-v3:sha256:` and
hashes source-set identity, module-map identity, and length-prefixed canonical
HIR containing flavored declarations, locals, roots, nodes, and the
`ResolutionMap`.

Source spans remain diagnostic metadata and are forbidden as identity inputs.
Locators may remain readable across some edits, but exact `DefId`/`NodeId`
values intentionally change after any source-set or module-map identity change.
The project makes no cross-edit evidence-identity promise.

### HIR

`ResolvedProgram` contains no unresolved textual reference. Every reference is
one of:

```text
ResolvedRef ::= Local(LocalId)
              | Definition(DefId)
              | StateField(SystemId, StateId)
              | Constructor(TypeId, ConstructorId)
              | Capability(SystemId, CapabilityId)
```

No generic `Symbol(String)` escape is allowed in a checked program. HIR retains
source spans and declaration provenance for diagnostics but is otherwise
independent of CST node allocation.

M9 resolution is closed and intentionally simple. Qualified lookup is exact;
unqualified lookup considers the local module and explicitly imported modules
and succeeds only when exactly one declaration exists in the required
namespace. Globs, implicit prelude entries, shadowing, and import re-export are
not part of v1. Duplicate names are rejected per namespace.

`ResolvedProgram` also contains a canonical `ResolutionMap` covering every
textual-reference origin with its namespace, spelling, and selected target.
The resolver remains a named M9 trusted component because it chooses what a
source spelling denotes. The elaboration kernel checks HIR closure, target
existence, namespace/kind/system agreement, local scope, and exact HIR/core
bindings; it does not parse imports or silently become a second lexical
resolver. A separate deterministic resolver readback check replays
`SurfaceProgram -> ResolutionMap`. Until that check is mechanized or moved
inside an accepted kernel boundary, no claim may remove the resolver from its
TCB.

### Explicit typed core

M9-004's `CoreProgram` is span-free and string-free. Each `CoreTerm` records a
`CoreNodeId`, its exact HIR-origin `NodeId`, its owning `DefId`, an explicit
`CoreType`, and one dedicated constructor. Types are `Bool`, `Nat`, `Int`,
`Enum(DefId)`, `Once(protocol NodeId)`, `StateProp(system DefId)`, and
`TemporalProp(system DefId)`. Constructors cover typed literals, locals,
state, enum constructors, fixed unary/binary operations, `IntFromNat`, Boolean
to state-predicate formation, `always`, `eventually`, `next`, `until`,
enabledness, and action occurrence. There is no ordinary call or textual symbol
constructor.

Core systems contain scalar state with total initializers, capabilities,
actions, properties, and observations. Every action carries scalar parameters,
Boolean guards, simultaneous updates, a disjoint exact frame partition over
all system state, scalar outputs, and a capability-consumption set. Every
property body has `TemporalProp(B)` for its declared system `B`; state formulas
enter that world only through explicit `StatePredicate` and temporal
constructors.

One HIR origin may require inserted type-directed nodes. `CoreNodeId` therefore
hashes `raw(HIR NodeId) || u64be(path length) || u32be(path segment)*` under
`NMLT-CORE-NODE\0v1\0`; the path has at most 32 segments and empty means direct
translation. `CoreProgramId` hashes the exact `ResolutionId` and canonical core
under `NMLT-CORE-PROGRAM\0v1\0`, with prefix
`nmlt-core-program-v1:sha256:`. Structural construction checks annotations,
scope/system indices, integer minimality, graph closure/cycles/reachability,
action frames, and the frozen resource ceilings. It does not establish
HIR-to-core correspondence and cannot produce `CheckedProgram`; those remain
M9-005/M9-006 obligations.

### Bidirectional elaboration

The principal expression judgments are:

```text
Gamma; Sigma; Delta; B |- e => A ~> t ; D
Gamma; Sigma; Delta; B |- e <= A ~> t ; D
```

They mean that surface expression `e` synthesizes or checks type `A`, emits
core term `t`, and produces derivation `D`. `B` is the behavior/system index.
Conversion is explicit and narrow. There is no implicit `Nat`/`Int`
compatibility in v1.

`Nat` and `Int` are disjoint arbitrary-precision semantic domains. A bare
nonnegative literal checks as either type through distinct expected-type rules;
without numeric evidence it synthesizes `Nat`. This literal overloading is not
a coercion. Negative literals are `Int` only; `-0` and nonminimal magnitudes
are rejected. The only v1 injection is the unshadowable surface operation
`to_int : Nat -> Int`, elaborated to the dedicated `IntFromNat` core node. No
`Int -> Nat` conversion exists. Arithmetic/comparison operands are homogeneous;
subtraction is `Int x Int -> Int` only in this slice. Runtime/model adapters may
apply explicit finite-domain bounds, but overflow or an out-of-adapter-range
value yields `unknown/unsupported`, never wrapping arithmetic or a language
type error.

Canonical core integers use a sign tag and a minimal unsigned big-endian
magnitude: no leading zero, one encoding for zero, and no negative zero.

Action elaboration extends RFCs 0005 and 0006:

```text
Gamma; Sigma; input:I; Delta; B
  |- action a ~> core_action : Action<I,O,W,Delta,Delta',B> ; D
```

The emitted core action includes its input type, guard, exact write set,
complete simultaneous post-state construction, output, capability-context
transition, observation events, and behavior index.

Property elaboration distinguishes value and temporal worlds:

```text
Gamma; Sigma; B |- p : StateProp(B) ~> p_core ; D
Gamma; Sigma; B |- q : TemporalProp(B) ~> q_core ; D
```

`always`, `eventually`, enabledness, and action predicates are dedicated core
constructors. They cannot resolve through the ordinary function namespace.

The fixed M9 ruleset bundle contains two versioned components:
`nmlt-core-typing-v1` and mandatory `nmlt-temporal-formation-v1`. The temporal
component checks formation and behavior indexing of dedicated state/temporal
constructors, including `always`, `eventually`, `next`, `until`, enabledness,
and action occurrence. It establishes neither satisfaction nor liveness. The
RFC 0007 stutter-transport ruleset is separate and rejects formulas containing
`next`; accepting temporal formation never emits `proved`.

### Certificate and kernel

The elaborator returns:

```text
ElaborationArtifact = {
  format_version,
  source_set_id,
  module_map_id,
  surface_program_id,
  resolved_hir_id,
  core_program_id,
  ruleset_bundle_id,
  resource_policy_id,
  required_roots,
  derivation_dag
}
```

Successful certificates contain no diagnostics; warnings/errors are separate
content-addressed diagnostic artifacts. The derivation is a canonical,
rule-explicit reconstruction DAG. Each node contains a fixed numeric rule tag,
one kernel-derived obligation key `(judgment_kind, HIR NodeId)`, conclusion
type/core identities, the minimal rule-local witness, ordered premise digests,
and its origin `NodeId`. Serialized context blobs are forbidden: the kernel
reconstructs `Gamma`, `Sigma`, `Delta`, and `B` from the exact HIR and accepted
premises.

Derivation nodes are hashed, excluding their own ID, under
`NMLT-DERIVATION-NODE\0v1\0`; the complete certificate is hashed under
`NMLT-ELABORATION-CERTIFICATE\0v1\0`. Canonical binary encoding uses fixed
tags, `u64be` lengths/counts, node-map order by raw node digest, and rule-defined
premise order. The kernel computes the required obligation set from HIR and
requires a bijection with certificate subjects. Unknown tags, nonminimal
integers, stale bindings, missing/duplicate coverage, cycles, unreachable
nodes, duplicate keys, noncanonical order, and resource excess fail closed.

The kernel API is conceptually:

```text
check(
  accepted_ruleset_bundle,
  accepted_resource_policy,
  exact_resolved_hir,
  exact_core_program,
  exact_certificate
) -> Result<CheckedProgram, KernelDiagnostic>
```

`CheckedProgram` has no public unchecked constructor. Engines may deserialize
core for inspection, but they must obtain a fresh kernel acceptance bound to
the same bytes before producing a semantic result.

The checker-supplied ruleset and resource policy are authoritative; a
certificate cannot select weaker values. The accepted
`nmlt-kernel-policy-v1` profile is content-addressed under
`NMLT-KERNEL-POLICY\0v1\0` and freezes these maxima:

| Dimension | Maximum |
|---|---:|
| modules | 256 |
| one source | 4 MiB |
| all source bytes | 16 MiB |
| canonical HIR bytes | 32 MiB |
| canonical core bytes | 32 MiB |
| certificate bytes | 64 MiB |
| HIR nodes | 262,144 |
| core nodes | 262,144 |
| derivation nodes | 524,288 |
| premise edges | 2,097,152 |
| premises per node | 32 |
| nesting/DAG depth | 256 |
| identifier bytes | 255 |
| logical-module or portable-path bytes | 4,096 |
| integer magnitude | 4,096 bytes |
| total integer payload | 16 MiB |
| context entries | 65,536 |

All counters are `u64` with checked addition/multiplication; declared lengths
are checked before allocation, and cycle/depth traversal is iterative. There
is no wall-clock timeout inside portable kernel acceptance. An outer watchdog
may return `unknown/resource_exhausted`. Crossing a policy bound returns stable
`KERNEL_RESOURCE_LIMIT`, constructs no `CheckedProgram`, and never means that
the source property was refuted.

## Required semantic correspondence

For each supported system, acceptance ultimately requires mechanized
statements of:

1. **Coverage:** every accepted surface declaration and expression has exactly
   one corresponding HIR/core subject; no semantic node is silently dropped.
2. **Resolution:** every HIR reference denotes the unique declaration selected
   by the import and namespace rules.
3. **Elaboration typing:** a kernel-accepted derivation implies that the emitted
   core is well typed under the declared state, capability, and behavior
   contexts.
4. **Initializer preservation:** evaluating an accepted surface initializer
   and its core term yields corresponding initial values.
5. **Action forward simulation:** every supported surface action step has the
   emitted core step with corresponding input, output, post-state, and
   capability store.
6. **Action backward simulation:** every emitted core action step corresponds
   to a permitted supported-source step; elaboration introduces no behavior.
7. **Affine preservation:** elaboration neither fabricates nor duplicates a
   `Once<T>` capability and preserves explicit discard/transition rules.
8. **Property preservation:** state and temporal propositions retain their
   system index and denotation through elaboration.

Testing a shared Rust/Lean vector is an interim bridge, not a substitute for
the program-level correspondence theorem.

### Rust/Lean boundary

The Rust `nmlt-kernel` is the normative executable M9 acceptance checker. Lean
is the mathematical specification and metatheory, not a runtime oracle and not
evidence that the Rust checker corresponds merely because both exist. M9-009
adds an extrinsic `RawCore`/`RawDerivation` mirroring canonical bytes, a
declarative `WellTyped`, and a Lean reference checker with a soundness theorem
of the form:

```text
check raw certificate = ok checked -> WellTyped raw
```

The existing intrinsic core may be the checked target, but cannot represent
malformed serialized input by itself. Rust and Lean consume shared canonical
decode/judgment vectors. SHA-256 identities remain opaque fixed bytes in Lean;
hashing, decoding, and Rust/Lean correspondence remain explicit TCB entries
until independently verified. The existing no-`sorryAx`, no project-axiom,
no-`native_decide`, no external-solver/FFI, and no generated-overwrite policies
continue to apply. Accepting this RFC freezes the boundary; it does not claim
M9-009 is already proved.

## Evidence consequences

A successful kernel check may support `type_checked` for the exact supported
fragment once that result class and schema are accepted. It does not support
`proved`, `model_checked`, temporal acceptance, or compiler correctness by
itself.

Every downstream semantic artifact derived from `CheckedProgram` binds:

- exact source-set membership and `SourceSetId`;
- logical-module assignment and `ModuleMapId`;
- complete surface-program and resolution-map identities;
- resolver and elaborator source-set/executable identities;
- HIR and core identities;
- ruleset bundle, resource policy, certificate, and kernel identities;
- unsupported-feature policy and any compilation bounds;
- the downstream engine's existing method, configuration, and TCB profile.

Until the Rust implementation is proved to produce kernel-accepted artifacts
or all elaborator output is independently replayed, the kernel can remove the
elaborator from claim acceptance but not the parser, resolver, core semantics,
identity algorithms, or host platform from the TCB. TCB reductions are
claim-specific and must be measured rather than asserted.

## Negative controls

The M9 gate must reject or distinguish at least:

- missing, duplicate, ambiguous, shadowed, and cyclic module/name cases;
- a changed logical-name/path mapping under unchanged source bytes;
- an unsupported CST node silently omitted from `SurfaceProgram`;
- the same source reference rebound by declaration allocation order;
- a same-typed malicious rebinding that is HIR-internally closed but disagrees
  with independent resolver readback;
- `Nat` accepted as `Int` without an explicit injection;
- mixed numeric arithmetic/comparison, negative `Nat`, `-0`, implicit
  `Int -> Nat`, M9 `Nat` subtraction, and values above legacy `i64` range;
- `always` or `enabled` resolved as an ordinary Boolean function;
- an action input ignored or substituted with ambient state;
- an update RHS reading a preceding update instead of the frozen pre-state;
- an undeclared write or an implicit change outside the write set;
- duplicate, fabricated, or silently discarded affine capability authority;
- a property from one system attached to another system;
- a certificate with a changed source, HIR, core, ruleset, or premise;
- a valid certificate replayed under a different import closure;
- duplicate keys, noncanonical IDs, unreachable nodes, cycles, and oversized
  certificate structures;
- a parser-recovery node accepted as a typed expression;
- a supported declaration dropped while all remaining derivations still pass.
- every resource dimension exactly at its maximum and at maximum plus one,
  including lying pre-allocation lengths and depth 256/257.

Each control needs both a producer-side diagnostic and an independent
kernel/readback expectation where applicable.

## Compatibility

The RFC preserves current file extensions and surface syntax where semantics
are already specified. It intentionally rejects some files that the structural
frontend can parse or the provider parser previously approximated. This is a
semantic tightening and must be called out in fixtures and release notes.

Core and certificate encodings are versioned and non-stable during pre-alpha.
Changing a semantic rule, source-set membership, canonical ID algorithm, or
core encoding invalidates dependent evidence. Diagnostic wording may evolve;
stable diagnostic codes and spans are the compatibility surface for M9.

## Alternatives

### Trust the elaborator directly

This is simpler but leaves a large, fast-changing compiler component in every
typed and semantic claim. It also gives untrusted or AI-generated elaboration
no receiver-checkable boundary.

### Elaborate directly from the CST

This avoids an explicit HIR but mixes trivia/recovery identity with semantic
resolution and makes unique-name and coverage invariants harder to inspect.

### Verify the complete compiler before integration

A full proof is a long-term goal, but waiting for it would preserve the current
split pipeline. M9 uses a narrow supported fragment, explicit certificate, and
independent kernel while stating the remaining parser/resolver/semantics trust.

### Keep backend-specific source translations

This repeats semantics and prevents results from sharing a single subject.
Backend lowering may remain specialized, but all promoted lowering starts from
the same `CheckedProgram` and records a separate correspondence obligation.

## Accepted decisions and remaining risks

M9-001 resolves the questions that previously blocked acceptance:

- certificates use a rule-explicit, content-addressed reconstruction DAG;
- the resolver chooses lexical denotations and remains in the TCB, while the
  kernel checks internal HIR closure and exact bindings;
- `Nat`/`Int` remain arbitrary-precision and disjoint, while bounded engines
  use explicit projections that preserve `unknown`;
- readable semantic paths and exact evidence identities are distinct;
- temporal formation is a mandatory separately versioned ruleset component;
- Rust is the executable checker and Lean supplies an extrinsic reference
  checker/specification plus soundness work;
- the canonical DAG and `nmlt-kernel-policy-v1` limits are fixed above.

Remaining risks are implementation and proof obligations, not permission to
change these semantics silently. They include resolver bugs while it remains a
trusted component, certificate size near accepted limits, incomplete
Rust/Lean correspondence, and finite-engine adapters accidentally laundering
an out-of-range arbitrary integer. Negative controls and evidence must keep
those boundaries visible.

## Implementation plan

1. **M9-001 — Freeze the contract.** Accept this RFC's supported fragment,
   failure policy, identity domains, and test vectors.
2. **M9-002 — Complete surface projection.** Replace omission with explicit
   supported/error nodes and add declaration-coverage tests.
3. **M9-003 — Resolve modules and names.** Implement closed acyclic imports,
   namespaces, stable IDs, and adversarial resolution fixtures in `nmlt-hir`;
   then parse every accepted raw type/expression into source-derived reference
   origins, assign local binders, emit the canonical `ResolutionMap`, and
   independently verify exact reference coverage and readback.
4. **M9-004 — Define explicit core.** Encode primitive values, systems,
   actions, capabilities, observations, and indexed property ASTs.
5. **M9-005 — Implement bidirectional elaboration.** Complete:
   `nmlt-elaborate` removes open-symbol and numeric/temporal shortcuts, emits
   canonical derivations, and checks exact root/origin coverage and DAG
   reachability before returning.
6. **M9-006 — Implement the kernel.** Complete: neutral certificate syntax is
   separated from the producer; `nmlt-kernel` independently recomputes
   identities, graph/coverage/resource conditions, every frozen rule, and the
   aggregate core before its private constructor returns `CheckedProgram`.
   Forged, canonically resealed semantic, stale, cyclic, unreachable,
   noncanonical, unknown-tag, duplicate, and oversized controls fail closed.
   Persisted byte decoding is deliberately assigned to M9-008.
7. **M9-007 — Migrate the engine.** Delete the second parser after the provider
   benchmark consumes `CheckedProgram` exclusively.
8. **M9-008 — Bind evidence.** Extend artifact schemas/readback with source,
   HIR, core, ruleset, certificate, elaborator, and kernel identities.
9. **M9-009 — Mechanize correspondence.** Prove the supported typing and
   two-way action obligations or keep the promotion gate explicitly open.
10. **M9-010 — Reproduce and audit.** Run canonical examples, all negative
    controls, seeded mutants, independent readback, and a clean-clone gate.

M9 is complete only when the canonical provider model reaches the existing
engine solely through the checked pipeline, supported examples have explicit
outcomes, seeded results remain reproducible, and no manual source-to-core path
remains in the promoted slice.

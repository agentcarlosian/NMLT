# RFC 0013: Integrated source-to-typed-core contract

- Status: Draft
- Authors: Carlosian <carlosian@agentmail.to>
- Created: 2026-07-19
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

NMLT currently has two useful but disconnected accomplishments:

- `nmlt-core` preserves tokens, trivia, recovery nodes, declarations, and
  stable spans in a lossless tree, then projects only partial untyped
  structure;
- `nmlt-engine` reparses a narrow provider fragment, resolves some names
  contextually, and constructs its own typed executable representation.

That split permits semantic omissions. An unsupported declaration can remain
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
[M9 research note](../docs/research-notes/source-to-typed-core-and-project-identity-2026-07-19.md).

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

- explicit, acyclic imports whose entire closure is present in the source set;
- closed enumeration declarations;
- `Bool`, arbitrary-precision `Nat`, and arbitrary-precision `Int`;
- system declarations with scalar state and total initializers;
- action inputs and total pure expressions;
- guards and simultaneous explicit updates;
- the `Once<T>` capability protocol used by provider-attempt;
- observation declarations;
- safety and temporal property syntax trees with explicit system indices.

The implementation may land this fragment incrementally, but no partial
substage may be presented as the completed M9 pipeline.

## Reference representation

### Source and module identity

The input is an ordered canonical set of entries:

```text
SourceEntry = (logical_module, repository_path, exact_bytes, source_id)
SourceSetId = H(domain, sorted(SourceEntry by logical_module))
```

The exact algorithm remains governed by RFC 0004. Resolution rejects duplicate
logical modules, imports outside the closed set, symlink/path-policy
violations, and cycles. Import order does not affect resolution; import
membership does affect `SourceSetId`.

### Stable semantic IDs

IDs are derived from the source-set identity and canonical declaration path,
not allocation order:

```text
ModuleId = H("nmlt-module-v1", SourceSetId, logical_module)
DefId    = H("nmlt-def-v1", ModuleId, namespace, declared_name)
NodeId   = H("nmlt-node-v1", DefId, semantic_path)
```

Source spans remain diagnostic metadata and do not alone establish semantic
identity. Duplicate names in one namespace are rejected before IDs are
constructed. The exact encoding and namespace separation require test vectors
before acceptance.

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

### Bidirectional elaboration

The principal expression judgments are:

```text
Gamma; Sigma; Delta; B |- e => A ~> t ; D
Gamma; Sigma; Delta; B |- e <= A ~> t ; D
```

They mean that surface expression `e` synthesizes or checks type `A`, emits
core term `t`, and produces derivation `D`. `B` is the behavior/system index.
Conversion is explicit and narrow. There is no implicit `Nat`/`Int`
compatibility in v1; any admitted injection has a named core node and rule.

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

### Certificate and kernel

The elaborator returns:

```text
ElaborationArtifact = {
  source_set_id,
  ruleset_id,
  resolved_hir_id,
  core_program_id,
  derivation,
  diagnostics
}
```

The derivation is a canonical tree or DAG whose nodes name a rule, premises,
input IDs, output type/core IDs, contexts, and source provenance. Sharing is
allowed only through content-addressed nodes. Cycles, unreachable certificate
nodes, unknown rules, duplicate map keys, noncanonical ordering, and resource
limit excess fail closed.

The kernel API is conceptually:

```text
check(
  accepted_ruleset,
  exact_resolved_hir,
  exact_core_program,
  derivation
) -> Result<CheckedProgram, KernelDiagnostic>
```

`CheckedProgram` has no public unchecked constructor. Engines may deserialize
core for inspection, but they must obtain a fresh kernel acceptance bound to
the same bytes before producing a semantic result.

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

## Evidence consequences

A successful kernel check may support `type_checked` for the exact supported
fragment once that result class and schema are accepted. It does not support
`proved`, `model_checked`, temporal acceptance, or compiler correctness by
itself.

Every downstream semantic artifact derived from `CheckedProgram` binds:

- exact source-set membership and `SourceSetId`;
- resolver and elaborator source-set/executable identities;
- HIR and core identities;
- ruleset, certificate, and kernel identities;
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
- an unsupported CST node silently omitted from `SurfaceProgram`;
- the same source reference rebound by declaration allocation order;
- `Nat` accepted as `Int` without an explicit injection;
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

## Risks and unresolved questions

- Whether certificates should contain full derivations or a smaller
  reconstruction trace.
- Which parts of name resolution can be checked by the kernel without making
  it a second compiler.
- Whether arbitrary-precision integers remain executable enough for the first
  bounded engine or need explicit finite-domain verification projections.
- How stable semantic paths are assigned under source edits without pretending
  that changed source has unchanged exact identity.
- Whether temporal property elaboration belongs in the first kernel ruleset or
  in a separately versioned extension checked against typed state/action core.
- What proof-assistant representation gives the smallest honest Rust/Lean
  correspondence boundary.
- Certificate denial-of-service limits and canonical DAG encoding.

These questions block RFC acceptance where they affect semantics, but they do
not block implementing resolver prototypes and negative controls behind an
experimental API.

## Implementation plan

1. **M9-001 — Freeze the contract.** Accept this RFC's supported fragment,
   failure policy, identity domains, and test vectors.
2. **M9-002 — Complete surface projection.** Replace omission with explicit
   supported/error nodes and add declaration-coverage tests.
3. **M9-003 — Resolve modules and names.** Implement closed acyclic imports,
   namespaces, stable IDs, and adversarial resolution fixtures in `nmlt-hir`.
4. **M9-004 — Define explicit core.** Encode primitive values, systems,
   actions, capabilities, observations, and indexed property ASTs.
5. **M9-005 — Implement bidirectional elaboration.** Remove open-symbol and
   numeric/temporal shortcuts and emit canonical derivations.
6. **M9-006 — Implement the kernel.** Add an independent `nmlt-kernel` checker,
   private unchecked constructors, resource limits, and forged-certificate
   controls.
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

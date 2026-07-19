# Research note: M9 contract, total projection, and deterministic resolution

- Search date: 2026-07-19
- Researcher: Carlosian <carlosian@agentmail.to>, with AI-assisted retrieval
  and synthesis
- Focused question: What is the smallest honest contract that can carry exact
  NMLT source through a complete surface projection and deterministic name
  resolution toward an independently checked typed core?
- Archive queries: `bidirectional typing`, `proof carrying code`, `verified
  compilation`, `name resolution`, `scope graphs`, `certificate checking
  resource`, and `incremental compilation identifiers`

## Retrieval result

The local archive surfaced adjacent verifier-guided work, especially
[AxDafny](https://arxiv.org/abs/2606.32007) and
[Generative Compilation](https://arxiv.org/abs/2607.13921), but no direct prior
item combining total CST projection, stable semantic identities, closed module
resolution, and a receiver-checked elaboration certificate. Several longer
queries returned no lexical matches at all. This is a limitation of archive
retrieval and is not evidence of novelty.

Current primary literature and official compiler documentation were therefore
used to test the design choices below.

## Current primary leads

### Bidirectional typing is a control boundary, not a proof status

[Dunfield and Krishnaswami's survey](https://arxiv.org/abs/1908.05839)
separates checking against a known type from synthesizing a type and reports
better control of inference and error locality. For NMLT, the useful boundary
is an explicit pair of judgments whose derivations can be inspected. It does
not justify treating an elaborator's success bit as evidence or adding general
implicit proof search.

### General scope machinery is informative but premature

[Stack graphs](https://arxiv.org/abs/2211.01224) represent name binding as
paths through a graph and support cross-file, file-incremental resolution at
large scale. M9 does not need that expressiveness yet. Its first resolver can
be substantially smaller and easier to audit: a closed source set, one simple
logical name per module, explicit acyclic imports, separate namespaces, no
globs, no shadowing, and exactly one candidate for every unqualified
reference. Scope-graph-style machinery remains a comparison point if the
language later gains open modules or richer lexical scopes.

### The receiver must bind the exact subject and policy

The proof-carrying-code framework for
[inlined reference monitors](https://arxiv.org/abs/1012.2995) lets a receiver
supply its own trusted policy object and check attached verification
conditions. The relevant NMLT lesson is structural: the receiver-side kernel
must receive the exact HIR, core, ruleset, resource policy, and derivation. A
certificate cannot select a weaker checker policy or authenticate a different
program merely because its internal proof graph is locally valid.

### A verified island does not erase its surrounding TCB

The [CompCert TCB audit](https://arxiv.org/abs/2201.10280) identifies possible
loopholes in source and target models and in external algorithms around a
verified compiler. NMLT must therefore continue naming the parser, source-set
construction, resolver, canonical encoders, Rust kernel, and host/runtime
assumptions by claim. A future Lean theorem about core typing does not by
itself verify that Rust resolved the intended source name.

### Stable locators and exact evidence identities serve different purposes

The Rust compiler guide explains why allocation-order IDs cannot safely cross
compilation sessions and uses definition paths and hashes as stable forms
([stable identifier discussion](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html#a-question-of-stability-bridging-the-gap-between-compilation-sessions)).
NMLT adopts the path idea but draws a sharper evidence boundary:

- a `DefPath` or `SemanticPath` is an allocation- and span-independent
  locator;
- an M9 `DefId` or `NodeId` also binds the exact source/module-map identity;
- any source-byte, path, membership, or logical-module-map change therefore
  invalidates exact evidence even when the human-readable locator is
  unchanged.

This deliberately sacrifices cross-edit identity reuse in the assurance path.
An incremental cache may later maintain a separate reuse key, but it cannot be
substituted for exact evidence identity.

### Portable acceptance needs structural limits

Certificates and canonical graphs are untrusted inputs. M9 therefore binds a
versioned policy covering byte sizes, node and edge counts, nesting depth,
identifier sizes, and integer magnitudes. Counters use checked arithmetic and
limits are checked before allocation. The kernel does not use wall-clock time
as a portable semantic limit. An outer watchdog may stop work, but that yields
`unknown/resource_exhausted`, never a semantic refutation. Lean's official
reference likewise distinguishes allocation-based heartbeats from wall-clock
time ([heartbeat documentation](https://lean-lang.org/doc/reference/latest/IO/Timing/#IO.getNumHeartbeats)).

## Contract decisions

### Complete projection

The lossless CST-to-surface boundary is total over semantic CST nodes:

1. module ownership and declaration order are retained;
2. imports and closed enumerations are structured rather than reparsed from
   strings;
3. system and action parameters remain distinct and ordered;
4. unsupported and recovered constructs remain explicit;
5. expression and type nodes retain exact CST origins and trimmed source
   spans;
6. an independent origin census detects missing, duplicated, or reordered
   projection subjects;
7. structural completeness remains weaker than M9 feature eligibility, name
   resolution, typing, execution, or verification.

The first M9 surface profile accepts simple ASCII module/import identifiers,
closed enumerations, scalar state, `Once<T>` capabilities, action parameters,
guards, whole-state simultaneous updates, emits/consumes, observations, and
safety/temporal property shells. System parameters, system-level constants and
inputs, data/record/function forms, ports, action grades, selected updates,
resource properties, and hiding receive stable feature-boundary diagnostics.

### Resolution

The initial resolver is deliberately closed:

- source-set membership and logical-module-to-path mapping are explicit;
- imports must name members of that exact set and form a DAG;
- duplicate logical modules, paths, declarations, and namespace entries fail;
- unqualified lookup succeeds only with one candidate across local and direct
  imports;
- allocation order cannot affect `ModuleId`, `DefId`, or the selected target;
- resolved references contain typed IDs, never an open `Symbol(String)` escape.

The resolver remains in the M9 TCB. The later elaboration kernel checks HIR
closure, type/kind/system agreement, and certificate bindings, but it does not
pretend to reconstruct lexical lookup. Independent resolver readback is a
separate future TCB-reduction step.

### Identities

Accepted RFC 0004 remains authoritative for path-sorted `SourceSetId`. M9 adds
a distinct `ModuleMapId` binding the bijection from logical names to the exact
source-set paths. `ModuleId` binds that map; `DefId` binds a complete typed
nested definition path; and `NodeId` binds a role-based semantic path inside
the definition. Arena indices, byte spans, and declaration allocation order
are forbidden identity inputs.

### Certificate and kernel direction

M9 uses a rule-explicit, content-addressed reconstruction DAG. The certificate
header binds source set, module map, surface program, HIR, core, ruleset bundle,
and resource policy. The kernel reconstructs contexts from the exact HIR and
premises, computes the required obligation set independently, and requires
bijective coverage. Unknown rules, stale bindings, cycles, unreachable nodes,
noncanonical order, duplicate keys, and resource excess fail closed.

Temporal constructors live in a mandatory, separately versioned formation
component of the M9 ruleset bundle. Kernel acceptance establishes formation
and system indexing only; it does not establish temporal satisfaction,
liveness, refinement, or `proved`.

## Implication for the implementation sequence

The first safe tranche is:

1. freeze RFC 0013's identity, numeric, temporal, certificate, Lean, and
   resource-policy decisions;
2. make surface projection ordered, total, origin-censused, and explicitly
   feature bounded;
3. implement `nmlt-hir` as a deterministic closed resolver with identity
   vectors and adversarial import/namespace tests;
4. only then define explicit typed core and bidirectional elaboration.

This tranche strengthens the semantic spine but does not yet produce a
`CheckedProgram`, remove the resolver from the TCB, or connect the provider
benchmark to the existing engine.

## Implementation status at this note

RFC 0013 is accepted and its total surface-projection gate is implemented.
`nmlt-hir` now seals exact source bytes to that projection and builds bounded,
deterministic module/import and named-declaration tables with typed lookup and
stable identities. This is the first half of M9-003, not its completion.

Raw type/expression references and action-local binders remain outside the
current resolver index. M9-003 closes only after those origins are parsed,
covered by a canonical `ResolutionMap`, and independently read back against the
surface program. Until then, `ResolvedProgram` is neither all-reference HIR nor
a typed or checked program.

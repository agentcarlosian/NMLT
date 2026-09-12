# RFC 0034: Lean proof dependency graphs

- Status: Under review
- Date: 2026-09-11
- Milestone: R3, fourth implementation increment

## Problem and scope

The local Lean task profile records the proof root's direct references and a
flat list of exported declarations. That list cannot explain why an interior
definition, constructor or recursor is present, or distinguish a proof reference
from declarations exported together as a group. Add a bounded, deterministic
dependency graph derived from the exact export sent to NanoDA.

This graph describes exported syntax. The independent kernel still establishes
proof validity; the graph parser does not become another proof checker. The
candidate, target selection, axiom policy and source import workflow are unchanged.
Draft/planning status does not occur in an accepted dependency graph.

## Graph contract

`nmlt-lean-proof-dependencies-v1` records the fixed proof root and target binding,
export SHA-256, sorted declaration nodes and sorted export groups. Every node
corresponds to exactly one independently checked exported declaration. Its kind
is axiom, definition, theorem, opaque, quotient, inductive, constructor or recursor.

Each node separates these reference sets:

- `type_references`: explicit constants in the declaration's type;
- `value_references`: explicit constants in its stored proof/definition value;
- `reduction_references`: recursor-rule constructors and explicit constants in
  rule right sides;
- `projection_references`: projection type names in the type, value and rule
  right sides;
- `literal_support`: declarations the exporter retains for primitive literals,
  specifically `Nat`, or `Char.ofNat` and `String.ofList`.

Names used only as binders, universe parameters or expression metadata are not
declaration references. Sets are sorted and unique. Every reference names an
exported node. The parser preserves the expression DAG and traverses it
iteratively, visiting shared expressions once per reference collection.
Declaration strings follow the existing bounded export-label convention. They
identify nodes within the checked export and are not a general round-trippable
representation of Lean source identifiers. Ambiguous declaration labels are
rejected; complex escaped names can also fail the Lean metadata comparison.

The implementation follows the pinned
[NDJSON 3.1.0 format](https://github.com/leanprover/lean4export/blob/411dce7db58a3afc60ecab2d211acd1042b593dc/format_ndjson.md)
and exporter source. Constant references deliberately match Lean's
`Expr.getUsedConstants`: it traverses a projection's argument but does not
include the projection type name. Projection references are therefore separate.
Literal support and group retention can include declarations that do not occur
as explicit constants in a particular proof value.

An inductive group records the types, constructors and recursors in that export
block. The four primitive quotient declarations form a quotient group. Group
membership is not an extra direct type/value edge. Mutual inductives, recursive
definitions and recursor reductions can contain cycles, so this is not a task
scheduling DAG or a proposed proof plan. The complete export is not claimed to
be a minimal proof-dependency set. Declaration grouping metadata on ordinary
definitions, such as their `all` field, is not interpreted as a direct reference.

## Acceptance, artifacts and compatibility

After capturing the export, the CLI derives the graph from its exact bytes.
The existing root/type inspection and independent NanoDA check still apply.
Before publishing acceptance, the graph's export digest and node sequence must
match the checked export. The root's explicit value references and the target
binding's explicit value references must also match Lean's separate proof and
target metadata.

Results use `nmlt-lean-result-v4` and embed `proof_dependencies`. The CLI also
writes the same graph as `proof-dependencies.json` and a linked readable report
as `proof-dependencies.md`. Both files must be written durably before the result
record is published. Report links use generated numeric anchors, and declaration
labels are escaped as text. These sidecars are derived views; recheck consumes
the embedded graph in `result.json` and regenerates the views.

Fresh rechecking validates the retained graph's structure and fixed bindings,
rebuilds the saved sources, and repeats export and independent kernel checking.
The freshly derived complete graph must equal the embedded graph. Changing an
interior reference, kind, projection, literal-support set, recursor reduction
or export group cannot inherit the original acceptance. The original project,
model session and old generated report files are not inputs to reconstruction.

Task version 2, candidate version 1 and project manifest versions 1/2 remain
unchanged. Old result formats require their retained original executable, under
the existing exact-implementation contract. No R2 format or Lean behavioral
semantics change is introduced.

## Bounds and trust

Graph construction permits at most 16,384 declarations, 262,144 reference/group
member entries, 8 MiB of stored name text and 16,777,216 expression visits across
all collections. The JSON graph and readable report each have a 16 MiB bound;
the containing result retains its 64 MiB JSON bound. Exceeding any bound rejects
the result. The existing 16 MiB raw export, source/module, candidate grammar
and subprocess bounds are unchanged.

The graph parser and renderer are explanatory implementation trust. The export
format includes redundant data, including recursor information, that an
independent checker can rederive or ignore; a diagram is not an independently
proved graph of kernel dependencies. The exporter, source/target binding, tools,
trusted project metaprograms and host remain named trust boundaries. Human
approval, statement faithfulness, novelty and usefulness remain separate reviews.

## Validation and remaining work

Unit fixtures cover all expression forms, separate type/value references,
projection type names, literal support, inductive/quotient groups, recursive
reduction references, missing/forward/duplicate expression references, deep
shared DAGs, malformed graph data and escaped report labels.

The real Lean fixture combines a structure projection, mutually defined tree
and forest, an opaque definition, numeric/string literals and quotients.
It must pass independent checking and a fresh retained-executable recheck with
the working project unavailable. A separate Lean helper compares every exported
node's explicit type/value, reduction and projection references against the
actual Lean environment. Known literal support and mutual/quotient groups are
also checked. Unused draft declarations must stay outside the accepted graph.

Rejection controls alter each reference category, the export binding, graph
presence, declaration kind and groups; add a planning field; use an old result
version; and attempt to prove the target using its draft body. Existing explicit
module, discovered-source and larger-export suites must remain green.

The [validation record](../docs/reviews/r3-proof-dependencies-2026-09-11.md)
records the executed controls and platform limits. R3 remains in progress:
broader library/package integration, editor/REPL comparison, proof automation,
asynchronous project jobs and the complete evaluation corpus remain open.
Independent cross-family/human review and RFC acceptance also remain open.

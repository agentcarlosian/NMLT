# RFC 0020: Workflow records and bounded collections

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-06
- Tracking issue: not assigned
- Design disposition: executable-only R2 increment; acceptance pending

The version 2 contract below describes the fourth increment.
[RFC 0021](0021-workflow-source-packages.md) adds local source packages,
file-aware locations, and complete source manifests under version 3.
The current executable explicitly rejects version 1/2 pure records.

## Problem and scope

RFC 0019's scalar inputs and outcomes cannot group related inputs or process
a bounded batch. Extend that pure workflow route with nominal records,
homogeneous lists, structured inputs, and an explicit bounded fold. All of
these constructs are **executable-only**. They neither execute host jobs nor
acquire Lean semantics from the existing behavior artifacts.

The canonical CST and complete surface projection remain the declaration
authority. Interpret the retained `RecordDecl` slices alongside `FunctionDecl`
slices, consume each slice completely, and reject other declaration kinds.
No parser, M9 judgment, finite artifact schema, or Lean definition changes.
Cross-file packages, effects, mutation, general recursion, closures, and
user-defined sums remain outside this increment.

## Static semantics

```nmlt
record Item { value: Int, enabled: Bool }
record Summary { count: Int, total: Int }

fn main(items: List<Item>) -> Summary {
  fold(items, new Summary { count: 0, total: 0 }, acc, item => {
    if item.enabled {
      new Summary { count: acc.count + 1, total: acc.total + item.value }
    } else { acc }
  })
}
```

`record Name { field: Type, ... }` declares a nominal type in its enclosing
module. Field separators are commas; an optional trailing comma and empty
records are allowed. Duplicate record names or fields are rejected, including
unused declarations. Built-in type names cannot be record names. Record and
function names occupy separate namespaces. Unqualified type/constructor names
search the current module, then the root; dotted names are exact root paths.
Forward type references are supported; no implicit ancestor lookup is added.
Cycles through records, `List`, or `Outcome` are rejected, even if a list could
be empty at runtime. Expanded type depth includes those constructors and must
not exceed sixteen; a memoized dependency traversal checks every declaration.

| Construct | Type and lowering |
|---|---|
| `new R { f: e, ... }` | Resolve nominal R; require every declared field exactly once and exact field types; retain source initializer order |
| `value.field` | Require a record and declared field; lower to a typed projection; supports call results and parenthesized expressions |
| `List<T>` | Homogeneous immutable list, with a global runtime capacity of 256 items; capacity is not a type parameter |
| `[e, ...]` | Equal element types, inferred from the first element or expected `List<T>`; empty lists require an expected type; trailing comma allowed |
| `length(items)` | Require `List<T>`, return Int |
| `get(items, index)` | Require `List<T>` and Int, return `Outcome<T>` |
| `fold(items, initial, acc, item => body)` | Require `List<T>`; initial has A; body checks as A under `acc: A, item: T`; result A |

Use `let xs: List<Int> = []; ...` when an empty list has no expected type.
Folds infer their accumulator from the initial value or an expected result;
their list operand is inferred independently. There is no implicit widening,
record subtyping, heterogeneous list, record/list equality, or missing-field
default. Scalar operators keep their earlier rules.

The fold binders must differ; they lexically shadow outer bindings only inside
the body. The list and initial expression use the surrounding scope. Every
fold body is checked, including for empty lists. Calls in it participate in
the existing acyclic function graph, so a fold cannot hide recursion.

Dotted call syntax still denotes a statically named function. A dotted value
name starts from a lexical local and projects its fields; it is not a method
call. `new`, `record`, `fold`, `length`, and `get` are reserved workflow names.
The explicit `new` keyword avoids ambiguity with the braces after an if or
match scrutinee. Postfix projection binds more tightly than binary operators.

## Dynamic semantics and limits

Evaluate record initializers in source order and list elements left to right.
Records store values by field name in deterministic key order. Values are
immutable and reusable. Projection returns the named value. `get` is zero-based;
a negative index or index at/above length returns
`Err("list index out of bounds")`, including an empty list. Indexing does not
panic, wrap, or return a default. A successful lookup returns `Ok(element)`.

Fold evaluates its list once, then its initial value once, and evaluates the
body once per element from left to right, replacing the accumulator each time.
An empty list returns the initial value without executing the body. Iterations
reuse the same expression depth rather than recursively growing a call stack.
Each body evaluation still enters typed nodes and consumes the shared step
budget. There is no separate free iteration or nested-fold budget. All existing
overflow, step, and runtime-depth stops propagate through folds unchanged.

| Additional or extended bound | Limit |
|---|---|
| Record declarations | 64 per source |
| Record fields / initializer fields | 16 |
| List literal / input / runtime list | 256 items |
| Syntactic type nesting | 8 `List`/`Outcome` constructors |
| Expanded/inferred type and runtime value nesting | 16 edges |
| Nodes in any input or completed expression value | 4,096 |
| Combined UTF-8 payload, record-name, and field-name bytes per value | 65,536 |
| Individual Text/Err payload | 4,096 UTF-8 bytes, unchanged |
| Value-work units per execution | 1,000,000 |
| CLI argument encoding / argument count | 32,768 bytes / 16 |

One value-work unit is one value node or one UTF-8 payload/name byte. Charge
all entry inputs and every completed expression value, including local lookups,
container values, projections, and control-expression results. This deliberately
counts reused values again. It is deterministic accounting that limits repeated
aggregate copying, not an exact allocation counter, wall-clock deadline, process
memory cap, or host quota. Validation precedes input copying; composite JSON
decoding checks incremental node/byte counts before constructing more values.

An invalid or oversized input fails before execution and creates no run record.
At expression completion, exceeding a per-value bound stops with `value_limit`;
otherwise exhausting cumulative value work stops with `value_work_limit`.
Both carry the source span and completed expression-entry step count, and return
no value. A typed result may exceed an aggregate size bound for particular
inputs; typing alone does not promise normal return. Intermediate child values
have already consumed their steps and work when a parent stops. Per-expression
checks also bound temporary container construction by its fixed arity and
previously validated children; this is not an allocator-level sandbox.

The existing source, function, expression, runtime-depth, and record-file bounds
from RFC 0019 otherwise remain. Dotted projections count toward actual expression
tree depth. A fold does not justify termination beyond these executable bounds.

## Inputs, records, and migration

`--arg name=value` now decodes exact JSON according to the chosen entry's type:

- Scalars use the existing JSON bool, signed integer, and string syntax.
- `List<T>` uses a JSON array.
- A record uses a JSON object with exactly its declared fields. Its nominal name
  comes from the compiled signature, not a caller-supplied tag.
- `Outcome<T>` uses exactly one key: `{"Ok": payload}` or `{"Err": "message"}`.

Duplicate object keys at any input depth fail; no last-key-wins normalization is
allowed. Programmatic typed inputs also validate nominal names, exact fields,
element types, and aggregate bounds before evaluation.

The current profile is `pure-workflow-v2`. Its emitted schemas are
`nmlt-pure-typecheck-v2`, `nmlt-pure-run-v2`, and `nmlt-pure-replay-v2`.
Typecheck output includes resolved record definitions. Typed-program identity
includes all record definitions, including unused ones. Serialized `Type` adds
`list` and `record` variants; `Value` adds a list of values and a record with
`name` and `fields`. Run output uses these tagged values rather than the ordinary
JSON used on the CLI. It adds the two explicit resource-stop variants above.

Version 1 records are explicitly rejected by this executable. Preserve the old
binary to replay them, or execute the source again to create version 2 evidence.
There is no silent migration of evidence, and no binary reconstruction from
this RFC alone. Earlier scalar programs retain their values and expression-step
counts unless they use newly reserved names or exceed the new aggregate/work
bounds. Their profile, schema, executable, and program digests change.
The finite run/replay route and behavioral artifacts retain their prior formats.

Replay still binds exact source and executable bytes, recompiles the typed
program, and recomputes the complete result. Unknown fields, duplicate keys,
unsupported profiles, wrong nominal names, altered payloads, and altered stops
fail. An internally consistent forged record is not authenticated; matching
replay is executable consistency with `assurance: none`.

## Controls, assurance, and open work

The [batch example](../examples/pivot/batch_summary.nmlt) combines records,
lists, function calls, fallback outcomes, and a summary accumulator. Three inputs
`(-3,5)`, `(4,9)`, `(-1,-2)` produce accepted 2, rejected 1, total 41.
Negative controls include recursive/nominally mismatched records, unknown or
duplicate fields, mixed lists, wrong fold accumulators, invalid dead bodies,
out-of-bounds lookup, oversized values, repeated copying, and replay mutations.

No new construct lowers to a Lean behavioral operation or has a new checked
preservation theorem. The complete existing Rust/Lean/NanoDA and baseline gates
must stay green, but they do not prove these new Rust semantics. Publication
review and RFC acceptance remain open gates. Cross-file packages and native
job/Lean integration are subsequent R2 increments; the R2 exit gate is not met.

Open design questions include capacity-indexed list types, broader sum types,
allocation-aware runtime accounting, and an explicit finite abstraction for
aggregate values. None is implied by this implementation.

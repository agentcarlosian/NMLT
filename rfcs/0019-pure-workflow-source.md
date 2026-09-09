# RFC 0019: Pure workflow source, entry points, and replay

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-06
- Tracking issue: not assigned
- Design disposition: executable-only R2 increment; acceptance pending

The version 1 contract below describes the third increment.
[RFC 0020](0020-workflow-records-and-collections.md) extends it with records,
bounded lists/folds and structured inputs, moving that increment's profile
and pure record formats to version 2. [RFC 0021](0021-workflow-source-packages.md)
adds the current version 3 package contract. Preserve the original executable
for version 1 replay; the current CLI does not silently migrate older records.

## Scope and source route

Add `nmlt-workflow` for pure functions with real inputs and reusable typed
outcomes. This is the value-layer prerequisite for native job effects. It does
not connect source programs to RFC 0018's job journal or subprocess adapter.

Reuse `nmlt-core::parse_cst` and its complete `project_untyped` declaration
projection. Local `module` wrappers retain their existing syntax. The existing
lossless `FunctionDecl` slices receive a new, explicit interpretation only in
the workflow profile. The frozen ordinary projection continues to label them
unsupported for M9; neither the M9 elaborator nor the finite behavior route
silently interprets these new functions. No existing source construct changes
meaning. No top-level scanner bypasses the canonical projection.

The workflow parser consumes each retained function slice completely using the
shared lexer and byte spans. It splits punctuation runs into the operators
listed below. Resolution and typed lowering are specific to the new judgments;
they do not reuse M9 proof certificates for unrelated typing rules.

Only `fn` declarations and local module wrappers are accepted. Imports, systems,
enums, records, properties, and other declarations are explicit errors on this
route, including otherwise unused declarations. Functions mixed with finite
systems cannot inherit behavioral evidence.

## Grammar and typing

Representative complete program:

```nmlt
fn positive(x: Int) -> Outcome<Int> {
  if x < 0 { Err("negative input") } else { Ok(x) }
}

fn main(input: Int) -> Int {
  match positive(input) {
    Ok(value) => value + value,
    Err(message) => 0
  }
}
```

A function has a name, typed positional parameters, an explicit result type,
and a braced expression body: `fn name(p: Type, ...) -> Type { expression }`.
The opening body brace must occur before an unparenthesized newline in the
header, as required by the existing lossless function-node boundary.
An entry point is a function selected by `--entry`; there is no new `entry`
declaration or implicit host action.

| Construct | Meaning and lowering |
|---|---|
| `Bool`, `Int`, `Text` | Boolean, signed 64-bit integer, bounded UTF-8 string |
| `Outcome<T>` | Immutable `Ok(T)` or `Err(Text)` domain result |
| Literals | `true`/`false`, decimal integers with optional underscores, JSON-escaped quoted text |
| `let name [: Type] = value; body` | Evaluate once, bind a reusable immutable value within `body`; local shadowing is lexical |
| `if condition { yes } else { no }` | Bool condition; equal branch types; only selected branch executes |
| `match value { Ok(x) => a, Err(e) => b }` | Exhaustive two-arm match on `Outcome<T>`; bind `x: T` or `e: Text`; equal arm result types |
| `function(args...)` | Statically resolved named function; exact arity and argument types; left-to-right evaluation |
| `+`, `-`, `*`, unary `-` | Checked Int arithmetic; overflow stops execution explicitly |
| `<`, `>`, `<=`, `>=` | Int comparisons returning Bool |
| `==`, `!=` | Equal scalar operand types; Bool result; no Outcome equality operator |
| `not`, `and`, `or` | Bool operators; `and` and `or` short circuit |

Precedence, lowest first: `or`, `and`, comparisons/equality, `+`/`-`, `*`,
unary operators. Binary operators associate left; parentheses override grouping.
Braced expressions may group match arms or other subexpressions. Match arms may
appear in either order and allow a trailing comma; duplicates, missing arms,
wildcards, and additional arms are rejected.

`Ok` can infer its payload type. `Err` requires an expected `Outcome<T>` from
a function result, call argument, branch, or annotated binding; an unconstrained
`let x = Err(...)` is rejected with an annotation diagnostic. The Err arm's text
never becomes a value of type T without explicit user code.

All functions and both branches are typechecked, including dead code. Duplicate
function names, duplicate parameters, and unbound names are rejected. Local
names resolve to lexical binder indices. Calls resolve to a typed function
index: an unqualified name searches the current module then the root module;
a qualified name is an exact root-qualified path. Forward calls are supported.
There are no cross-file imports, first-class functions, closures, or implicit
ancestor-module lookup. A cycle anywhere in the function call graph is rejected,
even if its calls are unreachable at runtime.

The compiler alone constructs `Program`; its fields and typed expressions are
private and no unchecked deserializer is exposed. Each lowered expression
retains its inferred type and source byte span. The program digest identifies
the deterministic serialized typed tree, signatures, and resolved indices.

## Execution and bounds

The evaluator visits only the selected branches and shares this same typed
tree for execution and replay. A step is entry to a typed expression node,
including literals, local lookup, calls, and control expressions. A call enters
its body after evaluating its arguments. Step count is an evaluator metric,
not a model grade, job reservation, or measurement of host expenditure.

| Bound | Limit |
|---|---|
| Source | 128 KiB |
| Functions | 64 across the file |
| Local module nesting | 8 wrappers |
| Parameters / arguments per function call | 16 |
| Type nesting | At most eight nested Outcome constructors |
| Expression parsing and constructed tree depth | 48 |
| Expression constructions per function | 2,048, including parenthesized reconstruction |
| Runtime expression depth across calls | 64 active expression evaluations |
| Execution steps | Explicit caller limit in 1–100,000 |
| Text/error payload | 4,096 UTF-8 bytes |
| Pure run record | 4 MiB |

There are no loops or recursive calls. Bounded evaluation still reports
`step_limit`, `depth_limit`, or `integer_overflow` when applicable, with a source
span and the completed step count. Step exhaustion takes precedence over depth
exhaustion at the next expression. A failed operation produces no returned
value. This increment does not claim every typed program returns normally for
every Int input or within every evaluation budget.

`returned` means the pure expression returned a value. That value can itself
be `Err("...")`; the domain failure stays explicit. CLI exit zero means value
evaluation completed, not that a job, proof, or domain-specific task succeeded.
Runtime stops use nonzero exit and retain their structured record.

## CLI and record contract

```bash
nmlt typecheck source.nmlt --profile workflow
nmlt run source.nmlt --entry main --arg input=-3 --max-steps 1000 --emit-run new.json
nmlt replay new.json --source source.nmlt
```

`--entry` explicitly selects this route; `--behavior` retains RFC 0017.
Mixing the two modes, duplicate options, unknown options, or duplicate argument
names fails. `--arg name=value` accepts JSON scalar syntax: Int, Bool, or quoted
Text. The API can also accept typed Outcome inputs; the CLI currently accepts
only scalar inputs. The supplied names must exactly match the chosen signature.
Ordinary `check` still requires a system; workflow typechecking is explicit.

`nmlt-pure-run-v1` records the `pure-workflow-v1` profile, `assurance: none`,
exact executable and source digests, typed-program digest, entry, all inputs,
step budget, and complete execution result. `source_path` is descriptive
metadata; exact source bytes can move without invalidating replay. Output files
use create-new semantics, never replacing source, existing records, or aliases.
Invalid options, source, or inputs produce no record. Partial I/O writes are
not valid completed records and are not repaired automatically.

Replay checks the format, profile, claim ceiling, source/executable identities,
recompiled typed-program identity, inputs, steps, and complete outcome. Unknown
fields, duplicate object keys at any depth, unsupported versions, and altered
results fail. Whitespace and object-key ordering are immaterial. A matching
incomplete run stays incomplete. An attacker who rewrites a complete consistent
record is not authenticated by this check; digests do not prove provenance.

These records contain pure evaluation results, not effect events or job state.
Replay performs no external work. Rebuilding the CLI changes its identity and
requires a fresh run or the preserved original binary for old-record replay.

## Semantic disposition and remaining work

Every new construct above is **executable-only**. Neither a typed workflow nor
its record is a `behavior-core-v1/v2` artifact, a finite execution witness, or a
Lean theorem. Existing artifact fixtures and Lean definitions remain unchanged.
Future finite abstractions need explicit mappings and separate validation.

The conformance corpus covers name/type/branch errors, recursion, malformed or
unsupported declarations, input-sensitive results, exhaustive outcome handling,
arithmetic boundaries, short circuiting, resource limits, record mutation, and
unchanged finite command routing. No host adapter, permission, cancellation,
proof checking, or exactly-once guarantee follows from a pure Outcome value.

Record types, bounded collections/iteration, cross-file packages, source effects,
Lean adapters, user-defined invariants, and the complete R2 workflow exit gate
remain future increments. RFC acceptance and publication review remain separate.

The [implementation evidence](../docs/reviews/r2-pure-workflow-increment-2026-09-06.md)
records executed controls, identities, and the current remaining gates.

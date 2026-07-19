# Surface-to-untyped-core boundary

Phase 1 has one explicit boundary between the lossless surface tree and later
semantic work: `nmlt_core::project_untyped`. The boundary is deliberately
partial. It is an auditable structural projection, not a type checker or an
alternate operational semantics.

## Inputs and outputs

The input is a `SyntaxParse`, including its immutable CST and diagnostics. The
output is an `UntypedProjection` containing:

- system names, spans, and members;
- binding names, raw declared types, and raw initializers;
- action parameters, optional grade source, and explicit statements;
- update targets separated from their values;
- safety, temporal, and resource property shells;
- observation and hiding shells;
- surface-only nodes for declarations outside the narrow behavioral core;
- error nodes and projection issues for recovery or unsupported structure.

Trivia stops controlling declaration structure, but every raw term retains the
exact non-trivia source slice and byte span. Expressions are intentionally
opaque `RawTerm` values. Operator precedence, totality, binding, type, and
behavioral meaning are assigned only by later stages.

`is_structurally_complete()` means only that the projection encountered no
parser diagnostic, recovery node, duplicate member name, or malformed update
target. It does not mean that the file resolves, type-checks, executes, or
satisfies any property. Surface-only nodes remain visible even in a
structurally complete projection and must be handled before typed elaboration.

## Candidate desugaring map

| Surface construct | Untyped representation | Deferred work |
|---|---|---|
| `state x: T = e` | state binding with raw `T` and `e` | resolve `x`, elaborate and type `T`/`e` |
| `action a(p: T) { ... }` | action, raw parameter types, ordered statements | action namespace, input typing, step meaning |
| `require e` | explicit require statement with raw `e` | Boolean typing and guard semantics |
| `set x = e` | explicit location plus raw value | state ownership, frame/effect and value typing |
| `set x[i] = e` | root `x` plus syntactic index selector | index type, location validity and update semantics |
| `emit e` / `consume e` | explicit ordered statement | port/capability resolution and effects |
| property declarations | kind, name, raw expression | property/system indexing and temporal meaning |
| `observe` / `hide` | kind plus raw names/expression | observation mapping and refinement semantics |
| modules, data, records, functions | source-spanned surface-only node | module, data, totality and function elaboration |
| recovered syntax | error node plus issue | no semantic elaboration permitted |

Update locations have one syntax-level restriction: they begin with an
identifier and may continue with field or balanced index selectors. Thus
`set pc[p] = critical` has a location shape, while `set pc + other = critical`
does not. The projection does not decide whether `pc` exists or denotes state.

## Negative controls and assurance boundary

The frontend test suite freezes the following distinctions:

| Control | Frontend outcome | What is not claimed |
|---|---|---|
| duplicate systems | parser diagnostic with the duplicate-name span | no general namespace policy |
| duplicate system members | projection issue referencing both declarations | no cross-module resolution policy |
| expression-shaped update target | unsupported target plus projection issue | no state/type judgment |
| update of undeclared `missing` | valid location shape retained for resolver | not accepted as a valid update |
| assignment-like `x := 1` without `set` | recovery/error node, never an update | no inference of an implicit effect |
| missing syntax that changes recovery grouping | deterministic partial CST and incomplete projection | recovered grouping has no semantics |
| unclosed or mismatched delimiters | lossless CST plus stable delimiter span | no repaired program is accepted |

Every diagnostic returned by `parse_cst` or `parse_source` has a UTF-8-valid
half-open byte span. Empty input uses `0..0`; a well-formed file with no system
uses an EOF span. Zero-width spans name insertion or file-level conditions and
remain stable without inventing source bytes.

The typed frontend remains a separate Phase 2 responsibility. In particular,
this Phase 1 boundary does not resolve imports, prove functions total, decide
whether a raw expression is ambiguous, establish frame conditions, or assign
transition semantics.

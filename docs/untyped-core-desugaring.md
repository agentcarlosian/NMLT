# Complete surface-projection boundary

Phase 1 has one explicit boundary between the lossless surface tree and later
semantic work: `nmlt_core::project_untyped`. The boundary is deliberately
untyped but no longer permits silent semantic-node omission. It is an
auditable structural projection, not a type checker or an alternate
operational semantics.

## Inputs and outputs

The input is a `SyntaxParse`, including its immutable CST and diagnostics. The
output is an `UntypedProjection` containing:

- system names, spans, and members;
- ordered module boundaries, imports, closed enumerations, and systems;
- ordered enum-body and parameter-list item sums whose supported, recovered,
  and surface-only nodes remain explicit;
- system parameters kept separately from action parameters;
- binding names, raw declared types, and raw initializers;
- action parameters, optional grade source, and explicit statements;
- update targets separated from their values;
- safety, temporal, and resource property shells;
- observation and hiding shells;
- explicit unsupported nodes for declarations outside the narrow behavioral
  core;
- error nodes and projection issues for recovery or unsupported structure.

`ProjectionCoverage` independently censuses every semantic CST origin and
compares it with the ordered origins represented by the projected artifact.
Missing, duplicated, or reordered subjects make the projection incomplete.
Adding a `SyntaxKind` also requires an exhaustive coverage-classification
decision. Enum bodies and parameter lists use explicit item sum types, so a
recovered or newly introduced direct child cannot disappear through a
`filter_map`; convenience iterators expose supported entries without changing
the complete stored sequence.

Trivia stops controlling declaration structure, but every raw term retains its
exact CST origin together with the non-trivia source slice and byte span.
Statement terminators remain statement tokens rather than becoming part of raw
expressions. Expressions are intentionally opaque `RawTerm` values. Operator
precedence, totality, binding, type, and behavioral meaning are assigned only
by later stages.

`is_structurally_complete()` means only that the projection encountered no
parser diagnostic, recovery node, duplicate member name, malformed update
target, or coverage discrepancy. It does not mean that the file resolves,
type-checks, executes, or satisfies any property. Unsupported nodes remain
visible even in a structurally complete projection.

`m9_surface_issues()` applies the narrower first-M9 syntax profile. An empty
list means no explicitly excluded surface form was seen; raw types and
expressions still require parsing, resolution, and elaboration. The stable
M9 feature-boundary diagnostics reject system parameters/constants/inputs,
data/record/function forms, ports, grades, selected updates, resource
properties, and hiding without confusing those rejections with parse failure.

## Candidate desugaring map

| Surface construct | Untyped representation | Deferred work |
|---|---|---|
| `module M { ... }` | ordered module node containing ordered declarations | source-set/module-map validation |
| `import M` | explicit import target and exact span | closed-set and cycle checking |
| `enum E { a, b }` | named enum and ordered variant/error/surface-only items | constructor namespace and typing |
| `state x: T = e` | state binding with raw `T` and `e` | resolve `x`, elaborate and type `T`/`e` |
| `action a(p: T) { ... }` | action, ordered parameter items with raw types, ordered statements | action namespace, input typing, step meaning |
| `require e` | explicit require statement with raw `e` | Boolean typing and guard semantics |
| `set x = e` | explicit location plus raw value | state ownership, frame/effect and value typing |
| `set x[i] = e` | root `x` plus syntactic index selector | index type, location validity and update semantics |
| `emit e` / `consume e` | explicit ordered statement | port/capability resolution and effects |
| property declarations | kind, name, raw expression | property/system indexing and temporal meaning |
| `observe` / `hide` | kind plus raw names/expression | observation mapping and refinement semantics |
| data, records, functions | source-spanned unsupported node | later rulesets only |
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
| punctuation joined to `;` or extra import/module-tail tokens | stable syntax diagnostic and incomplete projection | no token-prefix acceptance |
| module nesting at 257 (limit 256) | one bounded, lossless error node for the over-depth module | no unbounded recursive descent |
| missing declaration name before a nested type/expression | missing name remains `None` | descendant identifiers are never borrowed as names |
| recovered enum-body or parameter-list child | explicit ordered error/surface-only item | no `filter_map` omission |
| unknown/recovered action statement | explicit ordered unsupported/error statement | no `filter_map` omission |
| projection origin missing, duplicated, or reordered | coverage issue and incomplete projection | no downstream sealing |

Every diagnostic returned by `parse_cst` or `parse_source` has a UTF-8-valid
half-open byte span. Empty input uses `0..0`; a well-formed file with no system
uses an EOF span. Zero-width spans name insertion or file-level conditions and
remain stable without inventing source bytes.

The typed frontend remains a separate Phase 2 responsibility. In particular,
this boundary does not resolve imports, parse raw expression operators, prove
functions total, decide whether a raw expression is ambiguous, establish frame
conditions, or assign transition semantics.

# RFC 0021: Workflow source packages and import-bound replay

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-07
- Tracking issue: not assigned
- Design disposition: executable-only R2 increment; acceptance pending

## Problem and scope

The pure workflow profile needs reusable functions and records in separate
source files. Extend RFCs 0019–0020 with bounded local packages using the
canonical frontend's existing `import Name` declaration. This adds executable
meaning only on the explicit workflow route; M9, finite artifacts, and Lean
semantics are unchanged. Host effects, remote dependencies, version resolution,
package registries, lockfiles, and a build cache are outside this increment.

## Source layout and resolution

A package is an entry `.nmlt` file and the transitive closure of its imports,
all in the entry's directory. `import Arithmetic` loads `Arithmetic.nmlt` and
introduces the `Arithmetic` namespace in that file. No quoting, aliases,
subdirectory paths, dotted import paths, wildcard imports, or implicit directory
scan is added. Local `module` wrappers remain available inside each source.

```nmlt
// main.nmlt
import Arithmetic
fn main(input: Int) -> Outcome<Int> { Arithmetic.square(input) }
```

```nmlt
// Arithmetic.nmlt
fn square(input: Int) -> Outcome<Int> {
  if input < 0 { Err("negative input") } else { Ok(input * input) }
}
```

The compiler uses the complete canonical declaration projection for every
source. It interprets projected imports and the retained function/record slices;
it does not concatenate source strings, rescan top-level declarations, or alter
the shared parser. Imports are file-root declarations and duplicate imports
fail. Imports inside local module wrappers fail explicitly.

Entry and dependency basenames use a 1–64-byte ASCII identifier followed by
lowercase `.nmlt`. An identifier begins with a letter or underscore and continues
with letters, digits, or underscores. Windows device basenames are rejected on
every platform. Import names additionally exclude workflow keywords, built-in
types, and constructors. Module filenames must match import spelling exactly,
including on Windows. Distinct filenames differing only by ASCII case are
rejected within the closure. This is a deliberately portable first layout.

The entry file retains its unprefixed declarations and CLI entry names. Each
dependency receives its imported namespace: `Arithmetic.square`, or
`Library.Nested.function` for a local wrapper. Record names receive the same
prefix, giving them stable nominal identity across callers. Record-only files
are supported. At least one function must exist in the complete package.

Unqualified calls and record types search the current local module, then the
current file's root. Qualified references beginning with a directly imported
module name resolve there; other qualified references start at the current
file's root. A dependency cannot see entry-file locals. Loading a transitive
dependency does not introduce its name in the caller: explicit references to
`Shared.function` or `Shared.Record` require `import Shared`. Values returned
by a library may still expose a transitive nominal type through inference and
field access; imports control name resolution, not data secrecy.

An imported namespace cannot conflict with a file's top-level record, function,
or local module name. Entry-file declarations also cannot occupy a namespace
used by any loaded library, even if only transitively loaded. These collisions
fail instead of selecting a declaration implicitly. There are no access modifiers
or first-class module values; all library functions can be selected as entries
once the package loads them.

## Graph, typing, and source locations

Read the entry first, then visit each file's imports in lexical name order using
depth-first traversal. Shared dependencies load and parse once. Reject import
cycles and bound the longest root-to-dependency path, including already loaded
shared subtrees. Every declaration and branch in every loaded file is checked,
including unused library code. Unrelated sibling files are not part of the
closure and are neither loaded nor identified.

Typed lowering still resolves functions and locals to indices. Records and
function signatures resolve under their originating file's scope before
checking expressions. The existing record-dependency and function-call cycle
checks apply to the whole package. A library cannot hide invalid or recursive
code behind an unused import or empty fold.

Each `Location` now contains `source`, `start`, and `end`: a source-table index
and local UTF-8 byte offsets. No offset into concatenated text is used. Parser
and typing diagnostics retain the original file and span. Runtime stops in an
imported function retain that file's source index. `Program::source_path` maps
it to the logical package filename. The source table has deterministic traversal
order, so locations and typed-program identity are repeatable.

`compile(source)` remains a single-buffer API and rejects imports with an
instruction to use `compile_package`. Its synthetic source name is `source.nmlt`.
`compile_package(entry, reader)` loads the import closure through a caller-owned
text reader. The reader receives validated sibling basenames once each and must
bound its own I/O. The compiler validates returned source size before parsing.
The resulting `Program` remains privately constructed and is not deserializable.

## Bounds and filesystem behavior

| Package bound | Limit |
|---|---|
| Sources, including entry | 32 |
| Source bytes per file | 128 KiB |
| Source bytes across the closure | 1 MiB |
| Longest import path | 8 edges |
| Functions / records across the closure | 64 / 64 |
| Module nesting, expression/type/value limits, steps | Existing RFC 0020 limits |

The CLI roots dependencies in the canonical entry directory. It checks exact
filename case, regular-file metadata, and resolved parent paths; symbolic source
links are rejected for entries and dependencies. It does not fetch dependencies
or evaluate source during loading. Each source's bytes are captured once for
compilation, input decoding, and execution of a run. No second compilation after
argument decoding can silently select changed source.

These checks are not a filesystem capability sandbox or an atomic checkout
snapshot. A concurrently mutated filesystem can race metadata/path checks.
Hard links are not rejected, and a reader supplied to the library is trusted
to return the requested text. The record binds the actual per-file bytes used;
it does not attest that all files came from one Git revision or instant in time.
Existing run outputs still use create-new semantics.

## Record and compatibility contract

The current profile is `pure-workflow-v3`; pure typecheck, run, and replay
schemas move to version 3. The run record retains the entry-source digest and
adds a required ordered `sources` manifest. Each entry contains logical `path`,
byte length, and SHA-256 of its exact source bytes. Typecheck/replay output also
includes this table. Runtime locations add the required source index. The typed
program digest now includes the full manifest as well as resolved definitions
and expression trees, binding even unused imported comments/declarations.

Replay reloads the closure from the explicitly supplied source path, checks every
manifest identity and the exact executable, recompiles, and recomputes the entire
execution. Missing or edited dependencies, changed closure membership, malformed
manifests, altered source indices, unknown fields, and duplicate keys fail.
An incomplete run remains incomplete after successful replay. Replay performs
no host jobs or other source-level effects.

Moving a package directory is allowed when the source bytes and dependency
basenames remain the same. Renaming the physical entry file during replay is
also allowed: its recorded logical basename remains the source-table identity.
Dependency names are part of import resolution and are not remapped. Diagnostic
and runtime source-table names describe logical sources; the supplied source path
selects their physical directory.

Versions 1 and 2 are explicitly rejected by this executable. Retain their original
binaries for old-record replay or run the source again to create version 3 evidence.
There is no silent migration of proof or execution evidence. Existing scalar and
collection examples retain their values and expression-step counts; digests,
schemas, and location representation change. The portable entry-filename rule
is also a compatibility restriction. Finite schemas and routes remain unchanged.

## Validation, assurance, and open work

The [four-file batch](../examples/pivot/package_batch/main.nmlt) passes records
and outcomes between Work, Arithmetic, and Reports. Three inputs `(-3,5)`,
`(4,9)`, and `(-1,-2)` return accepted 2, rejected 1, total 41. Controls cover
diamond imports, direct-import visibility, record-only dependencies, original
diagnostic spans, runtime file locations, cycles, alias/case collisions, package
limits, changed/missing dependencies, moved packages, malformed manifests, and
old-version rejection. Linux additionally checks symbolic-source-link rejection.

All new constructs are **executable-only**, with `assurance: none`. A matching
manifest/replay does not authenticate provenance, verify source translation,
certify host execution, or create a Lean theorem. The complete unchanged
Rust/Lean/NanoDA and R0 reproduction gate remains required. RFC acceptance and
independent publication review are separate open gates.

Hierarchical packages, explicit exports, dependency locks, race-resistant file
opening, source-level host effects, supported Lean/worker adapters, and the full
R2 exit gate remain future work. This increment supplies reusable typed source
components without claiming those later capabilities.

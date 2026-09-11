# RFC 0032: Lean source import discovery

- Status: Under review
- Date: 2026-09-11
- Milestone: R3, second implementation increment

## Problem and scope

RFC 0031 requires a manually ordered list of local Lean modules. Existing
projects commonly use a source directory and vendored dependencies, making
that list tedious to maintain and easy to leave incomplete. Add a source-root
manifest that discovers the static local header-import closure of the target module.

Use the pinned Lean executable's `--deps-json` interface and its
[Lean/Lake header parser](https://github.com/leanprover/lean4/blob/819816b2e0a3bf405af45ae5c7af2491d8f5bee6/src/Lean/Elab/ParseImportsFast.lean).
This reuses Lean's handling of comments, `module`, implicit imports, and
`public`/`meta`/`all` import flags. NMLT does not implement another Lean lexer.
Complete source rebuilding and final proof checking remain required.

## Project manifest

`nmlt-lean-project-v2` contains `source_roots`, `target_module`, `target` and
`permitted_axioms`. Roots are explicit normalized paths inside the selected
project; `.` selects the project directory. Already available vendored source
trees, including ordinary directories under `.lake/packages`, may be selected.
Dependencies are pinned by their captured source bytes. This profile does not
interpret Lake manifests, run package hooks, fetch packages or attest to a
package's Git revision. The original explicit-module v1 manifest remains supported.

All roots are considered when resolving a reachable module. Multiple matches
are an error, even when their bytes are identical. Local modules cannot shadow
the pinned toolchain's compiled modules. Missing imports, cycles, casing aliases,
one canonical source path assigned multiple module names, path traversal,
symbolic links
and Windows junctions/reparse points are rejected. Overlapping roots are
permitted only when reachable module resolution and file identities are unique.

## Binding and rechecking

Binding reads each source into a bounded immutable in-memory snapshot before
asking Lean to parse a retained copy. The JSON parser result must contain one
successful header and no errors; Lean can report errors in JSON with exit code
zero. Each discovered local import is resolved recursively. Toolchain imports
must have compiled files in the already pinned Lean `lib/lean` tree. NMLT stores
the local sources in dependency order and excludes unrelated project files.

The task retains the normalized explicit module sequence, original source-root
configuration, each module's source root and full native header, and the sorted
toolchain import set. This import report is also written as `source-imports.json`.
Source origins describe capture locations; they are not signed provenance.

Both proving and rechecking reparse the retained sources with the pinned Lean
executable. They require identical headers, a complete local dependency order,
no unreachable captured modules and the same available toolchain imports.
They then rebuild ordinary Lean modules and perform RFC 0031's exact target,
axiom, export-root and independent NanoDA checks. The working project and its
vendored directories are not consulted during this reconstruction.

Source imports are build-environment dependencies. They are distinct from the
actual direct proof references and transitive exported proof closure. A module
being captured, imported or built does not certify all its declarations.

## Formats, bounds and compatibility

New tasks and results use `nmlt-lean-task-v2` and `nmlt-lean-result-v2`. The task
has an optional `discovery` report: null for an explicit-module v1 manifest,
or `nmlt-lean-import-closure-v1` for source-root discovery. Candidate format v1
is unchanged because it still contains only the selected task hash and proof.
Old task/result formats require their retained original CLI executable; no
acceptance record is silently upgraded to a different implementation.

The discovery profile permits at most 16 roots, 64 local modules, 1 MiB per
source, 16 MiB total source, and 256 header imports per module. Root strings are
at most 1,024 bytes and 16 path components; traversed source directories have
at most 4,096 entries. Module names keep the existing bounded ASCII identifier
profile. Retained JSON may not exceed its 64 MiB read bound. Source modules,
headers and policy are included in the separately selected task hash.

The 768 MiB Lean memory limit, OS process bounds, 30-second process deadlines,
64 KiB raw-output/export cap, 4 KiB candidate grammar and axiom policy are
unchanged. Complex headers or larger proof closures can still exceed these
bounds and are rejected. The project, parser, capture/target binding, host and
exporter remain trusted. Process supervision is not filesystem/network isolation.
Dynamic file/module access performed by project metaprograms remains part of
that trusted host boundary; header discovery does not capture those accesses.

## Validation and remaining work

The example uses `src` and a vendored library, nested misleading comments,
module/public/meta imports and an unrelated module. It must bind in dependency
order, produce a NanoDA-checked proof, and recheck with the working project
unavailable. Negative cases cover changed dependency bytes, rehashed false
header metadata, extraneous sources, reordered imports, missing imports, cycles,
cached objects without source, ambiguous roots, file aliases, toolchain shadowing,
header errors, root escape,
module casing and linked roots (including a real Windows junction).

R3 remains in progress. Package resolution and larger exports, broader proof
automation, LeanInteract/REPL comparison, editor integration, asynchronous
project-proof jobs and the complete evaluation corpus remain future work.
Independent cross-family/human review and RFC acceptance remain open gates.

# RFC 0035: Bound Lean inspection and diagnostics

- Status: Under review
- Date: 2026-09-11
- Milestone: R3, fifth implementation increment

## Problem and scope

The local proof workflow can bind and check a target but offers little help
finding a suitable declaration or locating a rejected candidate's error. Add
bounded declaration lookup in the reconstructed task environment and retain
Lean's native structured compilation diagnostics.

Use the same source snapshots, selected task hash, exact CLI and Lean installation
identities, target comparison and process supervisor. Inspection is context
retrieval with `assurance: none`; it does not publish proof acceptance. Final
proof acceptance continues to require `prove`/`recheck` and independent NanoDA
checking. The closed candidate grammar and axiom policy are unchanged.

## Declaration lookup

```text
nmlt lean-task inspect --task task.json --task-sha256 HASH \
  --prefix Example. --limit 16 --lean-bin FILE --output NEW_DIR
```

The prefix follows the current bounded ASCII declaration-name profile, optionally
ending in a dot. The limit is an explicit integer from 1 to 16. The CLI checks
the task hash and implementation, rebuilds its saved modules, reparses discovered
imports when applicable, and compares the actual target with the bound metadata.
The original working project is not consulted.

A fixed helper importing `NMLTTask` queries the actual Lean environment. It
selects imported declarations whose displayed names start with the prefix,
sorts them by name, and reports the first requested entries. Each entry includes
its name, originating module, declaration kind, pretty-printed type, universe
parameters, transitive axioms and an `axioms_within_policy` annotation. The
matched count and `has_more` make truncation explicit. A missing prefix produces
an empty successful lookup, not a fabricated declaration.

The environment includes the target-binding helper and its Lean imports, as does
proof reconstruction. Queries do not search the network or resolve new packages.
The axiom annotation says only whether the reported set fits the selected policy;
it does not establish that the declaration proves the target. Draft declarations
can be shown with `sorryAx` and a false policy annotation.

`nmlt-lean-inspection-v1` has `status: context_only`, `assurance: none`, the task,
its selected hash, query, context and process evidence. The output includes
`inspection.json`, readable `context.md`, copied `task.json`/`task.sha256`, the
exact CLI and reconstructed Lean files. A later inspection can use this bundle
without the original project or original bind directory. `recheck` explicitly
rejects an inspection record as a proof acceptance record.

## Native diagnostics

R3 module compilation uses Lean's `--json` message output. A compiled stage
retains `diagnostics-N.json` under `nmlt-lean-diagnostics-v1`, alongside the raw
`stage-N.json`. It contains the module, captured source path and SHA-256, stage
path, process exit code, native messages and a count of other stdout lines.
The native message fields, severity and range are preserved.

Lean reports one-based lines and zero-based Unicode scalar columns. These are
not LSP UTF-16 columns; see the pinned
[position implementation](https://github.com/leanprover/lean4/blob/819816b2e0a3bf405af45ae5c7af2491d8f5bee6/src/Lean/Data/Position.lean)
and [native message format](https://github.com/leanprover/lean4/blob/819816b2e0a3bf405af45ae5c7af2491d8f5bee6/src/Lean/Message.lean).
For messages naming the current captured file, the CLI
maps valid positions to UTF-8 byte ranges in that exact source. Foreign files,
synthetic/out-of-bounds positions and reversed ranges remain unmapped; the CLI
does not open a diagnostic-supplied path or invent a location. Error text also
names the structured report. Warnings remain warnings, including unused draft
goals in a successfully bound source environment.

Lean's JSON mode wraps the fixed helpers' printed metadata in information
messages. The marker reader accepts those information payloads and still
requires exactly one matching record. It does not interpret a warning/error
payload as successful helper metadata. A nonzero compilation exit or a native
error message rejects the stage. Non-message output stays in the raw stage and
is counted separately; the diagnostic view is not a claim that arbitrary
project output is native compiler evidence. Argument, discovery and host failures
retain their existing error routes when no compilation report is available.

## Bounds, compatibility and trust

Each inspected type is at most 4 KiB, with at most 16 universe parameters and
64 transitive axioms per entry. The complete helper payload is limited to
48 KiB. Compilation still has the ordinary 64 KiB raw-output, 30-second deadline
and existing memory/tree policy. A diagnostic stage permits at most 256 messages
and 4 KiB for each reported file/kind name. Existing source, export and retained
JSON limits remain in force.

Task version 2 and accepted result version 4 are unchanged. Inspection and
diagnostics are new artifact families. Raw compilation stage output now contains
native JSON messages; old records continue to require their retained original
CLI executable. Inspection files do not satisfy the proof-result schema.

Project imports, initializers and metaprograms remain trusted host code and can
execute while inspecting. This is not filesystem/network isolation. Lean's
reported environment, axiom collector, diagnostics and binding helpers remain
implementation trust. Axiom annotations, diagnostic locations and declaration
lookup do not constitute independent proof checking or human approval.

## Interface comparison and decision

The [local validation record](../docs/reviews/r3-lean-inspection-2026-09-11.md)
compares the normal CLI, direct community REPL and LeanInteract on lookup, a
wrong proof, a valid proof and an admitted proof, including retained environment
reuse. It records exact sources, a Lean 4.33.1 build overlay and the runtime
evidence. The optional comparison script is separate from the required product
gate and is not a production REPL adapter.

Keep the native CLI as the implemented task boundary. Its exact installation,
source reconstruction, output bounds and final independent-checking contract
are already integrated. Retained REPL environments offer a useful future
interactive interface, but need a separate bound-session, invalidation,
containment and recovery contract before use in accepted project-proof workflows.
The comparison establishes finite behavior observations, not a performance or
containment equivalence claim.

## Validation and remaining work

Unit tests cover prefix/count limits, context consistency, axiom annotations,
Unicode/CRLF positions, foreign/synthetic ranges, warning/program-output
classification and uniquely wrapped helper metadata. Integration must inspect
local and toolchain declarations, report truncation/empty results, reconstruct
context from its retained bundle, locate a wrong proof, select an inspected
lemma, independently check the repaired proof and freshly recheck it. Negative
controls cover altered pins/sources/targets, command-like prefixes, excess limits,
using context as proof acceptance and an actual error after non-ASCII text.

R3 remains in progress. Broader package/library support, editor/LSP integration,
a persistent REPL adapter, proof automation, asynchronous project-proof jobs and
the full evaluation corpus remain open. Independent cross-family/human review
and RFC acceptance remain open as well.

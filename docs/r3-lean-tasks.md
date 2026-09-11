# Bound tasks in local Lean projects

The first R3 increment adds `nmlt lean-task bind`, `prove`, and `recheck` under
[RFC 0031](../rfcs/0031-bound-lean-project-tasks.md). It supports trusted local
Lean source projects and closed candidate proof terms. An accepted result
requires a fresh Lean build, exact target binding, a transitive axiom check,
and an independent NanoDA check of the final exported proof.
The [validation record](reviews/r3-local-lean-tasks-2026-09-09.md) records the
executed cases, tool pins and remaining review gates.

The second increment adds automatic local source import discovery under
[RFC 0032](../rfcs/0032-lean-source-import-discovery.md), including source
directories and explicitly selected vendored dependencies.
Its [validation record](reviews/r3-import-discovery-2026-09-11.md) includes
the complete Windows reproduction, Linux CI and import-discovery controls.

The third increment streams proof exports to bounded files under
[RFC 0033](../rfcs/0033-bounded-lean-proof-exports.md). Exports may contain up to
16 MiB, with exact byte counts and SHA-256 receipts retained in the result.
The [validation record](reviews/r3-bounded-exports-2026-09-11.md) records local
process, artifact and independent proof checks.

Binding a task does not record human approval. A reviewer must select the task
hash after examining the statement, assumptions, definitions, sources and policy.
The candidate channel then supplies only a proof for that selected hash.

## A complete example

The [example project](../examples/lean-project/nmlt-lean.json) contains two
ordinary Lean modules, a reusable `offset` definition, and target placeholders.
An ordered module list makes the supported local dependencies explicit. Each
module is rebuilt from source in a new directory; old `.olean` files, Lake
build hooks, and the working project's search path are not reused.

First build the CLI and checker tools. `LEAN_BIN` must name the direct
executable in a Lean 4.33.1 installation. The preparation script fetches the
same immutable lean4export/NanoDA revisions used by the metatheory gate.
It records the Lean pin and an empty Cargo workspace table added to the
downloaded NanoDA manifest so the checker can build inside this checkout;
the checker's Rust sources and dependency lock are unchanged.

```bash
cargo build -p nmlt-cli
mkdir -p target
bash tools/prepare_lean_task_checkers.sh target/lean-task-checkers
target/debug/nmlt lean-task bind \
  --project examples/lean-project --lean-bin "$LEAN_BIN" \
  --output target/reviewed-task
```

Inspect `target/reviewed-task/task.json`, especially its exact elaborated type,
sources, policy and identities. Its `task.sha256` identifies this complete
snapshot. Create a separate candidate for that selected task:

```json
{
  "schema": "nmlt-lean-proof-candidate-v1",
  "task_sha256": "REPLACE_WITH_THE_SELECTED_TASK_HASH",
  "proof": "fun n => Example.offset_eq n"
}
```

Save it as `candidate.json`, then run:

```bash
TASK_HASH="$(cat target/reviewed-task/task.sha256)"
EXPORTER="target/lean-task-checkers/lean4export/.lake/build/bin/lean4export"
NANODA="target/lean-task-checkers/nanoda/target/release/nanoda_bin"
target/debug/nmlt lean-task prove \
  --task target/reviewed-task/task.json --task-sha256 "$TASK_HASH" \
  --candidate candidate.json --lean-bin "$LEAN_BIN" \
  --exporter "$EXPORTER" --nanoda "$NANODA" --output target/proof-result
target/debug/nmlt lean-task recheck \
  --record target/proof-result/result.json --task-sha256 "$TASK_HASH" \
  --lean-bin "$LEAN_BIN" --exporter "$EXPORTER" --nanoda "$NANODA" \
  --output target/proof-recheck
```

On Windows, use the corresponding `.exe` files. Every output directory must
be new. The result retains source snapshots, the candidate, target identity,
actual direct proof references, the exported declaration closure, exact tool
identities, process diagnostics and the independently checked export. It also
includes ordinary Lean files, an additive `proof.patch`, an explanation, and
the exact CLI executable. The bound task also retains that executable.
Applying that patch adds a new checked theorem; it does not silently overwrite
the original goal's proof body.

Recheck rebuilds the saved sources and repeats all checks. It requires the
selected task hash and the same trusted tool installations/CLI executable,
but works after the original project is edited or removed. Invoke the retained
`nmlt`/`nmlt.exe` if the current checkout's executable has changed. Old subprocess logs
are diagnostic history; fresh checking establishes the new acceptance.

## Discovering project imports

The [discovery example](../examples/lean-project-discovery/nmlt-lean.json) selects
two source roots instead of listing every module:

```json
{
  "schema": "nmlt-lean-project-v2",
  "source_roots": ["src", "vendor/support"],
  "target_module": "App.Goals",
  "target": "App.checked_value",
  "permitted_axioms": []
}
```

Run the same binding command against that project:

```bash
target/debug/nmlt lean-task bind \
  --project examples/lean-project-discovery --lean-bin "$LEAN_BIN" \
  --output target/discovered-task
```

Lean identifies static header imports from captured source copies. NMLT resolves them against
the selected roots and pinned standard library, then builds local sources in
dependency order. For this example the order is `Support.Core`,
`App.Definitions`, `App.Goals`; `App.Unused` is outside the closure. The candidate
proof is `fun n => App.value_eq n`. Use the same `prove` and `recheck` commands
above with the discovered task and its separately selected hash.

Inspect `source-imports.json` alongside `task.json`. The import report records
source roots and Lean's complete import headers, including implicit imports and
`public`/`meta`/`all` flags. Proving and rechecking regenerate those headers from
the saved sources and compare the closure before compiling. The original source
and vendor directories can be unavailable during rechecking.

Roots use normalized paths relative to the project; `.` selects the project
directory. An existing ordinary vendored directory under `.lake/packages` can
also be selected. Source bytes pin the dependency snapshot. Lake lockfiles,
download hooks and package resolution are not interpreted by this profile.
Ambiguous module matches, import cycles, missing imports, module/file aliases,
toolchain shadowing, casing mismatches, path escapes and linked source paths
(including Windows junctions) are errors. Root order does not resolve ambiguity.
Dynamic file/module access performed by trusted project metaprograms remains
within the host trust boundary described below.

New task artifacts use version 2, with an optional discovery report. New result
artifacts use version 3 to record the file export and its capture policy.
Explicit-module v1 project manifests remain supported. Old saved task/result
artifacts continue to require their retained original CLI executable.

## Larger proof exports

The [larger-export example](../examples/lean-project-large-export/nmlt-lean.json)
selects `Large.multiplication`, with candidate
`fun a => fun b => fun c => Nat.mul_assoc a b c`. Bind, prove and recheck it with
the commands above, substituting this project and the newly selected task hash.
Its proof closure contains 97 declarations and exports 136,729 bytes, exceeding
the earlier 64 KiB bound. Its manifest explicitly permits `propext`, which this
closure needs; removing that permission rejects the proof.

The export is `build/environment.ndjson` inside the result directory. The
`lean4export` stage records a `stdout_file` containing its relative path, capture
policy, byte count and SHA-256 digest; its ordinary stdout field is empty.
The result's `export_bytes` and `export_sha256` must match that receipt. NMLT
compares the saved bytes with the receipt before inspecting the declaration
closure, runs NanoDA, and checks the file identity again before acceptance.
Fresh rechecking creates a new export from the saved sources and compares it
with the recorded identity.

Raw stdout streams directly to a newly created file. Capture must reach a clean
EOF and flush successfully; timeout, cancellation, excess output and file I/O
errors prevent a capture receipt and a successful proof result. A failed run
can retain a bounded partial file for diagnosis. A complete capture still
preserves the process exit status, and proof acceptance requires exit zero and
empty stderr. The capture receipt identifies bytes; it does not certify a proof.

## What is fixed and what can change

The task hash binds all listed modules, their order, the exact selected
declaration type and universe parameters, the complete Lean `bin`/`lib` tree,
the CLI implementation and the permitted axioms. A change creates a new task.
Old acceptance remains about its original snapshot and does not migrate to a
changed statement or definition. Candidates cannot select another statement,
revise imports, or add checking options.
For a discovered project it also binds the selected roots and native import
headers. Source imports describe the build environment; the result's proof
references and exported declarations describe actual proof dependencies.

Only a selected subset of `propext`, `Quot.sound`, and `Classical.choice` may
be permitted. `sorryAx` is always forbidden. The example's placeholder goals
may remain in the project, but using one as a proof dependency is rejected.
Unresolved draft lemmas cannot become a completed root through bookkeeping.

## Current bounds and trust

The profile supports at most 64 ordered local modules, 1 MiB per module,
16 MiB of source in total, the existing 4 KiB closed proof-term grammar, and
64 KiB of raw output for ordinary processes. Only the independent proof export
uses the separate file policy: at most 16 MiB raw stdout, 64 KiB raw stderr and
a 64 KiB line-assembly buffer. The declaration parser still reads the bounded
export into memory; this is not a constant-memory proof checker.
Larger closures are rejected. Each process has a 30-second
deadline and the existing process memory/cleanup policy. Lean's own memory
limit is 768 MiB: the Linux target helper exceeded 512 MiB and measured about
596 MiB peak resident memory during calibration. The OS bounds remain 1 GiB
for a Windows process job and 2 GiB per POSIX process data segment. Lean thread
stacks are explicitly 64 MiB. Metadata and universe-name limits may reject complex
targets. Such rejection is not a claim that the target is false. Discovery
allows at most 16 roots, 256 imports per module and 4,096 entries in each
traversed source directory. Root paths are at most 1,024 bytes and 16 components.

The project is trusted host code: imports, initializers, elaborators and
instances can execute during a build. Process supervision is not filesystem
or network isolation. The host, loader, supplied tool installations, source
capture, target-binding helper and exporter remain trusted. Content hashes
are identities, not signatures. NanoDA establishes proof validity for the
exported declarations under the selected axioms; human faithfulness, novelty
and usefulness are separate reviews.

R3 remains in progress. Mathlib-scale package resolution and exports,
editor/REPL integration, broader proof automation, and asynchronous `.nmlt`
jobs for these project tasks are not implemented by this increment.

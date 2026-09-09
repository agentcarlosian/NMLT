# Bound tasks in local Lean projects

The first R3 increment adds `nmlt lean-task bind`, `prove`, and `recheck` under
[RFC 0031](../rfcs/0031-bound-lean-project-tasks.md). It supports trusted local
Lean source projects and closed candidate proof terms. An accepted result
requires a fresh Lean build, exact target binding, a transitive axiom check,
and an independent NanoDA check of the final exported proof.

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

## What is fixed and what can change

The task hash binds all listed modules, their order, the exact selected
declaration type and universe parameters, the complete Lean `bin`/`lib` tree,
the CLI implementation and the permitted axioms. A change creates a new task.
Old acceptance remains about its original snapshot and does not migrate to a
changed statement or definition. Candidates cannot select another statement,
revise imports, or add checking options.

Only a selected subset of `propext`, `Quot.sound`, and `Classical.choice` may
be permitted. `sorryAx` is always forbidden. The example's placeholder goals
may remain in the project, but using one as a proof dependency is rejected.
Unresolved draft lemmas cannot become a completed root through bookkeeping.

## Current bounds and trust

This increment supports at most 64 ordered local modules, 1 MiB per module,
16 MiB of source in total, the existing 4 KiB closed proof-term grammar, and
64 KiB of raw output per process. The independent export also has that 64 KiB
bound; larger proof closures are rejected. Each process has a 30-second
deadline and the existing process memory/cleanup policy. Lean thread stacks
are explicitly 64 MiB. Metadata and universe-name limits may reject complex
targets. Such rejection is not a claim that the target is false.

The project is trusted host code: imports, initializers, elaborators and
instances can execute during a build. Process supervision is not filesystem
or network isolation. The host, loader, supplied tool installations, source
capture, target-binding helper and exporter remain trusted. Content hashes
are identities, not signatures. NanoDA establishes proof validity for the
exported declarations under the selected axioms; human faithfulness, novelty
and usefulness are separate reviews.

R3 remains in progress. Mathlib-scale dependency discovery and exports,
editor/REPL integration, broader proof automation, and asynchronous `.nmlt`
jobs for these project tasks are not implemented by this increment.

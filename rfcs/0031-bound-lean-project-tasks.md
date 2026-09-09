# RFC 0031: Bound Lean project tasks

- Status: Under review
- Date: 2026-09-09
- Milestone: R3, first implementation increment

## Problem and scope

Init-only jobs cannot prove a task in an existing Lean source environment.
A candidate must not choose its own statement and then call that the requested
result. Introduce an executable-only `nmlt lean-task` artifact workflow that
binds an exact declaration type, source modules, Lean installation, and axiom
policy before accepting a separate proof candidate.

The first profile supports trusted local Lean projects whose explicitly ordered
module list builds using the pinned Lean installation and those modules alone.
Vendored dependencies may appear in that list. It does not download packages,
run Lake build hooks, accept arbitrary candidate commands/tactics, or claim an
OS filesystem/network sandbox. Imported project elaborators and initializers
are trusted host code. Mathlib-scale package discovery, editor integration and
native asynchronous source jobs for these tasks remain later R3 work.

## Artifact and command contracts

`lean-task bind` reads `nmlt-lean.json`, snapshots its ordinary `.lean` modules,
builds them into a new directory, and asks Lean for the exact target declaration
type, universe parameters, and type references. A task hash commits to the
complete snapshot, policy, toolchain identity and CLI implementation identity.
Creating this artifact records a binding, not human approval or mathematical
faithfulness. A reviewer selects its hash outside the candidate channel.

`lean-task prove` requires that hash and a candidate JSON containing only the
task hash and a bounded proof term. It reconstructs the saved modules in a new
directory, checks the target metadata again, and constructs a kernel-checked
`NMLTTask.target : Prop` whose value is precisely the selected declaration type.
An ordinary theorem in `NMLTProof.lean` proves this fixed proposition. Lean's
actual proof references and transitive axioms are recorded. The candidate
channel reuses RFC 0029's closed grammar; it cannot revise imports, definitions,
the statement or checking policy.

The pinned-format lean4export adapter exports this root and its dependency
closure. NanoDA must independently accept the resulting artifact under the
selected policy before an accepted record is published. The exporter and checker
identities, root, export digest and exact process outputs are retained. An empty
or wrong-root export is rejected. A subprocess exit alone is insufficient.

`lean-task recheck` takes the saved record and the externally selected task
hash. It rebuilds all saved sources and repeats Lean, export and NanoDA checking
in a fresh directory, requiring identical target and proof artifacts. It needs
the same trusted tool installations, but no generating model, original working
project, or cached `.olean` from the previous run.

## Revisions, resources and trust

Changes to any captured source, assumption, definition, target, module order,
axiom policy or tool identity produce another task hash. Existing acceptance
does not migrate to that revision. Allowed axioms are an explicitly selected
subset of `propext`, `Quot.sound`, and `Classical.choice`; `sorryAx` is forbidden.
Unresolved project lemmas may exist, but final proof acceptance depends only on
the actual exported closure and its axiom policy. Draft plans are not evidence.

The profile bounds source counts/bytes, JSON inputs, process output, memory and
per-process time using the existing process supervisor. The host filesystem,
loader, trusted project code, capture/comparison tools and exporter remain
trusted. Independent proof validity does not independently establish statement
faithfulness, novelty, or the correctness of the host's bindings. Byte hashes
identify inputs and are not signatures or tamper-proof storage.

The first export profile retains the supervisor's 64 KiB raw stdout limit;
larger proof closures fail closed. The native exporter avoids loading its full
Lean source environment into the elaborator during every export. A broader
streaming export profile is later R3 work.

## Compatibility and validation

This is a new versioned artifact family; R2 source/job/project formats and Lean
behavior semantics do not change. Unknown fields/versions and duplicate JSON
keys fail closed. Existing result directories are never overwritten.

Positive controls cover a local definition, an assumption-bearing lemma and a
polymorphic target. Negative controls cover wrong terms, unresolved lemmas,
transitive forbidden axioms, altered definitions/statements/pins, source path
escapes, malformed candidates, wrong-root exports and changed retained evidence.
A fresh recheck must succeed after the working project is changed or removed.
Independent review and broader R3 integration remain open gates.

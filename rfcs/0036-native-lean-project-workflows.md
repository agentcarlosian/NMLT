# RFC 0036: Native Lean project workflows

- Status: Under review
- Date: 2026-09-11
- Milestone: R3

## Contract

Extend bound tasks to ordinary pinned Lake projects, native proof development,
explicit revisions and durable source jobs. Every accepted route retains the
same target comparison, transitive axiom policy, complete exported dependency
graph and independent NanoDA check. A task binding records the supplied target
and policy; it does not assert human approval or statement faithfulness.

`nmlt-lean-project-v3` selects `target_module`, `target` and `permitted_axioms`.
Its directory contains the pinned `lean-toolchain`, ordinary Lake configuration
and native `lake-manifest.json`. Existing explicit-module v1 and discovered-root
v2 manifests keep their distinct reconstruction rules.

Native Lake resolves the selected lock with the exact Lean installation. NMLT
captures the root configuration, lock, source inputs and dependency provenance.
Git dependencies require clean checkouts at full pinned commits; path packages
are bound to their captured bytes. Versioned build inputs, including upstream
JavaScript and trace files outside build directories, remain byte exact. Lean
compiled artifacts and each package's Lake directory are excluded. Links,
Windows junctions, escapes and excessive inputs are rejected.

Reconstruction copies only captured inputs into new package directories and
supplies Lake's native path overrides. Original package configuration and
lockfile bytes remain unchanged. Lake builds the selected target with cache
fetching disabled and rehashing enabled. Its actual search environment and
target source are retained and compared on reconstruction. Helper compilation
uses native `lake lean`, preserving package options and plugin setup.

The task schema is `nmlt-lean-task-v3`; the result schema is
`nmlt-lean-result-v5`. A Lake task commits to its external `lake-sources`
manifest and bytes. Those files must accompany a task or result. The selected
task hash, exact CLI and tool installation pins remain mandatory.

## Development and revision

The original closed-term candidate remains v1. Candidate v2 is a bounded list
of structured native automation steps: intro, exact, apply, explicit
`simp only`/`simpa only`, assumption, rfl, constructor, omega, grind and decide.
Terms use the existing closed grammar and names are validated separately.
Candidate data cannot add imports, commands, arbitrary tactics, policy changes
or a different target. A fixed Lean tactic import supports the native steps.
Automation still must produce a closed final proof under the selected policy.

`lean-task candidate` converts a proof body or a generated `NMLTProof.lean`
into candidate JSON. Full files must preserve every wrapper byte outside the
proof body. `lean-task workspace` produces a normal pinned Lake project with
captured path dependencies, target and editable proof modules. The native
server provides diagnostics, hover and goal state. Workspace and inspection
results have `status: context_only` and `assurance: none`; a draft with `sorry`
is not an accepted proof.
The workspace's final native Lake build uses the project process policy even
for an explicit-module or discovered-root task; `workspace.json` records it.
This does not change that task's proof-checking policy.

`lean-task revise` freshly binds a source project and records its parent's task
hash and target with an explicit reason. A no-op is rejected. The revision
record invalidates dependent acceptance for the revised task: old candidates
and results remain historical evidence for their original hash and cannot
satisfy the new task. Records are not rewritten or silently migrated.

## Asynchronous project proofs

`job_start_lean_project(alias: Text, proof: Text) -> Job<Text>` selects an alias
from an operator-supplied registry outside the candidate channel. It uses the
existing affine handles, start/poll/cancel/collect operations, durable attempt
ledger, bounded slots and generation checks. Its transitive effect is separate
from Init-only Lean jobs. Source commands select `--lean-projects`; project
manifests select `tools.lean_projects`. Project locks bind the complete registry
identity as well as imported workflow sources and the executable.

The registry fixes task hashes, Lean/exporter/NanoDA paths and identities, and
the project deadline. The session retains task bytes, captured Lake inputs,
the registry configuration, input manifest and worker executable. A fixed
worker receives only the exact durable dispatch. A successful response contains
a small receipt binding alias, task, dispatch, result and export digests. The
worker runs the same complete `prove` pipeline. The source host validates the
retained record, candidate, target, policy, export, dependency graph, checker
count and accompanying proof files before returning its receipt.

On Windows, a worker executes native builds in a short unique directory under
the temporary root captured by the host. It durably records that location
beside the dispatch before building. It retains the completed bundle under the
dispatch digest before publishing a response. A cross-volume fallback copies
a bounded portable bundle and publishes `result.json` last. Interrupted working
directories remain diagnostic artifacts; their existence cannot settle an
unknown attempt. No global path or registry setting is changed.

Source replay validates recorded decisions and proof artifacts without
starting Lean or regenerating candidates. Resumption checks retained inputs,
the exact CLI and tool identities, and accepted artifacts. Dispatched work
without a settled observation becomes unknown and is never redispatched.
Explicit operator reconciliation can settle unknown work only as failed, with
its attempt charge retained. Cancellation and stale/late results preserve the
existing generation and settlement rules. A scheduler receipt cannot complete
an unresolved formal lemma.

## Resource bounds and trust

Ordinary R2 jobs retain their 30-second process deadline, 4 KiB protocol text
limit and existing OS resource policy. The project adapter alone permits a
canonical request up to 32 KiB, carrying at most 8 KiB of proof text. Workflow
`Text` inputs retain their existing 4 KiB bound. Its
deadline is independently selected in 1–1,800,000 ms. The project process
profile uses 8 GiB Windows whole-job commit or 8 GiB POSIX per-process data
limits. Windows bounds each contained tree to sixteen processes. Build and
checker stages have a one-core rate cap. The fixed worker coordinator uses
`nmlt-contained-project-worker-v1`, with no additional Windows CPU rate cap:
Windows calculates a nested cap relative to its parent, so repeating a
one-core cap divides the stage's allocation again. The coordinator retains
the project memory, process-count, deadline and tree-cleanup limits and runs
proof stages through their existing capped policies. See
[Microsoft's nested CPU-rate contract](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_cpu_rate_control_information).
POSIX uses its documented process-group and rlimit fallback. File exports
may use the project-specific 64 MiB capture policy. Ordinary exports retain
their 16 MiB policy. Pipe bounds remain 64 KiB.

Lake capture permits 64 dependencies, 32,768 files, 16 MiB per input file and
512 MiB total. Project registries permit sixteen aliases. Session inputs have
an independent 65,536-file, 64 MiB per-file and 1 GiB total bound. Graph and
export-parser structural limits remain unchanged. Bounds reject unsupported
work; they are not claims that a mathematical target is false.

Trusted project configurations, build hooks, metaprograms, the host filesystem,
loader and tool installations remain trusted. Process containment is not a
filesystem or network sandbox. Hashes and structural replay are not signatures
or authenticated execution history. NanoDA independently checks the exported
proof; host binding, human meaning and implementation review remain separate
trust obligations.

## Evidence required

The frozen R3 set must cover ordinary local and Lake projects, pinned library
dependencies, native editor diagnostics/goals, automation, definition and
statement changes, repaired candidates, asynchronous failure/cancellation and
interruption. Every reported accepted proof must pass fresh independent
rechecking from its retained bundle without the generating model or original
source project. Mutation controls must reject stale pins and changed artifacts.
Record the exact evidence, remaining platform limits and independent review
status in the completion audit. Passing implementation tests does not silently
complete an independent human review.

# R3 completion audit

Objective: finish R3. This audit follows the requirements in
[Plan.md](../../Plan.md#r3--support-existing-lean-projects) and the
[practical language plan](../practical-language-plan.md#r3--make-existing-lean-projects-a-supported-workflow).
It does not replace those requirements with the existing implementation's
smaller bounds. The starting local checkpoint is
`56c626209e0fd75c37b57e5cb03c931f76f7177a`.
The integration is committed locally as
`fd265ba03721869a65e397f2255d8ea2740a8297`; its audit status was still pending
the final library checks at that checkpoint.

Status: **R3 complete at local pre-alpha scope on 2026-09-11**. The full
reproduction, final Mathlib-inclusive frozen set and additional Mathlib source
job all passed. Every reported accepted proof in those sets passed a fresh
independent check. The final local requirement audit found no remaining R3
implementation or executable exit-gate work. All work remains local.

| Requirement | Implementation and executed evidence | Final audit state |
|---|---|---|
| Accept a pinned Lean project and dependency environment | Native Lake lock resolution, clean pinned Git provenance and byte-exact input capture; vendored example and a 10-package Mathlib environment built successfully | Final corpus and source job passed |
| Bind declaration, assumptions, definitions and formal target identity | Lean target reconstruction and equality checks; explicit statement, definition and policy revisions; old candidate/result pins rejected | Final positive and negative cases passed |
| Keep statements, candidates and results distinct | Task v3, separate closed-term/structured-automation candidates and result v5; candidate file converter and revision lineage; no-op revision rejected | Final positive and negative cases passed |
| Reuse Lean diagnostics, retrieval and compatible interfaces | Earlier CLI/REPL/LeanInteract comparison; real native Lake LSP initialization, hover, goal query, diagnostics, edit repair and shutdown | Native editor exercise passed |
| Reuse ordinary proof automation | Native exact, simp, omega and grind proofs; explicit policy controls; all seven local development cases freshly rechecked | Full reproduction and final corpus passed |
| Reconstruct ordinary Lean files in a clean pinned environment | New Lake package trees from captured inputs and native path overrides; original configs retained; compiled caches excluded; source identity checked before and after builds | Vendored and Mathlib reconstructions passed |
| Enforce transitive axioms and independently recheck every reported proof | All proving routes use target comparison, policy checks, export validation and NanoDA; every accepted final proof freshly rechecked | All 13 final proof/recheck pairs passed |
| Separate draft plans from actual dependencies | Exact graph rederived from the exported proof; draft editor output is context only; wrong terms and forbidden axioms cannot become completed proofs through scheduling | Positive and negative cases passed |
| Export readable ordinary artifacts for exact declarations | Lean modules, additive patches, definitions, graph reports and explanations; normal pinned editor workspace with captured path dependencies | Generated artifacts and native LSP exercised |
| Integrate asynchronous source and project workflows | Typed alias/proof dispatch; poll/cancel/collect; repair and parallel jobs; project lock/test/replay/resume; actual driver interruption, unknown-state reconciliation and preserved dispatch identity | Complete 36-case final runner passed |
| Complete the frozen R3 proof set | Native development and source-job scripts with explicit acceptance/failure/unknown outcomes; each accepted case has an independently generated fresh record | Full reproduction, final 8-case native set and additional Mathlib source job passed |
| Supply contracts and verification evidence | RFC 0036, current user guide, trust inventory, runtime/workflow tests, workspace Clippy and Linux cross-compilation | Full reproduction passed; RFC disposition and independent human/publication review remain separate |

Advanced coupled subgoal search is conditional in the plan and has no
demonstrated requirement yet. The proposed 30-task comparative corpus, paired
model trials and independent audience pilots belong to R5. They are not
substitutes for the integrated R3 correctness and replay gate.

Native Lake LSP is the implemented interactive editor route. The earlier
[CLI/REPL/LeanInteract comparison](r3-lean-inspection-2026-09-11.md#cli-repl-and-leaninteract-comparison)
retains sixteen agreeing frontend observations over lookup, wrong proof, valid
proof and admitted draft cases. It informed that choice; it is not a proof
acceptance adapter. The native final set additionally checks live hover,
an unfinished goal, a located type error and a repaired document.

## Completed focused evidence

The following ignored local directories retain exact CLIs and complete input,
result and diagnostic bundles. They are executable evidence, not just a report
of intended tests.

| Bundle | Executed scope |
|---|---|
| `target/run-4lhmlsko/summary.json` | Seven accepted native development cases, nine rejection controls and actual native LSP; all seven fresh rechecks passed after original authoring and binding paths became unavailable |
| `target/r3-mathlib-development/{bound-2,proof-2,fresh-2}` | Ten packages, 9,851 captured files and 116,959,166 input bytes; source rebuild, proof export, NanoDA and fresh independent reconstruction all passed |
| `target/r3-project-jobs-dbivj0un/summary.json` | Thirty-six source/project checks; direct, repaired, parallel and project-test proofs all passed fresh rechecking; cancellation, timeout, interruption, acknowledgement, changed lock/pin/export/source controls passed |

The Mathlib example pins commit
`0df444a360eaa60ab8c11dca51a86af692955474` with Lean 4.33.1. Its selected task
is `6cac1eac95aca65bab8ff66acc676155a46b2008f5faa583759aeb0786d19b6f`.
The proof and fresh record both contain 46 checked declarations, no axioms and
the 52,579-byte export digest
`d4249a06ad87222a4891cdcad33abfaa21410a9d63ebf28bf0d01110499a6202`.
This validates a large package environment with a small selected proof closure;
it is not an assertion that every Mathlib target fits the current bounds.

The seven development cases include 48-declaration direct, exact, editor and
revised proofs, a 62-declaration simp proof, a 1,405-declaration omega proof and
a 717-declaration grind proof. Their actual axiom sets are retained separately;
the arithmetic examples require explicitly permitted standard axioms.

The asynchronous set binds task
`606ab1386235e7e56bfa73eb51c0481cc44a4d35497f8d95ef062fa5334ff297`.
Its four fresh records match export
`f5c11e97d01e5354648a79dffd41138fa0c8e7ca81b732a414049ee40940e9cf`.
The timeout and killed-driver cases retain their dispatched attempt and settle
only after an explicit failure acknowledgement; neither reuses a late proof
file as an accepted completion. Completed project resumption leaves the
original journal unchanged and replays from a separate run directory.

All current runtime and workflow tests and workspace Clippy passed. Linux
cross-compilation passed; native execution in this session is on Windows.
The complete reproduction passed at 21:09:17 UTC. Its native development set is
`target/run-s187e6jx` and its source/project set is
`target/r3-project-jobs-05232ij8`. All eleven accepted proofs passed fresh
rechecking. A separate byte-level readback verifies their retained CLI hashes,
export identities, proof files, dependency graphs, axiom sets and NanoDA counts
against both records: `target/r3-readback-reproduction-20260911.json`.

## Final gate

- Full reproduction: passed, log `target/r3-full-reproduction-20260911-2.log`.
- Final native/Mathlib corpus: passed, `target/run-_i6exng7/summary.json`;
  eight accepted proofs, eight fresh rechecks, nine rejection controls and
  the native LSP exercise.
- Final source/project corpus: passed,
  `target/r3-project-jobs-05232ij8/summary.json`; 36 checks and four accepted
  proofs with four fresh rechecks.
- Combined Mathlib source job, replay and fresh recheck: passed,
  `target/r3-mathlib-job-dzvuqcgl/summary.json`. Its complete validation script,
  command arrays and raw outputs accompany the results.
- Final readback: passed for all 13 proof/recheck pairs in these three sets,
  `target/r3-final-evidence-20260911.json`; the copied reader is
  `target/r3-final-readback-20260911.py`.
- Local diff review, final evidence readback and milestone status update:
  complete. Both final library runners exited zero by 21:40:54 UTC.

Both final Mathlib routes bind task
`98b543a7300a2326998aa3a0ce78c8b1fe4f94920212e06ce60ceac0798228b3`.
They independently reconstruct the ten-package, 9,851-file environment and
freshly reproduce the 46-declaration, empty-axiom proof export
`d4249a06ad87222a4891cdcad33abfaa21410a9d63ebf28bf0d01110499a6202`.
The source-job receipt binds result digest
`021fce9d8a1fdd6b52f8c9dae3500eec63bb996bb856c8badeaa0e5accff3c7f`.

The final readback JSON SHA-256 is
`9df6a075aa173630d9eef1c0cebb48f9ed76032ac0f4dc3700602c7bc2c9779b`;
the full reproduction log SHA-256 is
`cc2175d2fa952fb85ceb4919d4061fb8beff01425c2f64e66b558ae99a0d6121`.
Readback compares task, candidate, tool identities, exact export bytes,
declaration counts, axiom sets, proof files and graphs across each original and
fresh result. It summarizes completed Lean/NanoDA executions; it does not
replace either checker with hash comparisons.

## Reproduction and implementation identity

The native Windows reproduction ran from 20:40:28 to 21:09:17 UTC on
2026-09-11. The process exited zero. It invokes the complete `make reproduce`
dependency graph, including Rust formatting/checks/Clippy/tests, public-surface
checks, the Lean metatheory and independent checker, finite execution/parity,
R0 baselines, R2 lifecycle/source/project gates and the default R3 runners.

The R3 runners are checked in under `tools/check_lean_*.py`. The optional library
run is explicit because it rebuilds a large dependency closure three times:

```bash
make SHELL=bash.exe PYTHON=python "R2_LEAN_BIN=$LEAN_BIN" reproduce
python tools/check_lean_lake.py --mathlib \
  --lean-bin "$LEAN_BIN" --exporter "$EXPORTER" --nanoda "$NANODA"
python tools/check_lean_project_jobs.py \
  --lean-bin "$LEAN_BIN" --exporter "$EXPORTER" --nanoda "$NANODA"
cargo check --workspace --all-targets --target x86_64-unknown-linux-gnu
```

Use a direct Lean executable and exporter/NanoDA installations prepared with
`tools/prepare_lean_task_checkers.sh`. This session used Rust 1.94.0 and
`CARGO_PROFILE_DEV_OPT_LEVEL=1`, `CARGO_PROFILE_TEST_OPT_LEVEL=1`. The checker
sources are pinned to Lean 4.33.1, lean4export
`411dce7db58a3afc60ecab2d211acd1042b593dc`, and NanoDA
`05055695879dfebb6628a67da88ceca6cd6b0421`.

The complete-gate CLI is
`b4ac650c55e296f9d771e7ab066e6c9912d0444f485529802b456ccd1f793108`.
The separately frozen final library run retains CLI
`d6bcfdafccf969e2912d2efc87edae15ba700f367bee17d0e03b11cffe572198`.
The only later product change selects and records the project process policy
for the final native editor build of an explicit/discovered task. The proof,
source-job and fresh-checking pipelines are unchanged. The full gate exercises
that editor correction with both explicit and native Lake tasks. Every task
and fresh result retains its own exact CLI; these pins are not interchanged.

The Lean installation digest is
`384af7d136e934931ae540b12cb6ae15799fe1c3d3f4d3a63ddfb66e39cee097`.
Every result also retains separate exporter, exporter-library, NanoDA and
adapter digests, since independent builds need not have identical executable
bytes. Linux cross-compilation passed after the editor correction; this is
not a claim of new native Linux execution. No remote CI ran for this change.

RFC acceptance and independent human/publication review are not claimed by
these automated checks. As in the completed R1/R2 milestones, they remain
separate from the local implementation and explicit executable exit gate.
No remote CI, push or pull request was requested or performed for this work.

# R3 completion audit

Objective: finish R3. This audit follows the requirements in
[Plan.md](../../Plan.md#r3--support-existing-lean-projects) and the
[practical language plan](../practical-language-plan.md#r3--make-existing-lean-projects-a-supported-workflow).
It does not replace those requirements with the existing implementation's
smaller bounds. The starting local checkpoint is
`56c626209e0fd75c37b57e5cb03c931f76f7177a`.

Status: implementation and full reproduction complete; the final
Mathlib-inclusive frozen set and its additional source-job check are running.
The completed focused evidence below does not substitute for those remaining
checks. All work remains local.

| Requirement | Implementation and executed evidence | Final audit state |
|---|---|---|
| Accept a pinned Lean project and dependency environment | Native Lake lock resolution, clean pinned Git provenance and byte-exact input capture; vendored example and a 10-package Mathlib environment built successfully | Proven in focused runs; final corpus running |
| Bind declaration, assumptions, definitions and formal target identity | Lean target reconstruction and equality checks; explicit statement, definition and policy revisions; old candidate/result pins rejected | Focused cases passed |
| Keep statements, candidates and results distinct | Task v3, separate closed-term/structured-automation candidates and result v5; candidate file converter and revision lineage; no-op revision rejected | Focused cases passed |
| Reuse Lean diagnostics, retrieval and compatible interfaces | Earlier CLI/REPL/LeanInteract comparison; real native Lake LSP initialization, hover, goal query, diagnostics, edit repair and shutdown | Native editor exercise passed |
| Reuse ordinary proof automation | Native exact, simp, omega and grind proofs; explicit policy controls; all seven local development cases freshly rechecked | Focused cases passed; final corpus running |
| Reconstruct ordinary Lean files in a clean pinned environment | New Lake package trees from captured inputs and native path overrides; original configs retained; compiled caches excluded; source identity checked before and after builds | Vendored and Mathlib reconstructions passed |
| Enforce transitive axioms and independently recheck every reported proof | All proving routes use target comparison, policy checks, export validation and NanoDA; focused Mathlib, seven development proofs and four source-job proofs freshly rechecked | Full reproduction passed; final Mathlib set running |
| Separate draft plans from actual dependencies | Exact graph rederived from the exported proof; draft editor output is context only; wrong terms and forbidden axioms cannot become completed proofs through scheduling | Positive and negative cases passed |
| Export readable ordinary artifacts for exact declarations | Lean modules, additive patches, definitions, graph reports and explanations; normal pinned editor workspace with captured path dependencies | Generated artifacts and native LSP exercised |
| Integrate asynchronous source and project workflows | Typed alias/proof dispatch; poll/cancel/collect; repair and parallel jobs; project lock/test/replay/resume; actual driver interruption, unknown-state reconciliation and preserved dispatch identity | Complete 36-case focused runner passed |
| Complete the frozen R3 proof set | Native development and source-job scripts with explicit acceptance/failure/unknown outcomes; each accepted case has an independently generated fresh record | Full reproduction passed; final 8-case native set running |
| Supply contracts and verification evidence | RFC 0036, current user guide, trust inventory, runtime/workflow tests, workspace Clippy and Linux cross-compilation | Full reproduction passed; RFC disposition and independent human/publication review remain separate |

Advanced coupled subgoal search is conditional in the plan and has no
demonstrated requirement yet. The proposed 30-task comparative corpus, paired
model trials and independent audience pilots belong to R5. They are not
substitutes for the integrated R3 correctness and replay gate.

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
- Final native/Mathlib corpus: running, bundle `target/run-_i6exng7`.
- Combined Mathlib source job and fresh recheck: running, bundle
  `target/r3-mathlib-job-dzvuqcgl`.
- Local diff review passed; final Mathlib evidence readback and milestone
  status update remain pending.

RFC acceptance and independent human/publication review are not claimed by
these automated checks. As in the completed R1/R2 milestones, they remain
separate from the local implementation and explicit executable exit gate.
No remote CI, push or pull request was requested or performed for this work.

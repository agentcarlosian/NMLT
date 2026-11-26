# R4/R5 continuation verification record

This record concerns the broader continuation on `codex/r4-discovery-slice`,
based at `7d0496d5f887fc4cd9672dbc4bbf63bc4c749f80`. The earlier six-file
[wiring slice report](r4-composition-premise.md) does not validate these additions.

The latest production gate also failed: **19 Make targets passed and 7 failed**
in 55m12s. The [latest verification and isolation](#latest-production-verification-and-async-isolation)
below supersedes earlier counts as the current aggregate result. No complete
passing gate or production lifecycle fix is claimed.

The R4 workflows and R5 engineering checks below passed on 2026-10-01.
The initial full workflow returned exit 2 because an existing R3 negative control hit an
unclassified process failure reported as `Io` before its intended axiom check.
That exact control passed on a fresh rerun with unchanged inputs and policy.
Five of the six remaining R3 scripts passed; the dependency script encountered
another unclassified `Io` process failure after compilation. Its failed control
and four unreached controls then passed separately. Every specified R3 case has
successful observed coverage, but neither failed aggregate command is designated
green by this record. A subsequent ordered diagnostic replay passed those two
controls, then failed
during a Lake binding. Its trace identifies post-exit descendant classification,
with a nonsignaled process object still in the exact job before cleanup. It does
not explain the original two failures or establish a supervisor defect. No
production fix or successful complete gate is claimed.

## Integrated attempt and completed evidence

On `DESKTOP-3SHE0AI`, `make fmt` returned 0 and `make -k reproduce` ran from
`2026-10-01T06:04:21Z` to `2026-10-01T06:53:58Z` (49 minutes 37 seconds).
`-k` preserved failures while allowing independent Make targets to run; it did
not convert failed checks into success. The sole failed Make target was
`r3-lean-projects`, described below.

The environment used Rust 1.94.0, Lean 4.33.1, Python 3.13.12, GNU Make 4.4.1,
and MSYS Bash on Windows. Rust dev/test optimization was 1, debug information
was disabled, and `RUST_TEST_THREADS=1` limited test-harness concurrency. Each
test's internal concurrency and production process policy remained intact.
The invocation supplied the direct pinned Lean executable through `R2_LEAN_BIN`
and `PYTHON=python`; full commands and tool versions are in the retained logs.

| Gate | Observed result |
|---|---|
| Rust formatting, workspace check, Clippy | Passed; warnings denied by Clippy. |
| Rust tests | 387 passed, 0 failed, across 49 test binaries. |
| Python harness tests | 14 baseline tests and 25 experiment tests passed. |
| Public surface and behavior artifact | Passed. |
| Lean metatheory | Build, explicit axiom probes, decoder controls, and no-sorry policy passed. The allowlist is unchanged. |
| Independent NanoDA | 9,226 declarations checked with no errors, from 2,694 enumerated NMLT roots; file exporter matched the pinned CLI on eight roots. |
| Frozen finite/execution checks | Value parity and v2 paths passed; 30 execution controls rejected; execution parity covered 1 initial state, 8 reachable states, and 12 transitions. |
| R0 workflows | All 3 proof, 2 discovery, and 4 worker tasks completed. |
| R2 integration | Worker, asynchronous/source jobs, project, invariant, and Lean-term gates passed; 15 invariant controls rejected. |
| R3 native projects | 19 accepted proofs, 16 fresh rechecks, and 72 required negative controls covered across the original attempts and explicit follow-ups. Two aggregate commands remain nonzero; see the per-script ledger below. |
| R4 discovery | All 16 classification proofs and 2 invalid controls passed; all 18 Python/Lean graph rows agreed. |
| R5 corpus preflight | 30 references elaborated; 30 type-invalid terms rejected; 12 paths accepted, freshly rechecked, and rejected in their specified invalid variants. |
| R5 recorded comparison | 2 reversed-order pairs, 4 accepted batch proofs, 4 fresh native rechecks, and 4 invalid-candidate rejections passed. |

The independent NMLT export is 51,704,177 bytes and has SHA-256
`f0f7235d4217989fde6ba3ed43401ce4bdcb9f5f7407451c68dbf1893132c36e`.
Its roots include all 16 composition cases, `exhaustiveObjects`,
`Evaluation.Discovery.largerObjects`, and `Evaluation.Corpus.bundle`.
Exporter revision: `411dce7db58a3afc60ecab2d211acd1042b593dc`.
NanoDA revision: `05055695879dfebb6628a67da88ceca6cd6b0421`.

All four R5 arms produced the same independently checked export hash:
`a7c90c50ca39d544f9f18d5435aeae04657385abb26326087a934304ea9c3ee5`.
Each batch's checked dependency closure includes all 30 reference solutions.
This is four batch-proof executions, not 120 independent operational trials.
The runner checked exact task/candidate identity, ordered source requests,
receipt bytes, tool/source hashes, specific rejection diagnostics, fresh native
results, and final input stability before writing its successful summary.

The R4 worker search checked all 28,224 policy/case combinations (64 by 441).
Only policies 0 and 1 satisfied every fixed outcome. Their executed bookkeeping
counts were 1,398 and 699. At R5's larger bound, both passed 9,801 cases each,
with counts 52,272 and 26,136. These are model operations, not measured host
speedups. R4's 14 accepted graph rows represent 7 labeled matrices and 4 cycle
isomorphism classes; R5 checked all 15 additional candidates at sizes 8 and 9.
None of these results establishes general liveness or mathematical novelty.

The canonical R5 manifest pin is
`3311ce3d913574bec62b94a34d9685e4b698cc34b770e668772225b3192f0597`;
the manifest file-byte hash is
`b3c3522320f4cfab468aaaf63fb6e467a0a172257a9c096f4dcbc0981849e330`.
The corpus and solutions are developer-exposed. Actual independent participants,
live model runs, and operational participant task trials are all zero. The
empty observation template reports no estimate and no met improvement target.

## R3 follow-up ledger

The six-script continuation ran from `2026-10-01T06:57:26Z` to
`2026-10-01T08:31:21Z` and returned 1. It continued independent scripts after
failure without changing their assertions, process policies, or timeouts.
It reused the same pinned R3 checker binaries as the failed integrated target.

| Script | Start UTC | Finish UTC | Exit | Accepted / fresh / required negatives covered |
|---|---|---|---:|---|
| `check_lean_imports.py` | 06:57:27 | 07:05:24 | 0 | 1 / 1 / 14 |
| `check_lean_exports.py` | 07:05:25 | 07:11:11 | 0 | 1 / 1 / 11 |
| `check_lean_dependencies.py` | 07:11:12 | 07:20:48 | 1 | 1 / 1 / 8 in this attempt; 5 more controls passed in the follow-up below. |
| `check_lean_inspection.py` | 07:20:48 | 07:32:41 | 0 | 1 / 1 / 8 |
| `check_lean_lake.py` | 07:32:42 | 08:11:07 | 0 | 7 / 7 / 9; includes native LSP checks. |
| `check_lean_project_jobs.py` | 08:11:07 | 08:31:20 | 0 | 4 / 4 / 8; all 36 commands completed. |

The initial `check_lean_tasks.py` had already produced four accepted proofs,
one fresh recheck, and eight successful negative controls before its last
control encountered I/O failure. That last control passed on the unchanged
rerun described below. The dependency continuation used exactly the retained
`changed-export-group` record, then the same three record mutations and draft
candidate that the original script had not reached. All five passed from
`08:32:15.659388Z` to `08:33:51.921908Z`; tool and retained-input hashes were
unchanged. Both accepted-result and dependency-graph output had to be absent,
and the expected diagnostic had to be present. No I/O failure or timeout was
counted as a semantic rejection. These combined counts are case coverage, not
a claim that either original aggregate invocation exited successfully.

The four smaller native scripts allow 600 seconds per command; Lake and
project-job scripts allow 2,400 seconds. Each native process additionally
retains its existing profile-specific bounds. Status logs, process observations,
and generated-file timestamps showed forward progress throughout the long
rechecks. No duplicate complete suite was started while one was active.

## Retained local evidence

These ignored paths are relative to the checkout; they are local run records,
not published proof receipts or committed benchmark data.

- `target/r4-r5-complete-environment.txt`, `r4-r5-complete-format.log`, and
  `r4-r5-complete-reproduce.log`: complete invocation and output.
- `target/r4-r5-complete-tree.json`: SHA-256 snapshot of 479 repository files
  before that run; subsequent verification-document edits are recorded separately.
- `target/r4-r5-complete-nanoda/run.GGvxLG/`: exact independent checker inputs
  and provenance, including exporter parity files.
- `target/discovery-g_rj9y2q/`: finite search, source/tool identities, parity,
  and all 34 proof-candidate observations.
- `target/evaluation-qfgdb2g0/`: corpus preflight, endpoint/accounting checks,
  12 path rechecks, specified invalid variants, and larger-bound results.
- `target/evaluation-replay-kgyi3yl1/`: all four native proof records and fresh
  records, ordered candidates, replay/recovery output, and successful summary.
- `target/r4-r5-current-pilot-summary.json`: zero-participant template output.
- `target/r4-r5-r3-remaining/`: per-script logs and statuses for the remaining
  R3 checks after the integrated run's abort.
- `target/r4-r5-dependency-controls-recheck/`: the five targeted controls,
  exact commands/diagnostics, durations, and before/after input identities.
- `target/r4-r5-process-diagnostic/`: isolated diagnostic source, copied
  failed compilation inputs, instrumentation provenance, and diagnostic logs.
  Diagnostic output is not production acceptance evidence.
- `target/r4-r5-r3-replay/`: ordered diagnostic replay, per-script statuses,
  pre-cleanup native observations, immutable instrumentation diff, analysis,
  and final tree/tool/evidence audit. Its isolated source and generated fixtures
  are under `%TEMP%/codex-nmlt-r3-native-replay`.
- `target/r4-r5-lake-lifecycle/`: bounded changed-context reproduction, job-scoped
  image/parent metadata, independent integrity audit, and filtered historical
  event-query outcomes. Its source is `%TEMP%/codex-nmlt-lake-lifecycle`.

## Changed-file inventory

There are 36 changed files relative to the base commit: nine tracked-file edits
and 27 additions, including the preserved first wiring slice. No production
Rust source changed.

- Build and policy integration: `Makefile`, `mechanization/lean/NMLT.lean`,
  `mechanization/lean/AXIOMS.md`, and `tools/check_metatheory.sh`.
- Documentation: `Plan.md`, `docs/README.md`, `examples/README.md`,
  `tests/README.md`, `docs/r4-composition-premise.md`,
  `docs/r4-discovery-workflows.md`, `docs/r5-evaluation-protocol.md`, and this
  `docs/r4-r5-verification.md` record.
- Formal examples: `mechanization/lean/NMLT/Counterexamples/WiringPremise.lean`,
  `mechanization/lean/NMLT/Discovery/CompositionPremises.lean`,
  `mechanization/lean/NMLT/Discovery/FiniteConstructions.lean`,
  `mechanization/lean/NMLT/Evaluation/Corpus.lean`,
  `mechanization/lean/NMLT/Evaluation/Discovery.lean`,
  `mechanization/lean/tests/DiscoveryParity.lean`, and
  `mechanization/lean/tests/EvaluationParity.lean`.
- Frozen examples and templates: `examples/discovery/contract.json`,
  `examples/evaluation/dispatch.nmlt`, `examples/evaluation/manifest.json`,
  `examples/evaluation/manifest.sha256`,
  `examples/evaluation/pilot-observations.csv`, and
  `examples/evaluation/replay.nmlt`.
- Experiment tools: `tools/check_evaluation_replay.sh`,
  `tools/experiments/check_discovery.py`,
  `tools/experiments/check_evaluation.py`, `tools/experiments/laboratory.py`,
  `tools/experiments/materialize_task.py`,
  `tools/experiments/replay_evaluation.py`, and
  `tools/experiments/summarize_pilot.py`.
- Regression tests: `tests/experiments/test_evaluation.py`,
  `tests/experiments/test_laboratory.py`, `tests/experiments/test_pilot.py`, and
  the existing `crates/nmlt-runtime/tests/process.rs` fixture correction below.

## Corrections and failed attempts

The first integrated attempt stopped on two existing Windows process tests at
their child-startup precondition. Serial execution resolved one failure, but
the file-capture case still failed with a 200 ms deadline. Its test-only deadline
now matches the companion test at 400 ms, below the descendant's 800 ms survival
marker. Readiness and absence-of-survivor assertions remain; timeout/cancellation
branches now additionally require the correct failure kind and `child_reaped`.
All 17 process tests passed separately and in the 387-test integrated run.

A second attempt hit `Io` with `child_reaped: true` in an existing project test.
That unchanged seven-test suite passed separately and in the subsequent full
attempt. Logs remain under `target/r4-r5-project-host-recheck*` and the earlier
`target/r4-r5-final-*` / `target/r4-r5-validated-*` prefixes.

The complete attempt's R3 failure was in
`target/r3-lean-tasks/run-fixq6unc/forbidden-axiom`. Lean had written
`Example/Definitions.olean`; the process failure preceded candidate compilation
and axiom checking. Ordinary-profile diagnostics suppress the underlying detail,
so retained evidence cannot distinguish detached-process classification from
an underlying ProcessKit output/wait error. CPU load does not establish causation.
The harness correctly refused to count this host failure as an axiom rejection.
The exact task, candidate, and checker binaries then produced the expected
`proof uses an axiom outside the selected transitive policy` rejection in
30.96 seconds, without writing an accepted result. See
`target/r4-r5-forbidden-axiom-recheck.json` and its stdout/stderr siblings.
No production timeout or containment rule was changed.

The continuation hit the same public error classification while rebuilding
`Graph.Goals` for `changed-export-group` in
`target/r3-lean-dependencies/run-dsztctmt`. Its `Graph/Goals.olean` had also
been written before the failure. The unchanged targeted control subsequently
returned `fresh proof artifacts differ from the retained acceptance record`
in 60.87 seconds. Passing this retry establishes the intended negative control;
it does not establish why the original execution failed.

Independent process-code review identified two still-plausible paths: sticky
detached-process classification from job membership observations (including
query errors during post-exit settling), and an underlying ProcessKit
output/wait error. ProcessKit documents that Windows job snapshots can briefly
omit or retain processes during creation/reaping. The retained ordinary-profile
failures do not distinguish these explanations. An isolated diagnostic copy
records existing membership observations and branch/error details without
adding queries, sleeps, retries, or policy changes. Its logging can perturb
timing, and its results cannot replace production tests or proof acceptance.

The isolated diagnostic built offline successfully in 224.72 seconds. It then
compiled copies of the two failing modules 88 times (44 each) from
`2026-10-01T08:38:04.071916Z` to `2026-10-01T08:43:05.940797Z`, returning 0.
All 88 module compilations exited successfully; no I/O failure or diagnostic
branch was reproduced. This is module compilation, not 88 accepted proofs.
The probe preserved the ordinary 30-second process bound, allowed at most
100 launches within a 300-second launch budget, and had a 360-second outer
limit. It stopped after that launch budget; actual probe duration was 301.87
seconds including its last in-flight compilation.

The diagnostic executable hash is
`918a444e9ebfadddb0b4835e688980bb2f75ce251bb60c03ccff34da81815f8b`.
Its copied-source hashes, exact instrumentation diff, Cargo lock hash, per-case
observations, and build/run statuses are retained in the diagnostic directory.
Review confirmed compiler-argument/environment fidelity for these Init-only
modules and preserved query counts, sleeps, thresholds, and classifications.
The reduced harness omits the surrounding CLI workload, and observation
formatting perturbs timing. Non-reproduction does not establish transient host
noise or rule out a supervision/output-collection defect.

The missing evidence is a failing full-CLI execution with the underlying
ProcessKit error or job-membership observations retained at the decision point.
That would distinguish `process.rs`'s detached/settling paths from its raw
output/wait error mapping. Existing normalized errors cannot recover that
information after the fact. No deadline, containment, rejection rule, or axiom
policy has been relaxed to make the aggregate gate pass.

### Full-CLI diagnostic follow-up

The subsequent diagnostic uses the complete CLI, with isolated copies of the
runtime and pinned ProcessKit under `target/r4-r5-full-cli-diagnostic/source`.
Production sources, the original CLI, and the failed-run evidence remain intact.
The instrumentation retains bounded membership histories at output completion,
at a detached-process decision, and before cleanup. It records native wait and
pipe-drain outcomes, byte counts, error kinds, OS error codes, and teardown
metadata. It excludes environment values, arguments, and pipe contents from
the added diagnostics. Independent review caught payload-bearing error formatting
before execution; that formatting was replaced with selected metadata. The
instrumentation preserves queries, thresholds, deadlines, classifications, and
cleanup actions, but its allocations and logging can perturb timing.

The diagnostic executable is
`9e857a4332b8ab783ff606ca211ebfb01176136bf6493606c8006948f8dd474d`.
Tasks pin their CLI identity, so the diagnostic uses genuine new `bind` commands;
it does not override identity checks or reuse the original task pins. Every
rebound task field must equal the original except `implementation_sha256`,
including import-discovery configuration, sources, target, Lean identity, and
axiom policy. The dependency baseline is proved under this executable and its
proof, dependency graph, exporter output, checker identities, and process policy
are compared with the retained original before making exactly the original
quotient-group deletion. The forbidden-axiom candidate retains its proof text.
These records are diagnostic evidence, not a replacement aggregate gate.

Each negative attempt requires native exit 1, the specific semantic rejection,
absence of both acceptance artifacts, and complete supervision diagnostics with
no runtime anomaly. The harness stops at its first unexpected result, with no
retry. Its predeclared bound is 12 pairs or 1,200 seconds of repeated-control
launches after setup; each already-launched command retains the original
600-second outer timeout. Final counts, timestamps, binary/input hashes, and
source-tree comparison are retained in `run-summary.json`, `trace-analysis.json`,
and `final-audit.json` in the diagnostic directory.

The final harness exited 0 after **19 expected negative outcomes**: 10
forbidden-axiom attempts and 9 changed-export-group rechecks. This is nine
complete pairs plus an axiom half-pair; the last dependency half was not launched
after the budget expired. Repetition ran from `2026-10-01T09:13:55.795029Z` to
`2026-10-01T09:34:14.305483Z`, **20m18.51s**, including the last in-flight command.
Together with the two bindings and diagnostic baseline, the logs contain
**128 supervised completions**, zero runtime anomalies, and zero membership-query
errors. Eleven completion sequences briefly listed a non-root PID after root
exit, then settled successfully; those observations do not establish the cause
of the earlier failures. Original executable, checker and copied-input hashes
were unchanged. The 479-file tree comparison identifies only this verification
document as changed during the diagnostic work.

Scoped process-name observations identify `conhost.exe` alongside the Lean and
checker processes. A name or PID alone does not establish whether a process is
still running at rejection. In particular, [Windows distinguishes process exit
from release of all handles](https://learn.microsoft.com/en-us/windows/win32/procthread/terminating-a-process).
A separate controlled probe therefore held a child handle acquired before its
exit handshake, confirmed the process object signaled and exit code 0, and kept
the handle for 0, 250, or 500 ms. All three final cases returned runtime success;
the two nonzero-retention cases completed successfully while the handle was
still held. This mechanism did not reproduce the failure. Its exact production
runtime copies and observations are in `target/r4-r5-handle-retention/`.

Two diagnostic-harness corrections are kept distinct from runtime failures.
Initial setup stopped before any negative attempt because it assumed an explicit
module manifest instead of the dependency task's import-discovery configuration;
the corrected setup reused only the successful, equivalence-checked axiom bind.
An empty assertion message also exposed a helper exit-status bug, corrected by
testing exception presence. `setup-v1-summary.json` preserves that attempt.
The separate handle probe initially observed a partially published PID file;
atomic rename fixed the fixture, and `probe-v1.*` preserves the failed attempt.
No production acceptance rule or supervisor implementation was changed.

Independent review found no demonstrated supervision defect and agreed with
stopping at the declared budget. It recommended one instrumented replay
preserving the surrounding original R3 workload, with typed failure branches
and native process-object status before cleanup. That bounded follow-up is
recorded next; the earlier evidence and nonzero aggregate results remain intact.

### Surrounding R3 workload replay

One diagnostic replay ran from `2026-10-01T10:07:17.845363Z` to
`2026-10-01T11:17:36.484079Z`, **70m18.64s**, and returned **1**. It used the
original seven Python scripts, byte-identical and in their original order,
with the original pinned Lean, exporter, and NanoDA binaries. It stopped at
the first failing script without retry; other Make targets were not repeated.
The first five scripts retained their 600-second per-command limits; Lake and
project-job commands retained 2,400 seconds. These are command limits, not
whole-script deadlines. The failing Lake process had its existing 1,800-second
native project limit; this failure was not a timeout.

| Script | Start UTC | Finish UTC | Seconds | Result |
|---|---|---|---:|---|
| `check_lean_tasks.py` | 10:07:17.880 | 10:23:02.933 | 945.03 | Passed, exit 0 |
| `check_lean_imports.py` | 10:23:02.980 | 10:32:01.387 | 538.38 | Passed, exit 0 |
| `check_lean_exports.py` | 10:32:01.428 | 10:37:32.175 | 330.72 | Passed, exit 0 |
| `check_lean_dependencies.py` | 10:37:32.221 | 10:49:16.469 | 704.22 | Passed, exit 0 |
| `check_lean_inspection.py` | 10:49:16.515 | 11:00:48.584 | 692.04 | Passed, exit 0 |
| `check_lean_lake.py` | 11:00:48.631 | 11:17:36.445 | 1007.79 | Failed, exit 1, during `changed-context` binding |
| `check_lean_project_jobs.py` | — | — | — | Not run after first failure |

The five completed scripts covered eight accepted proofs, five fresh rechecks,
and 55 required negatives. Both earlier failing controls (`forbidden-axiom`
and `changed-export-group`) passed in this ordered replay. Lake completed its
direct/native/editor, LSP, and revised-proof stages, then failed the
**expected-success** `bind("changed-context", previous=revised)` in
`tools/check_lean_lake.py`. Its following context-sensitive rejection was not
reached. The failed fixture is `target/run-eexpihw0/changed-context` in the
isolated source. `build/NMLTTask.olean` exists, but the binding did not produce
`task.json`, `result.json`, or `proof-dependencies.json`.

The diagnostic build adds metadata logging in an isolated runtime and vendored
ProcessKit copy; it changes no production source, threshold, deadline, or
acceptance policy. Native probes use one read-only handle per observed process
for creation identity, zero-time wait, exact-job membership, and exit code.
Unknown or inaccessible status remains unknown. Sequence numbers and loss
markers permit checking observed logs; no arguments, environment values, or
pipe payloads are added. The additional queries, allocation, and logging can
change timing. A final failed log write cannot report its own loss.

There were 380 supervised completions and one anomaly, with no observed
sequence gaps, loss markers, incomplete lines, or typed raw-error branches.
The decisive records are sequences 63–71 in `traces/17772.log`, correlated with
the Lake stderr and fixture:

- Root PID 31144 exited 0 and pipe collection completed without error. Output
  was ready at diagnostic elapsed time 5,441 ms. No overflow, truncation,
  cancellation, or deadline expiry was recorded.
- The existing settle queries continued to list PID 33188 through 5,543 ms.
  Before the first supervisor kill (`prior_kill_count: 0`), its creation
  identity was `134353270500788631`, exact-job membership was true, and
  `WaitForSingleObject` returned **258**, a nonsignaled observation. Native
  queries reported no API errors. The root object was already signaled.
- The residual process's exit code was queried later and returned 0. These
  sequential observations can straddle completion; the later exit-code result
  neither overrides the nonsignaled wait nor proves the state at the eventual
  kill. See [Windows wait semantics](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject).
- Cleanup succeeded and the job was empty at 5,612 ms. The runtime returned
  `Io`, `child_reaped: true`, with detail
  `descendants remained after the root process exited`.

Independent review agrees this identifies the replay's post-exit settle path
and is consistent with the existing fail-closed contract. It does not show a
stale PID referring to an already-signaled object. The residual process's image,
parent, and thread state were not captured; previous observations of named
helpers cannot identify this process. The original ordinary-profile failures
remain unattributed. Their public error class alone cannot link them to this
project-profile Lake failure.

Instrumentation review removed payload-bearing error formatting and added
the prior-kill count before execution. Initial diagnostic-only build/probe
attempts failed on a missing internal forwarding method and a test FFI conflict
with `forbid(unsafe_code)`; the forwarding method and safe retained-child-handle
fixture corrected them without weakening that lint. The final diagnostic build
passed, and its native-status probe passed one test with one fixture ignored.
All failed and successful preflight logs remain in the replay directory.
Diagnostic CLI SHA-256 is
`8c8e7e0be6d068aae184937203faf6df230c6b5f45cda832ac3a2d45eb115255`;
instrumentation patch SHA-256 is
`fd6b5bbbd3ba9822359e7278f4ff837ad75ca89d8648490d4f5af15ea20edbd6`.
The terminal analysis found all 479 root source files and the original CLI and
checker binaries unchanged. This later documentation update is recorded by the
final audit separately from that terminal snapshot.

No production fix follows from this evidence. Accepting nonsignaled helpers,
using exit code alone, or extending the settle policy would change the lifecycle
contract and require explicit RFC review under `CONTRIBUTING.md`; RFC 0028 is
still under review. A useful next diagnostic input would identify the residual
process and its OS lifecycle at an actual failing decision, or compare the
unchanged workload on another Windows environment. Another unchanged full
replay alone would not answer those questions. The complete verification gate
remains open; no further aggregate run was launched.

Final `cargo fmt --all --check`, public-surface validation, and `git diff --check`
all returned 0. The final audit retained hashes for 118 evidence files and found
only this verification document changed among the 479 source files since the
full attempt and replay snapshots. Original executable/tool hashes, the
instrumentation patch, and the earlier diagnostic evidence hashes were unchanged.
Independent review checked the final record; its wording correction distinguishes
unclassified `Io` reports from demonstrated raw I/O errors. The post-review
document hash and unchanged-evidence check are in `final-review-audit.json`.

### Targeted Lake lifecycle investigation

The failed positive binding invokes `lake lean` through
`crates/nmlt-cli/src/lean_task/lake.rs:589`. Pinned Lake's
`Workspace.evalLeanFile` prepares the Lean command, spawns it, and waits for
that direct child. Its Windows runtime implements the wait with
`WaitForSingleObject(INFINITE)`. The child launcher uses creation flags 0;
NMLT's outer process launch already supplies `CREATE_NO_WINDOW` through
ProcessKit. No missing direct-child wait was found. Replacing `lake lean` with
a bare Lean command would omit Lake's dependency preparation and `--setup`
configuration, so it is not a demonstrated equivalent fix.
See the [pinned runtime source](https://raw.githubusercontent.com/leanprover/lean4/v4.33.1/src/runtime/process.cpp)
and [Windows creation flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags).

A separately built diagnostic records image basename, creation time, parent
PID, and whether the image's parent directory matches `GetSystemDirectoryW`.
It queries only the root and PIDs returned by that exact job, using the same
read-only handle as the signal/exit queries; it does not enumerate other
processes, open parent processes, read process memory, or emit arguments,
environment values, full image paths, or pipe contents. First-seen observations
are bounded to 64 PID numbers per command, including the root. Parent PIDs can
be recycled, short-lived processes may be missed, and directory matching is not
executable authentication. The metadata never controls supervision outcomes.

One bounded probe ran from `2026-10-01T11:58:02.147248Z` to
`2026-10-01T12:04:09.050208Z` (366.89 seconds), returning **0**. It created a genuine
new baseline binding, restored all 11 uncached project files to the failed
fixture's exact input hashes, and successfully revised that baseline four times.
This reproduces the changed project with a fresh parent identity; it does not
replay the full earlier revision chain. The predeclared maximum was six
revisions or 240 seconds of launches after baseline setup, stopping at the first
unexpected result. The fourth revision was already running when that budget
expired; two further attempts were not launched. Each command retained a
600-second outer diagnostic limit and the unchanged 1,800-second native project
limit. No failed binding was retried.

The trace contains **50 completed supervisions, zero anomalies**, and no observed
sequence gaps or loss markers. All 50 stages observed a `conhost.exe` image in
the system directory, with its reported parent equal to the observed root.
These helpers were created 39.69–87.91 ms after their respective roots; Lake's
Lean child was a separately observed process. No residual member remained at
any post-output settle sample in this probe. This identifies the console-helper
path prospectively, not the image of historical PID 33188. Its 54 ms creation
offset is consistent with this pattern but is not an identity proof.

Lake waits for the Lean child handle it created, while Windows controls console
lifetime as clients detach or terminate. That separation is a plausible reason
for a console host to finish after a tool root; neither this probe nor the
documentation establishes that this caused the historical failure or that a
particular termination delay is required. [Windows console lifetime](https://learn.microsoft.com/en-us/windows/console/freeconsole).
The earlier LSP fixture is not a demonstrated source of the survivor: ProcessKit
creates a fresh unnamed job for each stage, and the historical residual was born
after that stage's root.

The metadata preflight passed one test with one fixture ignored. It verifies
the image and parent of a known live child, stable creation/signal/exit-code
observations, exact-job membership, and explicit unknowns. An initial assertion
incorrectly assumed the image remains queryable after exit; Windows returned
an error, and the revised test requires the recorded error rather than inventing
an identity. Independent review also caught unrelated UTF-8 decoding damage in
the diagnostic copy. The original source prefix was restored and mechanically
checked before the targeted run. A diagnostic-only type-inference build error
was corrected; all initial build/probe records remain retained.

The executed harness is retained as `runner-executed.py`. Independent review
identified final integrity flags that were recorded without gating its exit,
and revision counts recorded before their follow-up assertions. The completed
run's independent `analysis.json` enforces those conditions: all source/tool/
prior-evidence hashes match; the CLI matches both successful build and probe;
all four revision pins, parent links, and invalidation records pass. The helper
was corrected for future use and was not rerun. The sibling `lake.exe` fingerprint
was first sampled during the baseline command and matched after the run; this
does not constitute a pre-launch fingerprint. The diagnostic CLI is
`dfbab7364f4c0e2d39c9b413145fb271dcceaf48f5736d2276956fe3a498bb29`;
its instrumentation patch is
`40be07fbaeb49ea8f7ec415cf40742f51ca436489a2b9f3f5db63b65fd79ec2b`.

Historical process-creation queries were restricted to PID 33188 and the
four-second interval around `2026-10-01T11:17:30.0788631Z`. Security event 4688
access returned `UnauthorizedAccessException` both inside and outside the
sandbox; the Sysmon operational log was absent. No audit configuration was
changed. The specific missing external evidence is an administrator-supplied
process image/parent/timestamp record for that PID and interval, if one was
recorded. If none exists, the historical identity cannot be reconstructed from
these logs; first-seen metadata must instead accompany a future actual failure.

This remains an unresolved Windows/tool lifecycle interaction under the current
strict policy, not an established fixture or launcher bug. No production fix,
process-limit change, helper exception, RFC change, or further aggregate replay
was made. Accepting a nonsignaled helper by name, extending the settle period,
or changing console ownership would require a separate justified contract and
compatibility review. The full verification gate and R5 external gates remain
open; successful diagnostic bindings do not establish proof acceptance or user
usefulness.

### Earlier implementation corrections and remaining gates

Earlier focused preflights exposed a reversed return-handoff label, CLI flag
ordering, and Windows lookup of the matching Lake binary. These were corrected;
the manifest was explicitly re-frozen before the comparison. An earlier native
replay attempt stopped on the existing five-second Lean version-probe timeout;
that saved proof later rechecked successfully, but the incomplete comparison
was never counted as a completed four-arm result. Its evidence remains under
`target/evaluation-replay-first` and `target/evaluation-timeout-recheck*`.

Independent read-only review examined the complete premise classification,
countermodel enabledness, fixed conclusion, graph predicate, experiment
contracts, corpus freezing, and native replay. Findings led to executed
bookkeeping traces, separate graph row/matrix/isomorphism counts, complete
source/inventory binding, diagnostic-specific rejection classification,
endpoint/accounting expectations, and exact candidate/order checks.
It also caught a pilot-reporting mismatch: a single pair per audience could
satisfy the descriptive target despite the protocol requiring reversed-order
repetitions. Observations now record complementary arm positions, and every
participant/task requires at least two pairs covering both orders. Incomplete
groups keep descriptive statistics with explicit reasons and cannot meet the
target. Four regression tests cover this correction; independent review closed
the finding. Synthetic unit-test data is not participant evidence.

The implementation changes no production parser, type system, behavioral
definition, runtime lifecycle, proof-acceptance rule, or axiom allowlist.
The only new evidence schemas are non-authoritative experimental inventories.
Under `CONTRIBUTING.md`, no new language or trust-boundary RFC is required.

Open external gates: mathematical usefulness assessment, independent users,
paired live trials and their reviewed evidence, and cross-family review before
publication. No push, PR, merge, deployment, or participant contact is included.

## Latest production verification and async isolation

The approved production run used the unchanged 479-file source snapshot and
unchanged CLI. Its preflight verified the required GitHub access and supplied
MSYS `TMPDIR=/c/Users/Carlos/AppData/Local/Temp`, inside the authorized writable
root. No host security settings changed. `make fmt` returned 0 without changing
sources. `make --trace -k reproduce` ran once from
`2026-10-01T21:56:29.704743Z` to `22:51:41.279541Z`, **3311.57 seconds**, and
returned **2**. No duplicate aggregate was active. There was no aggregate outer
timeout; existing per-command and native process limits remained unchanged.

The exact target ledger is retained in
`target/r4-r5-approved-verification-20261001T215605Z-9833db86/results.json`,
with the complete `reproduce.log`, source/tool hashes, and `final-audit.json`.
All 26 Make targets were invoked: **19 passed, 7 failed**.

- Formatting, workspace check, Clippy, public/artifact checks, all 39 Python
  tests, Lean metatheory, NanoDA (9,226 declarations / 2,694 roots), finite and
  execution checks, R0 workflows, and the R4/R5 targets passed.
- Rust `test` returned 101: **37 tests passed and 3 failed** across the three
  binaries reached. Cargo stopped in `nmlt-cli`'s `async_jobs`; remaining
  workspace test binaries were not run in this attempt.
- `r2-async`, `r2-source-async`, `r2-project`, `r2-source-lean`, and
  `r2-project-lean` each returned 1 on host execution failures.
- `r3-lean-projects` returned 1. Tasks, imports, exports, dependencies, and
  inspection passed. Lake stopped at `native-grind` with an unclassified
  `Io`, `child_reaped: true` from the Lean version probe. The remaining Lake
  rechecks/retained-source control and project-job script were not reached.
  No accepted result was published for that failed command.

R4 again checked all 16 premise classifications and all 18 graph rows. R5
again checked 30 reference terms and their invalid controls, 12 finite paths,
and the larger bounds; recorded replay produced four checked batch proofs,
four fresh native rechecks, and four specified invalid-candidate rejections.
These engineering results retain the external-evaluation limits above.

The three failed Rust tests were then run individually, once each, against
the unchanged production executable. `concurrent_dispatch_fallback_reuse_and_replay_without_live_state`
passed (2.68s); `source_components_transfer_real_handles_and_replay_without_new_attempts`
failed (2.19s); `source_operations_results_boundaries_and_snapshot_tampering_fail_replay`
failed (2.13s). Both failures occurred after two workers started, at the first
collection, before the replay/tampering assertions, with `Io` and confirmed
cleanup. These filtered runs used native Cargo; the aggregate used Bash/Make.
The aggregate was already serial at the Rust test-harness level. Filtered
success does not repair the aggregate failure. Records and exact commands are
in `target/rust-async-isolation-20261001T230336Z-29871b54/`.

A temporary post-decision logger built offline, then ran the transfer and
basic async fixtures once. Transfer passed; async failed in 1.53s with the
`root_left_descendants` classification. The original source and executable
were unchanged. Evidence:
`target/rust-async-branch-20261001T230811Z-68eb1a9c/`.

A second isolated probe distinguished the live-monitor and post-output paths,
using the existing membership query and read-only native observations only
after the rejection decision. Its native-helper validation passed. Of at most
four planned async launches, the first passed and the second failed in 1.35s;
the remaining two were not launched. At the failed post-output settle decision,
the worker root (PID 20260) had output exit code 0. Membership listed PID 4688.
Before the NMLT supervisor's first kill call, one handle for that member reported
`WaitForSingleObject(0) = 258` (nonsignaled), membership in the exact Job Object,
image `conhost.exe` in the system directory, and parent PID 20260. The root
handle was signaled with exit code 0. The helper also reported member exit code
0; this does not override its nonsignaled wait result. Cleanup subsequently
completed and the run retained `Io`, `child_reaped: true`.

This identifies a process object with unconfirmed termination at observation
time in this diagnostic failure; it does not establish executing threads,
backdate liveness, or identify historical failures. It
shows a Windows console/process lifecycle interaction under the strict policy,
not a demonstrated launch, fixture, or supervisor defect. Independent review
found no basis for a policy-preserving production fix. The separate probe of
unchanged `Toolchain::open` passed once (13.83s including installation hashing;
its native version deadline remained 5s). The earlier Lean failure therefore
has no demonstrated shared cause. No helper exemption or deadline extension
was introduced, and no new aggregate run was started without a production fix.

Native observations, exact stage outcomes, instrumentation provenance, and
hashes are retained in `target/rust-async-native-20261001T232300Z-8acd0901/`.
Its polling interval was 20s, not an outer deadline. The runner's exit 0 means
diagnostics completed, not that all probes passed; stage outcomes and integrity
are audited separately. Native reads can delay cleanup/concurrent work and do
not constitute production acceptance evidence. The runtime policy, language,
proof checker, and axiom allowlist remain unchanged; no new RFC is required
for these isolated diagnostics. The complete verification gate remains open.

## Solo-policy supersession, 2026-10-03

The maintainer clarified that this is a solo project without an available
mathematical reviewer, recruited audience or external pilot. References above
to mandatory external mathematical/user/cross-family review are historical
gates, superseded by the current [Plan.md](../Plan.md), [governance](../GOVERNANCE.md)
and [solo validation protocol](solo-validation.md). Earlier test results, failures,
zero-participant/live-model counts and evidence hashes remain historical facts.
No new completion or external usefulness claim follows from changing the plan.
Current work must execute role-based scenarios, second uses, repair/revision/
interruption/recovery, negative controls and the full platform gates. Independent
kernel/NanoDA checking, conservative containment and explicit trust limits are
unchanged. Self/AI critique is labeled accordingly; external feedback is optional.

## Post-RFC 0037/0038 audit and re-verification, 2026-11-26

The earlier `Changed-file inventory` section above reflects the 2026-10-01
snapshot before the following additions on `codex/r4-discovery-slice`:

- **RFC 0037** (`rfcs/0037-console-free-contained-windows-processes.md`):
  vendored `processkit 3.3.4-nmlt.1` with SHA-256 provenance verification,
  `windows_no_console()` (`DETACHED_PROCESS`), v2 supervisor contracts, and
  the `windows-runtime` CI job.
- **RFC 0038** (`rfcs/0038-candidate-selected-lean-tactic-imports.md`):
  candidate-selected fixed tactic imports in `nmlt-cli` (plus CRLF wrapper
  handling and complete `GIT_CONFIG_*` forwarding in `lake.rs`).
- **Solo-validation tooling**: `NMLT.Discovery.NestedReuse`, `tests/SoloReuse.lean`,
  `examples/evaluation/solo-packet.json`, `tools/experiments/run_solo.py`,
  `tools/experiments/summarize_solo.py`, `tools/check_solo.sh`, and
  `tests/experiments/test_solo.py`.
- **Trust & CI alignment**: updated `SECURITY.md`, `security/trusted-components.toml`,
  `crates/nmlt-compile/Cargo.toml`, `Makefile`, and `.github/workflows/ci.yml`
  (adding `--lean-dir` support for staged CI execution of R4 discovery and R5
  corpus checks).

Targeted post-fix checks on Windows (`Rust 1.94.0`, `Python 3.13.12`) passed:
`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`nmlt-runtime --test process` (20/20), `nmlt-cli --bin nmlt` (34/34),
`nmlt-cli --test async_jobs` (7/7 — resolving the earlier `conhost.exe` settle
failures observed on 2026-10-01), `python tools/check_public_surface.py`,
`tests/experiments` (42/42), `tests/baselines` (14/14), and `git diff --check`.
A complete post-fix `make reproduce` across Lean, NanoDA, R3, R4, and R5
targets remains required before marking R4/R5 milestones complete.

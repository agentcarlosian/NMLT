# R2 seventh increment: initial Lean and asynchronous host APIs

Date: 2026-09-07 (America/Chicago). The completed source workflow, collection,
package, and synchronous job increments were committed as `0f4a9c8` at the user's
request before this work started. Validation below was recorded before this
increment was committed. R2 remains **In progress**, and
[RFC 0023](../../rfcs/0023-asynchronous-host-and-lean-adapter.md)
is Under review. See the [API guide](../r2-lean-async.md).

## Implemented scope

The new asynchronous process supervisor runs deadlines independently of polling,
drains bounded pipes, exposes exit/output observations, and reports cancellation
and direct-child cleanup. Session APIs start, poll, wait, cancel, and collect
through the existing durable journal. Handles have private construction, no
cloning or serialization, and an instance identity that prevents crossing live
sessions even when their run/context identifiers match.

The session persists its configuration before dispatch, enforces slot/attempt
bounds, charges declared work before spawning, and preserves uncertain work and
charges. Cancellation precedes signalling and requires confirmed child exit to
settle. Already settled results win before cancellation; late completion after
durable cancellation does not promote a result. Collection frees a slot exactly
once. Persistence failure stops the live session and requests cleanup.

The first Lean adapter checks fixed templates for `forall n : Nat, 0 + n = n`.
It records the pinned Lean executable/version, adapter contract, generated-source
identity, output, and empty-axiom policy. Wrong-term and admitted candidates are
negative controls; existing-lemma and induction candidates are positive cases.
The caller supplies an enum strategy, never arbitrary Lean syntax or a shell
command. The example returns reusable source digests, not source-language proofs.

Snapshots retain manifest, journal, and ordered process observations. Verification
reconstructs their binding and settlement consistency without launching work.
Recovery classifies journal state without recreating handles, discovering
processes, redispatching, or resuming source. The real worker and Lean examples
are part of the Rust and complete reproduction/Lean CI gates respectively.

This is an initial **Rust host API**, not asynchronous `.nmlt` syntax or a general
Lean adapter. No source grammar/evaluator, M9 rule, finite artifact schema, or
Lean definition changed. Host orchestration remains `assurance: none`. Captured
Lean observations are not fresh or independent checker results. The configured
standard/dynamic libraries remain trusted and are not fully fingerprinted.
OS process-tree isolation, complete toolchain locks, and automatic reconciliation
remain open.

## Validation

| Check | Result |
|---|---|
| Windows formatting / workspace Clippy, warnings denied | Passed |
| Windows / Linux workspace Rust tests | 305 / 307 passed |
| Added controls | 5 Lean-policy tests, 7 session tests, 6 supervisor tests, including two subprocess probes |
| Supervisor | Independent timeout, late polling, blocked stdin, both output bounds, nonzero exit, cancellation and reaping checked |
| Session | Parallel allocation, full pools, attempt exhaustion, slot generations, single collection, cross-instance handles, timeout uncertainty, and persistence failure checked |
| Snapshot | Dispatch/input, cleanup, missing/extra observation, configuration, and torn-journal mutations rejected |
| Lean policy | Exact target report, stderr/warnings, axiom dependencies, wrong/admitted controls, invalid output encoding, unsupported exits, and request/verdict identity checked |
| Windows real worker exercise | Three charged attempts, concurrent dispatch, cancellation, result reuse, snapshot replay, and completed-journal recovery passed |
| Linux Rust gate / worker exercise | Passed |
| Fresh Lean build / decoder controls / axiom audit | Passed |
| Independent NanoDA | 8,655 declarations checked with no errors |
| Frozen value graph | 1 initial, 4 states, 10 transitions; unchanged digest |
| Execution rejection controls / frozen resource graph | 30 controls passed; 1 initial, 8 states, 12 transitions; unchanged digest |
| Frozen R0 workflows | Proof 3/3, discovery 2/2, worker 4/4 passed |
| Final Lean-adapter gate | Passed: 6 charged attempts, both positive proofs, both negative controls, cancellation, replay, and recovery |
| Public links / trust inventory / diff whitespace | Passed |

All 29 Lean package files match the fresh stage
`.cache/r2-lean-async/lean.V5c55j/` by SHA-256. Existing proposition-definition
linter warnings remain. The unchanged metatheory checks do not verify the new
host API, supervisor, or interpretation of recorded Lean output.

The initial combined gate started at `2026-09-07T08:11:11Z` and passed all
retained checks through the R0 baselines. Its final new Lean exercise left the
admission-control attempt uncertain with the demonstration's ten-second budget.
A separate direct concurrent run of the admitted and existing-lemma templates
produced the expected outputs in approximately nine seconds each. That timing
suggests insufficient deadline margin on this filesystem, but the original
example did not retain the process observation needed to prove the cause.
The example now uses the supported thirty-second budget and includes its
observation in failure diagnostics; the Python driver also identifies the
evidence directory on failure. Runtime limits, verdict policy, and timeout
tests are unchanged. The original failure remains in the evidence.

The final validation passed formatting, workspace Clippy, runtime tests, and
the real worker and Lean exercises after those two example/driver changes,
running from `2026-09-07T08:27:24Z` to `2026-09-07T08:28:55Z`.
The other 108 of 110 recorded implementation/configuration hashes are unchanged.
The retained Lean, NanoDA, parity, and baseline results therefore apply to the
same runtime implementation and proof sources. Every required gate component
has passed across these runs; the original combined invocation is recorded as
failed, not rewritten as a successful uninterrupted run.

The final Lean example used two slots and six charged attempts. Its initial poll
was pending after two dispatches; the square returned 25 and was reused twice.
WrongTerm produced exit 1 and an explicit rejection; Admitted produced exit 0
with a `sorryAx` dependency and failed policy. ExistingLemma and Induction each
produced exit 0, empty stderr, and the exact empty-axiom report. Cancellation won
in this run, and the observation confirmed direct-child reaping. Snapshot replay
matched with `assurance: none`; replay and completed-session recovery both left
the journal unchanged.

The checker was Lean 4.33.1, commit `819816b2e0a3bf405af45ae5c7af2491d8f5bee6`.
Its executable SHA-256 was
`e8baaa71855a616dc351028f3ad2200051b0671f423a1696a100e809302d5550`, and the
adapter contract SHA-256 was
`3148ba3052629be332bd635f32e51f4fb1840b25a5dcc32835c1353def3b1f3b`.
The full source, executable, configuration, and journal identities are retained
in the final manifest and snapshot.

Unit fixtures that inject completions or failures establish host control behavior;
they do not establish physical process observations. The real `async_jobs`
example separately invokes the worker and pinned Lean. No native Windows Lean
run is claimed; real Lean adapter execution uses the Linux executable under WSL.

## Local evidence

Evidence is intentionally ignored by version control:

- `.cache/r2-lean-async/verify.sh` and `full-gate.log`
- `.cache/r2-lean-async/final-adapter-gate.sh` and `final-adapter-gate.log`
- `.cache/r2-lean-async/windows-tests.log`, `windows-clippy.log`, and `windows-async.log`
- `.cache/r2-lean-async/windows-clippy-final.log` and `windows-async-final.log`
- `.cache/r2-lean-async/source-hashes.json`, `source-hashes-final.json`, and `lean-source-hashes.json`
- `.cache/r2-lean-async/checker-artifacts/run.0AR5B9/` (NanoDA export, constants, configuration, and provenance)
- `target/r2-async/run-5d17lhrd/` (Windows worker)
- `target/r2-async/run-_xpdg_ze/` (Linux worker)
- `target/r2-async/run-e5eespbc/` (initial failed ten-second Lean exercise)
- `target/r2-async/run-7dcbxvos/` and `run-ul59refs/` (final Windows and Linux worker exercises)
- `target/r2-async/run-2e6ck164/` (final Linux Lean and worker exercise)
- `target/r0-baselines/run.CFsjvD/` (all three frozen baselines)
- `target/r2-jobs/run-v60b_a6u/` (retained subprocess gate)
- `target/async-session-tests/` (unit/fault evidence)

The exploratory first Lean run is retained in
`.cache/r2-lean-async/first-linux-session/`; final validation follows the gate
after subsequent policy hardening. The earlier source-job example's exact
Windows and Linux executables were preserved in `.cache/r2-effects/bin/`
before this increment changed the runtime build.
Initial and final asynchronous example executables are preserved in
`.cache/r2-lean-async/bin/`; the final Windows/Linux copies match their recorded
manifest identities. All 110 final implementation/configuration hashes were
rechecked after the successful adapter gate.

## Remaining work

Expose asynchronous handles in source with affine checking and branch/loop rules;
broaden the Lean request/interface beyond fixed templates; bind full dependency
identity; specify source recovery/reconciliation; and complete the remaining R2
invariant/project/real-input acceptance work. Independent publication review and
RFC acceptance remain open. No independent review is claimed.

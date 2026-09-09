# R2 eighth increment: scoped asynchronous source controls

Date: 2026-09-07 (America/Chicago). R2 remains **In progress**.
[RFC 0024](../../rfcs/0024-scoped-source-job-controls.md) is Under review;
no RFC acceptance or independent publication review is claimed.

## Implemented scope

The [source guide](../r2-source-async.md) and
[example](../../examples/pivot/async_fallback.nmlt) expose concurrent worker and
fixed-template Lean jobs to `.nmlt`. The typed workflow tree retains scoped
start/control operations and source locations. A separate pass checks single
collection, branch agreement, short-circuit restrictions, and loop affinity.
Handles never enter serializable values, functions, or aggregates. They are
scoped controls; handle transfer is not implemented.

The evaluator borrows handles for poll/cancel and consumes them for bounded
waiting and collection. Normal paths collect every local handle. Host responses
must match the typed operation result. The existing durable session owns actual
runtime controls and preserves timeout uncertainty, cancellation charges, and
generation/slot rules. Early evaluator stops retain unresolved journal state.

The CLI preflights source inputs, explicit slots/attempts/timeouts, and the
optional pinned Lean executable before dispatch. Its separate version 1 async
source record binds source/imports, typed program, inputs, exact executable,
session configuration, ordered operations, journal boundaries, and captured
process observations. Live record validation and replay check each operation's
allowed transitions and result. Replay then re-evaluates source against the
captured choices without launching jobs or requiring the live journal.

The existing synchronous builtin may be called within an asynchronous entry;
it uses the same session start/collect operations. Earlier finite, pure, and
synchronous-only entries keep their routes and record formats. Workflow
typecheck output advances to version 5 with separate asynchronous and Lean
effect flags. The new operation names are reserved.

No Lean definitions, finite artifacts, M9 rules, axiom policy, or interpretation
of captured Lean output changed. All new source constructs are executable-only
with `assurance: none`. A collected Lean receipt identifies a submitted fixed
template. General Lean input, full dependency identity, process-tree containment,
source reconciliation/resumption, invariants, and the project loop remain open.

## Validation

This increment was developed on a fresh native Windows x64 installation.
Installed Rust 1.94.0 with Clippy/rustfmt, Visual Studio 2022 C++ build tools,
Git for Windows, and Lean 4.33.1 through Elan. The user installed GNU Make 3.81.
Python 3.12.14 came from the
existing bundled workspace runtime. The native Lean version reports commit
`819816b2e0a3bf405af45ae5c7af2491d8f5bee6`.

| Check | Result |
|---|---|
| Complete `make reproduce` | Passed on native Windows, 2026-09-07 21:53:11–21:55:25 UTC |
| Workspace Rust tests | 316 passed in the complete reproduction run |
| Rust formatting / workspace Clippy, warnings denied | Passed |
| Static controls | Reuse, alias/aggregate/function escape, branch disagreement, short-circuit/fold consumption, missing collection, unsupported Lean strategy rejected |
| Execution controls | Concurrent starts, failure/fallback, reusable data, cancellation race, missing configuration, slot exhaustion, early stop, and uncertain recovery passed |
| Replay controls | Changed source, request, handle reference, position, journal boundary, result, outcome, missing/extra/reordered events, and altered snapshot rejected |
| Real source worker and Lean gate | Passed; worker returns `Ok(50)`, cancellation result recorded, wrong Lean term rejected, existing-lemma receipt collected |
| Source replay/recovery | Passed; completed journals unchanged; replay also tested with live state removed |
| Existing async host worker/Lean gate | Passed, six attempts including positive templates and admission rejection |
| Existing local worker subprocess gate | Passed, three cases |
| R0 Python harness tests | 14 passed |
| Frozen R0 real workflows | Passed: proof 3/3, discovery 2/2, worker 4/4 |
| Native Lean package / artifact controls / axiom audit | Passed, 33 build jobs and unchanged axiom policy |
| Pinned independent NanoDA check | Passed: all 2,354 NMLT roots and their closure, 8,655 declarations with no errors |
| Export file adapter | Matched the unchanged pinned CLI byte for byte on eight actual roots before exporting the complete set |
| Frozen finite value graph | Passed: 1 initial state, 4 states, 10 transitions; unchanged digest |
| V2 execution controls / frozen resource graph | Passed: 30 rejection controls; 1 initial state, 8 states, 12 transitions; unchanged digest |
| Public links / trusted-component inventory / diff whitespace | Passed |

One source test reuses one slot through four generations, mixing
an asynchronous start/collect, the synchronous builtin, and iteration-local
jobs. It returns `Ok(30)` and replays successfully. The final CLI executable
matches the copy retained by the real source gate, SHA-256
`4ad9d72cf8d2f43516b26b3c865c87881dd32077432091007e675a3da06cca08`.
Clippy and formatting passed in the complete gate.

The initial native Lean build reported that its linked executable was missing;
the linker output was inspected and a subsequent native Lake build completed.
No proof source or linker policy was changed. The first execution-control run
failed at source identity because native Python supplied backslash paths and
GNU `sha256sum` escaped its output. Direct comparison confirmed a backslash
path failed earlier with `stale execution artifact digest`, while forward-slash
paths reached the expected `stale source digest` rejection. The harness now
passes forward-slash paths to the same checker. All 30 controls pass without
changing expected verdicts or source/artifact bytes. The checker itself still
requires its ordinary unescaped digest output; no broader path support is claimed.

Native Make initially tried to execute a shell script directly (Windows error
193); the Make targets now invoke Bash explicitly and accept a `PYTHON`
executable override. The first NanoDA invocation exceeded Windows' command-line
length limit while passing all 2,354 roots. The file adapter calls the unchanged
pinned exporter entry point with the same ordered roots and streams its output
directly to a file. An initial buffered adapter run was stopped for excessive
output-buffer overhead after its parity comparison passed. The streamed adapter
and full independent check then passed, followed by the complete reproduction
run. Executable suffix handling and guarded scratch cleanup also support native
Windows. No Lean proof, exporter algorithm, checker pin, or axiom policy changed.

## Evidence and review status

The complete run is retained in the parent workspace at
`work/reproduce-final.log`. Earlier development logs remain in `work/`:
`rust-final.log`, `lean-build.log`, `metatheory-gate.log`, `parity-gate.log`,
`execution-gate.log`, `source-async-gate.log`, `host-async-gate.log`,
`baseline-tests.log`, `r0-baselines-gate.log`, `source-async-tests-final.log`, and
`job-runtime-gate.log`. Initial Make/adapter attempts are retained in
`full-reproduce.log`, `checker-bootstrap.log`, and `nanoda-native.log`; the first
successful streamed NanoDA run is in `nanoda-native-streamed.log`.

The complete run's source evidence is in
`target/r2-source-async/run-9gw7t5m3/`, including the exact CLI executable.
Host evidence is in `target/r2-async/run-s7r2rn31/`; frozen R0 evidence is in
`target/r0-baselines/run.miG0xo/`. These artifacts are local, ignored evidence.

The complete run retained NanoDA inputs and export-adapter parity evidence at
`work/nanoda-evidence/run.9UmxAb/` in the parent workspace. Its
`provenance.txt` records every retained input hash. The full export has 49,598,519
bytes and 959,988 NDJSON lines.

| Input | Immutable identity |
|---|---|
| lean4export commit | `411dce7db58a3afc60ecab2d211acd1042b593dc` |
| nanoda_lib commit | `05055695879dfebb6628a67da88ceca6cd6b0421` |
| Checker Cargo.lock SHA-256 | `8c5bf472ad83e7fd8694b93d1b172b5c3a63492e6976fdbc24d210948ee006a7` |
| Export SHA-256 | `7122b8b23698904920885f73813c038dc718accb8945c99ab2bf69bcd368094f` |
| Ordered roots SHA-256 | `e7dbc9f69915728ef80036a209f9393f8627968706592ddd8c5acd5dfce6c9c1` |
| Checker configuration SHA-256 | `556fa3f342120c2d37b59685902010080222c973895988ff106ffd2dcf695d56` |

The checker permitted only `propext`, `Quot.sound`, and `Classical.choice`, with
unpermitted axioms configured as hard errors. The earlier dependency/access
block is resolved. The native setup command is documented in
[getting started](../getting-started.md).
No Linux run or independent cross-family review is claimed for this increment.
RFC acceptance and publication review remain open, as do the R2 implementation
gaps listed above.

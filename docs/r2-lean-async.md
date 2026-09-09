# Initial Lean adapter and asynchronous controls

The R2 Rust runtime API supports local job orchestration. It starts jobs without
waiting, polls them, requests cancellation, and collects settled results once.
The first Lean adapter checks fixed proof templates for `forall n : Nat, 0 + n = n`.
[Scoped source controls](r2-source-async.md) now expose these operations to `.nmlt`.
The [proof-term interface](r2-lean-terms.md) adds dynamic statement/candidate inputs;
[transfer](r2-job-transfer.md) and [source resumption](r2-recovery.md) are also available.

The adapter explicitly sets Lean's thread stack to 64 MiB before runtime
initialization. Lean 4.33.1 otherwise defaults to 1 GiB per thread, which fails
to create the required threads under the Unix process data-segment limit.
The existing process memory ceiling is retained. This launch setting is part
of the adapter contract digest; older captures require their retained executable.

Run the real example using the direct executable from the pinned Lean installation:

```bash
cargo run -p nmlt-runtime --example async_jobs -- target/async-lean-demo /absolute/path/to/lean-4.33.1/bin/lean
cargo run -p nmlt-runtime --example async_jobs -- --replay target/async-lean-demo/snapshot.json
```

The directory must be new. The example uses two slots, six attempts, and a
thirty-second deadline per job. It starts two jobs before waiting, handles
a rejected proof, reuses the collected square, requests cancellation, then checks
the admission negative control and two valid proofs. `report.json` summarizes
outcomes; `snapshot.json`, `manifest.json`, and `journal.jsonl` retain the evidence.
Replay checks the recorded observations and journal without invoking Lean again.
Keep the exact example executable for later replay.

The worker-only exercise runs on Windows without a native Lean installation:

```powershell
cargo run -p nmlt-runtime --example async_jobs -- target/async-worker-demo --without-lean
```

Use the Linux Rust toolchain and Lean executable together under WSL for a Linux
Lean installation. A Windows process cannot directly launch a Linux Lean binary.

The public API is in `nmlt_runtime::session`:

| Operation | Behavior |
|---|---|
| `Session::create` | Saves a new manifest and opens a durable journal |
| `start_square` / `start_lean` / `start_lean_candidate` | Reserves and records dispatch before launching; returns a private handle |
| `poll` / `wait` | Returns pending, ready, uncertain, or collected state |
| `cancel` | Records intent before signalling; confirms cancellation or reports uncertainty |
| `collect` | Returns a settled outcome once; pending work keeps its handle |
| `snapshot` / `verify_snapshot` | Captures and checks journal/observation consistency |
| `recover` | Settles saved observations and classifies unresolved work without dispatch |
| `Session::resume` | Restores the locked session; only never-dispatched reservations may launch |
| `acknowledge_uncertain` | Records explicit operator failure settlement and retains spend |

Configure 1–4 slots, at most 16 attempts, and a timeout up to 30 seconds. Child
deadlines run independently of polling. Cancellation does not refund dispatched
work. Confirmed cancellation still needs collection before slot reuse. A timeout
retains an uncertain attempt and its charge even after direct-child cleanup.
Dropping a session requests best-effort cleanup and leaves unresolved journal
state for explicit recovery.

Lean acceptance requires the pinned executable version and bytes, a generated
source identity, a successful process exit, and the exact empty transitive-axiom
report for the named target. `sorry` and wrong-proof controls are never accepted.
A failed candidate does not refute the target. Returned text identifies submitted
source; it is not a source-language proof object or independent checker report.
The complete `bin`/`lib` identity is checked before dispatch and retained in
project locks. The common supervisor records tree/resource containment; its
[platform limits](../rfcs/0028-contained-process-lifecycle.md), system libraries
and concurrent filesystem stability remain explicit host trust boundaries.

The gates are:

```bash
make r2-async
make r2-lean R2_LEAN_BIN=/absolute/path/to/lean-4.33.1/bin/lean
```

`make ci` includes the worker exercise. `make reproduce` and the Lean CI job also
include the Lean exercise. With elan installed, the default Lean path is resolved
from the repository pin; `R2_LEAN_BIN` can specify a direct installation.
All host orchestration retains `assurance: none`.
See [RFC 0023](../rfcs/0023-asynchronous-host-and-lean-adapter.md) for the full contract.

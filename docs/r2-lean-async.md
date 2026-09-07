# Initial Lean adapter and asynchronous controls

The next R2 increment is available as a Rust runtime API. It starts jobs without
waiting, polls them, requests cancellation, and collects settled results once.
The first Lean adapter checks fixed proof templates for `forall n : Nat, 0 + n = n`.
Source-level asynchronous handles and general Lean proof input remain next work.

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
| `start_square` / `start_lean` | Reserves and records dispatch before launching; returns a private handle |
| `poll` / `wait` | Returns pending, ready, uncertain, or collected state |
| `cancel` | Records intent before signalling; confirms cancellation or reports uncertainty |
| `collect` | Returns a settled outcome once; pending work keeps its handle |
| `snapshot` / `verify_snapshot` | Captures and checks journal/observation consistency |
| `recover` | Classifies unfinished journal state without rebuilding handles or redispatching |

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
The standard/dynamic libraries remain part of the trusted local installation;
complete dependency locks and process-tree containment are not implemented.

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

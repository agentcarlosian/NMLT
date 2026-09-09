# R0 deterministic comparison baselines

These three small Python/Lean programs establish a comparison baseline for the
proposed NMLT workflow language. They are ordinary scripts with a frozen corpus,
not an implementation of that language or its future runtime guarantees.
They use Python's standard library and the exact Lean version in
[manifest.json](manifest.json), which must match the repository's Lean pin.
There are no model calls, credentials, paid jobs, or network requests.

From the repository root, `make r0-baselines` runs all three in a fresh directory
under `target/r0-baselines/`. It selects the committed Lean toolchain; the
`R0_LEAN_COMMAND_JSON` Make variable can select an equivalent extracted binary.
To control the output directory directly:

```sh
python3 tools/baselines/run_baselines.py --output-dir target/r0-example \
  --lean-command-json '["lean"]'
```

The command array is executed directly, without a shell. Its executable must
report the frozen version. Lean sources are sent through `--stdin`, so a WSL
prefix or an absolute executable path can be supplied without translating
artifact paths. Only `Init` is imported. The runner does not install Lean or
provide process-tree isolation; its subprocess watchdog is a checking timeout,
not a sandbox for arbitrary generated code.

Each selected scenario writes `record.json` and `report.md` in its own directory.
Proof and discovery scenarios also write the exact submitted `.lean` sources.
Records separate declared task budgets, strategy attempts, checker invocations,
observed wall time, and zero model expenditure. Wall times vary between runs;
the task inputs and expected outcomes are deterministic. Fresh runs have unique
instance IDs and a separate deterministic context digest.

| Scenario | Frozen work | Required output |
|---|---|---|
| `proof` | Three strategies for `forall n : Nat, 0 + n = n` | Ordinary wrong-term rejection, then accepted library-lemma and induction proofs |
| `discovery` | Two addition conjectures, each tested on 25 pairs in `{0,1,2,3,4}` | Lean-checked refutation of left projection using `(0,1)`, and the known universal commutativity theorem |
| `worker` | Four cases, one slot, at most two generations per case | Successful square calculation, typed negative-input failure, cancellation before dispatch, and cancellation with late/duplicate responses after slot reuse |

Proof acceptance means a successful Lean process and an empty transitive-axiom
report for the exact frozen theorem. It is **not independent NanoDA checking**.
Candidate proof bodies and mathematical statements are fixed inside the script;
there is no arbitrary proof-generation interface. A successful exit without the
expected axiom report is insufficient. Missing tools, a different version,
timeouts, and other tool failures do not establish mathematical falsity.

The discovery program is a known-result calibration. Finite tests alone do not
establish its universal theorem; that comes from the separately checked Lean
proof. It makes no novelty claim and does not satisfy the future useful-domain
discovery milestone by itself.

The worker computes a small pure function synchronously and simulates delayed
response delivery. Dispatch consumes received permission and charges one attempt.
Cancellation does not refund dispatched work. A response is bound to the exact
run/task/slot/attempt/generation/owner/input/output type; stale and duplicate
responses cannot settle a new attempt. These four traces do not establish all
interleavings, actual process cancellation, NMLT semantics, or exactly-once
execution of an external effect.

To demonstrate interruption and resume without killing a process:

```sh
python3 tools/baselines/run_baselines.py --scenario proof \
  --output-dir target/r0-resume --stop-after 1
python3 tools/baselines/run_baselines.py --scenario proof \
  --output-dir target/r0-resume --resume
```

Add the same `--lean-command-json` option to both commands when needed. The
first command deliberately records `incomplete`. Resume requires identical
manifest, implementation, and checker identities, preserves the run instance,
and freshly rechecks every completed proof candidate. Rechecks count as checker
work, not new strategy attempts. Discovery resume recomputes its finite tests,
rechecks Lean, and preserves earlier checks in `replay_history`. Worker resume
recomputes pure local cases; no external effect is replayed. Local records are
inspection artifacts, not tamper-proof receipts.

Exit codes: `0` means required outcomes matched or an intentional proof stop
left a matching incomplete prefix; inspect `status` before calling a run
complete. `1` means a required outcome did not match. `2` means configuration,
tool availability/version, or resume identity prevented the run. Existing output
records require `--resume` or a fresh output directory.

Run focused runner/acceptance controls with:

```sh
python3 -m unittest discover -s tests/baselines -v
```

Those unit tests use a labelled fake checker to test bookkeeping and error
boundaries; they are not Lean proof evidence. The real baseline command is a
separate integration check. See the [frozen contracts](../../docs/r0-baseline-contracts.md)
and [pilot protocol](../../docs/r0-pilot-protocol.md) for the comparison and
future user-evaluation boundaries.

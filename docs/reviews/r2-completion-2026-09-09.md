# R2 implementation and local validation complete

R2 is complete at its specified local, finite, pre-alpha scope on 2026-09-09.
The milestone's complete requirement set is audited in the
[completion tracker](../r2-completion-tracker.md). RFCs 0024–0030 remain Under
review; this implementation and automated validation record does not substitute
for independent human acceptance or publication review.

## Delivered behavior

- A project can initialize, accept changed real inputs, execute typed worker
  and Lean jobs, handle failure, reuse results, test expectations, format its
  sources, validate its lock and replay retained evidence.
- Parameterized components share source packages, records, outcomes and bounded
  collections. Affine job handles move through functions, branches and folds;
  external inputs and reusable data cannot fabricate or duplicate them.
- Variable Init statements and proof terms replace the four-template limitation
  for supported Lean jobs. The closed term grammar, exact generated source,
  complete installation identity and empty-axiom policy are explicit.
- Durable source intent/reply journals preserve decisions across interruption.
  Completed observations and lost collect replies are reused. Already-dispatched
  attempts are not relaunched; explicit operator failure acknowledgement retains
  their original spend. New project records can resume the original job session
  after working sources or the manifest have changed.
- A user `safety ... = always(...)` predicate has exact source/artifact binding,
  independently checked finite enumeration, and Lean-checked initialization and
  preservation or an initialized violating path. A supplied reached-set
  certificate is checked for closure; Rust exploration completeness is not
  assumed.
- A common ProcessKit supervisor supplies bounded raw pipes, independent
  deadlines, tree cleanup and a recorded platform resource policy. Lean jobs
  bind every file in the selected installation's `bin` and `lib` trees.

See the [project](../r2-projects.md), [source job](../r2-source-async.md),
[transfer](../r2-job-transfer.md), [proof term](../r2-lean-terms.md),
[recovery](../r2-recovery.md) and [finite invariant](../r2-safety-invariants.md)
guides for runnable commands and exact restrictions.

## Validation sequence

The complete native Windows `make reproduce` run exited **0**, from
`2026-09-09T05:35:26Z` to `2026-09-09T06:01:06Z`. It included 344 Rust tests and
all proof, parity, baseline and real-adapter gates below.

Final review then found a Rust API edge case: a restored reservation that had
never dispatched had no process to signal during cancellation. The fix cancels
that reservation locally, preserves its attempt identity, and leaves its charge
at zero. A regression covers cancellation, collection, empty observations and
accounting. A subsequent complete `make ci` run exited **0** with **345 Rust
tests**, Clippy warnings denied, the source worker and project workflows, and
14 Python baseline tests. The final code also passed Linux cross-compilation
for every workspace target. That final patch changes no Lean definitions,
generated Lean source, dependency identities or finite step operations.

| Check | Result |
|---|---|
| Final Rust CI | 345 tests, formatting, all-target checks, Clippy and worker/project gates passed |
| Python baseline harness | 14 tests passed; all 9 frozen real-input tasks completed |
| Lean package and focused axiom audit | Passed; no axiom-policy weakening |
| Independent NanoDA check | 9,004 declarations checked, 2,505 NMLT roots; no errors |
| Frozen finite-value graph | 1 initial state, 4 reachable states, 10 transitions; unchanged |
| Frozen authority/execution graph | 1 initial state, 8 reachable states, 12 transitions; unchanged |
| Execution witnesses | Real Lean acceptance and 30 rejection controls passed |
| User safety invariant | `UseAfterReceive` accepted; `NeverUsed` had a 2-step counterexample; `AlreadyReceived` failed initially |
| Invariant rejection controls | 15 independent Lean checks rejected mutations, including recomputed envelope hashes |
| Real Lean source jobs | Rejection/fallback, arithmetic, logic, axiom rejection, replay and saved-result resumption passed |
| Real Lean project | 14,972 installation files locked; test, replay and resumption passed |
| Recovery | Nine coherent durable-prefix boundaries; timeout/retry, retained spend, lost collect, incomplete-tail quarantine and tamper rejection passed |
| Native containment | Windows descendant cleanup, abrupt parent death, memory and process limits passed |
| Linux coverage | All-target compilation passed; Linux execution is not asserted by this Windows run |

The prefix fixtures deliberately reconstruct valid saved boundaries; they are
not presented as nine physical crash experiments. Actual process-death, stale
generation, cancellation/late-response and journal-lock tests remain separate
runtime regressions.

## Retained evidence

The local workspace `outputs/` directory retains the integrated reproduction log,
the final Rust CI log and the Linux compile log. Final native CLI SHA-256:

```text
37bd0cb9a74c6a92457aafcc95a976b25d193d90b49ae2c189b0383274770417
```

The integrated run's main evidence directories are:

```text
target/r2-async/run-fg4o0u7n
target/r2-source-async/run-qvusbq9l
target/r2-projects/run-726pog70
target/r2-invariants/run-5581u1m8
target/r2-lean-terms/run-eywgw58i
target/r0-baselines/run.OD4mBl
```

The final Rust CI retained fresh worker/project evidence in:

```text
target/r2-jobs/run-4mjbsyne
target/r2-async/run-crfjwkdz
target/r2-source-async/run-o98179i1
target/r2-projects/run-sybd5rws
```

Each run's retained executable is required to replay its exact records. The
integrated Lean receipts predate the final reservation-cancellation guard and
retain their original executables; the final worker/project records use the
final executable above.

NanoDA artifacts are retained in the workspace's
`work/nanoda-evidence/run.6fAwF7`. The exporter pin is
`411dce7db58a3afc60ecab2d211acd1042b593dc`, NanoDA is
`05055695879dfebb6628a67da88ceca6cd6b0421`, Lean is 4.33.1 and Rust is 1.94.0.
The file-input export adapter matched the unchanged pinned exporter on eight
sampled roots before the complete export.

```text
Export: 51,076,596 bytes; 987,533 lines
Export SHA-256: 9e5ef4a4796ea4ca90050eff4ac320a2b784eca6280e6b571e4d1d1e20cf7a1f
Root-list SHA-256: 6febb893c07c53d8f83109f34880615862bcbdbe2873a07c28f3b53d86f7ce34
Finite-value graph SHA-256: bcc6f4e6832eaa2d25330e6c79e324b0497915f2c4d0891fa333bab81bee2da0
Authority graph SHA-256: e027879e69c8ff5ccae51cffacc7980f1493aeff427124546bd9b017436c53a2
```

## Scope and residual trust

Workflow functions, scheduling and host effects are executable-only. The finite
invariant result applies to its supplied decoded model and predicate; source
translation and any abstraction connecting it to a real host execution remain
explicit assumptions. Captured replay/resumption is not a fresh Lean check,
authenticated OS observation or exactly-once external execution guarantee.

The Windows Job Object policy and Unix process-group/rlimit fallback have
different containment guarantees. Neither is a filesystem/network sandbox.
System libraries, cooperating local filesystem durability, concurrent tool
stability, the pinned Lean binary and OS observations remain trusted. Incomplete
tail repair does not turn multiple files into an atomic transaction or recover
arbitrary corruption.

Finite enums are part of the behavior profile; general workflow host data is
not silently treated as finite model state. Imported/reviewed Lean projects,
target-environment matching, arbitrary tactics, verified compilation, liveness
and distributed execution remain outside R2. The planned R3 work is separate.

# R0 checker and workflow baseline

Date: 2026-09-06. Status: **R0 complete at its calibration scope**.

The repository builds and independently rechecks with Lean 4.33.1. Three frozen
Python/Lean reference workflows run successfully, including proof interruption
and resume. This establishes the baseline for R1–R5; it does not implement an
NMLT interpreter or establish a usability improvement or new mathematics.

The P3 graph-depth correction was preserved separately as commit `33e7240`.
The results below cover that commit plus the reviewed R0 working changes.
These results were captured before the R0 commit. Remote publication was not
part of this execution.

## Compatible checker set

The selected release incorporates the fixes described in the August upstream
postmortem. This is maintenance of the checking baseline, not a claim that a
checker can never contain another bug. See the [official 4.33.1 release](https://github.com/leanprover/lean4/releases/tag/v4.33.1)
and [upstream postmortem](https://leodemoura.github.io/blog/2026-8-24-postmortem-for-the-kernel-soundness-bug-hunt/).

| Component | Exact version or commit |
|---|---|
| Lean | `4.33.1`, `819816b2e0a3bf405af45ae5c7af2491d8f5bee6` |
| lean4export | `411dce7db58a3afc60ecab2d211acd1042b593dc` |
| NanoDA | `05055695879dfebb6628a67da88ceca6cd6b0421` |
| Rust for NanoDA and NMLT | `1.94.0`, rustc `4a4ef493e` |
| Reference workflow Python | 3.14.4 in WSL; harness also checked with Windows 3.11.15 |

The official Linux archive was verified against its release metadata. The
exporter was rebuilt using NMLT's exact Lean pin, overriding its upstream
default. NanoDA was rebuilt with its committed Cargo lockfile. Neither final
checking nor the reference workflows depend on an incremental Lean adapter.

Compatibility required only `@[implicit_reducible]` on `toBehavior` in
[SemanticClosure.lean](../../mechanization/lean/NMLT/Artifact/SemanticClosure.lean).
Lean's instance search must see the concrete finite state type through this
constructor. Its body and type, the predicates and theorem statements, and
the axiom policies are unchanged. Independent source review confirmed this.

A package copied without `.lake` output completed all 22 build jobs. Artifact
invariants, both canonical source/artifact fixtures, existing rejection
controls, placeholder checks, and the focused axiom audit passed. The final
working Lean sources, package definition, pin, and axiom policy were byte-compared
with this checked copy. The package has no external Lake dependencies.

Fresh export selected **2,028 NMLT constants** and their dependency closure.
NanoDA checked **7,224 declarations with no errors**. The export contains
32,152,845 bytes and 619,762 lines. The full audit retains its allowlist of
`propext`, `Quot.sound`, and `Classical.choice`; the focused theorem audit retains
its narrower policy. No placeholder or compiler-trust allowance was added.

| Artifact | SHA-256 |
|---|---|
| Official Lean Linux archive | `890afd185370f85666025b883914ab4f4b339136f8c96167b69cfb62aecaf235` |
| Lean executable | `e8baaa71855a616dc351028f3ad2200051b0671f423a1696a100e809302d5550` |
| NanoDA Cargo.lock | `892346d3da3a6d728b34447e4cd8b6f063ac6acd9b514262000d85d58d17d9e4` |
| NanoDA executable | `37006cb9cb336c33ce72c37e73d1dd6eeb141b7f077eaebe820c4da329b3591f` |
| Export | `7a8c0dcca9087921f30b15dc6a1e2db107d6e4f483850574e1c62989d0357f44` |
| Selected constants | `ab0167dcd1527806174845d60009f1502b96270c68f83645e2d6bac19bab648b` |
| NanoDA configuration | `556fa3f342120c2d37b59685902010080222c973895988ff106ffd2dcf695d56` |

Successful export, selected constants, relative-path configuration, and provenance
are retained locally in `.cache/r0-toolchain/checker-artifacts/run.9hqflV`.
The input hashes were independently recomputed in PowerShell and matched.
[check_nanoda.sh](../../tools/check_nanoda.sh) now supports
`NMLT_NANODA_ARTIFACT_DIR` to retain each successful invocation in a new directory.
These are local evidence files; the checker can be rebuilt from the pinned source
to replay them. They are not a signed release or a theorem-statement comparator
against a separately supplied challenge. The repository audit does not audit the
separate baseline proofs below.

## Executed reference workflows

The [contracts](../r0-baseline-contracts.md), [manifest](../../examples/baselines/manifest.json),
and [guide](../../examples/baselines/README.md) freeze the small public calibration
corpus. All cases passed under the final runner. Baseline model calls, tokens,
and model charges are zero; this is not a measurement of the development session's
cost. Every accepted calibration proof reported an empty transitive axiom set.

| Workflow | Observed result |
|---|---|
| Proof strategies | Wrong term rejected; existing-lemma and induction proofs accepted for the same `forall n : Nat, 0 + n = n` target |
| Discovery | `(0,1)` found and the negation of the false universal checked in Lean; all 25 pairs tested for the revised commutativity claim and its known universal proof checked separately |
| Worker | Success returned 9; negative input returned typed failure; cancellation before dispatch spent zero attempts; cancellation and slot reuse returned 25, charged both dispatches, and ignored the late and duplicate responses |
| Interruption/resume | Stop after one strategy recorded `incomplete`; resume retained the run ID and completed with three charged strategies/four checker invocations; another resume rechecked all candidates, including accepted proofs, for seven total checker invocations and still only three strategies |

The final batch is `target/r0-baselines/run.srE8D2`; the resume exercise is
`target/r0-resume.lyuMkQ`, including `after-stop.json` and `after-resume.json`.
All final records identify the same finished runner and their saved Lean source
hashes match the files. A preliminary run overlapped the final edits; it is not
the completion evidence. The runner now captures implementation identity once
per process to avoid mixed identities across its scenarios.

| Final input or record | SHA-256 |
|---|---|
| Runner source | `d5fd0e5a9da2cb0aca607811eb0acc0be7fcab14d84a2b993f082794e16713ba` |
| Manifest file bytes | `6a1f5003702c0104792732b5fcb20c5147e1190193d7bb7d354915c3332ea4c8` |
| Canonical manifest identity | `d9b83650e404b2e0f45f496374425d727ca112caa158f12bf4991558683fa992` |
| Proof record | `4a25596fb1d8a227bb9f55a9895591075e157edee2fbfb99d3055e5b7f68081d` |
| Discovery record | `e95dfa33f11d762a08ff072e89d2f0ca878fc62ec32a35725cc6b82dc81ca605` |
| Worker record | `45832aaa338433a42789f9f08c1ede1e2fb076061c8acf7eaa213c2e02e43ec3` |

The final batch recorded 33.382 seconds of proof checking and 9.063 seconds of
discovery checking on this WSL/Windows-mounted setup. These single observations
exclude setup and the resume exercise and are not a throughput benchmark. The
worker is synchronous local computation with simulated delayed delivery. Its
cancellation ends local result acceptance; it does not demonstrate host-process
preemption or exactly-once external effects.

## Validation and reproduction

The native Rust gate passed formatting, workspace/all-target compilation,
Clippy with warnings denied, and **189 tests**. **14 Python harness tests** passed
on Windows and in WSL; their mocked checker results only test orchestration.
The real Lean batch and resume tests above establish the separate integration
result. Artifact reproduction/exploration, public links and trust inventory,
Bash syntax checks, and diff whitespace checks passed.

The complete gate is now exposed through:

```sh
make reproduce
```

This execution ran its components explicitly: native `cargo fmt --all --check`,
`cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`,
and `cargo test --workspace --all-targets`; WSL `make behavior-artifact`,
`make public-surface r0-baseline-tests r0-baselines`; and both Lean checker scripts
against the clean staged package. It did not run a remote CI job.

For a fresh independent audit with retained evidence:

```sh
make metatheory
NMLT_NANODA_ARTIFACT_DIR="$PWD/target/nanoda-records" make nanoda
make r0-baselines
```

Local evidence logs are `target/r0-rust-gate.log` and, under
`.cache/r0-toolchain/logs/`, `clean-checks.nmlt-lean-clean.5Ew00R.log`,
`nanoda-final.log`, `root-host-gates.log`, and `root-final-baselines.log`.
Downloaded Linux tools remain isolated in `.cache/r0-toolchain`. The initial
Lean compatibility failure and Cargo workspace-discovery failure were diagnosed
and corrected; neither was counted as a passing check. Use ordinary scratch
outside the repository for NanoDA's separate Cargo workspace. Existing `defProp`
style warnings remain. `.cache` is now excluded from public Markdown scanning
so downloaded third-party documentation is not mistaken for project material.

## Friction for the next milestones to address

These are concrete implementation observations and product hypotheses, not
findings from a participant study:

| Audience | Current script responsibility | Benefit NMLT must demonstrate |
|---|---|---|
| AI/Lean developer | Hand-written target templates, version checks, attempt accounting, record identity, and replay checks | A reviewed reusable proof-job abstraction that preserves the exact obligation while reducing custom recovery and bookkeeping code |
| Mathematician | Manually align a Python predicate, a Lean claim/negation, a counterexample, and a revised statement | Explicit statement revisions and evidence dependencies that prevent finite support, failed proof attempts, and checked universal results from being confused |
| Software engineer | Hand-written ownership state machine, response bindings, cancellation settlement, and late-result controls | Composable receive-then-use semantics and a working interpreter that retain these controls across programs, with scoped invariant and trace evidence |

R0 review itself exposed colliding run identities, incompletely enforced budgets,
discarded replay evidence, and Windows source-byte mismatch; the runner and
regression controls now address them. This supports investigating reusable
abstractions, but does not show that new language syntax outperforms a library.
Retain the library approach if the later same-task comparison cannot show a
meaningful composition, diagnostic, or user-effort advantage.

[RFC 0014](../../rfcs/0014-executable-workflow-profile.md) remains **Draft**.
[Pilot task cards and forms](../r0-pilot-protocol.md) are role-based; no named-user
targeting or recruitment is required for R0. Independent usability evaluation
and the larger held-out corpus remain R5 work. R1's next gate is unified dynamic
semantics with actual decoded initial execution and finite-path ownership
results; the reference worker does not satisfy that mathematical obligation.

Sequencing clarification: the earlier plan said the proposed larger 30/12/3
evaluation corpus would be frozen in R0. R0 instead freezes the smaller public
calibration corpus implemented here; the larger comparison set must be finalized
and frozen before comparative testing. Both plans now state that distinction.
This avoids treating development examples as held-out evaluation evidence.

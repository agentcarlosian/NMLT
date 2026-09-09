# R3 first increment: bound local Lean tasks

Date: 2026-09-09. Scope: executable-only, bounded trusted local Lean projects
under [RFC 0031](../../rfcs/0031-bound-lean-project-tasks.md).
R3 remains in progress. This is an automated implementation and validation
record; independent cross-family/human review and RFC acceptance remain open.

## Implemented behavior

`nmlt lean-task bind` captures explicitly ordered source modules, the exact
elaborated declaration type and universe parameters, the Lean installation,
axiom policy and CLI identity. A separately selected task hash constrains a
candidate to its proof term. Binding does not record human approval.

`prove` reconstructs ordinary Lean files, checks the formal target and actual
transitive axioms, exports the final proof root and requires independent NanoDA
acceptance. Its result includes an additive Lean patch, actual dependencies,
an explanation, tool identities and the exact CLI executable. `recheck` repeats
the checks from retained sources after the working project is removed. A
statement or definition revision does not inherit acceptance for the old task.

## Executed gates

- Full native Windows `make reproduce` passed at `b5fec7b` from 08:43:36Z to
  08:52:10Z (exit 0). It passed 351 Rust tests, 14 Python tests, Clippy with
  warnings denied, Lean metatheory and axiom auditing, independent NanoDA
  checking of 9,004 declarations from 2,505 roots, canonical artifact and finite
  graph comparisons, 30 execution rejection controls, all nine R0 tasks,
  the real R2 Lean/source/project/recovery workflows, 15 invariant rejection
  controls, and the complete R3 gate. The run used Rust dev/test optimization
  level 1 with debug assertions enabled. Local log: `work/r3-final-reproduce.log`.
- The [Linux R3 job at b5fec7b](https://github.com/agentcarlosian/NMLT/actions/runs/34330664321/job/102398229503)
  passed the same four proof cases, fresh recheck and nine rejection controls.
  Its preparation built the pinned exporter and checker from source inside
  the checkout. Evidence directory in that job: `target/r3-lean-tasks/run-qj_f2ooc`.
- Follow-up merge `09429e0` brings in the migration branch's invariant-checker
  startup settings. CLI Clippy and the real invariant/counterexample gate,
  including all 15 rejection controls, passed again on native Windows in
  `target/r2-invariants/run-w13h1ej4`. This follow-up does not change R3 proof
  handling. The full reproduction above remains attributed to its tested tip.
- `git apply --check` accepted the generated proof patch against the saved
  example project. Public-surface and whitespace checks passed.

The R3 gate accepts four cases and separately rejects nine invalid cases:

| Accepted proof | NanoDA declarations | Actual permitted axiom use |
|---|---:|---|
| Local definition lemma | 47 | none |
| Assumption-bearing lemma | 48 | none |
| Polymorphic target | 5 | none |
| Explicitly permitted equivalence proof | 9 | `propext` |

The retained-executable fresh recheck reproduces the 47-declaration definition
export while the working project is unavailable. Rejections cover a wrong
term, a directly unresolved lemma, a transitively unresolved lemma, a changed
definition with the old pin, rehashed false target metadata, altered proof
source, changed checker identity, an invalid exported proof value rejected by
NanoDA itself, and the equivalence proof without its axiom permission.

These finite denominators are regression evidence, not universal reliability
or mathematical-novelty claims. The examples are calibration cases, not R5's
proposed held-out evaluation corpus.

## Retained artifacts and tool identities

Native Windows R3 evidence: `target/r3-lean-tasks/run-youy2jea`. Checker source
and builds: `target/r3-checkers/run.eCoDy7/tools`. Each successful case retains
its exact executable and full task/result identities. In particular:

- Definition task SHA-256:
  `21c01bdea8c479c22d67403a94e1963b954961d3e82021daf97f7ca2f37cfbd8`.
- Definition export SHA-256, identical after fresh recheck:
  `566e99103f9566ed20d1af9353eaa5f5bd8447e7b403825b2fc6400ea0cf96ba`.
- Retained CLI SHA-256:
  `fe2c2c02043b57f0d46c8205f8289dcc6e11c9a0ca535f5b0f61328423a0a93c`.

The unchanged metatheory export is retained in
`work/nanoda-evidence/run.lOh0hW`, with SHA-256
`9e5ef4a4796ea4ca90050eff4ac320a2b784eca6280e6b571e4d1d1e20cf7a1f`.
Its root-list SHA-256 is
`6febb893c07c53d8f83109f34880615862bcbdbe2873a07c28f3b53d86f7ce34`.
Generated build outputs are retained locally and are not committed.

Tool pins are Lean 4.33.1, Rust 1.94.0, lean4export
`411dce7db58a3afc60ecab2d211acd1042b593dc`, and NanoDA
`05055695879dfebb6628a67da88ceca6cd6b0421`. Preparation selects the Lean pin
for the exporter and adds an empty Cargo workspace table to the downloaded
checker manifest; its Rust sources and dependency lock remain unchanged.
The build provenance records these changes and accepted results hash the
actual supplied executable/library identities.

## Bounds and remaining work

Linux calibration measured approximately 596 MiB peak resident memory for the
target helper: a 512 MiB Lean limit failed and 768 MiB passed. R3 therefore
uses 768 MiB while preserving the existing OS limits, 30-second deadlines,
64 KiB raw-output/export bound and all proof/axiom checks. Lean integration CI
optimizes host installation hashing; the Rust workspace job also exercises
the default optimization level.

The project, imports and elaborators are trusted host code. Process supervision
does not provide filesystem/network isolation. The host, source capture,
target binding and exporter remain trusted; identity hashes are not signatures.
The first profile supports explicitly listed local modules and the pinned
standard library, with at most 64 modules and 16 MiB total source.

Larger exports and project dependency discovery, editor/REPL integration,
broader proof automation, asynchronous source jobs for these tasks, and the
complete frozen R3/R5 evaluation remain future work. Independent proof validity
does not establish statement faithfulness, usefulness or novelty.

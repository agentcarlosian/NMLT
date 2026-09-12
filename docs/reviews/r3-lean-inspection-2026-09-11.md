# R3 fifth increment: bound inspection and native diagnostics

Date: 2026-09-11. Scope: [RFC 0035](../../rfcs/0035-bound-lean-inspection-and-diagnostics.md).
This is an automated implementation and local validation record. Independent
cross-family/human review and RFC acceptance remain open; R3 is not complete.

## Implemented behavior

`lean-task inspect` rebuilds a selected saved task and reports a bounded prefix
query over its imported Lean declarations. Results include originating modules,
types, universe parameters and transitive axioms, with an annotation showing
whether those axioms fit the selected policy. The output is context only with
`assurance: none`, and cannot be submitted to `recheck` as proof acceptance.
The inspection bundle retains its task, task hash, exact CLI, source copies and
readable context for another inspection without the original directories.

R3 compilation now retains native Lean JSON messages in `diagnostics-N.json`
beside raw stage evidence. Valid locations for the captured file are mapped from
Lean's Unicode scalar columns to UTF-8 byte ranges; foreign, synthetic and invalid
ranges stay unmapped. Reports bind their source text by SHA-256. Warnings stay
distinct from errors and from proof acceptance. Helper metadata is extracted
uniquely from native information messages.

Six new Rust tests pass for bounded queries, context/axiom annotations, Unicode
and CRLF mapping, foreign/synthetic locations, warning/program-output handling,
and wrapped metadata uniqueness. CLI Clippy passes with warnings denied.

The focused integration run passed five inspections, one repaired proof, one
fresh proof recheck and eight rejection controls. It found seven `Example.`
declarations, identified `Example.offset_eq` with an empty axiom set, and marked
draft and transitive draft declarations as depending on `sorryAx`. Lookup
reported truncation, an empty prefix result, a toolchain lemma and reconstructed
saved context. A wrong candidate produced a located diagnostic; the inspected
lemma then produced a 47-declaration NanoDA-checked proof. An actual error after
`λ😀` mapped to the exact `exact msg` bytes.

The rejection controls cover the wrong proof, using context as proof acceptance,
a changed task pin, a command-like prefix, an excessive limit, altered source
bytes, rehashed false target metadata and the Unicode source error. Focused
evidence is `target/r3-lean-inspection/run-bamjmxg3`, with log
`work/r3-inspection-controls-final.log`.

## Complete local validation

Full native Windows `make reproduce` passed from 17:18:20Z to 17:32:43Z on
2026-09-11 (exit 0), using Rust dev/test optimization level 1 with debug
assertions enabled. It passed 374 Rust tests, 14 Python harness tests, formatting,
Clippy, Lean metatheory and axiom checks, independent NanoDA checking of 9,004
declarations from 2,505 roots, canonical artifacts and finite comparisons,
30 execution rejection controls, all nine R0 tasks, real R2 workflows and
recovery, 15 invariant rejection controls, and all five R3 integration suites.

| R3 suite | Accepted proofs | Fresh rechecks | Rejection controls |
|---|---:|---:|---:|
| Explicit modules | 4 | 1 | 9 |
| Source import discovery | 1 | 1 | 14 |
| Bounded file exports | 1 | 1 | 11 |
| Proof dependency graphs | 1 | 1 | 13 |
| Declaration inspection and diagnostics | 1 | 1 | 8 |

The final suite also passed all five inspections. Reinspection reproduced the
context while both the original project and bind directory were unavailable,
using only the inspection bundle's retained task and executable. Those source
directories were restored afterward. The copied task, task hash and executable
bytes were checked against the inspection record. Both inspection records
remain context only with assurance none and contain no accepted proof result.

The repaired proof and fresh recheck each passed independent checking of 47
declarations and reproduced the same export digest. Their retained executable
hashes match the bound task. The wrong proof's native diagnostic selects `(n)`;
the actual Unicode source error selects `exact msg`. Both ranges were checked
against the captured source bytes and source hashes. The dependency suite also
repeated its reference comparison with Lean for all 397 declarations.

Local Linux cross-compilation passed with
`cargo check --locked --workspace --all-targets --target x86_64-unknown-linux-gnu
--target-dir target/r3-file-linux-check`. This is compile evidence only. Linux
runtime testing was unavailable because local WSL was not working, and no
remote CI run was used. Final public-link/trusted-component and whitespace
checks passed after the documentation updates.

## Source and retained evidence

The gate tested local changes on `codex/r3-lean-inspection`, based on local
dependency-graph checkpoint `15560f09441fc75d8a87344edc5246e14703b2ab`.
The base commit alone does not identify the new code. Before the run,
`work/r3-lean-inspection-source-manifest.json` captured 220 non-Markdown source,
test, fixture and build-input files under `crates`, `tools`, `examples` and the
root Cargo/toolchain/Makefile inputs. All 220 hashes still matched after the
gate. The source-manifest SHA-256 is
`3d76de58d3059564257772a557f459466819996de20fdf0f2b0e50a8892507f8`.
The Lean metatheory remained at the base commit. Documentation and the trusted
component inventory are recorded separately from this source manifest.

The full native log is `work/r3-lean-inspection-reproduce.log`; Linux compile
evidence is `work/r3-lean-inspection-linux-check.log`. The final artifact audit
is `work/r3-lean-inspection-final-audit.json`. Here `work` is the workspace
support directory beside the checkout. Final R3 evidence inside the checkout is:

- `target/r3-lean-tasks/run-9lkcq_hc`;
- `target/r3-lean-imports/run-md0it14o`;
- `target/r3-lean-exports/run-ja4cympi`;
- `target/r3-lean-dependencies/run-ok5msq23`;
- `target/r3-lean-inspection/run-v4oe9f4_`;
- `target/r3-checkers/run.nISakw/tools` for the freshly prepared checkers.

The inspection task SHA-256 is
`2fa5e8ddf70cdcb48e1fc86fdbed234f444ca530fa8204a8ed110d8f6bdd2f58`.
Its repaired proof and fresh recheck both export 47 declarations with SHA-256
`566e99103f9566ed20d1af9353eaa5f5bd8447e7b403825b2fc6400ea0cf96ba`.
The retained CLI SHA-256 is
`c7ce5e695037c9919d27a1f2953a05c0efa46f9b06705b74114adbdece23340e`.

Metatheory artifacts are in `work/nanoda-evidence/run.N9RsMv`. Their unchanged
export SHA-256 is
`9e5ef4a4796ea4ca90050eff4ac320a2b784eca6280e6b571e4d1d1e20cf7a1f`;
the root-list SHA-256 is
`6febb893c07c53d8f83109f34880615862bcbdbe2873a07c28f3b53d86f7ce34`.
Generated checker/build evidence remains local and outside tracked source.

Tool pins remain Lean 4.33.1, Rust 1.94.0, lean4export
`411dce7db58a3afc60ecab2d211acd1042b593dc` and NanoDA
`05055695879dfebb6628a67da88ceca6cd6b0421`. The preparation script records its
Lean selection and empty Cargo workspace table in the downloaded NanoDA
manifest; checker Rust sources and the dependency lock are unchanged.

## CLI, REPL and LeanInteract comparison

The optional [comparison driver](../../tools/compare_lean_interfaces.py) ran the
same four fixed cases through the normal Lean CLI, the community REPL and
LeanInteract's `run_dict`, then repeated them through a retained LeanInteract
environment. All 16 observations agreed:

| Case | CLI | Direct REPL | LeanInteract | Retained environment |
|---|---|---|---|---|
| Lookup `Example.offset_eq` | Context | Context | Context | Context |
| Wrong candidate | Rejected | Rejected | Rejected | Rejected |
| Valid candidate and axiom report | Elaborated, empty reported axioms | Same | Same | Same |
| `sorry` candidate and axiom report | Admitted | Admitted | Admitted | Admitted |

These are frontend observations. The comparison does not run NanoDA or produce
an NMLT proof acceptance record. The separate integration test performs that
independent check. A successful process, a REPL environment identifier or an
admitted declaration cannot replace the final task/proof contract.

The native Windows run is retained at
`target/r3-interface-comparison-20260911-171639`, with log
`work/r3-interface-comparison-final.log`. It stores individual replies, exact
executable/script/package-source hashes and the isolated Python environment's
package list.
The four fresh CLI invocations and four direct REPL commands were compared with
both fresh and retained-environment LeanInteract calls. Timings are recorded as
local observations only; this is not a performance benchmark or a containment
equivalence claim.

The final comparison script SHA-256 is
`939f9d9a0fe047140fa0055d752aa2c3984a71526b61a286aed4336cf9d25759`.
The REPL executable SHA-256 is
`d1e5740c47730682179d1a522d0b54386d121d0ef1c25c8279578d7680a7e650`.
All eight installed LeanInteract Python source files matched the pinned source
checkout. Their canonical source-manifest SHA-256 (sorted compact JSON) is
`4014313285af0110b16917a1a9abea571932a79529122733a1688d8b98e1e33a`.

LeanInteract 0.11.5 was installed into a workspace-local virtual environment
from [commit 976edd7](https://github.com/augustepoiroux/LeanInteract/tree/976edd7d38a99e1ea4c2dfabeb8ad98baffca3c8).
The [community REPL v4.33.0 source](https://github.com/leanprover-community/repl/tree/bbeedf38e0898869fc3b7c009e1ea877b46204e4)
was rebuilt at commit `bbeedf38e0898869fc3b7c009e1ea877b46204e4`, changing only
its `lean-toolchain` to the repository's `leanprover/lean4:v4.33.1` pin.
Neither inspected repository supplied a matching v4.33.1 tag in this check.
This is a tested local build overlay and custom REPL configuration, not a claim
that LeanInteract's default setup or typed command API supports that exact pin.
The comparison supplies the direct pinned Lake executable and Lean DLL search
path. No system settings or repository Lean pin were changed.

Reproduce the comparison in an isolated Python environment containing that
LeanInteract version, after building the specified REPL source with the stated
toolchain overlay:

```bash
python tools/compare_lean_interfaces.py \
  --lean-bin /absolute/path/to/lean-4.33.1/bin/lean \
  --repl-project /absolute/path/to/prepared-repl \
  --output target/new-interface-comparison
```

Use the corresponding `.exe` executable on Windows. The output directory must
be new. The normal `make reproduce` gate uses the native inspection integration
and does not require this optional Python/REPL installation.

## Decision and remaining scope

Keep the native CLI as the implemented bound-task interface: it already has
source reconstruction, full tool identities, bounded subprocesses and final
independent checking. Retained REPL environments provide useful interactive
state and are a candidate for later integration. They still need a defined
session identity, invalidation, cancellation, containment and recovery contract
before participating in accepted project-proof workflows. LeanInteract's wrapper
and the standalone REPL do not inherit the NMLT supervisor's policy from this
experiment.

The project, Lean environment/axiom reporting, diagnostics, binding helpers,
host and loader remain trusted. Inspection can execute trusted project
metaprograms and is not filesystem/network isolation. Types and axiom annotations
are retrieval context, not human approval or independently checked proof.

Broader package/library support, editor/LSP integration, a persistent REPL
adapter, proof automation, asynchronous project-proof jobs and the full
evaluation corpus remain R3 work.

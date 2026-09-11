# R3 second increment: local source import discovery

Date: 2026-09-11. Scope: the executable-only local Lean project profile under
[RFC 0032](../../rfcs/0032-lean-source-import-discovery.md).
This is an automated implementation and validation record. Independent
cross-family/human review and RFC acceptance remain open; R3 is not complete.

## Implemented behavior

A v2 project manifest selects source roots instead of listing modules by hand.
Binding uses the pinned Lean/Lake header parser, discovers the static local
import closure, and captures source bytes in dependency order. Selected roots
can include a source directory and ordinary vendored library directories.
Each task retains the original configuration, native import headers and module
origins; `source-imports.json` provides a separate readable report.

Proving and fresh rechecking regenerate the headers from retained source copies,
check the graph and toolchain imports, rebuild the source modules, and repeat
the existing target, transitive axiom and independent NanoDA checks. Source
imports describe the build environment; actual proof references and exported
declarations remain separate. Unrelated source files do not join the closure.

New task/result artifacts use version 2. Explicit-module v1 project manifests
remain supported, and old saved artifacts require their retained original CLI.
The candidate format, proof grammar, axiom policy and process/export bounds are
unchanged. JSON publication also respects the existing 64 MiB input bound.

## Executed validation

- Full native Windows `make reproduce` passed at
  `8aeecd467d52d3ec7cb0e5a9065393e833615cd6`, from 14:01:36Z to 14:10:43Z on
  2026-09-11 (exit 0). The run used Rust dev/test optimization level 1 with
  debug assertions enabled. It passed 353 Rust tests, 14 Python harness tests,
  Clippy, Lean metatheory and axioms, independent NanoDA checking of 9,004
  declarations from 2,505 roots, the canonical artifacts and finite comparisons,
  30 execution rejection controls, all nine R0 tasks, real R2 workflows and
  recovery, 15 invariant rejection controls, and both R3 integration scripts.
- The [complete Linux pull-request CI run](https://github.com/agentcarlosian/NMLT/actions/runs/34607770349)
  passed at the same commit: 354 Rust tests, 14 Python tests and all three jobs.
  The differing Rust count reflects platform-specific tests. The
  [Lean project job](https://github.com/agentcarlosian/NMLT/actions/runs/34607770349/job/103290342989)
  built the pinned independent tools and passed both R3 scripts.
- The original explicit-module profile passed all four proof cases, one fresh
  recheck and nine rejection controls with the new task/result formats.
- The discovery profile passed a 49-declaration NanoDA-checked proof across
  three source modules, plus a retained-executable fresh recheck while the
  working project was unavailable. The fresh export digest matched exactly.
  All 14 discovery rejection controls passed on both platforms; the linked-root
  case used a real Windows junction and a Linux directory symlink.
- `git apply --check` accepted the generated patch against the captured example
  project. Public-surface and whitespace checks passed.

The discovery fixture includes source and vendor roots, nested comments that
contain misleading import text, public/meta imports and a malformed unrelated
module created by the test. The captured order is exactly `Support.Core`,
`App.Definitions`, `App.Goals`; only `Init` is needed from the pinned toolchain.

The 14 rejected cases are changed dependency bytes with an old task pin;
rehashed false header metadata; an unreachable added source; reordered imports;
a missing import; a cached `.olean` without its source; a cycle; ambiguous
roots; a canonical source-path alias; toolchain shadowing; a header-parser error
reported with process exit zero; root traversal; module casing mismatch; and
a linked source root. These are finite regression cases, not a universal
reliability claim or the proposed held-out R5 evaluation corpus.

## Retained evidence

The full native log is `work/r3-import-reproduce.log`. Discovery evidence is
`target/r3-lean-imports/run-0orcow3d`; the explicit-module cases are in
`target/r3-lean-tasks/run-fw116xuv`. Prepared checker sources and binaries are
retained under `target/r3-checkers/run.fvrGLB/tools`. The Linux discovery
directory in the successful job is `target/r3-lean-imports/run-8fns955d`.

The native discovered task has SHA-256
`e2198f7bef04d59f93296fac2f14c0fe8ee8a2cf52975d3f09407cafe112ee1a`.
Its proof and fresh recheck both export SHA-256
`7e627e94e90225a44ec414523525ce11046465b32e50fcd0f7e77598a7a86442`.
The retained CLI SHA-256 is
`3b015ad0e3a52e12d82ed03533e592d78bd3a43b2540eeed331227dcd097b2c1`.

Metatheory checker artifacts are in `work/nanoda-evidence/run.wK6XDn`. Their
unchanged export SHA-256 is
`9e5ef4a4796ea4ca90050eff4ac320a2b784eca6280e6b571e4d1d1e20cf7a1f`,
and the root-list SHA-256 is
`6febb893c07c53d8f83109f34880615862bcbdbe2873a07c28f3b53d86f7ce34`.
Generated build outputs are retained locally and are not committed.

Tool pins remain Lean 4.33.1, Rust 1.94.0, lean4export
`411dce7db58a3afc60ecab2d211acd1042b593dc` and NanoDA
`05055695879dfebb6628a67da88ceca6cd6b0421`. The preparation script records its
Lean selection and empty Cargo workspace table in the downloaded checker
manifest; the checker's Rust sources and dependency lock are unchanged.

## Remaining scope

Discovery covers static header imports inside explicitly selected project roots.
Dynamic access by trusted project metaprograms remains a host trust boundary.
Source hashes are identities, not authenticated provenance or package-revision
attestations. The project, parser, host, source/target binding and exporter remain
trusted; process supervision does not provide filesystem/network isolation.

The profile still has the 64-module/16-MiB source bounds, 64-KiB export limit,
closed proof-term grammar and original proof acceptance policy. Package
resolution and larger exports, broader automation, LeanInteract/REPL comparison,
editor integration and asynchronous project-proof jobs remain later R3 work.

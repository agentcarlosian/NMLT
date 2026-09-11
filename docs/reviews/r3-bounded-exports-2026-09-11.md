# R3 third increment: bounded file proof exports

Date: 2026-09-11. Scope: the executable-only local Lean project profile under
[RFC 0033](../../rfcs/0033-bounded-lean-proof-exports.md).
This is an automated implementation and local validation record. Independent
cross-family/human review and RFC acceptance remain open; R3 is not complete.

## Implemented behavior

Proof export stdout streams to a newly created file with a 16 MiB bound.
The separate file process policy preserves 64 KiB stderr and line-assembly
bounds, explicit environments, process deadlines, resource limits and tree
cleanup. Ordinary process/R2 policy validation remains unchanged.

The capture records exact accepted byte counts and SHA-256, requires successful
EOF flushing and final file synchronization, and rejects sink errors or overflow
even if the underlying process library reports a successful child exit. Failed
captures may retain a bounded partial file but cannot produce a receipt or an
accepted proof result. Successful captures still preserve the child exit code.

R3 result version 3 stores the export file receipt and policy separately from
ordinary process output. The root byte count and digest must agree with the
single successful exporter stage. The saved export identity is compared before
inspection and again after independent NanoDA checking. Fresh rechecking repeats
the source build and proof checks and requires an identical fresh export.
Task version 2, candidate version 1 and project manifest versions 1/2 remain
supported; old artifacts require their retained original CLI executable.

## Executed validation

- Full native Windows `make reproduce` passed from 15:15:30Z to 15:24:27Z
  on 2026-09-11 (exit 0). It used Rust dev/test optimization level 1 with debug
  assertions enabled. The run passed 360 Rust tests, 14 Python harness tests,
  formatting, Clippy, Lean metatheory/axioms, independent NanoDA checking of
  9,004 declarations from 2,505 roots, canonical artifacts and finite comparisons,
  30 execution rejection controls, all nine R0 tasks, the real R2 workflows and
  recovery, 15 invariant rejection controls and all three R3 integration scripts.
- Seven added Rust tests cover the file sink and runtime integration: exact
  binary/CRLF bytes, the 16 MiB boundary and overflow, stderr overflow, a
  newline-free payload beyond the line buffer, nonzero exit, destination
  preservation, missing EOF flush, a real failed write/flush, and timeout,
  cancellation, drop and root-exit descendant cleanup. Existing ordinary
  process fixtures also pass with the shared supervisor refactor.
- The original explicit-module R3 suite passed four proof cases, one fresh
  recheck and nine rejection controls using the new result format. The source
  discovery suite passed its 49-declaration proof, retained-executable fresh
  recheck and all 14 rejection controls, including a real Windows junction.
- The new multiplication-associativity proof passed with 97 declarations and
  a 136,729-byte export, exceeding the old 64 KiB limit. Its manifest explicitly
  permits the required `propext`. Fresh rechecking used the retained CLI while
  the original working project was unavailable and produced identical bytes.
  All 11 new rejection controls passed.
- Local Linux cross-compilation passed with
  `cargo check --locked --workspace --all-targets --target x86_64-unknown-linux-gnu
  --target-dir target/r3-file-linux-check`. This is compile evidence only.
  Linux runtime validation was unavailable locally because WSL was not working;
  no remote CI run was used for this increment.
- Final public-link/trusted-component and whitespace checks passed after the
  documentation updates.

The 11 export rejection controls cover a changed receipt hash; inconsistent
receipt size; coherently changed receipt/root sizes rejected by fresh checking;
changed capture policy; changed output path; duplicate and missing receipts;
failed exporter status; an old result version; the same proof under an explicitly
stricter axiom policy; and an invalid larger proof rejected directly by NanoDA.
These are finite regression cases, not the complete held-out R5 corpus or a
universal reliability claim.

## Source and retained evidence

This run tested local changes based on merged main
`7f7b2107ae3bba2a9bb660947854b28e86a09a7f`, on local branch
`codex/r3-bounded-exports`. The base commit alone does not identify the new code.
Before the gate, `work/r3-bounded-export-source-manifest.json` captured hashes
of 211 non-Markdown source, test, fixture and build-input files under
`crates`, `tools`, `examples` and the root Cargo/toolchain/Makefile inputs.
All 211 hashes still matched after the run. Other source files, including the
Lean metatheory, remain at the base commit. Documentation and trust-inventory
updates were finished separately. The source-manifest SHA-256 is
`2b917a7843779fde2c97d1359cd59640efee4e54a432af0a5e38f2add641f1a2`.

The full native log is `work/r3-bounded-exports-reproduce.log`; the cross-target
compile log is `work/r3-bounded-exports-linux-check.log`. Here `work` is the
workspace support directory beside the checkout. Integration evidence inside
the checkout is:

- `target/r3-lean-tasks/run-5wndq7d2` for the original profile;
- `target/r3-lean-imports/run-8a_6m0p_` for source discovery;
- `target/r3-lean-exports/run-y_d6m2y6` for larger exports;
- `target/r3-checkers/run.x80WTc/tools` for the independently prepared tools.

The larger-export task SHA-256 is
`055a08e62477d77a532af95cfe0fc61ba1f1adc9384827e94a1aed54c8ab855d`.
Its proof and fresh recheck both export 136,729 bytes with SHA-256
`fc64efd4b3461648bed02d84f8a447e89a27436f9f23d24f16c203dfa791454d`.
The retained CLI SHA-256 is
`a1c20b5299ef6973a9fcfe6fc6f50a07ac3798562b5668a313a4a9a818d18415`.

Metatheory checker artifacts are in `work/nanoda-evidence/run.zS4UxI`.
The unchanged export SHA-256 is
`9e5ef4a4796ea4ca90050eff4ac320a2b784eca6280e6b571e4d1d1e20cf7a1f`;
the root-list SHA-256 is
`6febb893c07c53d8f83109f34880615862bcbdbe2873a07c28f3b53d86f7ce34`.
Generated build and checker evidence is retained locally, outside tracked source.

Tool pins remain Lean 4.33.1, Rust 1.94.0, lean4export
`411dce7db58a3afc60ecab2d211acd1042b593dc` and NanoDA
`05055695879dfebb6628a67da88ceca6cd6b0421`. The preparation script records the
Lean selection and empty Cargo workspace table added to the downloaded checker
manifest; the checker's Rust sources and dependency lock are unchanged.

## Remaining scope

The exporter, source/target binding, trusted project metaprograms, tools and
host remain trust boundaries. Capture hashes identify bytes and do not establish
authenticated provenance, human approval or statement faithfulness. Process
supervision does not provide filesystem/network isolation. The bounded parser
still loads the export into memory.

This fixture demonstrates a real closure above the old limit and runtime
transport up to the new ceiling; it does not establish Mathlib-scale package
support. Source/module and candidate grammar bounds remain explicit. Package
resolution, broader library validation and proof automation, LeanInteract/REPL
comparison, editor integration and asynchronous project-proof jobs remain R3
work. Linux runtime validation of this increment is also still open.

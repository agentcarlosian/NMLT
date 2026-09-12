# R3 fourth increment: actual proof dependency graphs

Date: 2026-09-11. Scope: the executable-only local Lean project profile under
[RFC 0034](../../rfcs/0034-lean-proof-dependency-graphs.md).
This is an automated implementation and local validation record. Independent
cross-family/human review and RFC acceptance remain open; R3 is not complete.

## Implemented behavior

The CLI derives a complete declaration graph from the exact bounded export sent
to NanoDA. Each node distinguishes explicit constants in types and stored values,
recursor-rule references, projection type names and literal support. Inductive
blocks and the primitive quotient package form separate export groups.

The graph's node sequence and export digest match the checked artifact. Its root
and target-binding explicit references are also compared with Lean metadata.
Result version 4 embeds the graph and retains JSON and linked Markdown views.
Fresh rechecking rebuilds saved sources, repeats independent checking and
requires the entire graph to match before publishing another accepted result.
Task version 2, candidate version 1 and both project manifest versions remain
unchanged; old records require the original retained CLI executable.

Graph traversal follows a shared expression DAG without recursive traversal or
tree expansion. Declaration, reference, name-byte, traversal-work and artifact
size bounds reject excessive graphs. Readable reports use generated numeric
anchors and escaped declaration labels. Draft status cannot be inserted into
the accepted graph, and unused draft declarations remain outside the closure.

## Focused validation

Eight new Rust tests pass, covering the expression forms, distinct reference
categories, inductive/quotient groups, reduction cycles, malformed references,
10,000 levels of shared expressions, a dense graph over the reference bound,
and escaped report labels. CLI Clippy passes with warnings denied.

The real fixture has a structure projection, mutually defined tree/forest,
an opaque definition, numeric and string literals, and quotients. It produces
a 587,108-byte export with 397 NanoDA-checked declarations under the empty-axiom
policy. A separate Lean helper compares all 397 nodes' explicit type/value,
recursor reduction and projection references with the actual Lean environment.
Known literal-support sets and mutual/quotient groups are also checked.

The comparison exposed an important distinction during development: Lean's
`Expr.getUsedConstants` follows a projection argument but omits its stored type
name. Projection references are now separate, and the complete comparison
passes. The helper enumerates actual environment names, including private
numeric name components, rather than reconstructing names from display strings.

The focused integration run passed a retained-executable fresh recheck with the
original project unavailable, reproduced both graph files byte for byte, and
passed all 13 rejection controls. Those controls change the export binding,
root/interior references, reduction references, projection references, literal
support, declaration kind and export groups; add a missing endpoint or planning
field; remove the graph; use an old result version; or prove the goal from its
draft body. Failed cases publish neither an accepted result nor a graph view.

Focused evidence is `target/r3-lean-dependencies/run-iuqtjvj8`, with log
`work/r3-dependency-controls-final.log`.

## Complete local validation

Full native Windows `make reproduce` passed from 15:54:18Z to 16:04:25Z on
2026-09-11 (exit 0), using Rust dev/test optimization level 1 with debug
assertions enabled. It passed 368 Rust tests, 14 Python harness tests, formatting,
Clippy, Lean metatheory and axiom checks, independent NanoDA checking of 9,004
declarations from 2,505 roots, canonical artifacts and finite comparisons,
30 execution rejection controls, all nine R0 tasks, real R2 workflows and
recovery, 15 invariant rejection controls, and all four R3 integration suites.

| R3 suite | Accepted proofs | Fresh rechecks | Rejection controls |
|---|---:|---:|---:|
| Explicit modules | 4 | 1 | 9 |
| Source import discovery | 1 | 1 | 14 |
| Bounded file exports | 1 | 1 | 11 |
| Proof dependency graphs | 1 | 1 | 13 |

The final dependency fixture again passed the Lean comparison for all 397
nodes. Its fresh recheck reproduced the embedded graph and both report files
byte for byte. The graph JSON is 166,071 bytes; the readable report is 199,805
bytes with 397 unique declaration anchors and 3,370 valid internal links.
The retained CLI hash was checked against the executable bytes.

Local Linux cross-compilation also passed with
`cargo check --locked --workspace --all-targets --target x86_64-unknown-linux-gnu
--target-dir target/r3-file-linux-check`. This is compile evidence only. Linux
runtime testing was unavailable because local WSL was not working, and no
remote CI run was used. Final public-link/trusted-component and whitespace
checks passed after the documentation updates.

## Source and retained evidence

The gate tested local changes on `codex/r3-proof-dependencies`, based on local
bounded-export checkpoint `eb645d002eb250855b8b570e5581942faa80fc90`.
The base commit alone does not identify the new code. Before the run,
`work/r3-proof-dependencies-source-manifest.json` captured 216 non-Markdown
source, test, fixture and build-input files under `crates`, `tools`, `examples`
and the root Cargo/toolchain/Makefile inputs. All 216 hashes still matched after
the gate. The source-manifest SHA-256 is
`f9e685f1ec1a216de06b71681f07538d2877d7037ec81a0456ca7d006c175ffd`.
Other source, including the Lean metatheory, remains at the base commit.
Documentation was finalized separately.

The full native log is `work/r3-proof-dependencies-reproduce.log`; Linux compile
evidence is `work/r3-proof-dependencies-linux-check.log`. Here `work` is the
workspace support directory beside the checkout. Final R3 evidence inside
the checkout is:

- `target/r3-lean-tasks/run-v7g8quuf`;
- `target/r3-lean-imports/run-thjyeacz`;
- `target/r3-lean-exports/run-s7bieql_`;
- `target/r3-lean-dependencies/run-zqxyuunv`;
- `target/r3-checkers/run.Rcd56b/tools` for the freshly prepared checkers.

The dependency task SHA-256 is
`7a67f8d44d4fbac38ef1ecc0320f9631e3ae79fd0e60d9e3eaf21d8ad405a1ea`.
Its proof and fresh recheck both export 587,108 bytes with SHA-256
`9d1fd94cb869e3fb0c61567ab646a086322d2a1d071f30cb42debaf56849cb6b`.
The graph JSON SHA-256 is
`727ad3ad2a03dcd4a0660d3cb48dbf09f7b9a52f61f23e52f4c9f094cf1d7af5`.
The retained CLI SHA-256 is
`aacf098584b7af124c0ec83789abecb0a7e4a8edce6b8a6c4867ac3a4d105add`.

Metatheory artifacts are in `work/nanoda-evidence/run.o1A67u`. Their unchanged
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

## Remaining scope

This is a graph of exported syntax, including redundant information the kernel
can rederive or ignore. It is not an independently proved dependency graph,
minimal dependency claim or draft scheduling DAG. Recursive reductions and
mutual blocks can have cycles. Sidecar reports are derived views; recheck reads
the embedded graph and regenerates them.
Declaration labels retain the existing export-label convention; general
round-tripping of escaped Lean identifiers is outside this profile. Ambiguous
labels and binding-metadata mismatches reject acceptance.

The graph parser/renderer, exporter, source and target binding, tools, trusted
project metaprograms and host remain named trust boundaries. Hashes identify
bytes and do not attest to human approval or statement faithfulness. Process
supervision does not provide filesystem/network isolation.

Broader package/library validation, proof automation, LeanInteract/REPL
comparison, editor integration and asynchronous project-proof jobs remain R3
work. The complete evaluation corpus and independent human review remain open.

# RFC 0033: Bounded Lean proof exports

- Status: Under review
- Date: 2026-09-11
- Milestone: R3, third implementation increment

## Problem and scope

RFCs 0031 and 0032 capture independent proof exports through the ordinary
64 KiB process-output interface. Even a small multiplication-associativity
proof can exceed that bound because the independent checker needs its
transitive declaration closure. Add a separate bounded file-capture interface
and use it for `lean4export` stdout.

This increment raises the proof-export bound to 16 MiB. It preserves ordinary
process policy validation and the existing R2 capture contract. It does not
establish library-scale support or change the target, candidate or axiom policy.

## Process and capture contract

`FileProcess` shares the existing process-tree supervisor and records a distinct
`nmlt-contained-file-process-v1` contract through `FilePolicy`. It permits at
most 16 MiB of raw stdout to a newly created file, 64 KiB of raw stderr, and
64 KiB of decoded line assembly. The file destination must not already exist.
The existing explicit environment, maximum 30-second deadline, platform-specific
OS resource limits and descendant cleanup still apply. Filesystem and network
isolation are outside this contract.

The pinned ProcessKit 3.3.4 raw stdout tee streams bytes in order before text
decoding. File capture uses the draining path so stdout is not retained again
as a process result. An incremental SHA-256 digest and byte count cover accepted
write bytes. The file is flushed at EOF, synchronized before a receipt is
returned, and checked for the expected length. The retained process result has
empty stdout, bounded raw stderr and the observed exit code.

ProcessKit can disable a failing raw tee without failing the child result.
NMLT therefore tracks sink errors and overflow explicitly, requests process
cancellation on failure, and requires successful EOF flushing before producing
a receipt. Timeout, cancellation, output overflow, failed writes/flushes and
failed final synchronization prevent a receipt. A bounded partial file can
remain as diagnostic evidence. A complete capture can accompany a nonzero
child exit; callers must separately classify command success.

The ordinary `Process` interface and its policy validator continue to require
the original contract. A file policy cannot validate as an ordinary R2 process
policy. Increasing the export bound does not relax header, compilation, checker
diagnostic or R2 worker output limits.

## Proof acceptance and artifacts

The exporter writes `build/environment.ndjson` in the fresh result directory.
Its stage retains `stdout_file` with a fixed relative path, `FilePolicy`, byte
count and SHA-256 receipt. Acceptance requires exactly one such stage, named
`lean4export`, with exit zero, empty stderr and empty in-memory stdout. Ordinary
stages have no file capture. The export must be nonempty.

NMLT reads the file with the 16 MiB bound and compares its bytes and digest with
the capture receipt before validating the exported root, target type,
declaration closure and axioms. NanoDA must then check exactly the expected
nonzero declaration count with exit zero and empty stderr. A final file
identity check must match the original byte count and digest before publication.
Existing full Lean installation and tool identity checks still apply.

New result artifacts use `nmlt-lean-result-v3`, including root `export_bytes`
and `export_sha256` fields consistent with the export stage. Task version 2,
candidate version 1 and project manifest versions 1 and 2 remain unchanged.
Old artifacts require their retained original CLI executable; changing result
formats does not upgrade old acceptance records.

Fresh rechecking validates the retained file metadata, rebuilds the saved
sources with the exact implementation/tools, repeats proof checking and creates
a fresh file export. Its identity must match the old result. It can run while
the original working project is unavailable. Old subprocess diagnostics and
file receipts do not replace that fresh reconstruction.

## Bounds and trust

File transport streams stdout, but the declaration parser still reads at most
16 MiB into memory and retains its existing row limits. This is not a claim of
constant-space proof checking. Source bounds remain 64 modules, 1 MiB per
module and 16 MiB total; discovery permits at most 16 roots. The 4 KiB closed
proof-term grammar and selected subset of the permitted axioms are unchanged.

The project, Lean build, host, loader, source/target binding and exporter remain
trusted. Concurrent filesystem stability remains a host assumption. A capture
receipt identifies bytes; NanoDA's independent checking establishes validity
of the exported declarations under the selected axioms. Neither mechanism
records human task approval or establishes statement faithfulness, novelty or
usefulness.

## Validation and remaining work

Runtime cases cover exact binary/CRLF bytes, stdout beyond 64 KiB, the exact
16 MiB limit and overflow, stderr overflow, destination preservation, nonzero
exit, timeout, cancellation, drop and descendant cleanup. A newline-free payload
beyond the decoded line bound must retain all raw bytes. Separate sink cases
require EOF flushing and reject a real failed file write/flush.

The integration fixture proves multiplication associativity with a
136,729-byte export and 97 declarations, including explicitly permitted
`propext`. It must pass independent NanoDA checking and fresh rechecking from
the retained executable without the original project. Rejection controls cover
inconsistent or coherently false byte counts, changed hashes, paths and policies,
duplicate/missing receipts, failed exporter status, old result format and a
stricter axiom policy. NanoDA must separately reject an invalid larger proof.

The [validation record](../docs/reviews/r3-bounded-exports-2026-09-11.md) identifies
the actual local runs and platform limits. Independent cross-family/human review
and RFC acceptance remain open. R3 still needs broader library/package
integration, editor/REPL comparison, proof automation and asynchronous project
proof jobs, along with the complete evaluation corpus.

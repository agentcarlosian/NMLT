# RFC 0025: Local projects and dependency locks

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-07

## Purpose and disposition

R2 needs a reproducible project loop around its existing interpreter. This RFC
adds `nmlt.toml`, `nmlt.lock`, initialization, project checking, execution, tests,
replay, and a source formatter. These are executable-only facilities. They do
not reinterpret behavioral artifacts or promote workflow results to proofs.

The manifest selects a source entry, typed inputs, bounded execution, optional
local tools, and named examples with expected typed results. Tests execute the
same interpreter and adapters as ordinary runs. The initialized example accepts
real input, handles worker failure, reuses its successful output, and has tests
for both fallback and direct success. CLI input overrides require no JSON file
editing. Existing explicit source commands retain their contracts.

## Identity and compatibility

The lock binds the exact NMLT executable, imported local source dependencies,
and configured Lean installation. Main source and manifest inputs remain editable;
every run records their exact identities separately. Changed imports or tool
installations require an explicit `lock` update. The Lean installation identity
covers all files under its `bin` and `lib` directories, including relative link
identities, with bounded streaming hashes. Escaping links and directory links
are unsupported errors. External OS libraries remain residual host trust.

Project records bind the manifest, lock, selected entry and typed inputs, and
the underlying source execution record. Each run retains its complete source
snapshot and retains the exact executable once per identity. Replay validates these bindings and
delegates to the same source/session replay implementation, with no job dispatch.
The manifest/lock/entry/input digest is also carried in the underlying source
context and therefore in the job journal's parent context. Non-project source
records omit this optional metadata and retain their formats. Project intent is
flushed before dispatch so interrupted runs retain the original configuration.
Unknown versions, fields, duplicate keys, stale dependencies, malformed inputs,
changed execution records, and compatibility mismatches fail explicitly.

No lock authenticates an actively hostile filesystem or closes the OS race
between reading and execution. Stronger process isolation and source resumption
are separate remaining R2 requirements; this RFC does not satisfy them merely
by recording dependencies.

## Formatting and diagnostics

Formatting preserves every non-whitespace token, including exact string and
comment contents. The compiler validates the complete formatted import closure
before any file is replaced. Check mode performs no writes. Formatting imported
files makes their lock stale until an explicit update. Multi-file writes are
preflighted but are not claimed as one filesystem transaction.

CLI diagnostics share stable codes, messages, source byte spans, one-based
line/column coordinates, expected/actual values, and related locations. JSON is
an alternate rendering of the same diagnostic, rather than a separate checker.

## Validation

Exercise initialization through changed-input run, failure/fallback, result reuse,
test assertions, and replay on the real executable. Reject incompatible locks,
changed imports/tool files, manifest errors, changed embedded execution, invalid
expectations, and unsupported files. Verify formatter idempotence, comment and
string preservation, no-write check mode, and an invalid import preventing writes.
The complete repository gate remains required before R2 completion.

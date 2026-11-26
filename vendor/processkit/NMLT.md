# NMLT ProcessKit patch

This library snapshot is ProcessKit 3.3.4 (MIT), pinned by the upstream crate
SHA-256 and per-file before/after hashes in `NMLT-PROVENANCE.json`. The source
change adds `Command::windows_no_console`; the upstream `create_no_window`
behavior is unchanged. Cargo metadata names the local `3.3.4-nmlt.1` version
and omits non-library targets that are not vendored. `Cargo.toml.orig` retains
upstream metadata. No diagnostic logger or native helper is included.

The new builder selects DETACHED_PROCESS, which concerns console attachment,
through the existing managed suspended Job Object launch. It is unrelated to
ProcessKit's `spawn_detached`. Resource limits, assignment, teardown and Unix
implementation are unchanged. See RFC 0037 in the NMLT repository. Public NMLT
link validation excludes upstream vendored documentation; file integrity is
checked separately. Upstream tests/documentation outside this snapshot are not
claimed as locally run.

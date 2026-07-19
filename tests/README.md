# Integration tests

This directory holds cross-component fixtures. Crate-local unit, negative,
benchmark, and CLI integration tests remain beside their Rust crates.

`fixtures/malformed-unclosed.nmlt` is expected to fail structural parsing and
serves as a small repository-level diagnostic fixture. The frozen malformed
benchmark control and crate-level frontend tests exercise the same rejection
boundary in `make ci`.

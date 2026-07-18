# Integration tests

This directory holds cross-component fixtures and future end-to-end tests.
Crate-local unit and CLI integration tests remain beside their Rust crates.

`fixtures/malformed-unclosed.nmlt` is expected to fail structural parsing and
will be used by the repository-level diagnostic test suite.

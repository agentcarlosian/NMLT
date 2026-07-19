# nmlt-agent

`nmlt-agent` implements RFC 0011's authority boundary. An assistant receives a
candidate, declared editable spans, and structured checker feedback. It cannot
receive or replace trusted intent, property, oracle, result, or evidence
artifacts through the assistant interface.

The crate is dependency-free and includes:

- SHA-256-backed artifact and protected-span identities;
- exact repository-relative path and half-open byte-span policies;
- parse, type, counterexample, unknown, and conflict feedback types;
- localized proposals that cannot claim a checker result;
- property-linked semantic mutation descriptors;
- deterministic artifact-graph serialization and checked readback;
- a three-task held-out evaluation of a deterministic repair assistant.

The deterministic assistant is a **protocol-conformance baseline, not an LLM
evaluation**. Its rules consume only the public `AssistantInput`; no expected
patch exists in that type or in the benchmark corpus.

Run it independently while the crate is not yet a root-workspace member:

```sh
cargo test --manifest-path crates/nmlt-agent/Cargo.toml
cargo run --manifest-path crates/nmlt-agent/Cargo.toml --bin nmlt-agent-evaluate
```

## Research provenance

The 2026-07-18 CDT `search-the-archives` pass found adjacent agentic program
repair work but no close lexical match for the complete authority-bounded
protocol. Archive and newly consulted sources, search failures, and their
concrete design implications are recorded in
[`benchmarks/agentic/README.md`](../../benchmarks/agentic/README.md). The code
does not treat absence from a lexical archive search as novelty evidence.

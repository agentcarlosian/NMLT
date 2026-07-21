# Devpost Judge Instructions and Update

## Ready-to-paste copy

NMLT's current user is a formal-methods, verification, or agent-platform
developer who needs to inspect a finite workflow change before trusting it. The
fastest evaluation compares a valid provider workflow with a seeded defect.

Demo: https://www.youtube.com/watch?v=-PbhZ9me46Y

Repository: https://github.com/agentcarlosian/NMLT

Linux x86_64 release and bundled fixtures:
https://github.com/agentcarlosian/NMLT/releases/tag/build-week-submission-2026

Submission revision: `<FINAL_COMMIT_SHA>`

Release SHA-256: `<RELEASE_SHA256>`

For a source checkout, only the Rust toolchain pinned by
`rust-toolchain.toml` is needed for these two commands. Lean, TLC, Quint, P,
Node, and Python are not required for this judge path.

```bash
cargo run -p nmlt-cli -- model-check --json benchmarks/seeded-defects/provider-attempt/reference.nmlt
cargo run -p nmlt-cli -- model-check --json benchmarks/seeded-defects/provider-attempt/dispatch-before-authorize.nmlt
```

Expected bounded outcomes:

- The reference reports `result: "model_checked"`, `complete: true`, 9 explored
  states, 12 explored transitions, and four checked properties.
- The mutant reports `result: "refuted"`, `complete: true`, 2 explored states,
  2 explored transitions, and a `DispatchRequiresArm` witness whose `dispatch`
  step has `armed: false`.

`model_checked` means the stated properties held over every reachable state and
transition explored within the result's declared finite bounds. It is not an
unbounded proof or a claim that arbitrary NMLT source is verified. The
structural `evidence` command reports `unknown`.

GPT-5.6, running as Sol in the Codex CLI, accelerated implementation, recovery,
debugging, negative controls, and documentation across the repository's Rust,
Lean, Python, TLA+, Quint, and P work. The human author directed the
architecture, semantics, trust assumptions, and assurance boundaries. NMLT has
no runtime LLM dependency; these judge commands are deterministic local runs.

The repository's standard `make ci` gate does not imply that every optional
external checker ran. Lean is a separate CI job, TLC requires
`TLA2TOOLS_JAR`, and P runs only when P 3.1.0 is installed.


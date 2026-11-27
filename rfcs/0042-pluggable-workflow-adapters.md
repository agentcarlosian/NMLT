# RFC 0042: Pluggable Pinned Subprocess Adapters (`[[adapters]]`)

- Status: Under review
- Authors: NMLT maintainers
- Target: `crates/nmlt-workflow`, `crates/nmlt-runtime`, `crates/nmlt-cli`

## Summary

Generalize NMLT's asynchronous workflow job system beyond its hardcoded
`square` and `lean` adapters by allowing users to register arbitrary local
subprocess executables via `[[adapters]]` in `nmlt.toml` (or `--adapters FILE`),
pinning every executable by SHA-256 in `nmlt.lock` and session manifests, and
invoking them from NMLT workflow source via `job_start_adapter(alias, input) -> Job<Text>`.

## Motivation

`nmlt-runtime` already implements a generic, crash-recoverable, generation-pinned,
OS-contained job lifecycle (`Dispatch` -> `nmlt-job-response-v1`) paired with
`nmlt-workflow`'s compile-time affine handle checker (`Job<T>`). However, users
could previously only invoke hardcoded `job_start_square` and Lean adapters.
Adding pinned pluggable subprocess adapters unlocks NMLT's affine durability,
crash recovery, fallback branching, and deterministic replay for any external
tool (SMT solvers, linters, code generators, test runners, build verifiers, or
LLM workers) without modifying the NMLT compiler or runtime.

## Design

### 1. Workflow Surface (`nmlt-workflow`)

Add a single builtin expression:

```text
job_start_adapter(alias: Text, input: Text) -> Job<Text>
```

The returned `Job<Text>` handle participates identically in `nmlt-workflow`'s
affine type system: it must be collected (`job_collect`) or transferred on every
normal control-flow path, cannot be duplicated or stored inside lists/records,
and supports `job_poll` and `job_cancel`.

Functions calling `job_start_adapter` transitively set `requires_jobs`,
`requires_async_jobs`, and `requires_custom_jobs`. Serializing `custom_jobs`
with `skip_serializing_if` preserves identical `program_sha256` hashes for all
existing workflows that do not use custom adapters.

### 2. Runtime Adapter Registry & Protocol (`nmlt-runtime::custom_adapter`)

- Adapter identity: `Adapter { name: "custom-adapter", version: 1, input_type: Text, output_type: Text }`.
- Registry identity (`nmlt-custom-adapters-v1`): binds `process_contract` and
  up to 16 entries `{ alias, executable_sha256, args, timeout_ms }`.
- Request envelope (`nmlt-custom-adapter-request-v1`): canonical JSON passed as
  `Dispatch.input` (`Value::Text`) binding `{ schema, alias, executable_sha256, input }`.
- Wire protocol: the runtime invokes the pinned executable inside `process::Process`
  OS containment (`env_clear()`, bounded pipes, bounded timeout, process-group /
  Windows Job Object cleanup), writes the JSON `Dispatch` to `stdin`, and reads a
  canonical `nmlt-job-response-v1` JSON `Response` from `stdout`.
- Fail-closed checks:
  - Executable SHA-256 is verified on open, before every launch, and in `nmlt.lock`.
  - Unregistered aliases, tampered binaries, non-zero exit codes, non-empty `stderr`,
    mismatched `binding` (attempt/slot/generation/input hash), or non-canonical JSON
    responses are rejected and never silently promoted to `Completed`.

### 3. Project & CLI Integration (`nmlt-cli`)

Projects declare adapters in `nmlt.toml`:

```toml
[[adapters]]
alias = "formatter"
executable = "bin/my-adapter"
args = ["--mode", "check"]
timeout_ms = 5000
```

`nmlt lock` hashes every adapter executable into `nmlt.lock`. `nmlt run`,
`nmlt test`, `nmlt resume`, and `nmlt replay` verify the lock and pass
`--adapters` to the underlying async workflow runner. Standalone `nmlt run` and
`nmlt jobs-resume` also accept `--adapters <FILE>`.

## Security and Trust Boundary

- Custom adapters are trusted local executables explicitly chosen and locked by
  the user; NMLT does not claim formal soundness about what a custom adapter
  computes (`assurance: "none"`).
- OS containment (`ProcessKit` / job object / rlimit / clean environment),
  durable intent-before-dispatch journaling, generation pinning, and replay
  consistency apply identically to custom adapters.

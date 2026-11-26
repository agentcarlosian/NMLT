# RFC 0041: Stdio MCP Tamper-Proof Lean Verification Oracle (`nmlt mcp`)

- Status: Under review
- Authors: NMLT maintainers
- Target: `crates/nmlt-cli`

## Summary

Add `nmlt mcp [--work-dir DIR]`, a zero-dependency stdio JSON-RPC 2.0 Model
Context Protocol (MCP) server exposing NMLT's bound Lean task workflow as six
structured tools (`nmlt_tools_status`, `nmlt_lean_bind`, `nmlt_lean_inspect`,
`nmlt_lean_prove`, `nmlt_lean_recheck`, `nmlt_lean_revise`).

## Motivation

AI coding agents attempting Lean 4 formalization routinely cheat when given raw
shell access to a Lean repository: they weaken theorem statements, add axioms,
drop `sorry`, edit build settings, or misread Lean compiler diagnostics as
independent proof acceptance. NMLT already prevents every one of those failure
modes via `lean-task`, guarded candidates (RFC 0039), ProcessKit containment,
and independent `lean4export` + `nanoda` kernel verification. Exposing this
pipeline directly over MCP lets Claude Code, Cursor, Codex, and custom agent
loops use NMLT as a drop-in tamper-proof proof oracle.

## Protocol and Surface

`nmlt mcp` speaks newline-delimited JSON-RPC 2.0 over `stdin`/`stdout` (protocol
version `2024-11-05`), enforcing a strict 256 KiB per-message byte bound:

- `initialize` — advertises `tools` capability and server metadata.
- `notifications/initialized` — acknowledged silently per MCP spec.
- `ping` — returns `{}`.
- `tools/list` — publishes JSON Schemas for six tools:
  1. `nmlt_tools_status` — inspect resolved Lean, `lean4export`, and `nanoda`
     executables, SHA-256 digests, and readiness.
  2. `nmlt_lean_bind` — bind a Lean project into an immutable `task.json` +
     `task.sha256` directory. Reports `assurance: "none (binding only; not proof acceptance)"`.
  3. `nmlt_lean_inspect` — query declarations in a bound task by bounded prefix.
     Reports `assurance: "none (context inspection only)"`.
  4. `nmlt_lean_prove` — validate and independently verify an inline proof
     (`v1` term, `v2` automation, or `v3` guarded tactic script) against a bound
     task via Lean, `lean4export`, and `nanoda`.
  5. `nmlt_lean_recheck` — independently recheck a retained `result.json`
     acceptance record against the same pinned task and checker tools.
  6. `nmlt_lean_revise` — explicitly revise a bound task with an audit reason,
     producing a new task hash and invalidating prior acceptance for the revised
     task.
- `tools/call` — runs the requested tool in-process through `tools` and
  `lean_task`, returning structured JSON inside MCP text content. Proof
  rejections and static guard rejections return `isError: true` with the exact
  Lean diagnostics so the caller can iterate without corrupting the bound task.

## Security and Trust Boundary

- `nmlt mcp` adds zero new crates and zero new verification claims.
- Every proof acceptance still flows through `Candidate::from_file` (RFC 0039),
  `lean::Toolchain::open`, `process::Process` containment, `lean4export`, and
  `nanoda`.
- Each tool call writes to a fresh, non-existent output directory inside
  `--work-dir` (or an explicit caller-supplied new `output` path), never
  overwriting prior task or acceptance records.
- Non-proof tools explicitly label `assurance: "none (...)"` so clients cannot
  confuse binding or declaration lookup with kernel verification.

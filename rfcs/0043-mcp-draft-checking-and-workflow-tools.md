# RFC 0043: MCP draft checking, workflow tools, and client configuration

- Status: Under review
- Date: 2026-11-26
- Related: RFC 0039, RFC 0040, RFC 0041, RFC 0042

## Problem

RFC 0041 introduced `nmlt mcp` as a stdio JSON-RPC 2.0 server exposing the core
bound Lean proof oracle (`nmlt_tools_status`, `nmlt_lean_bind`,
`nmlt_lean_inspect`, `nmlt_lean_prove`, `nmlt_lean_recheck`,
`nmlt_lean_revise`). In interactive AI-agent proof development, three friction
points remained:

1. Calling `nmlt_lean_prove` on every intermediate tactic draft pays the cost of
   `lean4export` and `nanoda_bin` even when Lean itself rejects the tactic
   script with a type error or unsolved goal.
2. Agents authoring `.nmlt` workflow programs or `nmlt.toml` projects had no
   structured MCP tools to typecheck workflows, run pure workflows, or validate
   project locks without shelling out.
3. Connecting `nmlt mcp` to MCP clients (Claude Code, Cursor, Codex, generic
   JSON clients) required hand-crafting configuration snippets.

## Proposal

1. **Fast draft compilation tool (`nmlt_lean_check_draft`)**:
   - Enforces the exact same bound task verification (`task.json` +
     `task_sha256`), pinned Lean toolchain identity, wrapper immutability, and
     RFC 0039 static candidate guards as `nmlt_lean_prove`.
   - Compiles `NMLTProof.lean` with Lean `--json` and returns structured
     `diagnostics` (`nmlt-lean-diagnostics-v1`), `axioms`, and
     `axioms_within_policy` without invoking `lean4export` or `nanoda`.
   - Always reports `status: "draft_compiled"` or `"draft_rejected"` and
     `assurance: "none (Lean draft compilation only; run nmlt_lean_prove for independent NanoDA verification)"`.
   - Never emits `nmlt-lean-result-v5` or `independently_checked`.

2. **Workflow and project MCP tools**:
   - `nmlt_workflow_typecheck`: compiles a `.nmlt` workflow package and returns
     `program_sha256`, entries, records, and capability requirement flags
     (`assurance: "none"`).
   - `nmlt_workflow_run`: executes a pure `.nmlt` workflow entry point with
     JSON inputs and `max_steps`, returning the structured execution result
     (`assurance: "none"`).
   - `nmlt_project_check`: opens a local `nmlt.toml` project, verifies
     `nmlt.lock`, and validates all declared tests (`assurance: "none"`).

3. **Client configuration generator (`nmlt mcp init-config`)**:
   - `nmlt mcp init-config [--client claude-code|cursor|codex|generic] [--work-dir DIR]`
     prints a ready-to-paste JSON or TOML MCP server entry pointing at the
     current `nmlt` executable.

4. **Output directory confinement**:
   - Caller-supplied `output` paths in `nmlt mcp` are confined inside
     `--work-dir` by default (rejecting `..` traversal, symlinks, and external
     paths) unless the operator explicitly starts `nmlt mcp` with
     `--allow-external-output`.

## Trust boundary and honesty

- `nmlt_lean_check_draft`, `nmlt_workflow_typecheck`, `nmlt_workflow_run`, and
  `nmlt_project_check` all report `assurance: "none"`.
- Only `nmlt_lean_prove` and `nmlt_lean_recheck` produce
  `independently_checked` Lean proof results via `lean4export` + `nanoda`.
- Every JSON-RPC 2.0 response line emitted by `nmlt mcp` stays strictly pure
  JSON on stdout, verified by real-process stdio integration tests.

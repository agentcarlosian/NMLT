# Pluggable workflow adapters and portfolio proof search

This directory provides starter reference examples for NMLT's SHA-256-pinned
subprocess adapter protocol (`[[adapters]]`, [RFC 0042](../../rfcs/0042-pluggable-workflow-adapters.md)).

## Files

- [`tactic_proposer.py`](tactic_proposer.py): zero-dependency Python 3 reference
  adapter implementing `nmlt-job-response-v1`. It accepts a strategy+goal input
  string and deterministically returns a guarded Lean 4 tactic candidate (`by ...`).
- [`portfolio_search.nmlt`](portfolio_search.nmlt): NMLT workflow demonstrating
  concurrent multi-strategy orchestration with `job_start_adapter`, `job_poll`,
  `job_cancel` on the losing branch, affine `Job<Text>` handle collection, and
  optional downstream verification via `job_start_lean_project`.

## Wire contract (`nmlt-job-response-v1`)

NMLT spawns each registered adapter with a cleared environment, exact SHA-256
verification of both `executable` and every file listed in `inputs`, and a
single JSON `Dispatch` payload on stdin:

```json
{
  "binding": { "attempt": { ... }, "adapter": { ... }, ... },
  "input": {
    "type": "text",
    "value": "{\"schema\":\"nmlt-custom-adapter-request-v1\",\"alias\":\"proposer\",\"executable_sha256\":\"...\",\"input\":\"fast:add_zero\"}"
  }
}
```

The adapter must write one canonical compact JSON line to stdout, write nothing
to stderr, and exit with status `0`:

```json
{"schema":"nmlt-job-response-v1","binding":{...},"outcome":{"kind":"completed","value":{"type":"text","value":"by\n  intro n\n  simpa only [Nat.add_zero]\n"}},"observed_work":1}
```

## Registering and locking an adapter in `nmlt.toml`

Pin both the interpreter binary (via `executable`) and the script file (via
`inputs`) so any drift in either binary or script fails closed at lock/launch:

```toml
[[adapters]]
alias = "proposer_fast"
executable = "/usr/bin/python3"
args = ["tactic_proposer.py", "--profile", "fast"]
inputs = ["tactic_proposer.py"]
timeout_ms = 5000

[[adapters]]
alias = "proposer_deep"
executable = "/usr/bin/python3"
args = ["tactic_proposer.py", "--profile", "deep"]
inputs = ["tactic_proposer.py"]
timeout_ms = 5000
```

Then lock, check, run, and replay:

```sh
nmlt typecheck examples/adapters/portfolio_search.nmlt --profile workflow
nmlt lock <project-dir>
nmlt test <project-dir>
nmlt run <project-dir>
nmlt replay <project-dir>/.nmlt/run-.../project.json --project <project-dir>
```

## Honesty boundary

Custom adapters report `assurance: "none"` by themselves: proposing a candidate
proof script is never proof acceptance. To independently verify a proposed
candidate, pass it to `job_start_lean_project(alias, candidate)` or
`nmlt lean-task prove` / `nmlt_lean_prove` so Lean + `lean4export` + `nanoda`
check the proof against the bound immutable target.

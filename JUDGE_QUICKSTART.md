# NMLT Judge Quickstart

Release: `build-week-submission-2026`

Release page:
https://github.com/agentcarlosian/NMLT/releases/tag/build-week-submission-2026

Submission revision: `<FINAL_COMMIT_SHA>`

Bundle SHA-256: `<RELEASE_SHA256>`

This Linux x86_64 bundle is expected to contain:

```text
nmlt
fixtures/provider-attempt/reference.nmlt
fixtures/provider-attempt/dispatch-before-authorize.nmlt
JUDGE_QUICKSTART.md
```

From the extracted bundle directory, run:

```bash
chmod +x ./nmlt
./nmlt model-check --json fixtures/provider-attempt/reference.nmlt
./nmlt model-check --json fixtures/provider-attempt/dispatch-before-authorize.nmlt
```

These two runs do not require Rust, Lean, TLC, Quint, P, Python, Node, or
network access.

## Expected bounded outcomes

The reference run reports:

- `result: "model_checked"`
- `complete: true`
- `explored_states: 9`
- `explored_transitions: 12`
- four properties with `result: "model_checked"`

The seeded-defect run reports:

- `result: "refuted"`
- `complete: true`
- `explored_states: 2`
- `explored_transitions: 2`
- `DispatchRequiresArm` as refuted
- a witness whose `dispatch` step has `armed: false`

`model_checked` means the properties held throughout the reachable state space
exhausted within the result's declared finite bounds. It is not an unbounded
proof, a production-safety claim, or verification of arbitrary NMLT source.

Demo: https://www.youtube.com/watch?v=-PbhZ9me46Y

Repository: https://github.com/agentcarlosian/NMLT

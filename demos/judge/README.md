# NMLT judge demo

Run from the repository or extracted release:

    ./judge-demo.sh

For screen-recording pauses:

    ./judge-demo.sh --paced

## What the demo establishes

The guarded NMLT model is explored to completion within its displayed finite
bounds. The dropped-guard model is refuted with a concrete state-by-state
counterexample. A deterministic readback gate then rejects an earlier report
after the exact NMLT model bytes change.

## Manual-abstraction boundary

The C and Rust files are reviewable scenario fixtures. NMLT does not currently
parse them, translate between them, execute their native semantics, or prove
them equivalent. A developer manually maps the relevant workflow behavior into
the paired NMLT models. Fidelity of that mapping is a trusted review obligation.

A model counterexample is decisive for the authored model. A model_checked
result is finite evidence over the reported reachable state space and bounds,
not a general theorem about native C or Rust code.

The stale-evidence control proves that a prior report belongs to different
NMLT model bytes. It does not diagnose the semantic meaning of the edit and
does not prove that the manual model matches the source snippets.

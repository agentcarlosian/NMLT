# NMLT

NMLT—New Mathematics, Languages, and Techniques—is a pre-alpha programming
language research project. Its central experiment is to design a language and
new compositional mathematics together: programs elaborate into an explicit
behavioral core, and Lean defines the normative semantics of that core.

Rust is the language frontend and reference evaluator. It is not the semantic
prover.

## Current vertical slice

```text
.nmlt source
  → lossless syntax, resolution, and typed behavioral elaboration (Rust)
  → canonical behavior-core-v1 artifact
  → artifact decoding and composition/refinement obligations (Lean)
  → optional bounded exploration with no verification claim (Rust)
```

The first slice is deliberately finite, binary, and safety-oriented. It has:

- finite `Bool`, `Unit`, and enum state with explicit observations;
- typed input/output ports and one-to-one binary connections;
- affine nominal capabilities with exact transfer/receive matching;
- named natural-number grades with pointwise addition and refinement order;
- nominal rely/guarantee facts discharged at synchronization;
- explicit refinement state maps and hidden actions; and
- a Lean theorem lifting resource-aware weak refinement through actual product
  transitions under wiring, isolation, ownership, transfer, grade, and contract
  premises.

Fairness, liveness, infinite traces, general grade algebras, arbitrary state
maps, and compiler-correctness are not claimed in this milestone.

## Try it

The primary fixture is
[`examples/pivot/visible_resource_sync.nmlt`](examples/pivot/visible_resource_sync.nmlt).

```bash
cargo run -p nmlt-cli -- typecheck examples/pivot/visible_resource_sync.nmlt

cargo run -p nmlt-cli -- elaborate \
  examples/pivot/visible_resource_sync.nmlt \
  --emit-core /tmp/visible-resource-sync.json

cd mechanization/lean
lake exe nmlt-artifact-check /tmp/visible-resource-sync.json \
  ../../examples/pivot/visible_resource_sync.nmlt
cd ../..

cargo run -p nmlt-cli -- explore \
  --behavior ConcreteNetwork --max-states 8 \
  /tmp/visible-resource-sync.json
```

Exploration prints `assurance: none`. In the positive fixture it shows the
receiver state change, one-time `permit` transfer, and the synchronized
`work=3` grade.

Run the Rust gate with `make ci`, the Lean gate with `make metatheory`, or both
with `make reproduce`.

## Trust boundary

| Component | Role | Claim ceiling |
|---|---|---|
| `nmlt-core`, `nmlt-hir`, `nmlt-ir`, `nmlt-elaborate`, `nmlt-compile` | Frontend and explicit core production | Auditability through exact source binding and snapshots; no verified compilation theorem |
| `nmlt-kernel` | Independent validator for the retained typed-elaboration certificate | Formation/type acceptance only; never behavior proof |
| `NMLT.Behavior.ResourceBehavior` | Normative behavior/product/refinement definitions and theorem | Safety/resource theorem at the stated finite conditional scope |
| `NMLT.Artifact.BehaviorCore` | Fail-closed artifact decoder and semantic side-condition checks | Acceptance of the exact finite schema; no source-to-core correctness theorem |
| `nmlt-eval` | Reference operational exploration | No proof, model-check, or evidence claim |

See [`docs/architecture.md`](docs/architecture.md) for the component boundary
and [`schemas/behavior-core-v1.schema.json`](schemas/behavior-core-v1.schema.json)
for the artifact envelope.

## History

The former contest-oriented verifier remains reproducible from the immutable
[`build-week-judge-demo-2026`](https://github.com/agentcarlosian/NMLT/tree/build-week-judge-demo-2026)
tag at commit `0417f6e`. It is historical work, not part of the active language
architecture or default gate. The reviewed post-event resource experiment is
preserved separately on `codex/quarantine-grok-resource-pack`; it is not merged
into this branch.

NMLT is licensed under Apache-2.0. See [`LICENSE`](LICENSE).

# OpenAI Build Week Guide

This document contains the submission story and evaluation workflow for the OpenAI Build Week (July 2026).

## OpenAI Build Week & Codex Story

- **What problem does NMLT address?** Software and AI-generated claims can outrun their evidence. NMLT ensures that all semantic claims (like safety or refinement properties) carry explicit, independent, machine-checkable evidence bound to precise sources, tools, and bounds.
- **What can a judge run today?** You can run the CLI to parse, typecheck, inspect, and model-check behavior-first `.nmlt` programs, producing structured JSON outputs with counterexamples when properties are refuted.
- **Where did Codex/GPT-5.6 accelerate development?** Codex accelerated construction and recovery of a multi-language verification chain spanning Rust, Lean, pinned Charon/Aeneas translation, evidence manifests, mutation controls, TLA+, and P.
- **Which decisions remained human-directed?** High-level architectural boundaries, trust assumptions, relational and behavioral semantics, and the selection of formal mechanization strategies were entirely human-designed.
- **How does NMLT prevent AI-generated output from being blindly trusted?** NMLT treats all generated work as a proposal: acceptance remains bound to independent kernels, exact identities, explicit trust boundaries, and adversarial controls. It fails closed on any mismatch or stale evidence.

## Five-Minute Judge Workflow

To quickly evaluate NMLT's behavior-first modeling and bounded verification capabilities without building the full reproduction stack (such as Lean, TLC, or P), you can run the following commands to check both a valid reference model and a seeded-defect mutant.

### 1. Run the Accepted Reference Model
Run the model checker on the reference provider attempt model to verify it meets all properties:
```bash
cargo run -p nmlt-cli -- model-check --json benchmarks/seeded-defects/provider-attempt/reference.nmlt
```

**Expected Abbreviated JSON Output:**
```json
{
  "system": "ProviderAttemptReference",
  "result": "model_checked",
  "complete": true,
  "explored_states": 9,
  "explored_transitions": 12,
  "properties": [
    { "property": "SelectionRequiresPass", "result": "model_checked" },
    { "property": "DispatchRequiresArm", "result": "model_checked" },
    { "property": "NoBlindReplay", "result": "model_checked" },
    { "property": "EvaluationRequiresIntactResponse", "result": "model_checked" }
  ]
}
```

### 2. Run a Seeded-Defect Mutant (Failure Path)
Run the model checker on a mutant that allows a dispatch to occur before being authorized:
```bash
cargo run -p nmlt-cli -- model-check --json benchmarks/seeded-defects/provider-attempt/dispatch-before-authorize.nmlt
```

**Expected Abbreviated JSON Output:**
Showing the refuted property along with the structured step-by-step counterexample witness:
```json
{
  "system": "DispatchBeforeAuthorizeMutant",
  "result": "refuted",
  "complete": true,
  "explored_states": 2,
  "explored_transitions": 2,
  "properties": [
    {
      "property": "DispatchRequiresArm",
      "result": "refuted",
      "reason": "property `DispatchRequiresArm` is false in reachable state 1",
      "witness": {
        "steps": [
          { "index": 0, "action": null, "state": { "armed": false, "dispatched": false, "phase": "proposed" } },
          { "index": 1, "action": "dispatch", "state": { "armed": false, "dispatched": true, "phase": "proposed" } }
        ]
      }
    }
  ]
}
```

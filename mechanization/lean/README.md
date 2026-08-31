# NMLT Lean mechanization

This directory is the Lean 4.30 home for NMLT's current checked mathematics.
Lean defines the behavioral semantics used by the language-and-mathematics
pivot. Rust does not prove behavioral claims.

## Active modules

`NMLT.lean` imports four groups:

1. **Retained typed-core metatheory**
   - `Core/Transition.lean`
   - `Core/TypedCore.lean`
   - `Typing/Judgments.lean`
   - `Metatheory/Soundness.lean`
   - `Correspondence/M9Kernel.lean`
2. **Behavior and composition**
   - `Behavior/ResourceBehavior.lean`
   - `Behavior/ResourceWorld.lean`
3. **Artifact interpretation**
   - `Artifact/BehaviorCore.lean`
   - `Artifact/SemanticClosure.lean`
   - `Artifact/CheckMain.lean`
4. **Positive and negative instances**
   - `Examples/ResourceWorldTransfer.lean`
   - `Counterexamples/ResourceBehaviorControls.lean`
   - `Counterexamples/ResourceWorldControls.lean`

No removed OpenComposition, temporal, standalone grade-algebra, generated open
kernel, or Build Week evidence module belongs to the active package.

## Current semantic results

`ResourceBehavior` defines:

- a state/initial/step/observation behavior object;
- action direction, payload, hiding, ownership, and complete resource profiles;
- static left, right, and synchronized product steps;
- product-formation conditions for wiring, visibility, ports, ownership,
  transfer, and contracts;
- resource-aware weak refinement; and
- `liftParallel`, a checked conditional static step-lifting result.

The product-formation controls show that malformed products fail their named
rules. They do not establish that every formation field is logically necessary
for every weakened lifting theorem.

`ResourceWorld` adds:

- one optional owner per nominal capability;
- enabled local and synchronized resource effects;
- consumption, exact bidirectional transfer, and preservation of unaffected
  capabilities;
- a dynamic product state and step relation; and
- conditional one-step lifting for visible local, peer, synchronized, and
  resource-compatible hidden steps.

The dynamic layer remains auxiliary: it is not yet a `Behavior`, is not
proved equivalent to the static product, and has no reachability or liveness
theorem.

## Artifact checker

`nmlt-artifact-check`:

1. decodes canonical `behavior-core-v1` JSON;
2. compares the artifact's asserted digest to separately supplied source bytes;
3. constructs finite Lean states, terms, actions, resources, wiring, and
   refinement maps;
4. decides the static and dynamic theorem premises; and
5. constructs conditional theorem witnesses.

It does not run the Rust compiler and therefore does not prove that an arbitrary
artifact was translated from the supplied source. The repository's primary
fixture is separately regenerated and byte-compared by the Rust gate.

The dynamic witness maps any supplied concrete step. It does not prove that the
decoded initial state admits that step or that the step is reachable.

## Build and audit

Install the pinned toolchain with
[Elan](https://lean-lang.org/doc/reference/latest/Build-Tools-and-Distribution/Managing-Toolchains-with-Elan/),
then run:

```sh
cd mechanization/lean
lake build
```

From the repository root, the complete Lean gate is:

```sh
make metatheory
```

The gate:

- rejects `sorry`, `sorryAx`, `admit`,
  `native_decide`, and project-defined axioms;
- builds the package and executable checker;
- accepts the canonical fixture;
- rejects stale digest, malformed resource, semantic-step, and dynamic
  requirement mutations;
- audits focused theorem dependencies; and
- independently checks the exported environment with the pinned NanoDA path.

The exact foundational policy and current outputs are documented in
[`AXIOMS.md`](AXIOMS.md).

## Trust and non-claims

The current checked results are claims about the exact Lean definitions and
explicit premises. They are not:

- compiler correctness;
- a proof that NMLT source captures human intent;
- general open-interface preservation;
- a proof that every formation rule is a necessary theorem hypothesis;
- artifact-derived step existence or reachability;
- fairness or liveness transport; or
- a production verification or authorization system.

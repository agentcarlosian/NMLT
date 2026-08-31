# NMLT Lean mechanization

Lean defines the semantic authority for the current finite static behavioral
slice. Rust produces artifacts carrying a source digest, but no verified
compiler theorem connects Rust elaboration to these definitions.

## Active modules

- `NMLT.Behavior.ResourceBehavior` defines behaviors, typed action directions,
  resource profiles, binary product steps, weak refinement, product formation,
  and the conditional static lifting theorem.
- `NMLT.Counterexamples.ResourceBehaviorControls` contains permanent controls
  for failed product and hidden-step conditions.
- `NMLT.Examples.VisibleResourceSync` is the hand-written theorem instance for
  the primary sender/receiver design at this checkpoint.
- `NMLT.Artifact.BehaviorCore` decodes canonical `behavior-core-v1` structure.
- `NMLT.Artifact.CheckMain` checks artifact structure and compares its declared
  source digest with supplied source bytes.

The decoder does not yet derive the behavior or theorem witness from artifact
contents. That semantic closure is the next stack layer.

## Build and audit

From the repository root:

```bash
make metatheory
```

The gate builds the package, rejects unchecked placeholders, checks stale and
malformed artifacts fail closed, and audits the focused static theorem axioms.
The approved focused dependencies are `propext` and, where explicitly shown,
`Quot.sound`.

The complete package can additionally be checked with the pinned independent
NanoDA/exporter pair:

```bash
./tools/check_nanoda.sh mechanization/lean NMLT
```

See [`AXIOMS.md`](AXIOMS.md) for the policy and current declaration inventory.

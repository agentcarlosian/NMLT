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
- `NMLT.Artifact.BehaviorCore` decodes canonical `behavior-core-v1` structure.
- `NMLT.Artifact.SemanticClosure` reconstructs finite behaviors, compositions,
  state maps, and the conditional theorem instance from decoded artifact data.
- `NMLT.Artifact.CheckMain` checks artifact structure and compares its declared
  source digest with supplied source bytes.

The decoder and semantic-closure layer validate the artifact's own semantic
contents. They do not prove that Rust elaboration correctly translated the
identified source.

## Build and audit

From the repository root:

```bash
make metatheory
```

The gate builds the package, rejects unchecked placeholders, checks stale,
malformed, and semantically inconsistent artifacts fail closed, and audits the
focused static theorem axioms.
The approved focused dependencies are `propext` and, where explicitly shown,
`Quot.sound`.

The complete package can additionally be checked with the pinned independent
NanoDA/exporter pair:

```bash
./tools/check_nanoda.sh mechanization/lean NMLT
```

See [`AXIOMS.md`](AXIOMS.md) for the policy and current declaration inventory.

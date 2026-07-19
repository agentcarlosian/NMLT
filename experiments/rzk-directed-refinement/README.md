# Rzk directed-refinement experiment

This experiment asks whether NMLT refinements can be organized as directed
arrows before attempting a synthetic infinity-categorical semantics. It is
pinned to **Rzk v0.10.0** and is outside the NMLT trusted computing base.

The checked seed model treats a behavior only through its state type and a
refinement as a one-way state map. It establishes pointwise left identity,
right identity, and associativity by computation. Pointwise laws are used
deliberately: equality of functions would require an extensionality principle.

This is not yet a claim that arbitrary NMLT refinement witnesses form a Segal
type. The next experiment must replace the bare function model with the
proof-relevant forward simulations from the Lean development and ask whether
their composable chains have contractible composites.

## Check

Install or download Rzk v0.10.0, then run:

```sh
rzk typecheck directed-refinement.rzk
```

The root CI does not install Rzk. This experiment therefore records its tool
version separately and remains an optional research gate until the Rzk model
has enough semantic content to justify adding it to CI.

## Trust boundary

Rzk is a research instrument here, not an authority for accepting NMLT
programs or certificates. In particular, current `rzk-1` uses `U : U`, which
the Rzk reference explicitly describes as unsound. Results must ultimately be
re-expressed in the trusted Lean metatheory or proved by NMLT's independent
kernel before they can support an acceptance claim.

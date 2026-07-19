# Rzk directed-refinement experiment

This experiment asks whether NMLT refinements can be organized as directed
arrows before attempting a synthetic infinity-categorical semantics. It is
pinned to **Rzk v0.10.0** and is outside the NMLT trusted computing base.

The checked model treats a behavior through its state and observation types.
A refinement witness packages a one-way state map with a dependent proof that
observations are preserved. Identity and composition construct both maps and
witnesses; pointwise category laws avoid assuming function extensionality.

This is not yet a claim that arbitrary NMLT refinement witnesses form a Segal
type. Step-simulation witnesses and contractible composition spaces remain the
next directed-type-theory experiment.

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

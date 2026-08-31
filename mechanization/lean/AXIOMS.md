# Approved foundational axioms

The Lean gate rejects `sorry`, `sorryAx`, `admit`, `native_decide`, and locally
declared axioms. Its focused theorem audit permits only these Lean foundations:

- `propext`, for propositional extensionality;
- `Quot.sound`, in the retained typed-elaboration metatheory's quotient
  reasoning.

`NMLT.Behavior.ResourceBehavior.liftParallel` and
`NMLT.Examples.VisibleResourceSync.visibleResourceSync_lifts` currently report
only `propext`. `Classical.choice` and `Lean.trustCompiler` are not approved for
these behavioral theorems.

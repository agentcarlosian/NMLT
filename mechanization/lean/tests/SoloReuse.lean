import NMLT.Discovery.NestedReuse

open NMLT.Discovery.CompositionPremises
open NMLT.Discovery.NestedReuse

example : Sufficient [.refinement, .wiring, .abstractFormation] := case11
example : concrete.init ((false, false), false) ∧
    concrete.step ((false, false), false) action ((true, true), true) :=
  concreteInitialAndEnabled
example : abstract.init ((false, false), false) ∧
    abstract.step ((false, false), false) action ((true, true), true) := nestedStepLifts
example : ¬ NMLT.Behavior.ResourceBehavior.Composable inner monitor monitorConnection :=
  formationStillRejected
example : ¬ Nonempty (NMLT.Behavior.ResourceBehavior.ResourceWeakRefinement concrete undersized) :=
  undersizedBoundRejected

def main : IO Unit := IO.println "nested reuse: initialized step, lifting, formation and resource controls checked"

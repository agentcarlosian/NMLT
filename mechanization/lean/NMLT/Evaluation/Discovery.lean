import NMLT.Discovery.FiniteConstructions

namespace NMLT.Evaluation.Discovery
open NMLT.Discovery.FiniteConstructions

/-! Separate larger-size objects for the exposed evaluation corpus. Reusing
the predicate does not turn this into a blind benchmark or an all-sizes proof. -/
def largerRows : List (Nat × Nat × Bool × Bool) :=
  [8, 9].flatMap fun size =>
    ((List.range size).drop 1).map fun step =>
      (size, step, predicate size step, cycleIsomorphism size step)

set_option maxRecDepth 10000 in
set_option maxHeartbeats 4000000 in
theorem largerObjects : largerRows =
    [(8, 1, true, true), (8, 2, false, false), (8, 3, true, true),
     (8, 4, false, false), (8, 5, true, true), (8, 6, false, false), (8, 7, true, true),
     (9, 1, true, true), (9, 2, true, true), (9, 3, false, false),
     (9, 4, true, true), (9, 5, true, true), (9, 6, false, false),
     (9, 7, true, true), (9, 8, true, true)] := by decide

end NMLT.Evaluation.Discovery

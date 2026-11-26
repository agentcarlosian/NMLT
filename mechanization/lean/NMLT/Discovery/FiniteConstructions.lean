import NMLT.Behavior.ResourceBehavior

/-! Finite calibration objects only: no claim for all sizes or new mathematics.
The predicate includes all representation, degree, triangle, and reachability
conditions. Checking a search process is distinct from checking these objects. -/
namespace NMLT.Discovery.FiniteConstructions

def vertices (size : Nat) : List Nat := List.range size

def edge (size step a b : Nat) : Bool :=
  a < size && b < size && a != b &&
    ((a + step) % size == b || (b + step) % size == a)

def expand (size step : Nat) (seen : List Nat) : List Nat :=
  (vertices size).filter fun b => seen.contains b || seen.any (fun a => edge size step a b)

def reach (size step : Nat) : Nat → List Nat
  | 0 => [0]
  | rounds + 1 => expand size step (reach size step rounds)

/-- The fixed finite graph predicate, including its bounded reachability test. -/
def predicate (size step : Nat) : Bool :=
  size >= 4 &&
    (vertices size).all (fun a => !edge size step a a &&
      ((vertices size).filter (edge size step a)).length == 2) &&
    (vertices size).all (fun a => (vertices size).all (fun b =>
      edge size step a b == edge size step b a)) &&
    (vertices size).all (fun a => (vertices size).all (fun b =>
      (vertices size).all (fun c => !(edge size step a b && edge size step b c &&
        edge size step c a)))) &&
    (reach size step size).length == size

/-- An explicit permutation preserving and reflecting every edge to the cycle. -/
def cycleIsomorphism (size step : Nat) : Bool :=
  let mapping := (vertices size).map (fun a => a * step % size)
  mapping.eraseDups.length == size &&
    (vertices size).all (fun a => (vertices size).all (fun b =>
      edge size 1 a b == edge size step (a * step % size) (b * step % size)))

def rows : List (Nat × Nat × Bool × Bool) :=
  [4, 5, 6, 7].flatMap fun size =>
    ((List.range size).drop 1).map fun step =>
      (size, step, predicate size step, cycleIsomorphism size step)

set_option maxRecDepth 10000 in
set_option maxHeartbeats 2000000 in
theorem exhaustiveObjects : rows =
    [(4, 1, true, true), (4, 2, false, false), (4, 3, true, true),
     (5, 1, true, true), (5, 2, true, true), (5, 3, true, true), (5, 4, true, true),
     (6, 1, true, true), (6, 2, false, false), (6, 3, false, false),
     (6, 4, false, false), (6, 5, true, true),
     (7, 1, true, true), (7, 2, true, true), (7, 3, true, true),
     (7, 4, true, true), (7, 5, true, true), (7, 6, true, true)] := by decide

theorem triangleRejected : predicate 3 1 = false := by decide
theorem disconnectedRejected : predicate 6 2 = false := by decide
theorem wrongDegreeRejected : predicate 6 3 = false := by decide

end NMLT.Discovery.FiniteConstructions

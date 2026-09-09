import Example.Definitions

namespace Example

-- These placeholders are task statements. A final candidate cannot depend on
-- their sorry proofs: the transitive axiom policy rejects such a dependency.
theorem zero_offset (n : Nat) : offset n = n := by sorry

-- Negative control: hiding an unresolved goal behind a lemma is still rejected.
theorem borrowed (n : Nat) : offset n = n := zero_offset n

theorem use_assumption (n m : Nat) (h : n = m) : offset n = m := by sorry

theorem polymorphic.{u} (alpha : Type u) (x : alpha) : x = x := by sorry

theorem equivalence (p q : Prop) (h : p ↔ q) : p = q := by sorry

end Example

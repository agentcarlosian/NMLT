import Init

namespace Graph

structure Box where
  value : Nat

def project (box : Box) : Nat := box.value

mutual
  inductive Tree where
    | leaf : Nat → Tree
    | node : Forest → Tree
  inductive Forest where
    | nil : Forest
    | cons : Tree → Forest → Forest
end

def leaf : Tree := .leaf 7
opaque fixed : Nat := 3
def word : String := "dependency graph"
def quotient : Quot (fun (a b : Nat) => a = b) := Quot.mk _ 0

theorem checked (box : Box) :
    project box = project box ∧ leaf = leaf ∧ fixed = fixed ∧
    word = word ∧ quotient = quotient :=
  And.intro rfl (And.intro rfl (And.intro rfl (And.intro rfl rfl)))

-- Draft declarations stay outside accepted proof dependencies.
theorem unused_draft : False := by sorry

theorem goal (box : Box) :
    project box = project box ∧ leaf = leaf ∧ fixed = fixed ∧
    word = word ∧ quotient = quotient := by sorry

end Graph

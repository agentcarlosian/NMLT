import Init

/-! Developer-exposed R5 corpus, separate from R0. These are reference solutions,
not evidence of model performance or of executing the operational task cards. -/
namespace NMLT.Evaluation.Corpus

def twice (n : Nat) := n + n
def offset (n : Nat) := n + 3
def revision : Nat := 3

structure Packet where
  ready : Bool
  payload : Nat

def decode (packet : Packet) : Option Nat :=
  if packet.ready then some packet.payload else none

-- Ordinary statements.
theorem ordinary01 (p q : Prop) : p ∧ q → q ∧ p := fun h => ⟨h.2, h.1⟩
theorem ordinary02 (p q r : Prop) : p ∨ (q ∨ r) → r ∨ (p ∨ q) := by
  intro h
  cases h with
  | inl hp => exact Or.inr (Or.inl hp)
  | inr hqr =>
    cases hqr with
    | inl hq => exact Or.inr (Or.inr hq)
    | inr hr => exact Or.inl hr
theorem ordinary03 (value : Option Nat) : value.map id = value := by cases value <;> rfl
theorem ordinary04 (pair : Nat × Bool) : (pair.1, pair.2) = pair := by cases pair; rfl
theorem ordinary05 (values : List Nat) : values ++ [] = values := List.append_nil values
theorem ordinary06 (flag : Bool) : !(!flag) = flag := by cases flag <;> rfl

-- Definitions and actual proof dependencies.
theorem dependency01 (n : Nat) : twice n = n + n := rfl
theorem dependency02 (n : Nat) : twice (n + 1) = twice n + 2 := by
  simp [twice, Nat.add_assoc, Nat.add_comm, Nat.add_left_comm]
theorem dependency03 (n : Nat) : decode ⟨true, n⟩ = some n := rfl
theorem dependency04 (n : Nat) : decode ⟨false, n⟩ = none := rfl
theorem dependency05 (n : Nat) : offset (twice n) = n + n + 3 := rfl
theorem dependency06 (xs ys : List Nat) : (xs ++ ys).length = xs.length + ys.length :=
  List.length_append

-- Repair cards supply a failing recorded candidate before these solutions.
theorem repair01 (p q : Prop) : p ∧ q → p := fun h => h.1
theorem repair02 (p q r : Prop) : (p → q) → (q → r) → p → r := fun pq qr hp => qr (pq hp)
theorem repair03 (n m : Nat) : some n = some m → n = m := by intro h; cases h; rfl
theorem repair04 (n m : Nat) : Nat.succ n = Nat.succ m → n = m := Nat.succ.inj
theorem repair05 (xs : List Nat) : ([] ++ xs).length = xs.length := rfl
theorem repair06 (flag : Bool) : flag = true → !flag = false := by intro h; cases h; rfl

-- Context cards bind these definitions and invalidate acceptance after revision.
theorem context01 : revision = 3 := rfl
theorem context02 (n : Nat) : offset n = n + revision := rfl
theorem context03 : offset 4 = 7 := rfl
theorem context04 : twice revision = 6 := rfl
theorem context05 : decode ⟨true, revision⟩ = some 3 := rfl
theorem context06 : [revision, offset 0].length = 2 := rfl

-- Interrupted-run cards use these distinct goals; a theorem alone is not a
-- successful interruption/recovery trial.
theorem interrupted01 (p : Prop) : p → p := fun h => h
theorem interrupted02 (p q : Prop) : p → p ∨ q := Or.inl
theorem interrupted03 (p q : Prop) : q → p ∨ q := Or.inr
theorem interrupted04 (p q : Prop) : p → q → p ∧ q := fun hp hq => ⟨hp, hq⟩
theorem interrupted05 (n : Nat) : (n, true).1 = n := rfl
theorem interrupted06 (xs : List Nat) : xs.map id = xs := by
  induction xs with
  | nil => rfl
  | cons head tail ih => simp [ih]

-- A single batch target ensures the same 30 reference solutions are present in
-- the transitive proof closure of both orchestration arms. Count it as ONE run.
def Bundle : Prop :=
  (∀ p q : Prop, p ∧ q → q ∧ p) ∧
  (∀ p q r : Prop, p ∨ (q ∨ r) → r ∨ (p ∨ q)) ∧
  (∀ value : Option Nat, value.map id = value) ∧
  (∀ pair : Nat × Bool, (pair.1, pair.2) = pair) ∧
  (∀ values : List Nat, values ++ [] = values) ∧
  (∀ flag : Bool, !(!flag) = flag) ∧
  (∀ n, twice n = n + n) ∧
  (∀ n, twice (n + 1) = twice n + 2) ∧
  (∀ n, decode ⟨true, n⟩ = some n) ∧
  (∀ n, decode ⟨false, n⟩ = none) ∧
  (∀ n, offset (twice n) = n + n + 3) ∧
  (∀ xs ys : List Nat, (xs ++ ys).length = xs.length + ys.length) ∧
  (∀ p q : Prop, p ∧ q → p) ∧
  (∀ p q r : Prop, (p → q) → (q → r) → p → r) ∧
  (∀ n m : Nat, some n = some m → n = m) ∧
  (∀ n m : Nat, Nat.succ n = Nat.succ m → n = m) ∧
  (∀ xs : List Nat, ([] ++ xs).length = xs.length) ∧
  (∀ flag : Bool, flag = true → !flag = false) ∧
  revision = 3 ∧
  (∀ n, offset n = n + revision) ∧
  offset 4 = 7 ∧ twice revision = 6 ∧
  decode ⟨true, revision⟩ = some 3 ∧ [revision, offset 0].length = 2 ∧
  (∀ p : Prop, p → p) ∧
  (∀ p q : Prop, p → p ∨ q) ∧
  (∀ p q : Prop, q → p ∨ q) ∧
  (∀ p q : Prop, p → q → p ∧ q) ∧
  (∀ n : Nat, (n, true).1 = n) ∧
  (∀ xs : List Nat, xs.map id = xs)

theorem bundle : Bundle :=
  ⟨ordinary01, ordinary02, ordinary03, ordinary04, ordinary05, ordinary06,
   dependency01, dependency02, dependency03, dependency04, dependency05, dependency06,
   repair01, repair02, repair03, repair04, repair05, repair06,
   context01, context02, context03, context04, context05, context06,
   interrupted01, interrupted02, interrupted03, interrupted04, interrupted05, interrupted06⟩

end NMLT.Evaluation.Corpus

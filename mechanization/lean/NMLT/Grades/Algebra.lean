import Lean.Elab.Tactic.Omega

namespace NMLT.Grades

/-- The implemented uncertainty scale: one whole unit in parts per million. -/
def uncertaintyScale : Nat := 1_000_000

/-- Mathematical uncertainty coordinates are naturals bounded by the fixed
    parts-per-million scale. -/
abbrev Ppm := { value : Nat // value ≤ uncertaintyScale }

/-- The exact mathematical carrier corresponding to successful Rust grade
    operations. Rust represents the first three coordinates with `u64` and
    returns `unknown` on overflow; this model deliberately uses `Nat`. -/
structure Grade where
  cost : Nat
  privacy : Nat
  energy : Nat
  uncertainty : Ppm
  deriving DecidableEq, Repr

def zeroPpm : Ppm := ⟨0, by simp [uncertaintyScale]⟩

def zero : Grade where
  cost := 0
  privacy := 0
  energy := 0
  uncertainty := zeroPpm

/-- Saturated addition on the bounded uncertainty coordinate. -/
def Ppm.add (left right : Ppm) : Ppm :=
  ⟨min uncertaintyScale (left.val + right.val), Nat.min_le_left _ _⟩

/-- Componentwise maximum on the bounded uncertainty coordinate. -/
def Ppm.choice (left right : Ppm) : Ppm :=
  ⟨max left.val right.val, by omega⟩

/-- Sequential composition: addition for exact resource coordinates and
    saturated addition for abstract uncertainty. -/
def sequential (left right : Grade) : Grade where
  cost := left.cost + right.cost
  privacy := left.privacy + right.privacy
  energy := left.energy + right.energy
  uncertainty := left.uncertainty.add right.uncertainty

/-- Conservative parallel composition is the same additive operation in the
    implemented Phase-7 profile. -/
def parallel : Grade → Grade → Grade := sequential

/-- Exclusive choice takes a componentwise upper envelope. -/
def choice (left right : Grade) : Grade where
  cost := max left.cost right.cost
  privacy := max left.privacy right.privacy
  energy := max left.energy right.energy
  uncertainty := left.uncertainty.choice right.uncertainty

/-- Product budget order. -/
def Le (left right : Grade) : Prop :=
  left.cost ≤ right.cost ∧
  left.privacy ≤ right.privacy ∧
  left.energy ≤ right.energy ∧
  left.uncertainty.val ≤ right.uncertainty.val

instance (left right : Grade) : Decidable (Le left right) := by
  unfold Le
  infer_instance

/-- Executable presentation of the same componentwise budget test. -/
def budgetAccept (usage budget : Grade) : Bool :=
  decide (Le usage budget)

@[simp] theorem Ppm.zero_add (value : Ppm) : zeroPpm.add value = value := by
  apply Subtype.ext
  simp [Ppm.add, zeroPpm, Nat.min_eq_right value.property]

@[simp] theorem Ppm.add_zero (value : Ppm) : value.add zeroPpm = value := by
  rw [Ppm.add]
  apply Subtype.ext
  simp [zeroPpm, Nat.min_eq_right value.property]

theorem Ppm.add_comm (left right : Ppm) : left.add right = right.add left := by
  apply Subtype.ext
  simp [Ppm.add, Nat.add_comm]

theorem Ppm.add_assoc (first second third : Ppm) :
    (first.add second).add third = first.add (second.add third) := by
  apply Subtype.ext
  simp only [Ppm.add]
  omega

@[simp] theorem Ppm.zero_choice (value : Ppm) : zeroPpm.choice value = value := by
  apply Subtype.ext
  simp [Ppm.choice, zeroPpm]

@[simp] theorem Ppm.choice_zero (value : Ppm) : value.choice zeroPpm = value := by
  apply Subtype.ext
  simp [Ppm.choice, zeroPpm]

theorem Ppm.choice_comm (left right : Ppm) :
    left.choice right = right.choice left := by
  apply Subtype.ext
  simp [Ppm.choice, Nat.max_comm]

theorem Ppm.choice_assoc (first second third : Ppm) :
    (first.choice second).choice third = first.choice (second.choice third) := by
  apply Subtype.ext
  simp [Ppm.choice, Nat.max_assoc]

@[simp] theorem Ppm.choice_self (value : Ppm) : value.choice value = value := by
  apply Subtype.ext
  simp [Ppm.choice]

theorem Ppm.add_choice_distrib (first second third : Ppm) :
    first.add (second.choice third) =
      (first.add second).choice (first.add third) := by
  apply Subtype.ext
  simp only [Ppm.add, Ppm.choice]
  omega

theorem Ppm.add_mono {left₁ left₂ right₁ right₂ : Ppm}
    (leftOrdered : left₁.val ≤ left₂.val)
    (rightOrdered : right₁.val ≤ right₂.val) :
    (left₁.add right₁).val ≤ (left₂.add right₂).val := by
  simp only [Ppm.add]
  omega

@[simp] theorem zero_sequential (value : Grade) : sequential zero value = value := by
  cases value
  simp [sequential, zero]

@[simp] theorem sequential_zero (value : Grade) : sequential value zero = value := by
  cases value
  simp [sequential, zero]

theorem sequential_comm (left right : Grade) :
    sequential left right = sequential right left := by
  cases left
  cases right
  simp [sequential, Nat.add_comm, Ppm.add_comm]

theorem sequential_assoc (first second third : Grade) :
    sequential (sequential first second) third =
      sequential first (sequential second third) := by
  cases first
  cases second
  cases third
  simp [sequential, Nat.add_assoc, Ppm.add_assoc]

@[simp] theorem zero_parallel (value : Grade) : parallel zero value = value :=
  zero_sequential value

@[simp] theorem parallel_zero (value : Grade) : parallel value zero = value :=
  sequential_zero value

theorem parallel_comm (left right : Grade) :
    parallel left right = parallel right left :=
  sequential_comm left right

theorem parallel_assoc (first second third : Grade) :
    parallel (parallel first second) third = parallel first (parallel second third) :=
  sequential_assoc first second third

@[simp] theorem zero_choice (value : Grade) : choice zero value = value := by
  cases value
  simp [choice, zero]

@[simp] theorem choice_zero (value : Grade) : choice value zero = value := by
  cases value
  simp [choice, zero]

theorem choice_comm (left right : Grade) : choice left right = choice right left := by
  cases left
  cases right
  simp [choice, Nat.max_comm, Ppm.choice_comm]

theorem choice_assoc (first second third : Grade) :
    choice (choice first second) third = choice first (choice second third) := by
  cases first
  cases second
  cases third
  simp [choice, Nat.max_assoc, Ppm.choice_assoc]

@[simp] theorem choice_self (value : Grade) : choice value value = value := by
  cases value
  simp [choice]

@[simp] theorem le_refl (value : Grade) : Le value value := by
  simp [Le]

theorem le_trans {first second third : Grade} :
    Le first second → Le second third → Le first third := by
  intro firstSecond secondThird
  rcases firstSecond with ⟨firstCost, firstPrivacy, firstEnergy, firstUncertainty⟩
  rcases secondThird with ⟨secondCost, secondPrivacy, secondEnergy, secondUncertainty⟩
  exact ⟨
    Nat.le_trans firstCost secondCost,
    Nat.le_trans firstPrivacy secondPrivacy,
    Nat.le_trans firstEnergy secondEnergy,
    Nat.le_trans firstUncertainty secondUncertainty
  ⟩

theorem le_antisymm {left right : Grade} : Le left right → Le right left → left = right := by
  intro leftRight rightLeft
  rcases left with ⟨leftCost, leftPrivacy, leftEnergy, leftUncertainty⟩
  rcases right with ⟨rightCost, rightPrivacy, rightEnergy, rightUncertainty⟩
  rcases leftRight with ⟨costForward, privacyForward, energyForward, uncertaintyForward⟩
  rcases rightLeft with ⟨costBackward, privacyBackward, energyBackward, uncertaintyBackward⟩
  have costSame : leftCost = rightCost := Nat.le_antisymm costForward costBackward
  have privacySame : leftPrivacy = rightPrivacy :=
    Nat.le_antisymm privacyForward privacyBackward
  have energySame : leftEnergy = rightEnergy := Nat.le_antisymm energyForward energyBackward
  have uncertaintySame : leftUncertainty = rightUncertainty := by
    apply Subtype.ext
    exact Nat.le_antisymm uncertaintyForward uncertaintyBackward
  cases costSame
  cases privacySame
  cases energySame
  cases uncertaintySame
  rfl

theorem zero_le (value : Grade) : Le zero value := by
  simp [Le, zero, zeroPpm]

theorem le_choice_left (left right : Grade) : Le left (choice left right) := by
  exact ⟨
    Nat.le_max_left _ _,
    Nat.le_max_left _ _,
    Nat.le_max_left _ _,
    Nat.le_max_left _ _
  ⟩

theorem le_choice_right (left right : Grade) : Le right (choice left right) := by
  exact ⟨
    Nat.le_max_right _ _,
    Nat.le_max_right _ _,
    Nat.le_max_right _ _,
    Nat.le_max_right _ _
  ⟩

theorem choice_le {left right upper : Grade} :
    Le left upper → Le right upper → Le (choice left right) upper := by
  intro leftUpper rightUpper
  rcases leftUpper with ⟨leftCost, leftPrivacy, leftEnergy, leftUncertainty⟩
  rcases rightUpper with ⟨rightCost, rightPrivacy, rightEnergy, rightUncertainty⟩
  exact ⟨
    Nat.max_le.mpr ⟨leftCost, rightCost⟩,
    Nat.max_le.mpr ⟨leftPrivacy, rightPrivacy⟩,
    Nat.max_le.mpr ⟨leftEnergy, rightEnergy⟩,
    Nat.max_le.mpr ⟨leftUncertainty, rightUncertainty⟩
  ⟩

theorem le_iff_choice_eq_right (left right : Grade) :
    Le left right ↔ choice left right = right := by
  constructor
  · intro ordered
    apply le_antisymm
    · exact choice_le ordered (le_refl right)
    · exact le_choice_right left right
  · intro joined
    rw [← joined]
    exact le_choice_left left right

theorem sequential_mono {left₁ left₂ right₁ right₂ : Grade} :
    Le left₁ left₂ → Le right₁ right₂ →
      Le (sequential left₁ right₁) (sequential left₂ right₂) := by
  intro leftOrdered rightOrdered
  rcases leftOrdered with ⟨leftCost, leftPrivacy, leftEnergy, leftUncertainty⟩
  rcases rightOrdered with ⟨rightCost, rightPrivacy, rightEnergy, rightUncertainty⟩
  exact ⟨
    Nat.add_le_add leftCost rightCost,
    Nat.add_le_add leftPrivacy rightPrivacy,
    Nat.add_le_add leftEnergy rightEnergy,
    Ppm.add_mono leftUncertainty rightUncertainty
  ⟩

theorem sequential_choice_distrib (first second third : Grade) :
    sequential first (choice second third) =
      choice (sequential first second) (sequential first third) := by
  cases first
  cases second
  cases third
  simp only [sequential, choice, Grade.mk.injEq]
  constructor
  · omega
  constructor
  · omega
  constructor
  · omega
  · exact Ppm.add_choice_distrib _ _ _

/-- Componentwise upper envelope of a nonempty finite set of alternatives. -/
def nonemptyChoices (head : Grade) (tail : List Grade) : Grade :=
  tail.foldr choice head

theorem sequential_nonempty_choices_distrib
    (first head : Grade) (tail : List Grade) :
    sequential first (nonemptyChoices head tail) =
      nonemptyChoices (sequential first head) (tail.map (sequential first)) := by
  induction tail with
  | nil => rfl
  | cons grade rest inductionHypothesis =>
      simp only [nonemptyChoices, List.foldr, List.map]
      rw [sequential_choice_distrib]
      congr 1

theorem budgetAccept_iff (usage budget : Grade) :
    budgetAccept usage budget = true ↔ Le usage budget := by
  simp [budgetAccept]

/-- A positive executable budget decision exposes every coordinate inequality;
    no scalar tradeoff or unknown-to-success rule is present in this model. -/
theorem budgetAccept_sound {usage budget : Grade}
    (accepted : budgetAccept usage budget = true) : Le usage budget :=
  (budgetAccept_iff usage budget).1 accepted

#print axioms zero_sequential
#print axioms sequential_zero
#print axioms sequential_comm
#print axioms sequential_assoc
#print axioms zero_parallel
#print axioms parallel_zero
#print axioms parallel_comm
#print axioms parallel_assoc
#print axioms zero_choice
#print axioms choice_zero
#print axioms choice_comm
#print axioms choice_assoc
#print axioms choice_self
#print axioms zero_le
#print axioms le_iff_choice_eq_right
#print axioms sequential_mono
#print axioms sequential_choice_distrib
#print axioms sequential_nonempty_choices_distrib
#print axioms budgetAccept_iff
#print axioms budgetAccept_sound

end NMLT.Grades

namespace NMLT.Behavior

/-- A grade algebra is explicit semantic input. The laws are data required by
    behavior formation, not assumptions silently supplied by an analyzer. -/
structure GradeAlgebra where
  Carrier : Type
  one : Carrier
  tensor : Carrier → Carrier → Carrier
  le : Carrier → Carrier → Prop
  tensor_assoc : ∀ a b c, tensor (tensor a b) c = tensor a (tensor b c)
  one_tensor : ∀ a, tensor one a = a
  tensor_one : ∀ a, tensor a one = a
  le_refl : ∀ a, le a a
  le_trans : ∀ {a b c}, le a b → le b c → le a c
  tensor_mono : ∀ {a b c d}, le a b → le c d → le (tensor a c) (tensor b d)

/-- The abstract RFC 0001 behavior object. Its canonical identity appears in
    the type, preventing properties for one behavior from being reused for a
    different behavior without an explicit refinement or equality. -/
structure Model (identity : Nat) where
  State : Type
  Input : Type
  Output : Type
  Label : Type
  grades : GradeAlgebra
  init : State → Prop
  step : State → Label → Input → State → Output → grades.Carrier → Prop
  observe : State → Output
  silent : Output

/-- One event position in a behavior trace. `none` is the distinguished
    semantic identity stutter, never an arbitrary hidden transition. -/
structure Event {identity : Nat} (behavior : Model identity) where
  label : Option behavior.Label
  input : behavior.Input
  output : behavior.Output
  grade : behavior.grades.Carrier

def Event.Valid {identity : Nat} {behavior : Model identity}
    (before : behavior.State) (event : Event behavior)
    (after : behavior.State) : Prop :=
  match event.label with
  | none => before = after ∧ event.output = behavior.silent ∧ event.grade = behavior.grades.one
  | some label => behavior.step before label event.input after event.output event.grade

/-- Mathematical traces are infinite. Finite prefixes and lassos remain
    representations used by bounded engines; they do not redefine this type. -/
structure Trace {identity : Nat} (behavior : Model identity) where
  state : Nat → behavior.State
  event : Nat → Event behavior
  starts : behavior.init (state 0)
  advances : ∀ position, (event position).Valid (state position) (state (position + 1))

/-- A typed action relation whose behavior index, input, output, and grade are
    fixed by its enclosing behavior. -/
structure TypedAction {identity : Nat} (behavior : Model identity) where
  label : behavior.Label
  relation : behavior.State → behavior.Input → behavior.State →
    behavior.Output → behavior.grades.Carrier → Prop
  included : ∀ {before input after output grade},
    relation before input after output grade →
      behavior.step before label input after output grade

/-- Classical trace propositions express truth conditions. Atomic predicates
    are propositions and excluded-middle reasoning is not encoded as evidence. -/
inductive TraceProperty {identity : Nat} (behavior : Model identity) where
  | atom (predicate : behavior.State → Prop)
  | top
  | bottom
  | and (left right : TraceProperty behavior)
  | or (left right : TraceProperty behavior)
  | implies (left right : TraceProperty behavior)
  | not (property : TraceProperty behavior)
  | next (property : TraceProperty behavior)
  | always (property : TraceProperty behavior)
  | eventually (property : TraceProperty behavior)
  | until (invariant goal : TraceProperty behavior)

def TraceProperty.HoldsAt {identity : Nat} {behavior : Model identity}
    (trace : Trace behavior) : TraceProperty behavior → Nat → Prop
  | .atom predicate, position => predicate (trace.state position)
  | .top, _ => True
  | .bottom, _ => False
  | .and left right, position => left.HoldsAt trace position ∧ right.HoldsAt trace position
  | .or left right, position => left.HoldsAt trace position ∨ right.HoldsAt trace position
  | .implies left right, position => left.HoldsAt trace position → right.HoldsAt trace position
  | .not property, position => ¬ property.HoldsAt trace position
  | .next property, position => property.HoldsAt trace (position + 1)
  | .always property, position => ∀ later, position ≤ later → property.HoldsAt trace later
  | .eventually property, position => ∃ later, position ≤ later ∧ property.HoldsAt trace later
  | .until invariant goal, position =>
      ∃ witness, position ≤ witness ∧ goal.HoldsAt trace witness ∧
        ∀ earlier, position ≤ earlier → earlier < witness → invariant.HoldsAt trace earlier

def TraceProperty.Holds {identity : Nat} {behavior : Model identity}
    (property : TraceProperty behavior) : Prop :=
  ∀ trace : Trace behavior, property.HoldsAt trace 0

/-- Constructive properties describe positive evidence. Their atoms are
    types, conjunction is product, disjunction is sum, and eventuality carries
    a concrete witness position. Constructive implication is postponed until
    its evidence-erasure boundary is fixed without choice. -/
inductive EvidenceProperty {identity : Nat} (behavior : Model identity) where
  | atom (evidence : behavior.State → Type)
  | top
  | and (left right : EvidenceProperty behavior)
  | or (left right : EvidenceProperty behavior)
  | next (property : EvidenceProperty behavior)
  | always (property : EvidenceProperty behavior)
  | eventually (property : EvidenceProperty behavior)

def EvidenceProperty.EvidenceAt {identity : Nat} {behavior : Model identity}
    (trace : Trace behavior) : EvidenceProperty behavior → Nat → Type
  | .atom evidence, position => evidence (trace.state position)
  | .top, _ => PUnit
  | .and left right, position => left.EvidenceAt trace position × right.EvidenceAt trace position
  | .or left right, position => Sum (left.EvidenceAt trace position) (right.EvidenceAt trace position)
  | .next property, position => property.EvidenceAt trace (position + 1)
  | .always property, position => ∀ later, position ≤ later → property.EvidenceAt trace later
  | .eventually property, position =>
      Σ later, PLift (position ≤ later) × property.EvidenceAt trace later

def EvidenceProperty.erase {identity : Nat} {behavior : Model identity} :
    EvidenceProperty behavior → TraceProperty behavior
  | .atom evidence => .atom fun state => Nonempty (evidence state)
  | .top => .top
  | .and left right => .and left.erase right.erase
  | .or left right => .or left.erase right.erase
  | .next property => .next property.erase
  | .always property => .always property.erase
  | .eventually property => .eventually property.erase

/-- Constructive evidence can always be forgotten into classical trace truth.
    The reverse direction is intentionally absent. -/
theorem EvidenceProperty.sound {identity : Nat} {behavior : Model identity}
    (property : EvidenceProperty behavior) (trace : Trace behavior) (position : Nat) :
    property.EvidenceAt trace position → property.erase.HoldsAt trace position := by
  induction property generalizing position with
  | atom evidence =>
      intro witness
      exact ⟨witness⟩
  | top =>
      intro _
      trivial
  | and left right leftIH rightIH =>
      intro witness
      exact ⟨leftIH position witness.1, rightIH position witness.2⟩
  | or left right leftIH rightIH =>
      intro witness
      cases witness with
      | inl value => exact Or.inl (leftIH position value)
      | inr value => exact Or.inr (rightIH position value)
  | next property propertyIH =>
      intro witness
      exact propertyIH (position + 1) witness
  | always property propertyIH =>
      intro witness later laterAfter
      exact propertyIH later (witness later laterAfter)
  | eventually property propertyIH =>
      intro witness
      exact ⟨witness.1, witness.2.1.down, propertyIH witness.1 witness.2.2⟩

/-- Behavior-level constructive evidence packages one evidence producer for
    every trace, rather than merely asserting classical satisfaction. -/
def EvidenceProperty.Evidence {identity : Nat} {behavior : Model identity}
    (property : EvidenceProperty behavior) : Type :=
  ∀ trace : Trace behavior, property.EvidenceAt trace 0

theorem EvidenceProperty.evidence_implies_truth {identity : Nat}
    {behavior : Model identity} (property : EvidenceProperty behavior) :
    property.Evidence → property.erase.Holds := by
  intro evidence trace
  exact property.sound trace 0 (evidence trace)

#print axioms EvidenceProperty.sound
#print axioms EvidenceProperty.evidence_implies_truth

end NMLT.Behavior

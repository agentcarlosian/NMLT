import NMLT.Behavior.Refinement

namespace NMLT.Behavior.Coinductive

abbrev StateRelation {concreteId abstractId : Nat}
    (concrete : Model concreteId) (abstract : Model abstractId) :=
  concrete.State → abstract.State → Prop

def Included {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (left right : StateRelation concrete abstract) : Prop :=
  ∀ concreteState abstractState, left concreteState abstractState →
    right concreteState abstractState

/-- The one-sided simulation generator. Observations are related immediately;
    each concrete step is matched by a finite, possibly empty, abstract path. -/
def Generator {concreteId abstractId : Nat}
    (concrete : Model concreteId) (abstract : Model abstractId)
    (observeRel : concrete.Output → abstract.Output → Prop)
    (relation : StateRelation concrete abstract) : StateRelation concrete abstract :=
  fun concreteState abstractState =>
    observeRel (concrete.observe concreteState) (abstract.observe abstractState) ∧
      ∀ concreteAfter, concrete.stateStep concreteState concreteAfter →
        ∃ abstractAfter, Reaches abstract.stateStep abstractState abstractAfter ∧
          relation concreteAfter abstractAfter

theorem generatorMonotone {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    {observeRel : concrete.Output → abstract.Output → Prop}
    {left right : StateRelation concrete abstract} :
    Included left right →
      Included (Generator concrete abstract observeRel left)
        (Generator concrete abstract observeRel right) := by
  intro inclusion concreteState abstractState generated
  constructor
  · exact generated.1
  · intro concreteAfter step
    obtain ⟨abstractAfter, path, related⟩ := generated.2 concreteAfter step
    exact ⟨abstractAfter, path, inclusion concreteAfter abstractAfter related⟩

abbrev Transformer {concreteId abstractId : Nat}
    (concrete : Model concreteId) (abstract : Model abstractId) :=
  StateRelation concrete abstract → StateRelation concrete abstract

def Monotone {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (transform : Transformer concrete abstract) : Prop :=
  ∀ {left right}, Included left right → Included (transform left) (transform right)

def Extensive {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (transform : Transformer concrete abstract) : Prop :=
  ∀ relation, Included relation (transform relation)

def IdempotentBelow {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (transform : Transformer concrete abstract) : Prop :=
  ∀ relation, Included (transform (transform relation)) (transform relation)

def Compatible {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (observeRel : concrete.Output → abstract.Output → Prop)
    (transform : Transformer concrete abstract) : Prop :=
  ∀ relation,
    Included (transform (Generator concrete abstract observeRel relation))
      (Generator concrete abstract observeRel (transform relation))

def PostFixed {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (observeRel : concrete.Output → abstract.Output → Prop)
    (relation : StateRelation concrete abstract) : Prop :=
  Included relation (Generator concrete abstract observeRel relation)

/-- Proof-relevant greatest-postfixed-point presentation, avoiding an appeal to
    a classical powerset construction. -/
def Refines {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (observeRel : concrete.Output → abstract.Output → Prop)
    (concreteState : concrete.State) (abstractState : abstract.State) : Prop :=
  ∃ relation, PostFixed observeRel relation ∧ relation concreteState abstractState

/-- A finite certificate candidate may close its recursive obligations only
    after applying an explicitly named up-to transformer. -/
structure UpToCertificate {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (observeRel : concrete.Output → abstract.Output → Prop)
    (transform : Transformer concrete abstract) where
  relation : StateRelation concrete abstract
  closes : Included relation
    (Generator concrete abstract observeRel (transform relation))

theorem compatibleUpToSound {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    {observeRel : concrete.Output → abstract.Output → Prop}
    {transform : Transformer concrete abstract}
    (monotone : Monotone transform)
    (idempotent : IdempotentBelow transform)
    (compatible : Compatible observeRel transform)
    (certificate : UpToCertificate observeRel transform) :
    PostFixed observeRel (transform certificate.relation) := by
  intro concreteState abstractState transformed
  have lifted :
      transform (Generator concrete abstract observeRel (transform certificate.relation))
        concreteState abstractState :=
    monotone certificate.closes concreteState abstractState transformed
  have generatedTwice :
      Generator concrete abstract observeRel (transform (transform certificate.relation))
        concreteState abstractState :=
    compatible (transform certificate.relation) concreteState abstractState lifted
  exact generatorMonotone (idempotent certificate.relation)
    concreteState abstractState generatedTwice

theorem certificateRefines {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    {observeRel : concrete.Output → abstract.Output → Prop}
    {transform : Transformer concrete abstract}
    (monotone : Monotone transform)
    (extensive : Extensive transform)
    (idempotent : IdempotentBelow transform)
    (compatible : Compatible observeRel transform)
    (certificate : UpToCertificate observeRel transform)
    {concreteState : concrete.State} {abstractState : abstract.State}
    (member : certificate.relation concreteState abstractState) :
    Refines observeRel concreteState abstractState := by
  exact ⟨transform certificate.relation,
    compatibleUpToSound monotone idempotent compatible certificate,
    extensive certificate.relation concreteState abstractState member⟩

#print axioms generatorMonotone
#print axioms compatibleUpToSound
#print axioms certificateRefines

end NMLT.Behavior.Coinductive

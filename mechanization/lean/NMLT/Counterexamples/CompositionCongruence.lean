import NMLT.Core.Transition

namespace NMLT.Counterexamples.CompositionCongruence

open NMLT

inductive SenderLabel
  | ping

inductive ReceiverLabel
  | receive

/-- The abstract sender has no steps. -/
def abstractSender : LTS SenderLabel Bool where
  State := Unit
  init _ := True
  step _ _ _ := False
  observe _ := false

/-- The concrete sender has an allegedly hidden `ping` step that leaves its
    own state and observation unchanged. -/
def concreteSender : LTS SenderLabel Bool where
  State := Unit
  init _ := True
  step _ label _ := label = .ping
  observe _ := false

/-- The peer changes an observable bit when it receives the ping. -/
def receiver : LTS ReceiverLabel Bool where
  State := Bool
  init state := state = false
  step before label after :=
    label = .receive ∧ before = false ∧ after = true
  observe state := state

def connection : Connection SenderLabel ReceiverLabel where
  linked left right := left = .ping ∧ right = .receive

def concreteComposite := parallel concreteSender receiver connection
def abstractComposite := parallel abstractSender receiver connection

def senderHidden (_ : SenderLabel) : Bool := true

/-- Standalone, the concrete sender satisfies the RFC 0001 candidate: its ping
    is hidden and maps to equality of the sole abstract state. -/
def senderRefinement :
    WeakRefines concreteSender abstractSender senderHidden id where
  mapState _ := ()
  init _ := trivial
  observe _ := rfl
  hiddenStep _ _ := rfl
  visibleStep _ impossible := by
    nomatch impossible

def compositeHidden : ParallelLabel SenderLabel ReceiverLabel -> Bool
  | .sync _ _ => true
  | _ => false

theorem concreteSynchronization :
    concreteComposite.step ((), false) (.sync .ping .receive) ((), true) := by
  exact ⟨⟨rfl, rfl⟩, rfl, ⟨rfl, rfl, rfl⟩⟩

/-- Any proposed observation-preserving refinement between the composites
    yields the contradiction `false = true`. -/
theorem compositeRefinementImpossible
    (refinement : WeakRefines concreteComposite abstractComposite compositeHidden id) : False := by
  have mappedEqual :
      refinement.mapState ((), false) = refinement.mapState ((), true) := by
    exact refinement.hiddenStep concreteSynchronization rfl
  have abstractObservationsEqual := congrArg abstractComposite.observe mappedEqual
  have concreteObservationsEqual :
      concreteComposite.observe ((), false) = concreteComposite.observe ((), true) :=
    (refinement.observe ((), false)).trans
      (abstractObservationsEqual.trans (refinement.observe ((), true)).symm)
  have falseEqualsTrue : false = true := congrArg Prod.snd concreteObservationsEqual
  nomatch falseEqualsTrue

/-- Counterexample to unconditional refinement congruence. Although the sender
    refines the abstract sender in isolation, after composition its hidden ping
    synchronizes with a peer and changes the peer's observable state. No weak
    refinement satisfying observation preservation can relate the composites. -/
theorem noCompositeRefinement :
    ¬ Nonempty (WeakRefines concreteComposite abstractComposite compositeHidden id) := by
  intro witness
  rcases witness with ⟨refinement⟩
  exact compositeRefinementImpossible refinement

#print axioms senderRefinement
#print axioms concreteSynchronization
#print axioms compositeRefinementImpossible
#print axioms noCompositeRefinement

end NMLT.Counterexamples.CompositionCongruence

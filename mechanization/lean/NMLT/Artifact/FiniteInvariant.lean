import NMLT.Artifact.ExecutionWitness

namespace NMLT.Artifact.FiniteInvariant
open NMLT.Artifact.SemanticClosure NMLT.Artifact.ExecutionClosure
open NMLT.Behavior.ResourceBehavior NMLT.Behavior.ResourceDynamics

def ownerVectors : Nat → List (List (Option Bool))
  | 0 => [[]]
  | n + 1 => [none, some false, some true].flatMap fun owner =>
      (ownerVectors n).map (owner :: ·)

theorem ownerVectors_complete (f : Fin n → Option Bool) :
    List.ofFn f ∈ ownerVectors n := by
  induction n with
  | zero => simp [ownerVectors]
  | succ n ih =>
    rw [List.ofFn_succ]
    apply List.mem_flatMap.mpr
    refine ⟨f 0, ?_, ?_⟩
    · cases h : f 0 with
      | none => simp
      | some b => cases b <;> simp
    · exact List.mem_map.mpr ⟨List.ofFn (fun i => f i.succ), ih _, rfl⟩

def fromOwners (owners : List (Option Bool)) : Fin n → Option Bool :=
  fun c => (owners[c.val]?).getD none

theorem fromOwners_ofFn (f : Fin n → Option Bool) : fromOwners (List.ofFn f) = f := by
  funext c
  simp [fromOwners, c.isLt]

def states (m : Model) : List m.State :=
  (List.finRange ((stateSpace m.program m.left).length + 1)).flatMap fun l =>
  (List.finRange ((stateSpace m.program m.right).length + 1)).flatMap fun r =>
  (ownerVectors ((capabilityNames m.program).length + 1)).map fun owners =>
    ⟨(l,r), ⟨fromOwners owners⟩⟩

theorem states_complete (m : Model) (s : m.State) : s ∈ states m := by
  rcases s with ⟨⟨l,r⟩,⟨owner⟩⟩
  change Fin ((stateSpace m.program m.left).length + 1) at l
  change Fin ((stateSpace m.program m.right).length + 1) at r
  apply List.mem_flatMap.mpr
  refine ⟨l, List.mem_finRange l, ?_⟩
  apply List.mem_flatMap.mpr
  refine ⟨r, List.mem_finRange r, ?_⟩
  apply List.mem_map.mpr
  exact ⟨List.ofFn owner, ownerVectors_complete owner, by rw [fromOwners_ofFn]⟩

def actions (m : Model) : List m.Act :=
  let ls := List.finRange (m.leftActions.length + 1)
  let rs := List.finRange (m.rightActions.length + 1)
  ls.map ProductAction.left ++ rs.map ProductAction.right ++
    ls.flatMap (fun l => rs.map (ProductAction.sync l))

theorem actions_complete (m : Model) (a : m.Act) : a ∈ actions m := by
  cases a <;> simp [actions]

def stateEqual (m : Model) (s t : m.State) : Bool :=
  ((s.control.1.val == t.control.1.val) && (s.control.2.val == t.control.2.val)) &&
    (List.finRange ((capabilityNames m.program).length + 1)).all
      (fun c => s.authority.owner c == t.authority.owner c)

theorem stateEqual_iff (m : Model) (s t : m.State) : stateEqual m s t = true ↔ s = t := by
  constructor
  · intro h
    have pair := Bool.and_eq_true_iff.mp h
    have fields := Bool.and_eq_true_iff.mp pair.1
    have controls : s.control = t.control := Prod.ext
      (Fin.eq_of_val_eq (of_decide_eq_true fields.1))
      (Fin.eq_of_val_eq (of_decide_eq_true fields.2))
    have owners : s.authority.owner = t.authority.owner := by
      funext c
      exact beq_iff_eq.mp (List.all_eq_true.mp pair.2 c (by simp))
    calc
      s = (⟨t.control, s.authority⟩ : m.State) :=
        congrArg (fun control => (⟨control, s.authority⟩ : m.State)) controls
      _ = t := congrArg (fun owner => (⟨t.control, ⟨owner⟩⟩ : m.State)) owners
  · rintro rfl
    apply Bool.and_eq_true_iff.mpr
    constructor
    · exact Bool.and_eq_true_iff.mpr ⟨decide_eq_true rfl, decide_eq_true rfl⟩
    · apply List.all_eq_true.mpr
      intro c _
      cases h : s.authority.owner c with
      | none => rfl
      | some b => cases b <;> rfl

def included (m : Model) (reached : List m.State) (s : m.State) : Bool :=
  reached.any (fun t => stateEqual m t s)

theorem included_iff (m : Model) (reached : List m.State) (s : m.State) :
    included m reached s = true ↔ s ∈ reached := by
  simp [included, List.any_eq_true, stateEqual_iff]

def initialization (m : Model) (reached : List m.State) (predicate : m.State → Bool) : Bool :=
  (states m).all fun s => !decide (m.behavior.init s) || (included m reached s && predicate s)

def preservation (m : Model) (reached : List m.State) (predicate : m.State → Bool) : Bool :=
  reached.all fun s => (actions m).all fun a => (states m).all fun t =>
    !decide (m.behavior.step s a t) || (included m reached t && predicate t)

theorem initialization_sound (m : Model) (reached : List m.State) (predicate : m.State → Bool)
    (h : initialization m reached predicate = true) (s : m.State) (hi : m.behavior.init s) :
    s ∈ reached ∧ predicate s = true := by
  have hc := List.all_eq_true.mp h s (states_complete m s)
  simpa [hi, included_iff] using hc

theorem preservation_sound (m : Model) (reached : List m.State) (predicate : m.State → Bool)
    (h : preservation m reached predicate = true)
    (s t : m.State) (a : m.Act) (hs : s ∈ reached) (step : m.behavior.step s a t) :
    t ∈ reached ∧ predicate t = true := by
  have hc := List.all_eq_true.mp (List.all_eq_true.mp (List.all_eq_true.mp h s hs)
    a (actions_complete m a)) t (states_complete m t)
  simpa [step, included_iff] using hc

theorem path_preserves (m : Model) (reached : List m.State) (predicate : m.State → Bool)
    (h : preservation m reached predicate = true)
    {s t : m.State} {as : List m.Act}
    (path : Path m.behavior s as t) (before : s ∈ reached ∧ predicate s = true) :
    t ∈ reached ∧ predicate t = true := by
  induction path with
  | nil => exact before
  | cons step _ ih => exact ih (preservation_sound m reached predicate h _ _ _ before.1 step)

theorem reachable_safe (m : Model) (reached : List m.State) (predicate : m.State → Bool)
    (hi : initialization m reached predicate = true)
    (hs : preservation m reached predicate = true)
    (s : m.State) (reachable : Reachable m.behavior s) : predicate s = true := by
  obtain ⟨initial, actions, initialized, path⟩ := reachable
  exact (path_preserves m reached predicate hs path
    (initialization_sound m reached predicate hi initial initialized)).2


end NMLT.Artifact.FiniteInvariant

import NMLT.Artifact.SemanticClosure
import NMLT.Artifact.FiniteAuthority
import NMLT.Behavior.ResourceDynamics

namespace NMLT.Artifact.ExecutionClosure

open Lean NMLT.Behavior.ResourceBehavior NMLT.Behavior.ResourceWorld
open NMLT.Artifact.BehaviorCore NMLT.Artifact.SemanticClosure
open NMLT.Artifact.FiniteAuthority
open NMLT.Behavior

structure Model where
  program : Program
  composition : Composition
  left : System
  right : System
  initialOwners : List (String × Option Bool)
  leftActionNames : Option (List String) := none
  rightActionNames : Option (List String) := none
  leftExtraHidden : List String := []

namespace Model

abbrev Cap (m : Model) := Fin ((capabilityNames m.program).length + 1)
def leftActions (m : Model) := m.leftActionNames.getD (m.left.actions.map Action.name)
def rightActions (m : Model) := m.rightActionNames.getD (m.right.actions.map Action.name)
def leftControl (m : Model) := toBehavior m.program m.left m.leftActions m.leftExtraHidden
def rightControl (m : Model) := toBehavior m.program m.right m.rightActions []

def connected (m : Model) (l : Fin (m.leftActions.length + 1))
    (r : Fin (m.rightActions.length + 1)) : Prop :=
  match nameAt? m.leftActions l, nameAt? m.rightActions r with
  | some left, some right => m.composition.connections.any (fun c =>
      (c.leftSystem == m.left.name && c.leftAction == left &&
       c.rightSystem == m.right.name && c.rightAction == right) ||
      (c.rightSystem == m.left.name && c.rightAction == left &&
       c.leftSystem == m.right.name && c.leftAction == right)) = true
  | _, _ => False

instance connectedDecidable (m : Model) (l r) : Decidable (m.connected l r) := by
  unfold connected
  split <;> infer_instance

def initialWorld (m : Model) : AuthorityWorld m.Cap Bool where
  owner := fun c => (nameAt? (capabilityNames m.program) c).bind fun name =>
    (m.initialOwners.lookup name).getD none

def initializedWorld (m : Model) (world : AuthorityWorld m.Cap Bool) : Prop :=
  ∀ c, world.owner c = m.initialWorld.owner c

def leftBehavior (m : Model) := ResourceDynamics.ofBehavior m.leftControl false m.initializedWorld
def rightBehavior (m : Model) := ResourceDynamics.ofBehavior m.rightControl true m.initializedWorld
def behavior (m : Model) := ResourceDynamics.parallel m.leftBehavior m.rightBehavior m.connected
abbrev State (m : Model) := m.behavior.State
abbrev Act (m : Model) := ProductAction (Fin (m.leftActions.length + 1)) (Fin (m.rightActions.length + 1))

def initialCondition (m : Model) (s : m.State) : Prop :=
  m.leftControl.init s.control.1 ∧ m.rightControl.init s.control.2 ∧ m.initializedWorld s.authority

instance (m : Model) (s : m.State) : Decidable (m.initialCondition s) := by
  letI : Decidable (m.leftControl.init s.control.1) :=
    behaviorInitDecidable m.program m.left m.leftActions m.leftExtraHidden s.control.1
  letI : Decidable (m.rightControl.init s.control.2) :=
    behaviorInitDecidable m.program m.right m.rightActions [] s.control.2
  letI : Decidable (m.initializedWorld s.authority) :=
    inferInstanceAs (Decidable (∀ c : Fin ((capabilityNames m.program).length + 1),
      s.authority.owner c = m.initialWorld.owner c))
  unfold initialCondition
  infer_instance

theorem initial_iff (m : Model) (s : m.State) : m.initialCondition s ↔ m.behavior.init s := by
  constructor
  · rintro ⟨a,b,c⟩; exact ⟨⟨a,b⟩,c,c⟩
  · rintro ⟨⟨a,b⟩,c,_⟩; exact ⟨a,b,c⟩

instance (m : Model) (s : m.State) : Decidable (m.behavior.init s) :=
  decidable_of_iff (m.initialCondition s) (m.initial_iff s)

def stepCondition (m : Model) (s : m.State) (a : m.Act) (t : m.State) : Prop :=
  match a with
  | .left a => m.leftControl.step s.control.1 a t.control.1 ∧
      s.control.2 = t.control.2 ∧ (∀ r, ¬ m.connected a r) ∧
      LocalStep false (m.leftControl.resources a) s.authority t.authority
  | .right a => m.rightControl.step s.control.2 a t.control.2 ∧
      s.control.1 = t.control.1 ∧ (∀ l, ¬ m.connected l a) ∧
      LocalStep true (m.rightControl.resources a) s.authority t.authority
  | .sync l r => m.connected l r ∧
      m.leftControl.step s.control.1 l t.control.1 ∧
      m.rightControl.step s.control.2 r t.control.2 ∧
      SyncStep false true (m.leftControl.resources l) (m.rightControl.resources r) s.authority t.authority

instance (m : Model) (s t : m.State) (a : m.Act) : Decidable (m.stepCondition s a t) := by
  letI : Decidable (s.control.1 = t.control.1) :=
    decidable_of_iff (s.control.1.val = t.control.1.val) ⟨Fin.eq_of_val_eq, congrArg Fin.val⟩
  letI : Decidable (s.control.2 = t.control.2) :=
    decidable_of_iff (s.control.2.val = t.control.2.val) ⟨Fin.eq_of_val_eq, congrArg Fin.val⟩
  letI : (l : Fin (m.leftActions.length + 1)) → Decidable (m.leftControl.step s.control.1 l t.control.1) :=
    fun l => behaviorStepDecidable m.program m.left m.leftActions m.leftExtraHidden s.control.1 l t.control.1
  letI : (r : Fin (m.rightActions.length + 1)) → Decidable (m.rightControl.step s.control.2 r t.control.2) :=
    fun r => behaviorStepDecidable m.program m.right m.rightActions [] s.control.2 r t.control.2
  cases a <;> simp only [stepCondition] <;> unfold leftControl rightControl at * <;> infer_instance

theorem step_iff (m : Model) (s t : m.State) (a : m.Act) :
    m.stepCondition s a t ↔ m.behavior.step s a t := by
  rcases s with ⟨⟨sl,sr⟩,sw⟩
  rcases t with ⟨⟨tl,tr⟩,tw⟩
  cases a with
  | left a =>
    constructor
    · rintro ⟨step, same, isolated, effect⟩
      cases same
      exact ⟨ProductStep.fromLeft step isolated, effect⟩
    · rintro ⟨control,effect⟩
      cases control with
      | fromLeft step isolated => exact ⟨step,rfl,isolated,effect⟩
  | right a =>
    constructor
    · rintro ⟨step, same, isolated, effect⟩
      cases same
      exact ⟨ProductStep.fromRight step isolated, effect⟩
    · rintro ⟨control,effect⟩
      cases control with
      | fromRight step isolated => exact ⟨step,rfl,isolated,effect⟩
  | sync l r =>
    constructor
    · rintro ⟨connected,ls,rs,effect⟩
      exact ⟨ProductStep.synchronize connected ls rs,effect⟩
    · rintro ⟨control,effect⟩
      cases control with
      | synchronize connected ls rs => exact ⟨connected,ls,rs,effect⟩

instance (m : Model) (s t : m.State) (a : m.Act) : Decidable (m.behavior.step s a t) :=
  decidable_of_iff (m.stepCondition s a t) (m.step_iff s t a)

def formationCondition (m : Model) : Prop :=
  (∀ l r, m.connected l r →
    Direction.Complementary (m.leftControl.direction l) (m.rightControl.direction r) ∧
    m.leftControl.payload l = m.rightControl.payload r ∧
    ¬ m.leftControl.hidden l ∧ ¬ m.rightControl.hidden r ∧
    SynchronizationCompatible (m.leftControl.resources l) (m.rightControl.resources r)) ∧
  (∀ c, m.leftControl.owns c → ¬ m.rightControl.owns c)

instance (m : Model) : Decidable m.formationCondition := by
  unfold formationCondition leftControl rightControl
  infer_instance

theorem formed (m : Model) (h : m.formationCondition) :
    ResourceDynamics.Composable m.leftBehavior m.rightBehavior m.connected where
  interface := {
    directions := fun connected => (h.1 _ _ connected).1
    payloads := fun connected => (h.1 _ _ connected).2.1
    hiddenLeft := fun connected => (h.1 _ _ connected).2.2.1
    hiddenRight := fun connected => (h.1 _ _ connected).2.2.2.1
    capabilities := h.2
    resources := fun connected =>
      ⟨fun _ => (h.1 _ _ connected).2.2.2.2,
       fun _ => (h.1 _ _ connected).2.2.2.2.symm⟩
  }
  participants := fun {l r} _ =>
    ⟨false,true,m.leftControl.resources l,m.rightControl.resources r,rfl,rfl,Bool.false_ne_true⟩

end Model

def Model.TransferTo (m : Model) (a : m.Act) (c : m.Cap) (actor : Bool) : Prop :=
  match a with
  | .sync l r => (m.leftControl.resources l |>.transfers c) ∧ actor = true ∨
      (m.rightControl.resources r |>.transfers c) ∧ actor = false
  | _ => False

theorem Model.step_owned_origin (m : Model) {s t : m.State} {a : m.Act} {c : m.Cap} {actor : Bool}
    (step : m.behavior.step s a t) (owned : t.authority.Owns actor c) :
    s.authority.Owns actor c ∨ m.TransferTo a c actor := by
  cases a with
  | left a =>
    have effect : LocalStep false (m.leftControl.resources a) s.authority t.authority := step.2
    have notConsumed : ¬ (m.leftControl.resources a).consumes c := by
      intro consumed
      have vacant := effect.consumed c consumed
      have impossible : (none : Option Bool) = some actor := vacant.symm.trans owned
      cases impossible
    exact Or.inl ((effect.preserved c notConsumed).symm.trans owned)
  | right a =>
    have effect : LocalStep true (m.rightControl.resources a) s.authority t.authority := step.2
    have notConsumed : ¬ (m.rightControl.resources a).consumes c := by
      intro consumed
      have vacant := effect.consumed c consumed
      have impossible : (none : Option Bool) = some actor := vacant.symm.trans owned
      cases impossible
    exact Or.inl ((effect.preserved c notConsumed).symm.trans owned)
  | sync l r =>
    have effect : SyncStep false true (m.leftControl.resources l) (m.rightControl.resources r) s.authority t.authority := step.2
    unfold Model.leftControl Model.rightControl at effect
    rcases effect.owner_after_is_explained c with vacant | ⟨transfer,received⟩ | ⟨transfer,received⟩ | same
    · have impossible : (none : Option Bool) = some actor := vacant.symm.trans owned
      cases impossible
    · exact Or.inr (Or.inl ⟨transfer,t.authority.ownership_unique owned received⟩)
    · exact Or.inr (Or.inr ⟨transfer,t.authority.ownership_unique owned received⟩)
    · exact Or.inl (same.symm.trans owned)

theorem Model.path_owned_origin (m : Model) {s t : m.State} {actions : List m.Act}
    {c : m.Cap} {actor : Bool} (path : ResourceDynamics.Path m.behavior s actions t)
    (owned : t.authority.Owns actor c) :
    s.authority.Owns actor c ∨ ∃ a, a ∈ actions ∧ m.TransferTo a c actor := by
  induction path with
  | nil => exact Or.inl owned
  | @cons before middle after action actions step tail ih =>
    rcases ih owned with atMiddle | ⟨a,member,transfer⟩
    · rcases m.step_owned_origin step atMiddle with atStart | transfer
      · exact Or.inl atStart
      · exact Or.inr ⟨action,List.mem_cons_self,transfer⟩
    · exact Or.inr ⟨a,List.mem_cons_of_mem action member,transfer⟩

theorem Model.sync_left_moves_once (m : Model) {s t : m.State} {l r} {c : m.Cap}
    (step : m.behavior.step s (.sync l r) t)
    (transfer : (m.leftControl.resources l).transfers c) :
    t.authority.Owns true c ∧ ¬ t.authority.Owns false c := by
  have effect : SyncStep false true (m.leftControl.resources l) (m.rightControl.resources r) s.authority t.authority := step.2
  have received := effect.transferredLeft c transfer
  exact ⟨received, fun other => Bool.false_ne_true (t.authority.ownership_unique other received)⟩

theorem Model.sync_right_moves_once (m : Model) {s t : m.State} {l r} {c : m.Cap}
    (step : m.behavior.step s (.sync l r) t)
    (transfer : (m.rightControl.resources r).transfers c) :
    t.authority.Owns false c ∧ ¬ t.authority.Owns true c := by
  have effect : SyncStep false true (m.leftControl.resources l) (m.rightControl.resources r) s.authority t.authority := step.2
  have received := effect.transferredRight c transfer
  exact ⟨received, fun other => Bool.false_ne_true (t.authority.ownership_unique received other)⟩

end NMLT.Artifact.ExecutionClosure

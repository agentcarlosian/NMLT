/-
  Case 7 first slice for the Paper 1 small LTS: capability/grade/rely through
  `NMLT.Core.Transition.parallel` (labels `ParallelLabel` = left | right | sync)
  under `compositeMapOf`.

  This is *not* the T4/mapped OpenComposition model
  (`liftOpenProductResources` / `liftResourceAwareParallel`). Those remain
  the exact-action resource congruence. Case 7 here is the small-model
  homomorphism fragment.

  `ResourceRefinement` is action-indexed independently of `LTS.step`; this
  file does not extend `LTS.step` with capabilities.
-/
import NMLT.Core.Transition
import NMLT.Behavior.OpenResourceCongruence
import NMLT.Behavior.WeakConditionalCongruence

namespace NMLT.Behavior.WeakResourceCongruence

open NMLT
open NMLT.Grades
open NMLT.Behavior.OpenResourceCongruence
open NMLT.Behavior.WeakConditionalCongruence

/-- Per-action resources of a small-LTS product. Independent left/right
    steps keep the originating component's profile. A synchronization uses
    `parallelAction`, which internalizes transfer/receive. -/
def smallProductActionResources
    {LeftAction RightAction Capability Fact : Type}
    (left : SystemResources LeftAction Capability Fact)
    (right : SystemResources RightAction Capability Fact) :
    ParallelLabel LeftAction RightAction → ActionResources Capability Fact
  | .left l => left.action l
  | .right r => right.action r
  | .sync l r => parallelAction (left.action l) (right.action r)

/-- System-level resources of a small-LTS product. Owned authority is the
    union, matching `parallelSystemResources`, but actions are indexed by
    `ParallelLabel` rather than `Left × Right`. -/
def smallProductSystemResources
    {LeftAction RightAction Capability Fact : Type}
    (left : SystemResources LeftAction Capability Fact)
    (right : SystemResources RightAction Capability Fact) :
    SystemResources (ParallelLabel LeftAction RightAction) Capability Fact where
  owned := fun capability => left.owned capability ∨ right.owned capability
  action := smallProductActionResources left right

/-- Reflexive per-action resource refinement (fixed peer profile). -/
def actionResourceRefinement_id {Capability Fact : Type}
    (resources : ActionResources Capability Fact) :
    ActionResourceRefinement resources resources where
  requires := fun _ h => h
  consumes := fun _ => Iff.intro id id
  transfers := fun _ => Iff.intro id id
  receives := fun _ => Iff.intro id id
  grade := ⟨Nat.le_refl _, Nat.le_refl _, Nat.le_refl _, Nat.le_refl _⟩
  rely := fun _ h => h
  guarantees := fun _ h => h

/-- Small-LTS analogue of `ActionResourceRefinement.parallel`. Same field
    obligations, proved primitively so Case 7 stays off `or_congr` /
    `sequential_mono` (`omega` / `Quot.sound`). -/
def actionResourceRefinement_parallel
    {Capability Fact : Type}
    {concreteLeft abstractLeft concreteRight abstractRight :
      ActionResources Capability Fact}
    (left : ActionResourceRefinement concreteLeft abstractLeft)
    (right : ActionResourceRefinement concreteRight abstractRight) :
    ActionResourceRefinement (parallelAction concreteLeft concreteRight)
      (parallelAction abstractLeft abstractRight) where
  requires := fun capability required =>
    match required with
    | .inl required => Or.inl (left.requires capability required)
    | .inr required => Or.inr (right.requires capability required)
  consumes := fun capability =>
    Iff.intro
      (fun consumed =>
        match consumed with
        | .inl consumed => Or.inl ((left.consumes capability).mp consumed)
        | .inr consumed => Or.inr ((right.consumes capability).mp consumed))
      (fun consumed =>
        match consumed with
        | .inl consumed => Or.inl ((left.consumes capability).mpr consumed)
        | .inr consumed => Or.inr ((right.consumes capability).mpr consumed))
  transfers := fun _ => Iff.intro id id
  receives := fun _ => Iff.intro id id
  grade :=
    ⟨Nat.add_le_add left.grade.1 right.grade.1,
     Nat.add_le_add left.grade.2.1 right.grade.2.1,
     Nat.add_le_add left.grade.2.2.1 right.grade.2.2.1,
     Nat.le_min.mpr
       ⟨Nat.min_le_left _ _,
        Nat.le_trans (Nat.min_le_right _ _)
          (Nat.add_le_add left.grade.2.2.2 right.grade.2.2.2)⟩⟩
  rely := fun fact relied =>
    match relied with
    | .inl leftRelied =>
        Or.inl ⟨left.rely fact leftRelied.1,
          fun abstractGuaranteed =>
            leftRelied.2 (right.guarantees fact abstractGuaranteed)⟩
    | .inr rightRelied =>
        Or.inr ⟨right.rely fact rightRelied.1,
          fun abstractGuaranteed =>
            rightRelied.2 (left.guarantees fact abstractGuaranteed)⟩
  guarantees := fun fact guaranteed =>
    match guaranteed with
    | .inl guaranteed => Or.inl (left.guarantees fact guaranteed)
    | .inr guaranteed => Or.inr (right.guarantees fact guaranteed)

/-- Every `ParallelLabel` constructor has a corresponding per-action
    resource refinement against a fixed peer. Reuses
    `ResourceRefinement.actionRefinement` on the left component and the
    `ActionResourceRefinement.parallel` closure (primitive form above)
    on synchronizations. -/
def liftParallelActionResources
    {ConcreteLeft AbstractLeft RightAction Capability Fact : Type}
    {concreteLeft : SystemResources ConcreteLeft Capability Fact}
    {abstractLeft : SystemResources AbstractLeft Capability Fact}
    {peer : SystemResources RightAction Capability Fact}
    {mapLeft : ConcreteLeft → AbstractLeft}
    (left : ResourceRefinement concreteLeft abstractLeft mapLeft)
    (action : ParallelLabel ConcreteLeft RightAction) :
    ActionResourceRefinement
      (smallProductActionResources concreteLeft peer action)
      (smallProductActionResources abstractLeft peer
        (compositeMapOf mapLeft action)) :=
  match action with
  | .left l => left.actionRefinement l
  | .right r => actionResourceRefinement_id (peer.action r)
  | .sync l r =>
      actionResourceRefinement_parallel
        (left.actionRefinement l)
        (actionResourceRefinement_id (peer.action r))

/-- Case 7 homomorphism fragment: a left-component `ResourceRefinement`
    lifts through `parallel` under `compositeMapOf`, with identical peer
    resources on both products (fixed D). Not OpenComposition, not I-FAIR,
    not a claim that T5 is CONDITIONAL-CONGRUENCE. -/
def liftParallelResources
    {ConcreteLeft AbstractLeft RightAction Capability Fact : Type}
    {concreteLeft : SystemResources ConcreteLeft Capability Fact}
    {abstractLeft : SystemResources AbstractLeft Capability Fact}
    {peer : SystemResources RightAction Capability Fact}
    {mapLeft : ConcreteLeft → AbstractLeft}
    (left : ResourceRefinement concreteLeft abstractLeft mapLeft) :
    ResourceRefinement
      (smallProductSystemResources concreteLeft peer)
      (smallProductSystemResources abstractLeft peer)
      (compositeMapOf mapLeft) where
  owned := fun capability owned =>
    match owned with
    | .inl owned => Or.inl (left.owned capability owned)
    | .inr owned => Or.inr owned
  requires := fun action =>
    (liftParallelActionResources left action).requires
  consumes := fun action =>
    (liftParallelActionResources left action).consumes
  transfers := fun action =>
    (liftParallelActionResources left action).transfers
  receives := fun action =>
    (liftParallelActionResources left action).receives
  grade := fun action =>
    (liftParallelActionResources left action).grade
  rely := fun action =>
    (liftParallelActionResources left action).rely
  guarantees := fun action =>
    (liftParallelActionResources left action).guarantees

#print axioms actionResourceRefinement_id
#print axioms actionResourceRefinement_parallel
#print axioms liftParallelActionResources
#print axioms liftParallelResources

/-! ### Positive instantiation: EmptyWiringTau (T5b) + matching inert profiles -/

namespace EmptyWiringInert

open EmptyWiringTau

/-- Shared stutter profile: no consume, epsilon grade, no rely. -/
def inertAction : ActionResources Empty Empty where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

def tauInert : SystemResources TauLabel Empty Empty where
  owned := fun _ => False
  action := fun _ => inertAction

def peerInert : SystemResources Unit Empty Empty where
  owned := fun _ => False
  action := fun _ => inertAction

/-- Matching inert stutter on the left tau (concrete and abstract). -/
theorem tauInertRefinement :
    ResourceRefinement tauInert tauInert (id : TauLabel → TauLabel) where
  owned := fun _ h => h
  requires := fun _ _ h => h
  consumes := fun _ _ => Iff.intro id id
  transfers := fun _ _ => Iff.intro id id
  receives := fun _ _ => Iff.intro id id
  grade := fun _ => ⟨Nat.le_refl 0, Nat.le_refl 0, Nat.le_refl 0, Nat.le_refl 0⟩
  rely := fun _ _ h => h
  guarantees := fun _ _ h => h

/-- First compose resource homomorphism on the small model: T5b product
    `WeakRefines` plus matching inert profiles lift through
    `liftParallelResources`. Ceiling: empty wiring, matching profiles,
    default `compositeMapOf`. Not I-FAIR, not hidden-connected ping, not
    OpenComposition. -/
theorem emptyWiring_inert_productResourceRefinement :
    Nonempty
      (WeakRefines
        (parallel concreteTau peerIdle emptyConn)
        (parallel abstractTau peerIdle emptyConn)
        (compositeHiddenOf tauHidden)
        (compositeMapOf (id : TauLabel → TauLabel))) ∧
      ResourceRefinement
        (smallProductSystemResources tauInert peerInert)
        (smallProductSystemResources tauInert peerInert)
        (compositeMapOf (id : TauLabel → TauLabel)) :=
  ⟨emptyWiring_productRefinement, liftParallelResources tauInertRefinement⟩

#print axioms tauInertRefinement
#print axioms emptyWiring_inert_productResourceRefinement

end EmptyWiringInert

/-! ### Negative independence: T5 does not give Case 7 / I-CAP through compose

  Same EmptyWiringTau systems, so InterfaceCompatible / T5 product
  WeakRefines still holds. The hidden left tau is decorated with a
  `token` consume unmatched by the abstract epsilon stutter (HiddenConsume
  pattern). Product ResourceRefinement fails. Ceiling: T5 ⇏ I-CAP through
  compose. Not a liveness result.
-/

namespace EmptyWiringHiddenConsume

open EmptyWiringTau

inductive TokenCap
  | token
  deriving DecidableEq

def tauConsumeAction : ActionResources TokenCap Empty where
  requires := fun c => c = .token
  consumes := fun c => c = .token
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

def tauNoConsumeAction : ActionResources TokenCap Empty where
  requires := fun c => c = .token
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

def peerIdleAction : ActionResources TokenCap Empty where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

def concreteTauConsume : SystemResources TauLabel TokenCap Empty where
  owned := fun c => c = .token
  action := fun
    | .tau => tauConsumeAction

def abstractTauNoConsume : SystemResources TauLabel TokenCap Empty where
  owned := fun c => c = .token
  action := fun
    | .tau => tauNoConsumeAction

def peerIdleResources : SystemResources Unit TokenCap Empty where
  owned := fun _ => False
  action := fun _ => peerIdleAction

theorem concrete_product_left_tau_consumes_token :
    ((smallProductSystemResources concreteTauConsume peerIdleResources).action
      (.left TauLabel.tau)).consumes TokenCap.token :=
  rfl

theorem abstract_product_left_tau_does_not_consume_token :
    ¬ ((smallProductSystemResources abstractTauNoConsume peerIdleResources).action
        (.left TauLabel.tau)).consumes TokenCap.token :=
  fun h => h

/-- T5 product `WeakRefines` still holds (observation only). The extra
    hidden-tau consume is unmatched by the abstract product stutter, so
    product `ResourceRefinement` fails along `compositeMapOf id`. -/
theorem emptyWiring_hiddenConsume_breaks_productResourceRefinement :
    Nonempty
      (WeakRefines
        (parallel concreteTau peerIdle emptyConn)
        (parallel abstractTau peerIdle emptyConn)
        (compositeHiddenOf tauHidden)
        (compositeMapOf (id : TauLabel → TauLabel))) ∧
      ¬ ResourceRefinement
          (smallProductSystemResources concreteTauConsume peerIdleResources)
          (smallProductSystemResources abstractTauNoConsume peerIdleResources)
          (compositeMapOf (id : TauLabel → TauLabel)) :=
  ⟨emptyWiring_productRefinement, fun refinement =>
    abstract_product_left_tau_does_not_consume_token
      ((refinement.consumes (.left TauLabel.tau) TokenCap.token).mp
        concrete_product_left_tau_consumes_token)⟩

#print axioms emptyWiring_hiddenConsume_breaks_productResourceRefinement

end EmptyWiringHiddenConsume

end NMLT.Behavior.WeakResourceCongruence

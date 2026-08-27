/-
  Case 7 small-LTS product resource pack: capability/grade/rely through
  `NMLT.Core.Transition.parallel` (labels `ParallelLabel` = left | right | sync)
  under `compositeMapOf`.

  This is *not* the T4/mapped OpenComposition model
  (`liftOpenProductResources` / `liftResourceAwareParallel`). Those remain
  the exact-action resource congruence. Case 7 here is the small-model
  homomorphism fragment plus EmptyWiringTau independence, VisibleSync
  I-CAP transfer independence, and VisibleSync I-GRADE through *sync*.

  Empty-wiring product pack (frozen): matching inert profiles lift; unmatched
  hidden-left consume, grade, and rely fail product `ResourceRefinement`
  while T5 product `WeakRefines` still holds.

  VisibleSync I-CAP transfer: unmatched ping transfer is not
  `SynchronizationCompatible` while T5 product `WeakRefines` (and product
  `ResourceRefinement`, because `parallelAction` zeros transfer/receive)
  still hold. Matching transfer is the extra I-CAP premise; it is not
  implied by T5.

  VisibleSync I-GRADE through sync: unmatched ping cost fails product
  `ResourceRefinement.grade` at `.sync ping receive` while T5a still holds.
  Matching epsilon grades lift the sync grade inequality.

  Not I-FAIR through sync.

  `ResourceRefinement` is action-indexed independently of `LTS.step`; this
  file does not extend `LTS.step` with capabilities.
-/
import NMLT.Core.Transition
import NMLT.Behavior.OpenResourceCongruence
import NMLT.Behavior.WeakConditionalCongruence
import NMLT.Counterexamples.HiddenGrade
import NMLT.Counterexamples.HiddenRely

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

/-! ### Negative independence: T5 does not give I-GRADE through compose

  Same EmptyWiringTau systems, so InterfaceCompatible / T5 product
  WeakRefines still holds. The hidden left tau is decorated with a
  positive cost atom unmatched by the abstract epsilon (`zero`) stutter
  (HiddenGrade pattern, on the product via `smallProductSystemResources`).
  Product `ResourceRefinement.grade` fails. Ceiling: T5 ⇏ I-GRADE through
  compose. Not I-FAIR, not sync.
-/

namespace EmptyWiringHiddenGrade

open EmptyWiringTau
open EmptyWiringInert
open NMLT.Counterexamples.HiddenGrade

/-- Hidden left tau carries `pingCost`. Capability, consume, transfer, and
    rely/guarantee fields stay inert so the failure isolates I-GRADE. -/
def concreteTauGrade : SystemResources TauLabel Empty Empty where
  owned := fun _ => False
  action := fun
    | .tau => pingCostAction

/-- Abstract stutter on the same tau *name*: epsilon (`zero`) grade.
    `ResourceRefinement.grade` requires `Le concrete.grade abstract.grade`. -/
def abstractTauEpsilon : SystemResources TauLabel Empty Empty where
  owned := fun _ => False
  action := fun
    | .tau => pingEpsilonAction

theorem concrete_product_left_tau_grade_is_cost :
    ((smallProductSystemResources concreteTauGrade peerInert).action
      (.left TauLabel.tau)).grade = pingCost :=
  rfl

theorem abstract_product_left_tau_grade_is_epsilon :
    ((smallProductSystemResources abstractTauEpsilon peerInert).action
      (.left TauLabel.tau)).grade = zero :=
  rfl

/-- T5 product `WeakRefines` still holds (observation only). The extra
    hidden-tau cost is unmatched by the abstract product stutter, so
    product `ResourceRefinement.grade` fails along `compositeMapOf id`. -/
theorem emptyWiring_hiddenGrade_breaks_productResourceRefinement :
    Nonempty
      (WeakRefines
        (parallel concreteTau peerIdle emptyConn)
        (parallel abstractTau peerIdle emptyConn)
        (compositeHiddenOf tauHidden)
        (compositeMapOf (id : TauLabel → TauLabel))) ∧
      ¬ ResourceRefinement
          (smallProductSystemResources concreteTauGrade peerInert)
          (smallProductSystemResources abstractTauEpsilon peerInert)
          (compositeMapOf (id : TauLabel → TauLabel)) :=
  ⟨emptyWiring_productRefinement, fun refinement =>
    pingCost_not_le_zero (refinement.grade (.left TauLabel.tau))⟩

#print axioms emptyWiring_hiddenGrade_breaks_productResourceRefinement

end EmptyWiringHiddenGrade

/-! ### Negative independence: T5 does not give I-RELY through compose

  Same EmptyWiringTau systems, so InterfaceCompatible / T5 product
  WeakRefines still holds. The hidden left tau relies on extra `ready`
  Fact unmatched by the abstract stutter (HiddenRely pattern, on the
  product via `smallProductSystemResources`). Product
  `ResourceRefinement.rely` fails. Ceiling: T5 ⇏ I-RELY through compose.
  Not I-FAIR, not sync.
-/

namespace EmptyWiringHiddenRely

open EmptyWiringTau
open NMLT.Counterexamples.HiddenRely

/-- Idle peer: no rely, epsilon grade. Reuses the HiddenRely inert stutter
    action so Capability/Fact stay `Empty`/`EnvFact`. -/
def peerIdleRely : SystemResources Unit Empty EnvFact where
  owned := fun _ => False
  action := fun _ => pingNoRelyAction

/-- Hidden left tau relies on `ready`. Capability, consume, transfer, and
    grade fields stay inert so the failure isolates I-RELY. -/
def concreteTauRely : SystemResources TauLabel Empty EnvFact where
  owned := fun _ => False
  action := fun
    | .tau => pingRelyAction

/-- Abstract stutter on the same tau *name*: does not rely on `ready`.
    `ResourceRefinement.rely` forbids widening assumptions. -/
def abstractTauNoRely : SystemResources TauLabel Empty EnvFact where
  owned := fun _ => False
  action := fun
    | .tau => pingNoRelyAction

theorem concrete_product_left_tau_relies_on_ready :
    ((smallProductSystemResources concreteTauRely peerIdleRely).action
      (.left TauLabel.tau)).rely EnvFact.ready :=
  rfl

theorem abstract_product_left_tau_does_not_rely_ready :
    ¬ ((smallProductSystemResources abstractTauNoRely peerIdleRely).action
        (.left TauLabel.tau)).rely EnvFact.ready :=
  fun h => h

/-- T5 product `WeakRefines` still holds (observation only). The extra
    hidden-tau `ready` rely is unmatched by the abstract product stutter,
    so product `ResourceRefinement.rely` fails along `compositeMapOf id`. -/
theorem emptyWiring_hiddenRely_breaks_productResourceRefinement :
    Nonempty
      (WeakRefines
        (parallel concreteTau peerIdle emptyConn)
        (parallel abstractTau peerIdle emptyConn)
        (compositeHiddenOf tauHidden)
        (compositeMapOf (id : TauLabel → TauLabel))) ∧
      ¬ ResourceRefinement
          (smallProductSystemResources concreteTauRely peerIdleRely)
          (smallProductSystemResources abstractTauNoRely peerIdleRely)
          (compositeMapOf (id : TauLabel → TauLabel)) :=
  ⟨emptyWiring_productRefinement, fun refinement =>
    abstract_product_left_tau_does_not_rely_ready
      ((refinement.rely (.left TauLabel.tau) EnvFact.ready)
        concrete_product_left_tau_relies_on_ready)⟩

#print axioms emptyWiring_hiddenRely_breaks_productResourceRefinement

end EmptyWiringHiddenRely


/-! ### Negative independence: T5 does not give I-CAP transfer on VisibleSync

  Same VisibleSync systems (visible ping wired to receive), so T5 product
  `WeakRefines` still holds. Concrete ping transfers `token`; the Lean
  `receiver` does not receive it (the reverse transfer/receive pair is
  empty). `SynchronizationCompatible` fails. Product `ResourceRefinement`
  can still hold via `liftParallelResources` because `parallelAction`
  internalizes (zeros) transfer/receive. Companion: T5 + product
  `ResourceRefinement` ⇏ `SynchronizationCompatible`. Ceiling: T5 ⇏
  I-CAP transfer on sync. Not hidden ping, not I-FAIR, not empty-wiring
  consume. Lean `receiver` is not the OpenSystem-receptive dual.
-/

namespace VisibleSyncUnmatchedTransfer

open VisibleSync
open NMLT.Counterexamples.CompositionCongruence

inductive TokenCap
  | token
  deriving DecidableEq

/-- Visible ping transfers `token`. Consume, grade, and rely stay inert so
    the failure isolates I-CAP transfer. -/
def pingTransferAction : ActionResources TokenCap Empty where
  requires := fun c => c = .token
  consumes := fun _ => False
  transfers := fun c => c = .token
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

/-- Lean `receiver` does not receive `token` (and does not transfer it). -/
def receiveNoReceiveAction : ActionResources TokenCap Empty where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

def pingTransferResources : SystemResources SenderLabel TokenCap Empty where
  owned := fun c => c = .token
  action := fun
    | .ping => pingTransferAction

def receiveNoReceiveResources : SystemResources ReceiverLabel TokenCap Empty where
  owned := fun _ => False
  action := fun
    | .receive => receiveNoReceiveAction

theorem ping_transfers_token :
    pingTransferAction.transfers TokenCap.token :=
  rfl

theorem receive_does_not_receive_token :
    ¬ receiveNoReceiveAction.receives TokenCap.token :=
  fun h => h

theorem receive_does_not_transfer_token :
    ¬ receiveNoReceiveAction.transfers TokenCap.token :=
  fun h => h

theorem ping_does_not_receive_token :
    ¬ pingTransferAction.receives TokenCap.token :=
  fun h => h

/-- `parallelAction` zeros transfer even when the ping/receive pair is
    unmatched. This is why product `ResourceRefinement` can hold. -/
theorem unmatched_parallelAction_zeros_transfer :
    ¬ (parallelAction pingTransferAction receiveNoReceiveAction).transfers
        TokenCap.token :=
  fun h => h

theorem pingTransferRefinement :
    ResourceRefinement pingTransferResources pingTransferResources
      (id : SenderLabel → SenderLabel) where
  owned := fun _ h => h
  requires := fun _ _ h => h
  consumes := fun _ _ => Iff.intro id id
  transfers := fun _ _ => Iff.intro id id
  receives := fun _ _ => Iff.intro id id
  grade := fun _ => ⟨Nat.le_refl 0, Nat.le_refl 0, Nat.le_refl 0, Nat.le_refl 0⟩
  rely := fun _ _ h => h
  guarantees := fun _ _ h => h

theorem ping_unmatched_not_synchronizationCompatible :
    ¬ SynchronizationCompatible pingTransferAction receiveNoReceiveAction :=
  fun compatible =>
    receive_does_not_receive_token
      ((compatible.leftTransfer TokenCap.token).mp ping_transfers_token)

/-- T5 product `WeakRefines` still holds (VisibleSync observation). Ping
    transfers `token` and receive does not receive it, so
    `SynchronizationCompatible` fails. Product `ResourceRefinement` still
    holds along `compositeMapOf id` because `parallelAction` internalizes
    transfer/receive. Companion: T5 + product `ResourceRefinement` ⇏
    I-CAP transfer on sync. -/
theorem visibleSync_unmatchedTransfer_not_synchronizationCompatible :
    Nonempty
      (WeakRefines
        concreteCompositeVisible
        abstractCompositeVisible
        (compositeHiddenOf senderVisible)
        (compositeMapOf (id : SenderLabel → SenderLabel))) ∧
      ¬ SynchronizationCompatible pingTransferAction receiveNoReceiveAction ∧
      ResourceRefinement
        (smallProductSystemResources pingTransferResources receiveNoReceiveResources)
        (smallProductSystemResources pingTransferResources receiveNoReceiveResources)
        (compositeMapOf (id : SenderLabel → SenderLabel)) :=
  ⟨visibleSync_productRefinement,
    ping_unmatched_not_synchronizationCompatible,
    liftParallelResources pingTransferRefinement⟩

#print axioms unmatched_parallelAction_zeros_transfer
#print axioms ping_unmatched_not_synchronizationCompatible
#print axioms visibleSync_unmatchedTransfer_not_synchronizationCompatible

end VisibleSyncUnmatchedTransfer

/-! ### Positive control: matching transfer is the extra I-CAP premise

  Same VisibleSync systems. Ping transfers `token` iff receive receives it;
  the reverse pair is empty. `SynchronizationCompatible` holds;
  `synchronized_transfer_exact` / `synchronized_rely_discharged` instantiate
  on this ping/receive pair. If profiles otherwise match, product
  `ResourceRefinement` via `liftParallelResources`. Ceiling: matching
  transfer is the extra I-CAP premise; it is not implied by T5.
-/

namespace VisibleSyncMatchedTransfer

open VisibleSync
open NMLT.Counterexamples.CompositionCongruence

inductive TokenCap
  | token
  deriving DecidableEq

def pingTransferAction : ActionResources TokenCap Empty where
  requires := fun c => c = .token
  consumes := fun _ => False
  transfers := fun c => c = .token
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

/-- Lean `receiver` receives `token`. Does not already own it. -/
def receiveReceiveAction : ActionResources TokenCap Empty where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun c => c = .token
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

def pingTransferResources : SystemResources SenderLabel TokenCap Empty where
  owned := fun c => c = .token
  action := fun
    | .ping => pingTransferAction

def receiveReceiveResources : SystemResources ReceiverLabel TokenCap Empty where
  owned := fun _ => False
  action := fun
    | .receive => receiveReceiveAction

theorem ping_transfers_token :
    pingTransferAction.transfers TokenCap.token :=
  rfl

theorem receive_receives_token :
    receiveReceiveAction.receives TokenCap.token :=
  rfl

def pingReceiveCompatible :
    SynchronizationCompatible pingTransferAction receiveReceiveAction where
  leftTransfer := fun _ => Iff.intro id id
  rightTransfer := fun _ => Iff.intro id id
  leftRely := fun _ h => h.elim
  rightRely := fun _ h => h.elim

theorem ping_receive_transfer_exact (capability : TokenCap) :
    (pingTransferAction.transfers capability ↔
      receiveReceiveAction.receives capability) ∧
      (receiveReceiveAction.transfers capability ↔
        pingTransferAction.receives capability) :=
  synchronized_transfer_exact pingReceiveCompatible capability

theorem ping_receive_rely_discharged (fact : Empty) :
    ¬ (parallelAction pingTransferAction receiveReceiveAction).rely fact :=
  synchronized_rely_discharged pingReceiveCompatible fact

theorem matched_capabilityPartition :
    CapabilityPartition pingTransferResources.owned receiveReceiveResources.owned :=
  fun _ _ hR => hR

theorem pingTransferRefinement :
    ResourceRefinement pingTransferResources pingTransferResources
      (id : SenderLabel → SenderLabel) where
  owned := fun _ h => h
  requires := fun _ _ h => h
  consumes := fun _ _ => Iff.intro id id
  transfers := fun _ _ => Iff.intro id id
  receives := fun _ _ => Iff.intro id id
  grade := fun _ => ⟨Nat.le_refl 0, Nat.le_refl 0, Nat.le_refl 0, Nat.le_refl 0⟩
  rely := fun _ _ h => h
  guarantees := fun _ _ h => h

/-- T5 product `WeakRefines` still holds. Matching ping/receive transfer
    witnesses `SynchronizationCompatible` (instantiating
    `synchronized_transfer_exact` / `synchronized_rely_discharged`). Product
    `ResourceRefinement` lifts because profiles otherwise match. Matching
    transfer is the extra I-CAP premise; T5 does not imply it. -/
theorem visibleSync_matchedTransfer_synchronizationCompatible :
    Nonempty
      (WeakRefines
        concreteCompositeVisible
        abstractCompositeVisible
        (compositeHiddenOf senderVisible)
        (compositeMapOf (id : SenderLabel → SenderLabel))) ∧
      SynchronizationCompatible pingTransferAction receiveReceiveAction ∧
      ResourceRefinement
        (smallProductSystemResources pingTransferResources receiveReceiveResources)
        (smallProductSystemResources pingTransferResources receiveReceiveResources)
        (compositeMapOf (id : SenderLabel → SenderLabel)) :=
  ⟨visibleSync_productRefinement,
    pingReceiveCompatible,
    liftParallelResources pingTransferRefinement⟩

/-- Named alias: matched-transfer profiles already use epsilon/`zero`
    grades on ping, receive, and both abstracts. Product
    `ResourceRefinement.grade` therefore holds on `.sync ping receive`
    via `liftParallelResources`. Point the claim ceiling at this rather
    than duplicating a zero-grade product. -/
theorem visibleSync_matchedTransfer_syncGrade :
    Le
      ((smallProductSystemResources pingTransferResources
          receiveReceiveResources).action
        (.sync SenderLabel.ping ReceiverLabel.receive)).grade
      ((smallProductSystemResources pingTransferResources
          receiveReceiveResources).action
        (compositeMapOf (id : SenderLabel → SenderLabel)
          (.sync SenderLabel.ping ReceiverLabel.receive))).grade :=
  (liftParallelResources pingTransferRefinement).grade
    (.sync SenderLabel.ping ReceiverLabel.receive)

#print axioms pingReceiveCompatible
#print axioms ping_receive_transfer_exact
#print axioms ping_receive_rely_discharged
#print axioms matched_capabilityPartition
#print axioms visibleSync_matchedTransfer_synchronizationCompatible
#print axioms visibleSync_matchedTransfer_syncGrade

end VisibleSyncMatchedTransfer

/-! ### Optional negative: shared ownership is independent of T5

  Same VisibleSync systems. Both sides own `token`, so
  `CapabilityPartition` fails, while T5 product `WeakRefines` still holds.
  Transfer/receive stay inert so this does not fight the transfer lemmas.
  Product `ResourceRefinement` can still hold (`ResourceRefinement.owned`
  does not require disjointness). Ceiling: T5 ⇏ I-CAP disjointness.
-/

namespace VisibleSyncSharedOwnership

open VisibleSync
open NMLT.Counterexamples.CompositionCongruence

inductive TokenCap
  | token
  deriving DecidableEq

def inertAction : ActionResources TokenCap Empty where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

def pingSharedResources : SystemResources SenderLabel TokenCap Empty where
  owned := fun c => c = .token
  action := fun
    | .ping => inertAction

def receiveSharedResources : SystemResources ReceiverLabel TokenCap Empty where
  owned := fun c => c = .token
  action := fun
    | .receive => inertAction

theorem pingSharedRefinement :
    ResourceRefinement pingSharedResources pingSharedResources
      (id : SenderLabel → SenderLabel) where
  owned := fun _ h => h
  requires := fun _ _ h => h
  consumes := fun _ _ => Iff.intro id id
  transfers := fun _ _ => Iff.intro id id
  receives := fun _ _ => Iff.intro id id
  grade := fun _ => ⟨Nat.le_refl 0, Nat.le_refl 0, Nat.le_refl 0, Nat.le_refl 0⟩
  rely := fun _ _ h => h
  guarantees := fun _ _ h => h

theorem shared_not_capabilityPartition :
    ¬ CapabilityPartition pingSharedResources.owned receiveSharedResources.owned :=
  fun partition =>
    partition TokenCap.token rfl rfl

/-- T5 product `WeakRefines` still holds. Both components own `token`, so
    `CapabilityPartition` fails. Product `ResourceRefinement` still holds
    because the owned-union lift does not require disjointness. -/
theorem visibleSync_sharedOwnership_not_capabilityPartition :
    Nonempty
      (WeakRefines
        concreteCompositeVisible
        abstractCompositeVisible
        (compositeHiddenOf senderVisible)
        (compositeMapOf (id : SenderLabel → SenderLabel))) ∧
      ¬ CapabilityPartition pingSharedResources.owned receiveSharedResources.owned ∧
      ResourceRefinement
        (smallProductSystemResources pingSharedResources receiveSharedResources)
        (smallProductSystemResources pingSharedResources receiveSharedResources)
        (compositeMapOf (id : SenderLabel → SenderLabel)) :=
  ⟨visibleSync_productRefinement,
    shared_not_capabilityPartition,
    liftParallelResources pingSharedRefinement⟩

#print axioms shared_not_capabilityPartition
#print axioms visibleSync_sharedOwnership_not_capabilityPartition

end VisibleSyncSharedOwnership

/-! ### Negative independence: T5 does not give I-GRADE through sync

  Same VisibleSync systems (visible ping wired to receive), so T5a product
  `WeakRefines` still holds. Concrete ping carries `HiddenGrade.pingCost`;
  receiver grade is epsilon (`zero`); abstract ping and abstract receive
  are epsilon. The `.sync ping receive` product grade is `parallelAction`
  = `Grades.parallel` of the two action grades. Concrete parallel cost is
  `pingCost` (`parallel pingCost zero`); abstract parallel is `zero`
  (`parallel zero zero`). Product `ResourceRefinement.grade` fails at
  that sync label. T5a still holds (observation only). Ceiling: T5 ⇏
  I-GRADE through sync. Not I-FAIR, not unmatched transfer. Keep the
  existing unmatched-transfer theorems. Lean `receiver` is not the
  OpenSystem-receptive dual.
-/

namespace VisibleSyncHiddenGrade

open VisibleSync
open NMLT.Counterexamples.CompositionCongruence
open NMLT.Counterexamples.HiddenGrade

/-- Lean `receiver` (and the abstract receive image) use epsilon grade.
    Capability, consume, transfer, and rely stay inert so the failure
    isolates I-GRADE through the sync label. -/
def receiveEpsilon : SystemResources ReceiverLabel Empty Empty where
  owned := fun _ => False
  action := fun
    | .receive => pingEpsilonAction

/-- Concrete visible ping carries `pingCost`. -/
def concretePingCost : SystemResources SenderLabel Empty Empty where
  owned := fun _ => False
  action := fun
    | .ping => pingCostAction

/-- Abstract ping on the same name is epsilon (`zero`). -/
def abstractPingEpsilon : SystemResources SenderLabel Empty Empty where
  owned := fun _ => False
  action := fun
    | .ping => pingEpsilonAction

theorem concrete_ping_grade_is_cost :
    (concretePingCost.action SenderLabel.ping).grade = pingCost :=
  rfl

theorem receive_grade_is_epsilon :
    (receiveEpsilon.action ReceiverLabel.receive).grade = zero :=
  rfl

theorem abstract_ping_grade_is_epsilon :
    (abstractPingEpsilon.action SenderLabel.ping).grade = zero :=
  rfl

/-- Sync grade is `Grades.parallel` of the two action grades. Concrete
    ping cost tensors with receiver epsilon, which is `pingCost`. -/
theorem concrete_product_sync_grade_is_cost :
    ((smallProductSystemResources concretePingCost receiveEpsilon).action
      (.sync SenderLabel.ping ReceiverLabel.receive)).grade = pingCost := by
  change Grades.parallel pingCost zero = pingCost
  exact parallel_zero pingCost

/-- Abstract sync grade is `parallel zero zero`, which is epsilon. -/
theorem abstract_product_sync_grade_is_epsilon :
    ((smallProductSystemResources abstractPingEpsilon receiveEpsilon).action
      (.sync SenderLabel.ping ReceiverLabel.receive)).grade = zero := by
  change Grades.parallel zero zero = zero
  exact parallel_zero zero

/-- T5a product `WeakRefines` still holds (VisibleSync observation). The
    extra ping cost is unmatched by the abstract product epsilon, so
    product `ResourceRefinement.grade` fails at `.sync ping receive`. -/
theorem visibleSync_hiddenGrade_breaks_productResourceRefinement :
    Nonempty
      (WeakRefines
        concreteCompositeVisible
        abstractCompositeVisible
        (compositeHiddenOf senderVisible)
        (compositeMapOf (id : SenderLabel → SenderLabel))) ∧
      ¬ ResourceRefinement
          (smallProductSystemResources concretePingCost receiveEpsilon)
          (smallProductSystemResources abstractPingEpsilon receiveEpsilon)
          (compositeMapOf (id : SenderLabel → SenderLabel)) :=
  ⟨visibleSync_productRefinement, fun refinement =>
    pingCost_not_le_zero <| by
      rw [← concrete_product_sync_grade_is_cost,
          ← abstract_product_sync_grade_is_epsilon]
      exact refinement.grade
        (.sync SenderLabel.ping ReceiverLabel.receive)⟩

#print axioms concrete_product_sync_grade_is_cost
#print axioms abstract_product_sync_grade_is_epsilon
#print axioms visibleSync_hiddenGrade_breaks_productResourceRefinement

end VisibleSyncHiddenGrade

end NMLT.Behavior.WeakResourceCongruence

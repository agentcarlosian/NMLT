import NMLT.Behavior.ResourceBehavior

namespace NMLT.Behavior.ResourceWorld

open NMLT.Behavior.ResourceBehavior

/--
A resource world gives every nominal capability at most one owner. Absence is
represented by `none`; it is the result of affine consumption.
-/
structure AuthorityWorld (Capability Owner : Type) where
  owner : Capability → Option Owner

def AuthorityWorld.Owns
    (world : AuthorityWorld Capability Owner) (actor : Owner)
    (capability : Capability) : Prop :=
  world.owner capability = some actor

def AuthorityWorld.Vacant
    (world : AuthorityWorld Capability Owner) (capability : Capability) : Prop :=
  world.owner capability = none

/-- Functional worlds make duplicate ownership unrepresentable. -/
theorem AuthorityWorld.ownership_unique
    {world : AuthorityWorld Capability Owner} {capability : Capability}
    {left right : Owner}
    (leftOwns : world.Owns left capability)
    (rightOwns : world.Owns right capability) :
    left = right := by
  exact Option.some.inj (leftOwns.symm.trans rightOwns)

/--
An action may require, consume, or transfer only authority held by its actor.
Its three affine effects are pairwise disjoint. Received authority must not
already be held by the receiver.
-/
structure Enabled
    (actor : Owner)
    (profile : ResourceProfile Capability Fact GradeAtom)
    (world : AuthorityWorld Capability Owner) : Prop where
  requires : ∀ capability, profile.requires capability → world.Owns actor capability
  consumes : ∀ capability, profile.consumes capability → world.Owns actor capability
  transfers : ∀ capability, profile.transfers capability → world.Owns actor capability
  receivesFresh : ∀ capability, profile.receives capability →
    ¬ world.Owns actor capability
  consumeTransfer : ∀ capability, profile.consumes capability →
    ¬ profile.transfers capability
  consumeReceive : ∀ capability, profile.consumes capability →
    ¬ profile.receives capability
  transferReceive : ∀ capability, profile.transfers capability →
    ¬ profile.receives capability

/--
Dynamic refinement strengthens profile refinement with the reverse requirement
implication needed to preserve world-step enabledness. Together with
`ResourceRefines.requires`, requirements are exact in this first world model.
-/
structure WorldResourceRefines
    (concrete abstract : ResourceProfile Capability Fact GradeAtom) : Prop where
  profile : ResourceRefines concrete abstract
  requiresBack : ∀ capability, abstract.requires capability →
    concrete.requires capability

theorem Enabled.refine
    {actor : Owner}
    {concrete abstract : ResourceProfile Capability Fact GradeAtom}
    {world : AuthorityWorld Capability Owner}
    (enabled : Enabled actor concrete world)
    (refines : WorldResourceRefines concrete abstract) :
    Enabled actor abstract world where
  requires := by
    intro capability required
    exact enabled.requires capability (refines.requiresBack capability required)
  consumes := by
    intro capability consumed
    exact enabled.consumes capability
      ((refines.profile.consumes capability).mpr consumed)
  transfers := by
    intro capability transferred
    exact enabled.transfers capability
      ((refines.profile.transfers capability).mpr transferred)
  receivesFresh := by
    intro capability received
    exact enabled.receivesFresh capability
      ((refines.profile.receives capability).mpr received)
  consumeTransfer := by
    intro capability consumed transferred
    exact enabled.consumeTransfer capability
      ((refines.profile.consumes capability).mpr consumed)
      ((refines.profile.transfers capability).mpr transferred)
  consumeReceive := by
    intro capability consumed received
    exact enabled.consumeReceive capability
      ((refines.profile.consumes capability).mpr consumed)
      ((refines.profile.receives capability).mpr received)
  transferReceive := by
    intro capability transferred received
    exact enabled.transferReceive capability
      ((refines.profile.transfers capability).mpr transferred)
      ((refines.profile.receives capability).mpr received)

/--
An isolated action may consume local authority, but boundary transfer and
receive effects require synchronization with a peer.
-/
structure LocalStep
    (actor : Owner)
    (profile : ResourceProfile Capability Fact GradeAtom)
    (before after : AuthorityWorld Capability Owner) : Prop where
  enabled : Enabled actor profile before
  noTransfer : ∀ capability, ¬ profile.transfers capability
  noReceive : ∀ capability, ¬ profile.receives capability
  consumed : ∀ capability, profile.consumes capability → after.Vacant capability
  preserved : ∀ capability, ¬ profile.consumes capability →
    after.owner capability = before.owner capability

/-- A synchronized pair has distinct component identities and compatible profiles. -/
structure SyncEnabled
    (leftOwner rightOwner : Owner)
    (left right : ResourceProfile Capability Fact GradeAtom)
    (before : AuthorityWorld Capability Owner) : Prop where
  ownersDistinct : leftOwner ≠ rightOwner
  leftEnabled : Enabled leftOwner left before
  rightEnabled : Enabled rightOwner right before
  compatible : SynchronizationCompatible left right

/--
The dynamic effect of synchronization. Consumption removes authority;
left-to-right and right-to-left transfers change its unique owner; every
unaffected capability is preserved.
-/
structure SyncStep
    (leftOwner rightOwner : Owner)
    (left right : ResourceProfile Capability Fact GradeAtom)
    (before after : AuthorityWorld Capability Owner) : Prop where
  enabled : SyncEnabled leftOwner rightOwner left right before
  consumedLeft : ∀ capability, left.consumes capability → after.Vacant capability
  consumedRight : ∀ capability, right.consumes capability → after.Vacant capability
  transferredLeft : ∀ capability, left.transfers capability →
    after.Owns rightOwner capability
  transferredRight : ∀ capability, right.transfers capability →
    after.Owns leftOwner capability
  preserved : ∀ capability,
    ¬ left.consumes capability → ¬ right.consumes capability →
    ¬ left.transfers capability → ¬ right.transfers capability →
    after.owner capability = before.owner capability

def localResult
    (profile : ResourceProfile Capability Fact GradeAtom)
    (before : AuthorityWorld Capability Owner)
    [DecidablePred profile.consumes] : AuthorityWorld Capability Owner where
  owner := fun capability =>
    if profile.consumes capability then none else before.owner capability

theorem Enabled.toLocalStep
    {actor : Owner}
    {profile : ResourceProfile Capability Fact GradeAtom}
    {before : AuthorityWorld Capability Owner}
    [DecidablePred profile.consumes]
    (enabled : Enabled actor profile before)
    (noTransfer : ∀ capability, ¬ profile.transfers capability)
    (noReceive : ∀ capability, ¬ profile.receives capability) :
    LocalStep actor profile before (localResult profile before) where
  enabled := enabled
  noTransfer := noTransfer
  noReceive := noReceive
  consumed := by
    intro capability consumed
    simp [localResult, consumed, AuthorityWorld.Vacant]
  preserved := by
    intro capability notConsumed
    simp [localResult, notConsumed]

namespace LocalStep

/-- Dynamic local effects are preserved by world-aware profile refinement. -/
theorem refine
    {actor : Owner}
    {concrete abstract : ResourceProfile Capability Fact GradeAtom}
    {before after : AuthorityWorld Capability Owner}
    (step : LocalStep actor concrete before after)
    (refines : WorldResourceRefines concrete abstract) :
    LocalStep actor abstract before after where
  enabled := step.enabled.refine refines
  noTransfer := by
    intro capability transferred
    exact step.noTransfer capability
      ((refines.profile.transfers capability).mpr transferred)
  noReceive := by
    intro capability received
    exact step.noReceive capability
      ((refines.profile.receives capability).mpr received)
  consumed := by
    intro capability consumed
    exact step.consumed capability
      ((refines.profile.consumes capability).mpr consumed)
  preserved := by
    intro capability notConsumedAbstract
    apply step.preserved capability
    intro consumedConcrete
    exact notConsumedAbstract
      ((refines.profile.consumes capability).mp consumedConcrete)

/--
A local step whose complete profile refines stutter preserves the entire
authority world. This is the dynamic premise that makes hidden stuttering
sound rather than merely a control-state equality.
-/
theorem world_preserved_of_refines_empty
    {actor : Owner}
    {profile : ResourceProfile Capability Fact GradeAtom}
    {before after : AuthorityWorld Capability Owner}
    (step : LocalStep actor profile before after)
    (stutter : ResourceRefines profile ResourceProfile.empty) :
    ∀ capability, after.owner capability = before.owner capability := by
  intro capability
  exact step.preserved capability (fun consumed =>
    (stutter.consumes capability).mp consumed)

end LocalStep

def syncResult
    (leftOwner rightOwner : Owner)
    (left right : ResourceProfile Capability Fact GradeAtom)
    (before : AuthorityWorld Capability Owner)
    [DecidablePred left.consumes] [DecidablePred right.consumes]
    [DecidablePred left.transfers] [DecidablePred right.transfers] :
    AuthorityWorld Capability Owner where
  owner := fun capability =>
    if left.consumes capability ∨ right.consumes capability then
      none
    else if left.transfers capability then
      some rightOwner
    else if right.transfers capability then
      some leftOwner
    else
      before.owner capability

theorem SyncEnabled.toStep
    {leftOwner rightOwner : Owner}
    {left right : ResourceProfile Capability Fact GradeAtom}
    {before : AuthorityWorld Capability Owner}
    [DecidablePred left.consumes] [DecidablePred right.consumes]
    [DecidablePred left.transfers] [DecidablePred right.transfers]
    (enabled : SyncEnabled leftOwner rightOwner left right before) :
    SyncStep leftOwner rightOwner left right before
      (syncResult leftOwner rightOwner left right before) where
  enabled := enabled
  consumedLeft := by
    intro capability consumed
    simp [syncResult, consumed, AuthorityWorld.Vacant]
  consumedRight := by
    intro capability consumed
    simp [syncResult, consumed, AuthorityWorld.Vacant]
  transferredLeft := by
    intro capability transferred
    have notConsumedLeft : ¬ left.consumes capability := by
      intro consumed
      exact enabled.leftEnabled.consumeTransfer capability consumed transferred
    have notConsumedRight : ¬ right.consumes capability := by
      intro consumed
      have leftOwns := enabled.leftEnabled.transfers capability transferred
      have rightOwns := enabled.rightEnabled.consumes capability consumed
      exact enabled.ownersDistinct
        (before.ownership_unique leftOwns rightOwns)
    simp [syncResult, notConsumedLeft, notConsumedRight, transferred,
      AuthorityWorld.Owns]
  transferredRight := by
    intro capability transferred
    have notConsumedRight : ¬ right.consumes capability := by
      intro consumed
      exact enabled.rightEnabled.consumeTransfer capability consumed transferred
    have notConsumedLeft : ¬ left.consumes capability := by
      intro consumed
      have leftOwns := enabled.leftEnabled.consumes capability consumed
      have rightOwns := enabled.rightEnabled.transfers capability transferred
      exact enabled.ownersDistinct
        (before.ownership_unique leftOwns rightOwns)
    have notTransferredLeft : ¬ left.transfers capability := by
      intro leftTransferred
      have leftOwns := enabled.leftEnabled.transfers capability leftTransferred
      have rightOwns := enabled.rightEnabled.transfers capability transferred
      exact enabled.ownersDistinct
        (before.ownership_unique leftOwns rightOwns)
    simp [syncResult, notConsumedLeft, notConsumedRight, notTransferredLeft,
      transferred, AuthorityWorld.Owns]
  preserved := by
    intro capability notConsumedLeft notConsumedRight
      notTransferredLeft notTransferredRight
    simp [syncResult, notConsumedLeft, notConsumedRight,
      notTransferredLeft, notTransferredRight]

namespace SyncStep

theorem refineLeft
    {leftOwner rightOwner : Owner}
    {concrete abstract right : ResourceProfile Capability Fact GradeAtom}
    {before after : AuthorityWorld Capability Owner}
    (step : SyncStep leftOwner rightOwner concrete right before after)
    (refines : WorldResourceRefines concrete abstract)
    (compatible : SynchronizationCompatible abstract right) :
    SyncStep leftOwner rightOwner abstract right before after where
  enabled := {
    ownersDistinct := step.enabled.ownersDistinct
    leftEnabled := step.enabled.leftEnabled.refine refines
    rightEnabled := step.enabled.rightEnabled
    compatible := compatible
  }
  consumedLeft := by
    intro capability consumed
    exact step.consumedLeft capability
      ((refines.profile.consumes capability).mpr consumed)
  consumedRight := step.consumedRight
  transferredLeft := by
    intro capability transferred
    exact step.transferredLeft capability
      ((refines.profile.transfers capability).mpr transferred)
  transferredRight := step.transferredRight
  preserved := by
    intro capability notConsumedAbstract notConsumedRight
      notTransferredAbstract notTransferredRight
    apply step.preserved capability
    · intro consumedConcrete
      exact notConsumedAbstract
        ((refines.profile.consumes capability).mp consumedConcrete)
    · exact notConsumedRight
    · intro transferredConcrete
      exact notTransferredAbstract
        ((refines.profile.transfers capability).mp transferredConcrete)
    · exact notTransferredRight

theorem left_transfer_changes_owner
    {leftOwner rightOwner : Owner}
    {left right : ResourceProfile Capability Fact GradeAtom}
    {before after : AuthorityWorld Capability Owner}
    (step : SyncStep leftOwner rightOwner left right before after)
    {capability : Capability} (transfers : left.transfers capability) :
    after.Owns rightOwner capability :=
  step.transferredLeft capability transfers

theorem left_transfer_not_retained
    {leftOwner rightOwner : Owner}
    {left right : ResourceProfile Capability Fact GradeAtom}
    {before after : AuthorityWorld Capability Owner}
    (step : SyncStep leftOwner rightOwner left right before after)
    {capability : Capability} (transfers : left.transfers capability) :
    ¬ after.Owns leftOwner capability := by
  intro retained
  have equalOwners : leftOwner = rightOwner :=
    after.ownership_unique retained (step.transferredLeft capability transfers)
  exact step.enabled.ownersDistinct equalOwners

theorem right_transfer_changes_owner
    {leftOwner rightOwner : Owner}
    {left right : ResourceProfile Capability Fact GradeAtom}
    {before after : AuthorityWorld Capability Owner}
    (step : SyncStep leftOwner rightOwner left right before after)
    {capability : Capability} (transfers : right.transfers capability) :
    after.Owns leftOwner capability :=
  step.transferredRight capability transfers

theorem right_transfer_not_retained
    {leftOwner rightOwner : Owner}
    {left right : ResourceProfile Capability Fact GradeAtom}
    {before after : AuthorityWorld Capability Owner}
    (step : SyncStep leftOwner rightOwner left right before after)
    {capability : Capability} (transfers : right.transfers capability) :
    ¬ after.Owns rightOwner capability := by
  intro retained
  have equalOwners : rightOwner = leftOwner :=
    after.ownership_unique retained (step.transferredRight capability transfers)
  exact step.enabled.ownersDistinct equalOwners.symm

theorem left_transfer_was_owned
    {leftOwner rightOwner : Owner}
    {left right : ResourceProfile Capability Fact GradeAtom}
    {before after : AuthorityWorld Capability Owner}
    (step : SyncStep leftOwner rightOwner left right before after)
    {capability : Capability} (transfers : left.transfers capability) :
    before.Owns leftOwner capability :=
  step.enabled.leftEnabled.transfers capability transfers

theorem right_transfer_was_owned
    {leftOwner rightOwner : Owner}
    {left right : ResourceProfile Capability Fact GradeAtom}
    {before after : AuthorityWorld Capability Owner}
    (step : SyncStep leftOwner rightOwner left right before after)
    {capability : Capability} (transfers : right.transfers capability) :
    before.Owns rightOwner capability :=
  step.enabled.rightEnabled.transfers capability transfers

theorem changed_is_explained
    {leftOwner rightOwner : Owner}
    {left right : ResourceProfile Capability Fact GradeAtom}
    {before after : AuthorityWorld Capability Owner}
    [DecidablePred left.consumes] [DecidablePred right.consumes]
    [DecidablePred left.transfers] [DecidablePred right.transfers]
    (step : SyncStep leftOwner rightOwner left right before after)
    (capability : Capability)
    (changed : after.owner capability ≠ before.owner capability) :
    left.consumes capability ∨ right.consumes capability ∨
      left.transfers capability ∨ right.transfers capability := by
  by_cases consumedLeft : left.consumes capability
  · exact Or.inl consumedLeft
  by_cases consumedRight : right.consumes capability
  · exact Or.inr (Or.inl consumedRight)
  by_cases transferredLeft : left.transfers capability
  · exact Or.inr (Or.inr (Or.inl transferredLeft))
  by_cases transferredRight : right.transfers capability
  · exact Or.inr (Or.inr (Or.inr transferredRight))
  exact False.elim (changed
    (step.preserved capability consumedLeft consumedRight
      transferredLeft transferredRight))

theorem owner_after_is_explained
    {leftOwner rightOwner : Owner}
    {left right : ResourceProfile Capability Fact GradeAtom}
    {before after : AuthorityWorld Capability Owner}
    [DecidablePred left.consumes] [DecidablePred right.consumes]
    [DecidablePred left.transfers] [DecidablePred right.transfers]
    (step : SyncStep leftOwner rightOwner left right before after)
    (capability : Capability) :
    after.Vacant capability ∨
      (left.transfers capability ∧ after.Owns rightOwner capability) ∨
      (right.transfers capability ∧ after.Owns leftOwner capability) ∨
      after.owner capability = before.owner capability := by
  by_cases consumedLeft : left.consumes capability
  · exact Or.inl (step.consumedLeft capability consumedLeft)
  by_cases consumedRight : right.consumes capability
  · exact Or.inl (step.consumedRight capability consumedRight)
  by_cases transferredLeft : left.transfers capability
  · exact Or.inr (Or.inl
      ⟨transferredLeft, step.transferredLeft capability transferredLeft⟩)
  by_cases transferredRight : right.transfers capability
  · exact Or.inr (Or.inr (Or.inl
      ⟨transferredRight, step.transferredRight capability transferredRight⟩))
  exact Or.inr (Or.inr (Or.inr
    (step.preserved capability consumedLeft consumedRight
      transferredLeft transferredRight)))

end SyncStep

/-- Dynamic state for a binary product: two control states and one shared world. -/
structure ProductState
    (left : Behavior LeftAction Capability Fact GradeAtom LeftObservation)
    (right : Behavior RightAction Capability Fact GradeAtom RightObservation)
    (Owner : Type) where
  leftState : left.State
  rightState : right.State
  authority : AuthorityWorld Capability Owner

/--
Binary product transitions with dynamic authority. Unconnected steps use the
local effect rule; synchronization uses the shared-world transfer rule.
-/
inductive ProductStep
    {LeftAction RightAction Capability Fact GradeAtom
      LeftObservation RightObservation Owner : Type}
    (left : Behavior LeftAction Capability Fact GradeAtom LeftObservation)
    (right : Behavior RightAction Capability Fact GradeAtom RightObservation)
    (leftOwner rightOwner : Owner)
    (connection : LeftAction → RightAction → Prop) :
    ProductState left right Owner → ProductAction LeftAction RightAction →
      ProductState left right Owner → Prop where
  | fromLeft {leftBefore leftAfter rightState beforeWorld afterWorld action} :
      left.step leftBefore action leftAfter →
      (∀ rightAction, ¬ connection action rightAction) →
      LocalStep leftOwner (left.resources action) beforeWorld afterWorld →
      ProductStep left right leftOwner rightOwner connection
        ⟨leftBefore, rightState, beforeWorld⟩ (.left action)
        ⟨leftAfter, rightState, afterWorld⟩
  | fromRight {leftState rightBefore rightAfter beforeWorld afterWorld action} :
      right.step rightBefore action rightAfter →
      (∀ leftAction, ¬ connection leftAction action) →
      LocalStep rightOwner (right.resources action) beforeWorld afterWorld →
      ProductStep left right leftOwner rightOwner connection
        ⟨leftState, rightBefore, beforeWorld⟩ (.right action)
        ⟨leftState, rightAfter, afterWorld⟩
  | synchronize
      {leftBefore leftAfter rightBefore rightAfter beforeWorld afterWorld
        leftAction rightAction} :
      connection leftAction rightAction →
      left.step leftBefore leftAction leftAfter →
      right.step rightBefore rightAction rightAfter →
      SyncStep leftOwner rightOwner
        (left.resources leftAction) (right.resources rightAction)
        beforeWorld afterWorld →
      ProductStep left right leftOwner rightOwner connection
        ⟨leftBefore, rightBefore, beforeWorld⟩ (.sync leftAction rightAction)
        ⟨leftAfter, rightAfter, afterWorld⟩

namespace ProductStep

theorem synchronized_left_transfer_moves_once
    {left : Behavior LeftAction Capability Fact GradeAtom LeftObservation}
    {right : Behavior RightAction Capability Fact GradeAtom RightObservation}
    {leftOwner rightOwner : Owner}
    {connection : LeftAction → RightAction → Prop}
    {before after : ProductState left right Owner}
    {leftAction : LeftAction} {rightAction : RightAction}
    (step : ProductStep left right leftOwner rightOwner connection before
      (.sync leftAction rightAction) after)
    {capability : Capability}
    (transfers : (left.resources leftAction).transfers capability) :
    before.authority.Owns leftOwner capability ∧
      after.authority.Owns rightOwner capability ∧
      ¬ after.authority.Owns leftOwner capability := by
  cases step with
  | synchronize _ _ _ worldStep =>
      exact ⟨
        worldStep.left_transfer_was_owned transfers,
        worldStep.left_transfer_changes_owner transfers,
        worldStep.left_transfer_not_retained transfers
      ⟩

end ProductStep

/-- A behavior refinement strengthened enough to preserve dynamic enabledness. -/
structure WorldWeakRefinement
    {Action Capability Fact GradeAtom ConcreteObservation AbstractObservation : Type}
    (concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation)
    (abstract : Behavior Action Capability Fact GradeAtom AbstractObservation) where
  behavior : ResourceWeakRefinement concrete abstract
  worldResources : ∀ action,
    WorldResourceRefines (concrete.resources action) (abstract.resources action)

def mapProductState
    {concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation}
    {abstract : Behavior Action Capability Fact GradeAtom AbstractObservation}
    {peer : Behavior PeerAction Capability Fact GradeAtom PeerObservation}
    {Owner : Type}
    (refinement : WorldWeakRefinement concrete abstract)
    (state : ProductState concrete peer Owner) : ProductState abstract peer Owner :=
  ⟨refinement.behavior.mapState state.leftState, state.rightState, state.authority⟩

/-- Exact dynamic-state agreement without quotienting a function-valued world. -/
structure DynamicStateEquivalent
    {left : Behavior LeftAction Capability Fact GradeAtom LeftObservation}
    {right : Behavior RightAction Capability Fact GradeAtom RightObservation}
    {Owner : Type}
    (before after : ProductState left right Owner) : Prop where
  leftState : before.leftState = after.leftState
  rightState : before.rightState = after.rightState
  authority : ∀ capability,
    before.authority.owner capability = after.authority.owner capability

/--
One-step dynamic weak matching: the abstract product either performs the same
product action, or a hidden left action maps to an unchanged control/resource
state.
-/
inductive DynamicStepMatch
    {Action PeerAction Capability Fact GradeAtom
      ConcreteObservation AbstractObservation PeerObservation Owner : Type}
    (concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation)
    (abstract : Behavior Action Capability Fact GradeAtom AbstractObservation)
    (peer : Behavior PeerAction Capability Fact GradeAtom PeerObservation)
    (leftOwner rightOwner : Owner)
    (connection : Action → PeerAction → Prop)
    (before : ProductState abstract peer Owner) :
    ProductAction Action PeerAction → ProductState abstract peer Owner → Prop where
  | stutter {leftAction after} :
      abstract.hidden leftAction →
      ResourceRefines (concrete.resources leftAction) ResourceProfile.empty →
      DynamicStateEquivalent before after →
      DynamicStepMatch concrete abstract peer leftOwner rightOwner connection
        before (.left leftAction) after
  | transition {action after} :
      ProductStep abstract peer leftOwner rightOwner connection
        before action after →
      DynamicStepMatch concrete abstract peer leftOwner rightOwner connection
        before action after

/-- A reusable witness that every concrete dynamic product step weakly matches. -/
structure DynamicProductRefinement
    {Action PeerAction Capability Fact GradeAtom
      ConcreteObservation AbstractObservation PeerObservation Owner : Type}
    (concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation)
    (abstract : Behavior Action Capability Fact GradeAtom AbstractObservation)
    (peer : Behavior PeerAction Capability Fact GradeAtom PeerObservation)
    (leftOwner rightOwner : Owner)
    (concreteConnection abstractConnection : Action → PeerAction → Prop) where
  mapState : ProductState concrete peer Owner → ProductState abstract peer Owner
  matchStep : ∀ {before action after},
    ProductStep concrete peer leftOwner rightOwner concreteConnection
      before action after →
    DynamicStepMatch concrete abstract peer leftOwner rightOwner abstractConnection
      (mapState before) action (mapState after)

/--
The first dynamic lifting result: every synchronized concrete product step has
an abstract synchronized step with the same peer transition and the same
post-step authority world.
-/
theorem liftSynchronized
    {concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation}
    {abstract : Behavior Action Capability Fact GradeAtom AbstractObservation}
    {peer : Behavior PeerAction Capability Fact GradeAtom PeerObservation}
    {leftOwner rightOwner : Owner}
    {concreteConnection abstractConnection : Action → PeerAction → Prop}
    (refinement : WorldWeakRefinement concrete abstract)
    (wiring : WiringEquivalent concreteConnection abstractConnection)
    (concreteComposable : Composable concrete peer concreteConnection)
    (abstractComposable : Composable abstract peer abstractConnection)
    {before after : ProductState concrete peer Owner}
    {leftAction : Action} {rightAction : PeerAction}
    (step : ProductStep concrete peer leftOwner rightOwner concreteConnection
      before (.sync leftAction rightAction) after) :
    ProductStep abstract peer leftOwner rightOwner abstractConnection
      (mapProductState refinement before) (.sync leftAction rightAction)
      (mapProductState refinement after) := by
  cases step with
  | synchronize connected concreteStep peerStep worldStep =>
      have abstractConnected := (wiring.connected leftAction rightAction).mp connected
      have concreteVisible : ¬ concrete.hidden leftAction :=
        concreteComposable.hiddenLeft connected
      have abstractStep := refinement.behavior.visibleStep concreteStep concreteVisible
      have abstractCompatible : SynchronizationCompatible
          (abstract.resources leftAction) (peer.resources rightAction) := by
        let clauses := abstractComposable.resources abstractConnected
        by_cases abstractOutput : abstract.direction leftAction = .output
        · exact clauses.1 abstractOutput
        · have peerOutput : peer.direction rightAction = .output := by
            have directions := abstractComposable.directions abstractConnected
            cases abstractDirection : abstract.direction leftAction <;>
              cases peerDirection : peer.direction rightAction <;>
              simp_all [Direction.Complementary]
          exact (clauses.2 peerOutput).symm
      exact ProductStep.synchronize abstractConnected abstractStep peerStep
        (worldStep.refineLeft
          (refinement.worldResources leftAction) abstractCompatible)

/--
Full one-step dynamic lifting for a binary product. Visible isolated steps and
synchronizations transition in the abstract product, peer-local steps are
preserved, and hidden concrete steps stutter only after both mapped control
state and the complete authority world are proved unchanged.
-/
def liftProductSteps
    {concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation}
    {abstract : Behavior Action Capability Fact GradeAtom AbstractObservation}
    {peer : Behavior PeerAction Capability Fact GradeAtom PeerObservation}
    {leftOwner rightOwner : Owner}
    {concreteConnection abstractConnection : Action → PeerAction → Prop}
    [DecidablePred concrete.hidden]
    (refinement : WorldWeakRefinement concrete abstract)
    (wiring : WiringEquivalent concreteConnection abstractConnection)
    (concreteComposable : Composable concrete peer concreteConnection)
    (abstractComposable : Composable abstract peer abstractConnection) :
    DynamicProductRefinement concrete abstract peer leftOwner rightOwner
      concreteConnection abstractConnection where
  mapState := mapProductState refinement
  matchStep := by
    intro before action after step
    cases step with
    | fromLeft concreteStep isolated worldStep =>
        rename_i leftBefore leftAfter rightState beforeWorld afterWorld leftAction
        by_cases hidden : concrete.hidden leftAction
        · apply DynamicStepMatch.stutter
          · exact (refinement.behavior.hiddenPreserved leftAction).mp hidden
          · exact refinement.behavior.hiddenResources leftAction hidden
          · exact {
              leftState := refinement.behavior.hiddenStep concreteStep hidden
              rightState := rfl
              authority := fun capability =>
                (worldStep.world_preserved_of_refines_empty
                  (refinement.behavior.hiddenResources leftAction hidden)
                  capability).symm
            }
        · apply DynamicStepMatch.transition
          exact ProductStep.fromLeft
            (refinement.behavior.visibleStep concreteStep hidden)
            (fun rightAction abstractConnected =>
              isolated rightAction
                ((wiring.connected _ _).mpr abstractConnected))
            (worldStep.refine (refinement.worldResources leftAction))
    | fromRight peerStep isolated worldStep =>
        apply DynamicStepMatch.transition
        exact ProductStep.fromRight peerStep
          (fun leftAction abstractConnected =>
            isolated leftAction
              ((wiring.connected _ _).mpr abstractConnected))
          worldStep
    | synchronize connected concreteStep peerStep worldStep =>
        apply DynamicStepMatch.transition
        exact liftSynchronized refinement wiring concreteComposable
          abstractComposable
          (ProductStep.synchronize connected concreteStep peerStep worldStep)

end NMLT.Behavior.ResourceWorld

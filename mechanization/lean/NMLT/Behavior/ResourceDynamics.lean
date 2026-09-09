import NMLT.Behavior.ResourceWorld

namespace NMLT.Behavior.ResourceDynamics

open ResourceBehavior ResourceWorld

/-- Deferred effects retain leaf identities across open binary products. -/
inductive Effect (Capability Fact GradeAtom Owner : Type) where
  | atomic (actor : Owner) (profile : ResourceProfile Capability Fact GradeAtom)
  | synchronize (left right : Effect Capability Fact GradeAtom Owner)

namespace Effect

def profile : Effect Capability Fact GradeAtom Owner →
    ResourceProfile Capability Fact GradeAtom
  | .atomic _ resources => resources
  | .synchronize left right => ResourceProfile.parallel left.profile right.profile

/-- Only a pair of atomic boundary participants can rendezvous. A completed
synchronization can run inside an outer product, but cannot rendezvous again. -/
def Step (effect : Effect Capability Fact GradeAtom Owner)
    (before after : AuthorityWorld Capability Owner) : Prop :=
  match effect with
  | .atomic actor resources => LocalStep actor resources before after
  | .synchronize (.atomic leftOwner left) (.atomic rightOwner right) =>
      SyncStep leftOwner rightOwner left right before after
  | .synchronize _ _ => False

end Effect

/-- The control presentation is reused from v1. It is not independently executed:
`step` below applies the deferred effect exactly once to the shared world. -/
structure Behavior (Action Capability Fact GradeAtom Observation Owner : Type) where
  control : ResourceBehavior.Behavior Action Capability Fact GradeAtom Observation
  effect : Action → Effect Capability Fact GradeAtom Owner
  effectProfile : ∀ action, (effect action).profile = control.resources action
  initialWorld : AuthorityWorld Capability Owner → Prop

namespace Behavior

structure State
    (behavior : Behavior Action Capability Fact GradeAtom Observation Owner) where
  control : behavior.control.State
  authority : AuthorityWorld Capability Owner

def init (behavior : Behavior Action Capability Fact GradeAtom Observation Owner)
    (state : behavior.State) : Prop :=
  behavior.control.init state.control ∧ behavior.initialWorld state.authority

/-- Full authority is retained in the semantic observation, separately from the
component's selected control observation. No resource-erasing quotient is used. -/
def observe (behavior : Behavior Action Capability Fact GradeAtom Observation Owner)
    (state : behavior.State) : Observation × AuthorityWorld Capability Owner :=
  (behavior.control.observe state.control, state.authority)

def step (behavior : Behavior Action Capability Fact GradeAtom Observation Owner)
    (before : behavior.State) (action : Action) (after : behavior.State) : Prop :=
  behavior.control.step before.control action after.control ∧
    (behavior.effect action).Step before.authority after.authority

end Behavior

def ofBehavior
    (control : ResourceBehavior.Behavior Action Capability Fact GradeAtom Observation)
    (actor : Owner) (initialWorld : AuthorityWorld Capability Owner → Prop) :
    Behavior Action Capability Fact GradeAtom Observation Owner where
  control := control
  effect := fun action => .atomic actor (control.resources action)
  effectProfile := fun _ => rfl
  initialWorld := initialWorld

/-- Reuse v1's control product relation, correcting its open-interface metadata.
The components' completed dynamic steps are deliberately not premises: an open
transfer is deferred until its final peer is supplied by an outer product. -/
def parallel
    (left : Behavior LeftAction Capability Fact GradeAtom LeftObservation Owner)
    (right : Behavior RightAction Capability Fact GradeAtom RightObservation Owner)
    (connection : LeftAction → RightAction → Prop) :
    Behavior (ProductAction LeftAction RightAction) Capability Fact GradeAtom
      (LeftObservation × RightObservation) Owner where
  control := {
    ResourceBehavior.parallel left.control right.control connection with
    hidden
      | .left action => left.control.hidden action
      | .right action => right.control.hidden action
      | .sync _ _ => False
    direction
      | .left action => left.control.direction action
      | .right action => right.control.direction action
      | .sync _ _ => .internal
    payload
      | .left action => left.control.payload action
      | .right action => right.control.payload action
      | .sync _ _ => "Unit"
  }
  effect
    | .left action => left.effect action
    | .right action => right.effect action
    | .sync leftAction rightAction =>
        .synchronize (left.effect leftAction) (right.effect rightAction)
  effectProfile := by
    intro action
    cases action with
    | left action => exact left.effectProfile action
    | right action => exact right.effectProfile action
    | sync leftAction rightAction =>
        simp only [Effect.profile, left.effectProfile, right.effectProfile]
        rfl
  initialWorld := fun world => left.initialWorld world ∧ right.initialWorld world

/-- Formation is language policy, separate from the hypotheses of step lifting. -/
structure Composable
    (left : Behavior LeftAction Capability Fact GradeAtom LeftObservation Owner)
    (right : Behavior RightAction Capability Fact GradeAtom RightObservation Owner)
    (connection : LeftAction → RightAction → Prop) : Prop where
  interface : ResourceBehavior.Composable left.control right.control connection
  participants : ∀ {leftAction rightAction}, connection leftAction rightAction →
    ∃ leftOwner rightOwner leftProfile rightProfile,
      left.effect leftAction = .atomic leftOwner leftProfile ∧
      right.effect rightAction = .atomic rightOwner rightProfile ∧
      leftOwner ≠ rightOwner

def leftState
    (state : (parallel left right connection).State) : left.State :=
  ⟨state.control.1, state.authority⟩

def rightState
    (state : (parallel left right connection).State) : right.State :=
  ⟨state.control.2, state.authority⟩

theorem parallel_init_iff
    (state : (parallel left right connection).State) :
    (parallel left right connection).init state ↔
      left.init (leftState state) ∧ right.init (rightState state) := by
  constructor
  · rintro ⟨⟨leftInit, rightInit⟩, leftWorld, rightWorld⟩
    exact ⟨⟨leftInit, leftWorld⟩, rightInit, rightWorld⟩
  · rintro ⟨⟨leftInit, leftWorld⟩, rightInit, rightWorld⟩
    exact ⟨⟨leftInit, rightInit⟩, leftWorld, rightWorld⟩

theorem parallel_observe
    (state : (parallel left right connection).State) :
    (parallel left right connection).observe state =
      (((left.observe (leftState state)).1, (right.observe (rightState state)).1),
        state.authority) := rfl

theorem step_projects
    (step : (parallel left right connection).step before action after) :
    ResourceBehavior.ProductStep left.control right.control connection
      before.control action after.control := step.1

def legacyState
    {left : ResourceBehavior.Behavior LeftAction Capability Fact GradeAtom LeftObservation}
    {right : ResourceBehavior.Behavior RightAction Capability Fact GradeAtom RightObservation}
    {leftOwner rightOwner : Owner}
    {leftInitial rightInitial : AuthorityWorld Capability Owner → Prop}
    {connection : LeftAction → RightAction → Prop}
    (state : (parallel (ofBehavior left leftOwner leftInitial)
      (ofBehavior right rightOwner rightInitial) connection).State) :
    ResourceWorld.ProductState left right Owner :=
  ⟨state.control.1, state.control.2, state.authority⟩

/-- Exact replacement of the old dynamic step relation for leaf pairs. The old
relation has no initial/observation predicates; those are supplied above. -/
theorem legacy_step_iff
    {left : ResourceBehavior.Behavior LeftAction Capability Fact GradeAtom LeftObservation}
    {right : ResourceBehavior.Behavior RightAction Capability Fact GradeAtom RightObservation}
    {leftOwner rightOwner : Owner}
    {leftInitial rightInitial : AuthorityWorld Capability Owner → Prop}
    {connection : LeftAction → RightAction → Prop}
    {before after : (parallel (ofBehavior left leftOwner leftInitial)
      (ofBehavior right rightOwner rightInitial) connection).State} :
    (parallel (ofBehavior left leftOwner leftInitial)
      (ofBehavior right rightOwner rightInitial) connection).step before action after ↔
    ResourceWorld.ProductStep left right leftOwner rightOwner connection
      (legacyState before) action (legacyState after) := by
  rcases before with ⟨⟨leftBefore, rightBefore⟩, beforeWorld⟩
  rcases after with ⟨⟨leftAfter, rightAfter⟩, afterWorld⟩
  constructor
  · rintro ⟨controlStep, worldStep⟩
    cases controlStep with
    | fromLeft localStep isolated =>
        exact ResourceWorld.ProductStep.fromLeft localStep isolated worldStep
    | fromRight localStep isolated =>
        exact ResourceWorld.ProductStep.fromRight localStep isolated worldStep
    | synchronize connected leftStep rightStep =>
        exact ResourceWorld.ProductStep.synchronize connected leftStep rightStep worldStep
  · intro step
    cases step with
    | fromLeft localStep isolated worldStep =>
        exact ⟨ResourceBehavior.ProductStep.fromLeft localStep isolated, worldStep⟩
    | fromRight localStep isolated worldStep =>
        exact ⟨ResourceBehavior.ProductStep.fromRight localStep isolated, worldStep⟩
    | synchronize connected leftStep rightStep worldStep =>
        exact ⟨ResourceBehavior.ProductStep.synchronize connected leftStep rightStep, worldStep⟩

/-- Full-state agreement used for hidden matching, without quotienting worlds. -/
structure StateEquivalent
    {behavior : Behavior Action Capability Fact GradeAtom Observation Owner}
    (before after : behavior.State) : Prop where
  control : before.control = after.control
  authority : ∀ capability,
    before.authority.owner capability = after.authority.owner capability

/-- Local refinement includes the dynamic obligation that a hidden action really
preserves the world. An empty aggregate profile alone is insufficient: an inner
synchronization can move authority while its summary has no boundary transfer. -/
structure Refinement
    (concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation Owner)
    (abstract : Behavior Action Capability Fact GradeAtom AbstractObservation Owner) where
  control : ResourceWeakRefinement concrete.control abstract.control
  initialWorld : ∀ world, concrete.initialWorld world → abstract.initialWorld world
  visibleEffect : ∀ action, ¬ concrete.control.hidden action →
    ∀ {before after}, (concrete.effect action).Step before after →
      (abstract.effect action).Step before after
  hiddenWorld : ∀ action, concrete.control.hidden action →
    ∀ {before after}, (concrete.effect action).Step before after →
      ∀ capability, before.owner capability = after.owner capability

def mapState (refinement : Refinement concrete abstract)
    (state : concrete.State) : abstract.State :=
  ⟨refinement.control.mapState state.control, state.authority⟩

inductive StepMatch
    (behavior : Behavior Action Capability Fact GradeAtom Observation Owner)
    (performed : ResourceProfile Capability Fact GradeAtom)
    (before : behavior.State) (action : Action) (after : behavior.State) : Prop where
  | transition : behavior.step before action after →
      StepMatch behavior performed before action after
  | stutter : behavior.control.hidden action →
      ResourceRefines performed ResourceProfile.empty → StateEquivalent before after →
      StepMatch behavior performed before action after

/-- A completed dynamic simulation preserves initial states, observations and
resource summaries, and matches each action by a transition or exact hidden stutter. -/
structure Simulation
    (concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation Owner)
    (abstract : Behavior Action Capability Fact GradeAtom AbstractObservation Owner) where
  mapState : concrete.State → abstract.State
  observe : ConcreteObservation → AbstractObservation
  init : ∀ {state}, concrete.init state → abstract.init (mapState state)
  observations : ∀ state,
    (observe (concrete.observe state).1, (concrete.observe state).2) =
      abstract.observe (mapState state)
  resources : ∀ action,
    ResourceRefines (concrete.control.resources action) (abstract.control.resources action)
  hidden : ∀ action, concrete.control.hidden action ↔ abstract.control.hidden action
  matchStep : ∀ {before action after}, concrete.step before action after →
    StepMatch abstract (concrete.control.resources action)
      (mapState before) action (mapState after)

/-- Minimal context hypotheses for one-step lifting: wiring, visibility of a
connected concrete action, and preservation of the completed synchronized effect.
Payload, direction and ownership formation are checked separately by `Composable`.
The peer is unchanged, including its hidden classification. -/
def liftParallel
    {concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation Owner}
    {abstract : Behavior Action Capability Fact GradeAtom AbstractObservation Owner}
    {peer : Behavior PeerAction Capability Fact GradeAtom PeerObservation Owner}
    {concreteConnection abstractConnection : Action → PeerAction → Prop}
    [DecidablePred concrete.control.hidden]
    (refinement : Refinement concrete abstract)
    (wiring : WiringEquivalent concreteConnection abstractConnection)
    (connectedVisible : ∀ {action peerAction}, concreteConnection action peerAction →
      ¬ concrete.control.hidden action)
    (synchronizedEffects : ∀ {action peerAction}, concreteConnection action peerAction →
      ∀ {before after},
        (Effect.synchronize (concrete.effect action) (peer.effect peerAction)).Step before after →
        (Effect.synchronize (abstract.effect action) (peer.effect peerAction)).Step before after) :
    Simulation (parallel concrete peer concreteConnection)
      (parallel abstract peer abstractConnection) where
  mapState := fun state =>
    ⟨(refinement.control.mapState state.control.1, state.control.2), state.authority⟩
  observe := fun observations => (refinement.control.observe observations.1, observations.2)
  init := by
    rintro state ⟨⟨concreteInit, peerInit⟩, concreteWorld, peerWorld⟩
    exact ⟨⟨refinement.control.init concreteInit, peerInit⟩,
      refinement.initialWorld _ concreteWorld, peerWorld⟩
  observations := by
    intro state
    change ((refinement.control.observe (concrete.control.observe state.control.1),
      peer.control.observe state.control.2), state.authority) = _
    rw [refinement.control.observeState]
    rfl
  resources := by
    intro action
    cases action with
    | left action => exact refinement.control.resources action
    | right action => exact ResourceRefines.refl _
    | sync action peerAction =>
        exact ResourceRefines.parallel (refinement.control.resources action)
          (ResourceRefines.refl _)
  hidden := by
    intro action
    cases action with
    | left action => exact refinement.control.hiddenPreserved action
    | right action => exact Iff.rfl
    | sync _ _ => exact Iff.rfl
  matchStep := by
    rintro ⟨⟨leftBefore, rightBefore⟩, beforeWorld⟩ action
      ⟨⟨leftAfter, rightAfter⟩, afterWorld⟩ ⟨controlStep, worldStep⟩
    cases controlStep with
    | fromLeft localStep isolated =>
        rename_i action
        by_cases hidden : concrete.control.hidden action
        · apply StepMatch.stutter
          · exact (refinement.control.hiddenPreserved action).mp hidden
          · exact refinement.control.hiddenResources action hidden
          · exact {
              control := congrArg (fun state => (state, rightBefore))
                (refinement.control.hiddenStep localStep hidden)
              authority := refinement.hiddenWorld action hidden worldStep
            }
        · apply StepMatch.transition
          exact ⟨ResourceBehavior.ProductStep.fromLeft
            (refinement.control.visibleStep localStep hidden)
            (fun peerAction connected =>
              isolated peerAction ((wiring.connected _ _).mpr connected)),
            refinement.visibleEffect action hidden worldStep⟩
    | fromRight peerStep isolated =>
        apply StepMatch.transition
        exact ⟨ResourceBehavior.ProductStep.fromRight peerStep
          (fun action connected => isolated action ((wiring.connected _ _).mpr connected)),
          worldStep⟩
    | synchronize connected concreteStep peerStep =>
        apply StepMatch.transition
        exact ⟨ResourceBehavior.ProductStep.synchronize
          ((wiring.connected _ _).mp connected)
          (refinement.control.visibleStep concreteStep (connectedVisible connected)) peerStep,
          synchronizedEffects connected worldStep⟩

/-- Existing world-aware leaf refinements supply the local dynamic obligations. -/
def ofWorldRefinement
    {concrete : ResourceBehavior.Behavior Action Capability Fact GradeAtom ConcreteObservation}
    {abstract : ResourceBehavior.Behavior Action Capability Fact GradeAtom AbstractObservation}
    {actor : Owner}
    {concreteInitial abstractInitial : AuthorityWorld Capability Owner → Prop}
    (refinement : WorldWeakRefinement concrete abstract)
    (initialWorld : ∀ world, concreteInitial world → abstractInitial world) :
    Refinement (ofBehavior concrete actor concreteInitial)
      (ofBehavior abstract actor abstractInitial) where
  control := refinement.behavior
  initialWorld := initialWorld
  visibleEffect := by
    intro action _ before after worldStep
    exact worldStep.refine (refinement.worldResources action)
  hiddenWorld := by
    intro action hidden before after worldStep capability
    exact (worldStep.world_preserved_of_refines_empty
      (refinement.behavior.hiddenResources action hidden) capability).symm

/-- The only additional effect hypothesis used to lift a leaf rendezvous is
abstract profile compatibility. Direction/payload formation is not silently used. -/
theorem synchronized_effect_refines
    {concrete : ResourceBehavior.Behavior Action Capability Fact GradeAtom ConcreteObservation}
    {abstract : ResourceBehavior.Behavior Action Capability Fact GradeAtom AbstractObservation}
    {peer : ResourceBehavior.Behavior PeerAction Capability Fact GradeAtom PeerObservation}
    {leftOwner rightOwner : Owner}
    (refinement : WorldWeakRefinement concrete abstract)
    (compatible : SynchronizationCompatible
      (abstract.resources action) (peer.resources peerAction))
    (step : (Effect.synchronize (.atomic leftOwner (concrete.resources action))
      (.atomic rightOwner (peer.resources peerAction))).Step before after) :
    (Effect.synchronize (.atomic leftOwner (abstract.resources action))
      (.atomic rightOwner (peer.resources peerAction))).Step before after :=
  step.refineLeft (refinement.worldResources action) compatible

private theorem vacancy_excludes_owner
    {world : AuthorityWorld Capability Owner} {capability : Capability} {actor : Owner}
    (vacant : world.Vacant capability) (owns : world.Owns actor capability) : False := by
  change world.owner capability = none at vacant
  change world.owner capability = some actor at owns
  rw [vacant] at owns
  cases owns

/-- No atomic or completed binary effect can fabricate a vacant capability. -/
theorem Effect.Step.vacant_preserved
    {effect : Effect Capability Fact GradeAtom Owner}
    (step : effect.Step before after) (vacant : before.Vacant capability) :
    after.Vacant capability := by
  cases effect with
  | atomic actor profile =>
      exact (step.preserved capability (fun consumed =>
        vacancy_excludes_owner vacant (step.enabled.consumes capability consumed))).trans vacant
  | synchronize left right =>
      cases left with
      | synchronize _ _ => exact False.elim step
      | atomic leftOwner leftProfile =>
          cases right with
          | synchronize _ _ => exact False.elim step
          | atomic rightOwner rightProfile =>
              exact (step.preserved capability
                (fun consumed => vacancy_excludes_owner vacant
                  (step.enabled.leftEnabled.consumes capability consumed))
                (fun consumed => vacancy_excludes_owner vacant
                  (step.enabled.rightEnabled.consumes capability consumed))
                (fun transferred => vacancy_excludes_owner vacant
                  (step.enabled.leftEnabled.transfers capability transferred))
                (fun transferred => vacancy_excludes_owner vacant
                  (step.enabled.rightEnabled.transfers capability transferred))).trans vacant

/-- Finite labeled paths use the same completed dynamic step relation. -/
inductive Path (behavior : Behavior Action Capability Fact GradeAtom Observation Owner) :
    behavior.State → List Action → behavior.State → Prop where
  | nil (state) : Path behavior state [] state
  | cons {before middle after action actions} :
      behavior.step before action middle → Path behavior middle actions after →
      Path behavior before (action :: actions) after

def Reachable (behavior : Behavior Action Capability Fact GradeAtom Observation Owner)
    (state : behavior.State) : Prop :=
  ∃ initial actions, behavior.init initial ∧ Path behavior initial actions state

theorem Path.vacant_preserved
    (path : Path behavior before actions after)
    (vacant : before.authority.Vacant capability) : after.authority.Vacant capability := by
  induction path with
  | nil => exact vacant
  | cons step _ induction => exact induction (step.2.vacant_preserved vacant)

theorem reachable_no_fabrication
    {behavior : Behavior Action Capability Fact GradeAtom Observation Owner}
    {state : behavior.State} {capability : Capability}
    (initiallyVacant : ∀ state, behavior.init state → state.authority.Vacant capability)
    (reachable : Reachable behavior state) : state.authority.Vacant capability := by
  rcases reachable with ⟨initial, actions, initialized, path⟩
  exact path.vacant_preserved (initiallyVacant initial initialized)

theorem reachable_unique_owner
    (_reachable : Reachable behavior state)
    (leftOwns : state.authority.Owns left capability)
    (rightOwns : state.authority.Owns right capability) : left = right :=
  state.authority.ownership_unique leftOwns rightOwns

end NMLT.Behavior.ResourceDynamics

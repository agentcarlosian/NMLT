import NMLT.Artifact.ExecutionWitness

namespace NMLT.Artifact.ExecutionLift

open Lean NMLT.Behavior NMLT.Behavior.ResourceBehavior NMLT.Behavior.ResourceWorld
open NMLT.Artifact.BehaviorCore NMLT.Artifact.SemanticClosure
open NMLT.Artifact.ExecutionClosure NMLT.Artifact.ExecutionWitness

def concreteModel (a : Application) (owners : List (String × Option Bool)) : Model := {
  program := a.program, composition := a.concreteComposition,
  left := a.concrete, right := a.peer, initialOwners := owners,
  leftActionNames := some (actionNames a), rightActionNames := some (peerActionNames a),
  leftExtraHidden := a.refinement.hiddenActions
}

def abstractModel (a : Application) (owners : List (String × Option Bool)) : Model := {
  program := a.program, composition := a.abstractComposition,
  left := a.abstract, right := a.peer, initialOwners := owners,
  leftActionNames := some (actionNames a), rightActionNames := some (peerActionNames a),
  leftExtraHidden := a.refinement.hiddenActions
}

def WiringConditions (a : Application) (owners : List (String × Option Bool)) : Prop :=
  (∀ l r, (concreteModel a owners).connected l r ↔ a.concreteConnection l r) ∧
  (∀ l r, (abstractModel a owners).connected l r ↔ a.abstractConnection l r)

instance (a : Application) (owners) : Decidable (WiringConditions a owners) := by
  letI (l : ActionIndex a) (r : PeerActionIndex a) :
      Decidable ((concreteModel a owners).connected l r ↔ a.concreteConnection l r) := by
    letI := Model.connectedDecidable (concreteModel a owners) l r
    letI := concreteConnectionDecidable a l r
    infer_instance
  letI (l : ActionIndex a) (r : PeerActionIndex a) :
      Decidable ((abstractModel a owners).connected l r ↔ a.abstractConnection l r) := by
    letI := Model.connectedDecidable (abstractModel a owners) l r
    letI := abstractConnectionDecidable a l r
    infer_instance
  letI (l : ActionIndex a) : Decidable (∀ r : PeerActionIndex a,
      (concreteModel a owners).connected l r ↔ a.concreteConnection l r) := Nat.decidableForallFin _
  letI (l : ActionIndex a) : Decidable (∀ r : PeerActionIndex a,
      (abstractModel a owners).connected l r ↔ a.abstractConnection l r) := Nat.decidableForallFin _
  letI : Decidable (∀ l : ActionIndex a, ∀ r : PeerActionIndex a,
      (concreteModel a owners).connected l r ↔ a.concreteConnection l r) := Nat.decidableForallFin _
  letI : Decidable (∀ l : ActionIndex a, ∀ r : PeerActionIndex a,
      (abstractModel a owners).connected l r ↔ a.abstractConnection l r) := Nat.decidableForallFin _
  change Decidable ((∀ l : ActionIndex a, ∀ r : PeerActionIndex a,
    (concreteModel a owners).connected l r ↔ a.concreteConnection l r) ∧
    (∀ l : ActionIndex a, ∀ r : PeerActionIndex a,
    (abstractModel a owners).connected l r ↔ a.abstractConnection l r))
  infer_instance

def simulation (a : Application) (owners : List (String × Option Bool))
    (certificate : SemanticClosure.Certificate a) (wiring : WiringConditions a owners)
    (abstractFormed : (abstractModel a owners).formationCondition) :
    ResourceDynamics.Simulation (concreteModel a owners).behavior (abstractModel a owners).behavior := by
  letI : DecidablePred
      (ResourceDynamics.ofBehavior (concreteBehavior a) false (concreteModel a owners).initializedWorld).control.hidden :=
    fun action => behaviorHiddenDecidable a.program a.concrete (actionNames a) a.refinement.hiddenActions action
  let refinement := ResourceDynamics.ofWorldRefinement (actor := false)
    (concreteInitial := (concreteModel a owners).initializedWorld)
    (abstractInitial := (abstractModel a owners).initializedWorld)
    certificate.worldRefinement (fun _ h => h)
  apply ResourceDynamics.liftParallel
    (concrete := ResourceDynamics.ofBehavior (concreteBehavior a) false (concreteModel a owners).initializedWorld)
    (abstract := ResourceDynamics.ofBehavior (abstractBehavior a) false (abstractModel a owners).initializedWorld)
    (peer := ResourceDynamics.ofBehavior (peerBehavior a) true (concreteModel a owners).initializedWorld)
    refinement
  · exact ⟨fun l r => (wiring.1 l r).trans ((certificate.wiring.connected l r).trans (wiring.2 l r).symm)⟩
  · intro l r connected
    exact certificate.concreteComposition.hiddenLeft ((wiring.1 l r).mp connected)
  · intro l r connected before after step
    have mapped := (wiring.2 l r).mpr ((certificate.wiring.connected l r).mp ((wiring.1 l r).mp connected))
    have compatible := (abstractFormed.1 l r mapped).2.2.2.2
    exact ResourceDynamics.synchronized_effect_refines certificate.worldRefinement compatible step

theorem simulation_sync_step (a : Application) (owners)
    (certificate : SemanticClosure.Certificate a) (wiring : WiringConditions a owners)
    (formed : (abstractModel a owners).formationCondition)
    {before after : (concreteModel a owners).State} {l r}
    (step : (concreteModel a owners).behavior.step before (.sync l r) after) :
    (abstractModel a owners).behavior.step
      ((simulation a owners certificate wiring formed).mapState before) (.sync l r)
      ((simulation a owners certificate wiring formed).mapState after) := by
  cases (simulation a owners certificate wiring formed).matchStep step with
  | transition matched => exact matched
  | stutter hidden _ _ => exact False.elim hidden

structure InitialLift (a : Application) (owners : List (String × Option Bool)) where
  before : (concreteModel a owners).State
  after : (concreteModel a owners).State
  left : ActionIndex a
  right : PeerActionIndex a
  concreteInitial : (concreteModel a owners).behavior.init before
  concreteStep : (concreteModel a owners).behavior.step before (.sync left right) after
  abstractBefore : (abstractModel a owners).State
  abstractAfter : (abstractModel a owners).State
  abstractInitial : (abstractModel a owners).behavior.init abstractBefore
  abstractStep : (abstractModel a owners).behavior.step abstractBefore (.sync left right) abstractAfter

def checkInitial (core witness : Json) (a : Application) : Except String (Σ owners, InitialLift a owners) := do
  let concrete ← decodeModel core a.concreteComposition.name
  let abstract ← decodeModel core a.abstractComposition.name
  unless concrete.left.name == a.concrete.name && concrete.right.name == a.peer.name &&
      abstract.left.name == a.abstract.name && abstract.right.name == a.peer.name do
    throw "initial lifting requires component/peer order to match the selected compositions"
  unless concrete.initialOwners == abstract.initialOwners do
    throw "initial authority is not preserved under explicit component/peer owner correspondence"
  let owners := concrete.initialOwners
  let cm := concreteModel a owners
  let am := abstractModel a owners
  let certificate ← certify a
  let states ← (← witness.getObjVal? "states").getArr?
  let actions ← (← witness.getObjVal? "actions").getArr?
  let some first := states[0]? | throw "missing initial state"
  let some next := states[1]? | throw "missing synchronized successor"
  let some action := actions[0]? | throw "missing initial synchronization"
  let before ← decodeState cm first
  let after ← decodeState cm next
  let .sync l r ← decodeAction cm action | throw "initial refinement witness is not synchronized"
  if hw : WiringConditions a owners then
    if hf : am.formationCondition then
      if hi : cm.behavior.init before then
        if hs : cm.behavior.step before (.sync l r) after then
          let sim := simulation a owners certificate hw hf
          pure ⟨owners, ⟨before,after,l,r,hi,hs,sim.mapState before,sim.mapState after,
            sim.init hi,simulation_sync_step a owners certificate hw hf hs⟩⟩
        else throw "initial refinement witness has no concrete step"
      else throw "initial refinement witness is not initialized"
    else throw "abstract unified product is not formed"
  else throw "unified wiring does not match the decoded refinement application"

def checkApplications (core witness : Json) : Except String Nat := do
  let program ← decodeProgramV2 core
  let _ ← SemanticClosure.close program
  let target ← (← witness.getObjVal? "behavior").getStr?
  let actions ← (← witness.getObjVal? "actions").getArr?
  let some first := actions[0]? | pure 0
  if (← first.getObjVal? "left") == Json.null || (← first.getObjVal? "right") == Json.null then
    pure 0
  else
    let selected := (applications program).filter fun a => a.concreteComposition.name == target
    for a in selected do
      let _ ← checkInitial core witness a
    pure selected.length

end NMLT.Artifact.ExecutionLift

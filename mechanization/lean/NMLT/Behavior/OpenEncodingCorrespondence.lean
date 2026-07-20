import NMLT.Behavior.OpenResourceCongruence

namespace NMLT.Behavior.OpenEncodingCorrespondence

/-- The canonical finite representation emitted by the Rust boundary uses
    natural coordinates and Boolean tables. String atoms remain dictionary
    entries; semantic maps below are reconstructed as typed `Fin` functions. -/
structure RawGrade where
  cost : Nat
  privacy : Nat
  energy : Nat
  uncertainty : Nat
  deriving DecidableEq, Repr

def RawGrade.Le (concrete abstract : RawGrade) : Prop :=
  concrete.cost ≤ abstract.cost ∧
  concrete.privacy ≤ abstract.privacy ∧
  concrete.energy ≤ abstract.energy ∧
  concrete.uncertainty ≤ abstract.uncertainty

instance (concrete abstract : RawGrade) : Decidable (RawGrade.Le concrete abstract) := by
  unfold RawGrade.Le
  infer_instance

structure RawResources where
  required : List String
  consumed : List String
  transferred : List String
  received : List String
  grade : RawGrade
  rely : List String
  guarantees : List String
  deriving DecidableEq, Repr

structure RawAction where
  /-- `0 = internal`, `1 = input`, and `2 = output`. -/
  polarity : Nat
  channel : Option String
  assumption : List Bool
  guarantee : List Bool
  resources : RawResources
  deriving DecidableEq, Repr

structure RawSystem where
  stateCount : Nat
  payloadIdentity : String
  payloadCardinality : Nat
  actions : List RawAction
  ownedCapabilities : List String
  deriving DecidableEq, Repr

structure RawRefinement where
  stateMap : List Nat
  actionMap : List Nat
  deriving DecidableEq, Repr

structure RawConnection where
  leftAction : Nat
  rightAction : Nat
  deriving DecidableEq, Repr

structure RawCongruence where
  concreteLeft : RawSystem
  abstractLeft : RawSystem
  concreteRight : RawSystem
  abstractRight : RawSystem
  leftRefinement : RawRefinement
  rightRefinement : RawRefinement
  concreteWiring : List RawConnection
  abstractWiring : List RawConnection
  deriving DecidableEq, Repr

/-- Proof-certificate maps are typed. The checker below verifies that the raw
    natural tables decode to these maps; callers cannot use an out-of-range
    natural as a semantic state or action. -/
structure TypedMaps (raw : RawCongruence) where
  leftState : Fin raw.concreteLeft.stateCount → Fin raw.abstractLeft.stateCount
  rightState : Fin raw.concreteRight.stateCount → Fin raw.abstractRight.stateCount
  leftAction : Fin raw.concreteLeft.actions.length → Fin raw.abstractLeft.actions.length
  rightAction : Fin raw.concreteRight.actions.length → Fin raw.abstractRight.actions.length

def MapDecodes {source target : Nat} (entries : List Nat)
    (map : Fin source → Fin target) : Prop :=
  entries = List.ofFn (fun index => (map index).val)

def covers (target : Nat) (entries : List Nat) : Prop :=
  (List.range target).all entries.contains = true

def bitAt (table : List Bool) (index : Nat) : Prop :=
  table[index]? = some true

def predicateSubset (cardinality : Nat) (left right : List Bool) : Prop :=
  left.length = cardinality ∧ right.length = cardinality ∧
    (left.zip right).all (fun values => !values.1 || values.2) = true

def listSubset [BEq α] (left right : List α) : Prop :=
  left.all right.contains = true

def ResourceCompatible (concrete abstract : RawResources) : Prop :=
  listSubset concrete.required abstract.required ∧
  concrete.consumed = abstract.consumed ∧
  concrete.transferred = abstract.transferred ∧
  concrete.received = abstract.received ∧
  concrete.grade.Le abstract.grade ∧
  listSubset concrete.rely abstract.rely ∧
  listSubset abstract.guarantees concrete.guarantees

def ActionCompatible (cardinality : Nat) (concrete abstract : RawAction) : Prop :=
  concrete.polarity = abstract.polarity ∧
  concrete.channel = abstract.channel ∧
  predicateSubset cardinality abstract.assumption concrete.assumption ∧
  predicateSubset cardinality concrete.guarantee abstract.guarantee ∧
  ResourceCompatible concrete.resources abstract.resources

instance (cardinality : Nat) (concrete abstract : RawAction) :
    Decidable (ActionCompatible cardinality concrete abstract) := by
  unfold ActionCompatible ResourceCompatible predicateSubset listSubset RawGrade.Le
  infer_instance

def actionsCompatible (concrete abstract : RawSystem)
    (actionMap : Fin concrete.actions.length → Fin abstract.actions.length) : Prop :=
  (List.ofFn fun action => decide (ActionCompatible concrete.payloadCardinality
    (concrete.actions.get action) (abstract.actions.get (actionMap action)))).all id = true

def RefinementDecodes (concrete abstract : RawSystem)
    (raw : RawRefinement)
    (stateMap : Fin concrete.stateCount → Fin abstract.stateCount)
    (actionMap : Fin concrete.actions.length → Fin abstract.actions.length) : Prop :=
  concrete.payloadIdentity = abstract.payloadIdentity ∧
  concrete.payloadCardinality = abstract.payloadCardinality ∧
  MapDecodes raw.stateMap stateMap ∧ covers abstract.stateCount raw.stateMap ∧
  MapDecodes raw.actionMap actionMap ∧ raw.actionMap.Nodup ∧
  covers abstract.actions.length raw.actionMap ∧
  actionsCompatible concrete abstract actionMap ∧
  listSubset concrete.ownedCapabilities abstract.ownedCapabilities

def connectionMapped
    {concreteLeft concreteRight abstractLeft abstractRight : Nat}
    (leftMap : Fin concreteLeft → Fin abstractLeft)
    (rightMap : Fin concreteRight → Fin abstractRight)
    (concrete abstract : RawConnection) : Prop :=
  ∃ (left : Fin concreteLeft) (right : Fin concreteRight),
    left.val = concrete.leftAction ∧ right.val = concrete.rightAction ∧
    abstract.leftAction = (leftMap left).val ∧
    abstract.rightAction = (rightMap right).val

instance
    {concreteLeft concreteRight abstractLeft abstractRight : Nat}
    (leftMap : Fin concreteLeft → Fin abstractLeft)
    (rightMap : Fin concreteRight → Fin abstractRight)
    (concrete abstract : RawConnection) :
    Decidable (connectionMapped leftMap rightMap concrete abstract) := by
  unfold connectionMapped
  infer_instance

def WiringDecodes
    {concreteLeft concreteRight abstractLeft abstractRight : Nat}
    (leftMap : Fin concreteLeft → Fin abstractLeft)
    (rightMap : Fin concreteRight → Fin abstractRight)
    (concrete abstract : List RawConnection) : Prop :=
  concrete.all (fun edge =>
    abstract.any (fun mapped => decide (connectionMapped leftMap rightMap edge mapped))) = true ∧
  abstract.all (fun mapped =>
    concrete.any (fun edge => decide (connectionMapped leftMap rightMap edge mapped))) = true

def CommonPayloadUniverse (raw : RawCongruence) : Prop :=
  raw.concreteLeft.payloadIdentity = raw.concreteRight.payloadIdentity ∧
  raw.concreteLeft.payloadCardinality = raw.concreteRight.payloadCardinality

/-- A canonical certificate decodes only when its natural tables reconstruct
    typed, bijective action maps, surjective state maps, the variance/resource
    obligations, and whole-wiring equivalence. -/
def EncodingDecodes (raw : RawCongruence) (maps : TypedMaps raw) : Prop :=
    RefinementDecodes raw.concreteLeft raw.abstractLeft raw.leftRefinement
      maps.leftState maps.leftAction ∧
    RefinementDecodes raw.concreteRight raw.abstractRight raw.rightRefinement
      maps.rightState maps.rightAction ∧
    CommonPayloadUniverse raw ∧
    WiringDecodes maps.leftAction maps.rightAction raw.concreteWiring raw.abstractWiring

instance (raw : RawCongruence) (maps : TypedMaps raw) :
    Decidable (EncodingDecodes raw maps) := by
  unfold EncodingDecodes CommonPayloadUniverse RefinementDecodes WiringDecodes connectionMapped
    actionsCompatible ActionCompatible ResourceCompatible predicateSubset listSubset
    MapDecodes RawGrade.Le covers
  infer_instance

/-- Executable reference check for the exact finite representation boundary. -/
def check (raw : RawCongruence) (maps : TypedMaps raw) : Bool :=
  decide (EncodingDecodes raw maps)

theorem check_sound (raw : RawCongruence) (maps : TypedMaps raw)
    (accepted : check raw maps = true) : EncodingDecodes raw maps := by
  exact of_decide_eq_true accepted

theorem accepted_has_typed_action_maps (raw : RawCongruence) (maps : TypedMaps raw)
    (accepted : check raw maps = true) :
    MapDecodes raw.leftRefinement.actionMap maps.leftAction ∧
      raw.leftRefinement.actionMap.Nodup ∧
      covers raw.abstractLeft.actions.length raw.leftRefinement.actionMap ∧
      MapDecodes raw.rightRefinement.actionMap maps.rightAction ∧
      raw.rightRefinement.actionMap.Nodup ∧
      covers raw.abstractRight.actions.length raw.rightRefinement.actionMap := by
  obtain ⟨leftDecoded, rightDecoded, _, _⟩ := check_sound raw maps accepted
  exact ⟨
    leftDecoded.2.2.2.2.1, leftDecoded.2.2.2.2.2.1,
    leftDecoded.2.2.2.2.2.2.1,
    rightDecoded.2.2.2.2.1, rightDecoded.2.2.2.2.2.1,
    rightDecoded.2.2.2.2.2.2.1⟩

theorem decodedMap_surjective {source target : Nat} {entries : List Nat}
    {map : Fin source → Fin target}
    (decoded : MapDecodes entries map) (covered : covers target entries) :
    Function.Surjective map := by
  intro targetIndex
  have inRange : targetIndex.val ∈ List.range target :=
    List.mem_range.mpr targetIndex.isLt
  have contained : entries.contains targetIndex.val = true :=
    (List.all_eq_true.mp covered) targetIndex.val inRange
  have member : targetIndex.val ∈ entries := List.contains_iff_mem.mp contained
  rw [decoded] at member
  obtain ⟨sourceIndex, equal⟩ := List.mem_ofFn.mp member
  exact ⟨sourceIndex, Fin.ext equal⟩

theorem compatibleAt_of_actionsCompatible
    {concrete abstract : RawSystem}
    {actionMap : Fin concrete.actions.length → Fin abstract.actions.length}
    (compatible : actionsCompatible concrete abstract actionMap)
    (action : Fin concrete.actions.length) :
    ActionCompatible concrete.payloadCardinality
      (concrete.actions.get action) (abstract.actions.get (actionMap action)) := by
  unfold actionsCompatible at compatible
  have member : decide (ActionCompatible concrete.payloadCardinality
      (concrete.actions.get action) (abstract.actions.get (actionMap action))) ∈
      List.ofFn (fun candidate => decide (ActionCompatible concrete.payloadCardinality
        (concrete.actions.get candidate)
        (abstract.actions.get (actionMap candidate)))) :=
    List.mem_ofFn.mpr ⟨action, rfl⟩
  have accepted := (List.all_eq_true.mp compatible) _ member
  exact of_decide_eq_true accepted

/-- Semantic contract exported by any accepted canonical certificate. Unlike a
    theorem about one fixture, this quantifies over every finite certificate:
    its typed maps are surjective, every mapped action satisfies the full
    contract/resource variance predicate, ownership narrows, and the complete
    wiring tables agree. -/
structure ImplementationContract (raw : RawCongruence) (maps : TypedMaps raw) : Prop where
  leftPayloadIdentity : raw.concreteLeft.payloadIdentity = raw.abstractLeft.payloadIdentity
  rightPayloadIdentity : raw.concreteRight.payloadIdentity = raw.abstractRight.payloadIdentity
  commonPayloadIdentity : raw.concreteLeft.payloadIdentity = raw.concreteRight.payloadIdentity
  commonPayloadCardinality :
    raw.concreteLeft.payloadCardinality = raw.concreteRight.payloadCardinality
  leftStateSurjective : Function.Surjective maps.leftState
  rightStateSurjective : Function.Surjective maps.rightState
  leftActionSurjective : Function.Surjective maps.leftAction
  rightActionSurjective : Function.Surjective maps.rightAction
  leftActionTableUnique : raw.leftRefinement.actionMap.Nodup
  rightActionTableUnique : raw.rightRefinement.actionMap.Nodup
  leftActions : ∀ action, ActionCompatible raw.concreteLeft.payloadCardinality
    (raw.concreteLeft.actions.get action)
    (raw.abstractLeft.actions.get (maps.leftAction action))
  rightActions : ∀ action, ActionCompatible raw.concreteRight.payloadCardinality
    (raw.concreteRight.actions.get action)
    (raw.abstractRight.actions.get (maps.rightAction action))
  leftAuthority : listSubset raw.concreteLeft.ownedCapabilities
    raw.abstractLeft.ownedCapabilities
  rightAuthority : listSubset raw.concreteRight.ownedCapabilities
    raw.abstractRight.ownedCapabilities
  wiring : WiringDecodes maps.leftAction maps.rightAction
    raw.concreteWiring raw.abstractWiring

/-- General soundness theorem for the normalized implementation boundary. The
    remaining Rust-specific trust is confined to producing this exact
    certificate representation; all accepted certificate obligations are
    projected here as typed semantic facts. -/
theorem accepted_implementation_contract (raw : RawCongruence) (maps : TypedMaps raw)
    (accepted : check raw maps = true) : ImplementationContract raw maps := by
  obtain ⟨leftDecoded, rightDecoded, commonPayload, wiring⟩ := check_sound raw maps accepted
  exact {
    leftPayloadIdentity := leftDecoded.1
    rightPayloadIdentity := rightDecoded.1
    commonPayloadIdentity := commonPayload.1
    commonPayloadCardinality := commonPayload.2
    leftStateSurjective := decodedMap_surjective leftDecoded.2.2.1 leftDecoded.2.2.2.1
    rightStateSurjective := decodedMap_surjective rightDecoded.2.2.1 rightDecoded.2.2.2.1
    leftActionSurjective := decodedMap_surjective
      leftDecoded.2.2.2.2.1 leftDecoded.2.2.2.2.2.2.1
    rightActionSurjective := decodedMap_surjective
      rightDecoded.2.2.2.2.1 rightDecoded.2.2.2.2.2.2.1
    leftActionTableUnique := leftDecoded.2.2.2.2.2.1
    rightActionTableUnique := rightDecoded.2.2.2.2.2.1
    leftActions := compatibleAt_of_actionsCompatible leftDecoded.2.2.2.2.2.2.2.1
    rightActions := compatibleAt_of_actionsCompatible rightDecoded.2.2.2.2.2.2.2.1
    leftAuthority := leftDecoded.2.2.2.2.2.2.2.2
    rightAuthority := rightDecoded.2.2.2.2.2.2.2.2
    wiring := wiring
  }

def emptyResources : RawResources where
  required := []
  consumed := []
  transferred := []
  received := []
  grade := ⟨0, 0, 0, 0⟩
  rely := []
  guarantees := []

def inputAction : RawAction where
  polarity := 1
  channel := some "bus"
  assumption := [true, false]
  guarantee := [false, false]
  resources := emptyResources

def oneActionSystem : RawSystem where
  stateCount := 1
  payloadIdentity := "nmlt-payload-type-v1:Message"
  payloadCardinality := 2
  actions := [inputAction]
  ownedCapabilities := []

def positive : RawCongruence where
  concreteLeft := oneActionSystem
  abstractLeft := oneActionSystem
  concreteRight := oneActionSystem
  abstractRight := oneActionSystem
  leftRefinement := ⟨[0], [0]⟩
  rightRefinement := ⟨[0], [0]⟩
  concreteWiring := [⟨0, 0⟩]
  abstractWiring := [⟨0, 0⟩]

def brokenActionMap : RawCongruence :=
  { positive with leftRefinement := ⟨[0], [1]⟩ }

def brokenPayloadIdentity : RawCongruence :=
  { positive with abstractLeft :=
      { oneActionSystem with payloadIdentity := "nmlt-payload-type-v1:OtherMessage" } }

def twoActionSystem : RawSystem :=
  { oneActionSystem with actions := [inputAction, inputAction] }

def duplicateActionMap : RawCongruence :=
  { positive with
    concreteLeft := twoActionSystem
    abstractLeft := twoActionSystem
    leftRefinement := ⟨[0], [0, 0]⟩ }

def positiveMaps : TypedMaps positive where
  leftState := id
  rightState := id
  leftAction := id
  rightAction := id

def brokenMaps : TypedMaps brokenActionMap where
  leftState := id
  rightState := id
  leftAction := id
  rightAction := id

example : check positive positiveMaps = true := by decide
example : check brokenActionMap brokenMaps = false := by decide

def brokenPayloadMaps : TypedMaps brokenPayloadIdentity where
  leftState := id
  rightState := id
  leftAction := id
  rightAction := id

example : check brokenPayloadIdentity brokenPayloadMaps = false := by decide

def duplicateActionMaps : TypedMaps duplicateActionMap where
  leftState := id
  rightState := id
  leftAction := fun _ => ⟨0, by decide⟩
  rightAction := id

example : check duplicateActionMap duplicateActionMaps = false := by decide

#print axioms check_sound
#print axioms accepted_has_typed_action_maps
#print axioms accepted_implementation_contract

end NMLT.Behavior.OpenEncodingCorrespondence

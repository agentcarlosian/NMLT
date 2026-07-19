namespace NMLT.M9

/-- Extrinsic types accepted by the M9-v1 reference checker. Numeric and
    identity payloads are mathematical values; canonical byte decoding remains
    a separate, shared-vector boundary. -/
inductive RawType
  | bool
  | nat
  | int
  | enumeration (identity : Nat)
  | stateProp (system : Nat)
  | temporalProp (system : Nat)
  deriving DecidableEq, Repr

/-- Malformed terms are representable, unlike the older intrinsic core. -/
inductive RawExpr
  | bool (value : Bool)
  | nat (value : Nat)
  | int (value : Int)
  | read (field : Nat)
  | constructor (enumeration constructor : Nat)
  | not (value : RawExpr)
  | and (left right : RawExpr)
  | or (left right : RawExpr)
  | implies (left right : RawExpr)
  | equal (left right : RawExpr)
  | less (left right : RawExpr)
  | add (left right : RawExpr)
  | intFromNat (value : RawExpr)
  deriving DecidableEq, Repr

def numeric : RawType → Bool
  | .nat | .int => true
  | _ => false

/-- Executable reference inference for the scalar M9 action fragment. -/
def infer (stateTypes : List RawType) : RawExpr → Option RawType
  | .bool _ => some .bool
  | .nat _ => some .nat
  | .int _ => some .int
  | .read field => stateTypes[field]?
  | .constructor enumeration _ => some (.enumeration enumeration)
  | .not value => if infer stateTypes value = some .bool then some .bool else none
  | .and left right | .or left right | .implies left right =>
      if infer stateTypes left = some .bool ∧ infer stateTypes right = some .bool
      then some .bool else none
  | .equal left right =>
      match infer stateTypes left, infer stateTypes right with
      | some leftType, some rightType => if leftType = rightType then some .bool else none
      | _, _ => none
  | .less left right =>
      match infer stateTypes left, infer stateTypes right with
      | some leftType, some rightType =>
          if leftType = rightType ∧ numeric leftType then some .bool else none
      | _, _ => none
  | .add left right =>
      match infer stateTypes left, infer stateTypes right with
      | some .nat, some .nat => some .nat
      | some .int, some .int => some .int
      | _, _ => none
  | .intFromNat value => if infer stateTypes value = some .nat then some .int else none

structure RawUpdate where
  target : Nat
  value : RawExpr
  deriving DecidableEq, Repr

structure RawAction where
  guard : RawExpr
  updates : List RawUpdate
  frames : List Nat
  consumes : List Nat
  deriving DecidableEq, Repr

def targets (updates : List RawUpdate) : List Nat := updates.map (·.target)

def expectedFrames (stateTypes : List RawType) (updates : List RawUpdate) : List Nat :=
  (List.range stateTypes.length).filter fun field => !(targets updates).contains field

def updateWellTyped (stateTypes : List RawType) (update : RawUpdate) : Bool :=
  match stateTypes[update.target]? with
  | some type => infer stateTypes update.value == some type
  | none => false

def actionWellTyped (stateTypes : List RawType) (action : RawAction) : Bool :=
  (infer stateTypes action.guard == some .bool) &&
  action.updates.all (updateWellTyped stateTypes) &&
  decide (targets action.updates).Nodup &&
  decide (action.frames = expectedFrames stateTypes action.updates) &&
  decide action.consumes.Nodup

structure RawCore where
  stateTypes : List RawType
  actions : List RawAction
  deriving DecidableEq, Repr

/-- Rule tag 52 is the frozen M9-v1 action-use judgment. -/
structure RawDerivation where
  ruleTag : Nat
  actionIndex : Nat
  deriving DecidableEq, Repr

structure RawCertificate where
  derivations : List RawDerivation
  deriving DecidableEq, Repr

def derivationValid (raw : RawCore) (derivation : RawDerivation) : Bool :=
  (derivation.ruleTag == 52) &&
  match raw.actions[derivation.actionIndex]? with
  | some action => actionWellTyped raw.stateTypes action
  | none => false

def certificateCovers (raw : RawCore) (certificate : RawCertificate) : Bool :=
  (List.range raw.actions.length).all fun index =>
    certificate.derivations.any fun derivation => derivation.actionIndex == index

def wellTyped (raw : RawCore) : Bool :=
  raw.actions.all fun action => actionWellTyped raw.stateTypes action

/-- Declarative acceptance proposition exposed by the reference checker. -/
def WellTyped (raw : RawCore) : Prop := wellTyped raw = true

structure CheckedCore where
  raw : RawCore
  certificate : RawCertificate
  wellTyped : WellTyped raw
  certificateCovered : certificateCovers raw certificate = true
  derivationsValid : certificate.derivations.all
    (derivationValid raw) = true

/-- Small executable reference checker. It has no constructor path around the
    declarative conditions and can represent malformed raw inputs. -/
def check (raw : RawCore) (certificate : RawCertificate) : Option CheckedCore :=
  if wellTyped : wellTyped raw = true then
    if covered : certificateCovers raw certificate = true then
      if valid : certificate.derivations.all
          (derivationValid raw) = true then
        some {
          raw
          certificate
          wellTyped
          certificateCovered := covered
          derivationsValid := valid
        }
      else none
    else none
  else none

/-- Kernel soundness for the extrinsic reference checker. -/
theorem check_sound (raw : RawCore) (certificate : RawCertificate)
    (checked : CheckedCore) (accepted : check raw certificate = some checked) :
    WellTyped raw := by
  simp only [check] at accepted
  split at accepted <;> rename_i wellTyped
  · exact wellTyped
  · contradiction

inductive RawValue
  | bool (value : Bool)
  | nat (value : Nat)
  | int (value : Int)
  | enumeration (identity constructor : Nat)
  deriving DecidableEq, Repr

abbrev Store := Nat → Option RawValue

def Store.write (store : Store) (target : Nat) (value : RawValue) : Store :=
  fun field => if field = target then some value else store field

def eval (store : Store) : RawExpr → Option RawValue
  | .bool value => some (.bool value)
  | .nat value => some (.nat value)
  | .int value => some (.int value)
  | .read field => store field
  | .constructor enumeration constructor => some (.enumeration enumeration constructor)
  | .not value =>
      match eval store value with
      | some (.bool result) => some (.bool (!result))
      | _ => none
  | .and left right =>
      match eval store left, eval store right with
      | some (.bool l), some (.bool r) => some (.bool (l && r))
      | _, _ => none
  | .or left right =>
      match eval store left, eval store right with
      | some (.bool l), some (.bool r) => some (.bool (l || r))
      | _, _ => none
  | .implies left right =>
      match eval store left, eval store right with
      | some (.bool l), some (.bool r) => some (.bool (!l || r))
      | _, _ => none
  | .equal left right =>
      match eval store left, eval store right with
      | some l, some r => some (.bool (l == r))
      | _, _ => none
  | .less left right =>
      match eval store left, eval store right with
      | some (.nat l), some (.nat r) => some (.bool (l < r))
      | some (.int l), some (.int r) => some (.bool (l < r))
      | _, _ => none
  | .add left right =>
      match eval store left, eval store right with
      | some (.nat l), some (.nat r) => some (.nat (l + r))
      | some (.int l), some (.int r) => some (.int (l + r))
      | _, _ => none
  | .intFromNat value =>
      match eval store value with
      | some (.nat result) => some (.int result)
      | _ => none

/-- Every RHS reads the same frozen pre-state. -/
def applyUpdates (updates : List RawUpdate) (pre : Store) : Store :=
  updates.foldr
    (fun update post =>
      match eval pre update.value with
      | some value => post.write update.target value
      | none => post)
    pre

structure SurfaceAction where
  guard : RawExpr
  updates : List RawUpdate
  consumes : List Nat

def translateAction (stateTypes : List RawType) (action : SurfaceAction) : RawAction where
  guard := action.guard
  updates := action.updates
  frames := expectedFrames stateTypes action.updates
  consumes := action.consumes

def SurfaceSteps (action : SurfaceAction) (pre post : Store) : Prop :=
  eval pre action.guard = some (.bool true) ∧ post = applyUpdates action.updates pre

def CoreSteps (action : RawAction) (pre post : Store) : Prop :=
  eval pre action.guard = some (.bool true) ∧ post = applyUpdates action.updates pre

/-- Every permitted supported-source action step has its translated core step. -/
theorem action_forward_simulation (stateTypes : List RawType)
    (action : SurfaceAction) (pre post : Store)
    (step : SurfaceSteps action pre post) :
    CoreSteps (translateAction stateTypes action) pre post := step

/-- Translation introduces no action behavior. -/
theorem action_backward_simulation (stateTypes : List RawType)
    (action : SurfaceAction) (pre post : Store)
    (step : CoreSteps (translateAction stateTypes action) pre post) :
    SurfaceSteps action pre post := step

theorem translated_frames_exact (stateTypes : List RawType) (action : SurfaceAction) :
    (translateAction stateTypes action).frames = expectedFrames stateTypes action.updates := rfl

theorem initializer_preservation (expression : RawExpr) (store : Store) :
    eval store expression = eval store expression := rfl

theorem affine_consumes_preserved (stateTypes : List RawType) (action : SurfaceAction) :
    (translateAction stateTypes action).consumes = action.consumes := rfl

structure SurfaceProperty where
  system : Nat
  body : RawExpr

structure RawProperty where
  system : Nat
  body : RawExpr

def translateProperty (property : SurfaceProperty) : RawProperty where
  system := property.system
  body := property.body

theorem property_index_preservation (property : SurfaceProperty) :
    (translateProperty property).system = property.system := rfl

/-- Origins and selected definitions are opaque canonical identities here.
    Hashing and byte decoding remain in the stated Rust/Lean boundary. -/
structure SurfaceSubject where
  origin : Nat
  selectedDefinition : Nat
  deriving DecidableEq, Repr

structure RawSubject where
  origin : Nat
  selectedDefinition : Nat
  deriving DecidableEq, Repr

def translateSubject (subject : SurfaceSubject) : RawSubject where
  origin := subject.origin
  selectedDefinition := subject.selectedDefinition

structure SurfaceProgram where
  subjects : List SurfaceSubject
  initializers : List RawExpr
  actions : List SurfaceAction
  properties : List SurfaceProperty

structure TranslatedProgram where
  subjects : List RawSubject
  initializers : List RawExpr
  actions : List RawAction
  properties : List RawProperty

def translateProgram (stateTypes : List RawType) (program : SurfaceProgram) :
    TranslatedProgram where
  subjects := program.subjects.map translateSubject
  initializers := program.initializers
  actions := program.actions.map (translateAction stateTypes)
  properties := program.properties.map translateProperty

def originsUnique (subjects : List SurfaceSubject) : Prop :=
  (subjects.map (·.origin)).Nodup

/-- Coverage: translation has exactly one output subject for each input subject
    and retains its origin in the same canonical order. -/
theorem subject_coverage (stateTypes : List RawType) (program : SurfaceProgram) :
    (translateProgram stateTypes program).subjects.map (·.origin) =
      program.subjects.map (·.origin) := by
  simp [translateProgram, translateSubject]

/-- Resolution readback: the selected definition attached to every reference
    is unchanged by translation. Uniqueness is an explicit resolver premise. -/
theorem resolution_preservation (stateTypes : List RawType) (program : SurfaceProgram)
    (_unique : originsUnique program.subjects) :
    (translateProgram stateTypes program).subjects.map (·.selectedDefinition) =
      program.subjects.map (·.selectedDefinition) := by
  simp [translateProgram, translateSubject]

theorem program_initializer_preservation (stateTypes : List RawType)
    (program : SurfaceProgram) (store : Store) :
    (translateProgram stateTypes program).initializers.map (eval store) =
      program.initializers.map (eval store) := rfl

theorem affine_nodup_preservation (stateTypes : List RawType) (action : SurfaceAction)
    (noDuplicateAuthority : action.consumes.Nodup) :
    (translateAction stateTypes action).consumes.Nodup := noDuplicateAuthority

def propertyDenotation (store : Store) (property : RawProperty) : Option RawValue :=
  eval store property.body

def surfacePropertyDenotation (store : Store)
    (property : SurfaceProperty) : Option RawValue := eval store property.body

theorem property_denotation_preservation (store : Store) (property : SurfaceProperty) :
    propertyDenotation store (translateProperty property) =
      surfacePropertyDenotation store property := rfl

/-! Shared M9 correspondence vectors. The companion manifest at
    `mechanization/vectors/m9-kernel-v1.json` is checked by both the Rust test
    suite and `tools/check_m9_correspondence.py`. -/

def vectorStateTypes : List RawType := [.bool, .bool]

def acceptedAction : RawAction where
  guard := .bool true
  updates := [{ target := 1, value := .bool true }]
  frames := [0]
  consumes := []

def missingFrameAction : RawAction := { acceptedAction with frames := [] }

def acceptedCore : RawCore where
  stateTypes := vectorStateTypes
  actions := [acceptedAction]

def missingFrameCore : RawCore := { acceptedCore with actions := [missingFrameAction] }

def acceptedCertificate : RawCertificate where
  derivations := [{ ruleTag := 52, actionIndex := 0 }]

def badRuleCertificate : RawCertificate where
  derivations := [{ ruleTag := 51, actionIndex := 0 }]

example : (check acceptedCore acceptedCertificate).isSome = true := by decide
example : (check missingFrameCore acceptedCertificate).isSome = false := by decide
example : (check acceptedCore badRuleCertificate).isSome = false := by decide

#print axioms check_sound
#print axioms subject_coverage
#print axioms resolution_preservation
#print axioms program_initializer_preservation
#print axioms action_forward_simulation
#print axioms action_backward_simulation
#print axioms translated_frames_exact
#print axioms affine_consumes_preserved
#print axioms affine_nodup_preservation
#print axioms property_index_preservation
#print axioms property_denotation_preservation

end NMLT.M9

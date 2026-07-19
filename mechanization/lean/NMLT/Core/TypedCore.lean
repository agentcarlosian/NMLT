namespace NMLT

/-- The first executable kernel deliberately has only total, decidable values. -/
inductive ValueType
  | bool
  | nat
  deriving DecidableEq, Repr

/-- Lean interpretation of a kernel value type. -/
abbrev ValueType.denote : ValueType -> Type
  | .bool => Bool
  | .nat => Nat

/-- A state signature is indexed by the canonical identity of its system. -/
structure Signature (system : Nat) where
  Field : Type
  fieldDecidableEq : DecidableEq Field
  fieldType : Field -> ValueType

instance {system : Nat} (signature : Signature system) :
    DecidableEq signature.Field := signature.fieldDecidableEq

/-- An intrinsically typed total state. A value of the wrong field type cannot
    inhabit this function type. -/
abbrev State {system : Nat} (signature : Signature system) :=
  (field : signature.Field) -> (signature.fieldType field).denote

/-- Pure expressions may inspect only the frozen pre-state. -/
inductive Expr {system : Nat} (signature : Signature system) : ValueType -> Type
  | bool (value : Bool) : Expr signature .bool
  | nat (value : Nat) : Expr signature .nat
  | read (field : signature.Field) : Expr signature (signature.fieldType field)
  | not (value : Expr signature .bool) : Expr signature .bool
  | and (left right : Expr signature .bool) : Expr signature .bool
  | or (left right : Expr signature .bool) : Expr signature .bool
  | add (left right : Expr signature .nat) : Expr signature .nat
  | less (left right : Expr signature .nat) : Expr signature .bool
  | equalBool (left right : Expr signature .bool) : Expr signature .bool
  | equalNat (left right : Expr signature .nat) : Expr signature .bool

/-- Total evaluation of a typed expression against one frozen pre-state. -/
def Expr.eval {system : Nat} {signature : Signature system} {type : ValueType}
    (expression : Expr signature type) (pre : State signature) : type.denote :=
  match expression with
  | .bool value => value
  | .nat value => value
  | .read field => pre field
  | .not value => !(value.eval pre)
  | .and left right => left.eval pre && right.eval pre
  | .or left right => left.eval pre || right.eval pre
  | .add left right => left.eval pre + right.eval pre
  | .less left right => left.eval pre < right.eval pre
  | .equalBool left right => left.eval pre == right.eval pre
  | .equalNat left right => left.eval pre == right.eval pre

namespace State

/-- Replace exactly one dependent field. -/
def write {system : Nat} {signature : Signature system}
    (state : State signature) (target : signature.Field)
    (value : (signature.fieldType target).denote) : State signature :=
  fun field =>
    if same : field = target then
      same.symm ▸ value
    else
      state field

@[simp] theorem write_same {system : Nat} {signature : Signature system}
    (state : State signature) (target : signature.Field)
    (value : (signature.fieldType target).denote) :
    write state target value target = value := by
  simp [write]

@[simp] theorem write_other {system : Nat} {signature : Signature system}
    (state : State signature) (target field : signature.Field)
    (value : (signature.fieldType target).denote) (different : field ≠ target) :
    write state target value field = state field := by
  simp [write, different]

end State

/-- One typed simultaneous-update clause. Its right-hand side has exactly the
    type of its target field. -/
structure Update {system : Nat} {signature : Signature system} where
  field : signature.Field
  value : Expr signature (signature.fieldType field)

/-- Whether a list of update clauses grants write authority to a field. -/
def Writes {system : Nat} {signature : Signature system} :
    List (@Update system signature) -> signature.Field -> Prop
  | [], _ => False
  | update :: rest, field => update.field = field ∨ Writes rest field

/-- Construct the post-state. Every RHS is evaluated against `pre`, even when
    several clauses are present. -/
def applyUpdates {system : Nat} {signature : Signature system}
    (updates : List (@Update system signature)) (pre : State signature) :
    State signature :=
  match updates with
  | [] => pre
  | update :: rest =>
      State.write (applyUpdates rest pre) update.field (update.value.eval pre)

/-- Provider authority is never an ordinary value. The store is a finite-map
    interface represented extensionally: each identity has exactly one live bit. -/
abbrev CapStore (CapId : Type) := CapId -> Bool

namespace CapStore

/-- Consume or explicitly discard one authority identity. -/
def remove {CapId : Type} [DecidableEq CapId]
    (store : CapStore CapId) (target : CapId) : CapStore CapId :=
  fun identity => if identity = target then false else store identity

@[simp] theorem remove_same {CapId : Type} [DecidableEq CapId]
    (store : CapStore CapId) (target : CapId) :
    remove store target target = false := by
  simp [remove]

@[simp] theorem remove_other {CapId : Type} [DecidableEq CapId]
    (store : CapStore CapId) (target identity : CapId)
    (different : identity ≠ target) :
    remove store target identity = store identity := by
  simp [remove, different]

/-- The representation-level multiplicity of an authority identity. -/
def multiplicity {CapId : Type} (store : CapStore CapId) (identity : CapId) : Nat :=
  if store identity then 1 else 0

end CapStore

/-- Explicit affine effects: preserving authority, consuming it at a trusted
    provider operation, or visibly discarding it. -/
inductive CapEffect (CapId : Type)
  | preserve
  | consume (identity : CapId)
  | discard (identity : CapId)
  deriving Repr

def CapEffect.target {CapId : Type} : CapEffect CapId -> Option CapId
  | .preserve => none
  | .consume identity => some identity
  | .discard identity => some identity

/-- A typed action has a Boolean guard, typed simultaneous updates, and one
    auditable affine-authority effect. -/
structure Action {system : Nat} (signature : Signature system) (CapId : Type) where
  guard : Expr signature .bool
  updates : List (@Update system signature)
  capability : CapEffect CapId

structure Config {system : Nat} (signature : Signature system) (CapId : Type) where
  state : State signature
  caps : CapStore CapId

inductive Event (CapId : Type)
  | internal
  | providerConsumed (identity : CapId)
  | capabilityDiscarded (identity : CapId)
  deriving Repr

inductive BlockReason (CapId : Type)
  | falseGuard
  | missingCapability (identity : CapId)
  deriving Repr

/-- The complete reason function for this core. `none` means the action has an
    executable successor. -/
def Action.blockedBy {system : Nat} {signature : Signature system} {CapId : Type}
    (action : Action signature CapId) (config : Config signature CapId) :
    Option (BlockReason CapId) :=
  if action.guard.eval config.state then
    match action.capability with
    | .preserve => none
    | .consume identity | .discard identity =>
        if config.caps identity then none else some (.missingCapability identity)
  else
    some .falseGuard

def Action.nextCaps {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (action : Action signature CapId)
    (config : Config signature CapId) : CapStore CapId :=
  match action.capability with
  | .preserve => config.caps
  | .consume identity | .discard identity => config.caps.remove identity

def Action.event {system : Nat} {signature : Signature system} {CapId : Type}
    (action : Action signature CapId) : Event CapId :=
  match action.capability with
  | .preserve => .internal
  | .consume identity => .providerConsumed identity
  | .discard identity => .capabilityDiscarded identity

structure StepOutcome {system : Nat} (signature : Signature system) (CapId : Type) where
  next : Config signature CapId
  event : Event CapId

/-- The unique successor construction, called only when `blockedBy = none`. -/
def Action.execute {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (action : Action signature CapId)
    (config : Config signature CapId) : StepOutcome signature CapId where
  next := {
    state := applyUpdates action.updates config.state
    caps := action.nextCaps config
  }
  event := action.event

inductive RunResult {system : Nat} (signature : Signature system) (CapId : Type)
  | stepped (outcome : StepOutcome signature CapId)
  | blocked (reason : BlockReason CapId)

/-- Deterministic executable semantics for one action. -/
def Action.run {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (action : Action signature CapId)
    (config : Config signature CapId) : RunResult signature CapId :=
  match action.blockedBy config with
  | none => .stepped (action.execute config)
  | some reason => .blocked reason

/-- Relational presentation of the same executable step. -/
inductive Action.Steps {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (action : Action signature CapId)
    (config : Config signature CapId) : StepOutcome signature CapId -> Prop
  | execute (enabled : action.blockedBy config = none) :
      Steps action config (action.execute config)

/-- Safety propositions carry both the system identity and its exact state
    signature in their type. -/
structure Property (system : Nat) (signature : Signature system) where
  name : String
  predicate : State signature -> Bool

def Property.check {system : Nat} {signature : Signature system} {CapId : Type}
    (property : Property system signature) (config : Config signature CapId) : Bool :=
  property.predicate config.state

/-- Re-indexing a property requires an explicit equality of canonical system
    identities. No unchecked cross-system cast exists in the core. -/
def Property.transport {source target : Nat} (same : source = target)
    {signature : Signature source} (property : Property source signature) :
    Property target (same ▸ signature) := by
  cases same
  exact property

end NMLT

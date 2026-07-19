import NMLT.Metatheory.Soundness

namespace NMLT.Provider

open NMLT

abbrev systemIdentity : Nat := 0

inductive Field
  | authorized
  | dispatched
  deriving DecidableEq, Repr

abbrev signature : Signature systemIdentity where
  Field := Field
  fieldDecidableEq := inferInstance
  fieldType _ := .bool

inductive Capability
  | attempt
  deriving DecidableEq, Repr

def initialState : State signature
  | .authorized => true
  | .dispatched => false

def inputCaps : CapStore Capability
  | .attempt => true

def outputCaps : CapStore Capability
  | .attempt => false

def dispatch : Action signature Capability where
  guard := .read Field.authorized
  updates := [{ field := Field.dispatched, value := .bool true }]
  capability := .consume Capability.attempt

/-- The provider dispatch action has a checked affine context transition. -/
theorem dispatch_typing : ActionTyping inputCaps outputCaps dispatch := by
  constructor
  · rfl
  · change (¬ False) ∧ True
    exact ⟨fun impossible => impossible.elim, True.intro⟩
  · intro identity
    cases identity
    rfl

def initialConfig : Config signature Capability where
  state := initialState
  caps := inputCaps

theorem initial_realizes : initialConfig.Realizes inputCaps := by
  intro identity
  rfl

theorem dispatch_enabled : dispatch.blockedBy initialConfig = none := by
  rfl

theorem dispatch_step : dispatch.Steps initialConfig (dispatch.execute initialConfig) :=
  .execute dispatch_enabled

theorem dispatch_preserves_types :
    StateTyping (dispatch.execute initialConfig).next.state ∧
    (dispatch.execute initialConfig).next.Realizes outputCaps :=
  action_preservation dispatch initialConfig (dispatch.execute initialConfig)
    dispatch_typing initial_realizes dispatch_step

theorem dispatch_consumes_attempt :
    (dispatch.execute initialConfig).next.caps .attempt = false := by
  rfl

theorem dispatch_sets_flag :
    (dispatch.execute initialConfig).next.state .dispatched = true := by
  rfl

def safeDispatch : Property systemIdentity signature where
  name := "dispatch-requires-authority"
  predicate state := !state .dispatched || state .authorized

theorem reference_property_holds : safeDispatch.check initialConfig = true := by
  rfl

/-- A statically absent authority makes the same provider action untypable. -/
def noCaps : CapStore Capability
  | .attempt => false

theorem missing_capability_rejected :
    ¬ ActionTyping noCaps outputCaps dispatch := by
  intro typing
  exact Bool.noConfusion typing.capabilityAvailable

/-- Two simultaneous writes to one target are rejected by action formation. -/
def duplicateDispatch : Action signature Capability where
  guard := .read Field.authorized
  updates := [
    { field := Field.dispatched, value := .bool true },
    { field := Field.dispatched, value := .bool false }
  ]
  capability := .consume Capability.attempt

theorem duplicate_update_rejected :
    ¬ ActionTyping inputCaps outputCaps duplicateDispatch := by
  intro typing
  have distinct := typing.writesDistinct
  change (¬ (Field.dispatched = Field.dispatched ∨ False)) ∧
    ((¬ False) ∧ True) at distinct
  exact distinct.1 (Or.inl rfl)

/-- A second canonical system identity is provably distinct. Its configurations
    cannot be supplied to `safeDispatch.check`, because that application would
    require the impossible equality below. -/
def unrelatedSystemIdentity : Nat := 1

theorem provider_and_unrelated_are_distinct :
    systemIdentity ≠ unrelatedSystemIdentity := by
  decide

#print axioms dispatch_typing
#print axioms dispatch_preserves_types
#print axioms missing_capability_rejected
#print axioms duplicate_update_rejected
#print axioms provider_and_unrelated_are_distinct

end NMLT.Provider

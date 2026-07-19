import NMLT.Core.TypedCore

namespace NMLT

/-- Static capability availability. A target must be present before either a
    trusted operation or an explicit discard can consume it. -/
def CapEffect.WellTyped {CapId : Type}
    (input : CapStore CapId) : CapEffect CapId -> Prop
  | .preserve => True
  | .consume identity => input identity = true
  | .discard identity => input identity = true

/-- The exact output capability context inferred for an action. -/
def CapEffect.output {CapId : Type} [DecidableEq CapId]
    (input : CapStore CapId) : CapEffect CapId -> CapStore CapId
  | .preserve => input
  | .consume identity => input.remove identity
  | .discard identity => input.remove identity

/-- Simultaneous update lists may name each target at most once. This removes
    source-order ambiguity rather than choosing which duplicate wins. -/
def DistinctWrites {system : Nat} {signature : Signature system} :
    List (@Update system signature) -> Prop
  | [] => True
  | update :: rest => ¬ Writes rest update.field ∧ DistinctWrites rest

/-- The principal action judgment. Value/state/update typing is intrinsic;
    this judgment checks affine availability and records the exact context
    transition. -/
structure ActionTyping {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (input output : CapStore CapId)
    (action : Action signature CapId) : Prop where
  capabilityAvailable : action.capability.WellTyped input
  writesDistinct : DistinctWrites action.updates
  outputExact : forall identity,
    output identity = (action.capability.output input) identity

/-- A runtime configuration realizes a static capability context pointwise. -/
def Config.Realizes {system : Nat} {signature : Signature system} {CapId : Type}
    (config : Config signature CapId) (context : CapStore CapId) : Prop :=
  forall identity, config.caps identity = context identity

/-- Intrinsic expression typing is exposed as a judgment for theorem
    statements and future elaboration correspondence. -/
inductive ExprTyping {system : Nat} {signature : Signature system} :
    {type : ValueType} -> Expr signature type -> ValueType -> Prop
  | intro {type : ValueType} (expression : Expr signature type) :
      ExprTyping expression type

/-- State typing is also intrinsic: every field is inhabited by its declared
    Lean interpretation. -/
def StateTyping {system : Nat} {signature : Signature system}
    (state : State signature) : Prop :=
  forall field, Exists fun value : (signature.fieldType field).denote =>
    state field = value

theorem stateTyping_intro {system : Nat} {signature : Signature system}
    (state : State signature) : StateTyping state := by
  intro field
  exact ⟨state field, rfl⟩

/-- A field is in an action's generated frame exactly when it is absent from
    the syntactic write set. -/
def Action.Framed {system : Nat} {signature : Signature system} {CapId : Type}
    (action : Action signature CapId) (field : signature.Field) : Prop :=
  ¬ Writes action.updates field

end NMLT

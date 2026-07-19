import NMLT.Typing.Judgments

namespace NMLT

theorem applyUpdates_frame {system : Nat} {signature : Signature system}
    (updates : List (@Update system signature)) (pre : State signature)
    (field : signature.Field) (outside : ¬ Writes updates field) :
    applyUpdates updates pre field = pre field := by
  induction updates with
  | nil => rfl
  | cons update rest inductionHypothesis =>
      have headDifferent : update.field ≠ field := by
        intro same
        exact outside (Or.inl same)
      have tailOutside : ¬ Writes rest field := by
        intro written
        exact outside (Or.inr written)
      simp only [applyUpdates]
      rw [State.write_other]
      exact inductionHypothesis tailOutside
      exact fun same => headDifferent same.symm

/-- Frame soundness for every executable step: fields outside the generated
    write set are copied from the frozen pre-state. -/
theorem frame_soundness {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (action : Action signature CapId)
    (config : Config signature CapId) (outcome : StepOutcome signature CapId)
    (step : action.Steps config outcome) (field : signature.Field)
    (framed : action.Framed field) :
    outcome.next.state field = config.state field := by
  cases step
  exact applyUpdates_frame action.updates config.state field framed

/-- The operational relation is definitionally deterministic. -/
theorem step_deterministic {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (action : Action signature CapId)
    (config : Config signature CapId) (left right : StepOutcome signature CapId)
    (leftStep : action.Steps config left) (rightStep : action.Steps config right) :
    left = right := by
  cases leftStep
  cases rightStep
  rfl

/-- `run` and the relational step judgment agree exactly on successors. -/
theorem run_stepped_iff_step {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (action : Action signature CapId)
    (config : Config signature CapId) (outcome : StepOutcome signature CapId) :
    action.run config = .stepped outcome ↔ action.Steps config outcome := by
  constructor
  · intro ran
    unfold Action.run at ran
    cases enabled : action.blockedBy config with
    | none =>
        simp [enabled] at ran
        cases ran
        exact .execute enabled
    | some reason =>
        simp [enabled] at ran
  · intro step
    cases step with
    | execute enabled =>
        simp [Action.run, enabled]

/-- Executable blocking agrees exactly with the reason function. -/
theorem run_blocked_iff_reason {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (action : Action signature CapId)
    (config : Config signature CapId) (reason : BlockReason CapId) :
    action.run config = .blocked reason ↔ action.blockedBy config = some reason := by
  constructor
  · intro ran
    unfold Action.run at ran
    cases actual : action.blockedBy config with
    | none => simp [actual] at ran
    | some blocked =>
        simp [actual] at ran
        have same : blocked = reason := ran
        cases same
        rfl
  · intro blocked
    simp [Action.run, blocked]

/-- The reason function is complete: blockage is exactly a false guard or an
    absent targeted capability after a true guard. -/
theorem blocking_reason_exact {system : Nat} {signature : Signature system} {CapId : Type}
    (action : Action signature CapId) (config : Config signature CapId)
    (reason : BlockReason CapId) (blocked : action.blockedBy config = some reason) :
    (reason = .falseGuard ∧ action.guard.eval config.state = false) ∨
    Exists fun identity =>
      reason = .missingCapability identity ∧
      action.guard.eval config.state = true ∧
      action.capability.target = some identity ∧
      config.caps identity = false := by
  cases guardValue : action.guard.eval config.state with
  | false =>
      left
      simp [Action.blockedBy, guardValue] at blocked
      exact ⟨blocked.symm, rfl⟩
  | true =>
      cases effect : action.capability with
      | preserve =>
          simp [Action.blockedBy, guardValue, effect] at blocked
      | consume identity =>
          cases available : config.caps identity with
          | false =>
              right
              simp [Action.blockedBy, guardValue, effect, available] at blocked
              exact ⟨identity, blocked.symm, rfl, rfl, available⟩
          | true =>
              simp [Action.blockedBy, guardValue, effect, available] at blocked
      | discard identity =>
          cases available : config.caps identity with
          | false =>
              right
              simp [Action.blockedBy, guardValue, effect, available] at blocked
              exact ⟨identity, blocked.symm, rfl, rfl, available⟩
          | true =>
              simp [Action.blockedBy, guardValue, effect, available] at blocked

/-- Bidirectional form of the reason classification. -/
theorem blocking_reason_iff {system : Nat} {signature : Signature system} {CapId : Type}
    (action : Action signature CapId) (config : Config signature CapId)
    (reason : BlockReason CapId) :
    action.blockedBy config = some reason ↔
      (reason = .falseGuard ∧ action.guard.eval config.state = false) ∨
      Exists fun identity =>
        reason = .missingCapability identity ∧
        action.guard.eval config.state = true ∧
        action.capability.target = some identity ∧
        config.caps identity = false := by
  constructor
  · exact blocking_reason_exact action config reason
  · intro classified
    rcases classified with falseGuard | missing
    · rcases falseGuard with ⟨reasonIsGuard, guardFalse⟩
      cases reasonIsGuard
      simp [Action.blockedBy, guardFalse]
    · rcases missing with ⟨identity, reasonIsMissing, guardTrue, targets, unavailable⟩
      cases reasonIsMissing
      cases effect : action.capability with
      | preserve =>
          simp [CapEffect.target, effect] at targets
      | consume target =>
          have same : target = identity := by
            simpa [CapEffect.target, effect] using targets
          cases same
          simp [Action.blockedBy, guardTrue, effect, unavailable]
      | discard target =>
          have same : target = identity := by
            simpa [CapEffect.target, effect] using targets
          cases same
          simp [Action.blockedBy, guardTrue, effect, unavailable]

/-- A statically typed action running in a realizing capability store can be
    blocked only by its guard. Missing authority is a static/runtime mismatch,
    not an admitted well-typed outcome. -/
theorem typed_blocked_only_false_guard
    {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] {input output : CapStore CapId}
    (action : Action signature CapId) (config : Config signature CapId)
    (reason : BlockReason CapId) (typing : ActionTyping input output action)
    (realizes : config.Realizes input)
    (blocked : action.blockedBy config = some reason) :
    reason = .falseGuard ∧ action.guard.eval config.state = false := by
  rcases blocking_reason_exact action config reason blocked with guardBlocked | capBlocked
  · exact guardBlocked
  · rcases capBlocked with ⟨identity, _, _, targets, unavailable⟩
    cases effect : action.capability with
    | preserve =>
        simp [CapEffect.target, effect] at targets
    | consume target =>
        have same : target = identity := by
          simpa [CapEffect.target, effect] using targets
        cases same
        have inputLive : input identity = true := by
          simpa [CapEffect.WellTyped, effect] using typing.capabilityAvailable
        have runtimeLive : config.caps identity = true :=
          (realizes identity).trans inputLive
        have impossible : true = false := runtimeLive.symm.trans unavailable
        nomatch impossible
    | discard target =>
        have same : target = identity := by
          simpa [CapEffect.target, effect] using targets
        cases same
        have inputLive : input identity = true := by
          simpa [CapEffect.WellTyped, effect] using typing.capabilityAvailable
        have runtimeLive : config.caps identity = true :=
          (realizes identity).trans inputLive
        have impossible : true = false := runtimeLive.symm.trans unavailable
        nomatch impossible

/-- Exact progress/blocked characterization. Well-formed actions do not get a
    fictitious progress theorem: they either have their unique step or expose
    one concrete blocked reason. -/
theorem progress_or_blocked {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (action : Action signature CapId)
    (config : Config signature CapId) :
    (Exists fun outcome => action.Steps config outcome) ∨
    (Exists fun reason => action.run config = .blocked reason) := by
  cases reason : action.blockedBy config with
  | none =>
      left
      exact ⟨action.execute config, .execute reason⟩
  | some blocked =>
      right
      exact ⟨blocked, (run_blocked_iff_reason action config blocked).2 reason⟩

theorem blocked_iff_no_step {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (action : Action signature CapId)
    (config : Config signature CapId) :
    (Exists fun reason => action.run config = .blocked reason) ↔
    ¬ (Exists fun outcome => action.Steps config outcome) := by
  cases reason : action.blockedBy config with
  | none =>
      constructor
      · intro blocked
        rcases blocked with ⟨actual, runBlocked⟩
        have blockedReason := (run_blocked_iff_reason action config actual).1 runBlocked
        have impossible : (none : Option (BlockReason CapId)) = some actual :=
          reason.symm.trans blockedReason
        nomatch impossible
      · intro noStep
        exfalso
        exact noStep ⟨action.execute config, .execute reason⟩
  | some blocked =>
      constructor
      · intro _ step
        rcases step with ⟨outcome, step⟩
        cases step with
        | execute enabled =>
            have impossible : some blocked = (none : Option (BlockReason CapId)) :=
              reason.symm.trans enabled
            nomatch impossible
      · intro _
        exact ⟨blocked, (run_blocked_iff_reason action config blocked).2 reason⟩

/-- A characteristic-function capability store has multiplicity at most one
    for every identity; contraction is unrepresentable. -/
theorem capability_store_affine {CapId : Type}
    (store : CapStore CapId) (identity : CapId) :
    store.multiplicity identity ≤ 1 := by
  cases live : store identity <;> simp [CapStore.multiplicity, live]

theorem remove_no_fabrication {CapId : Type} [DecidableEq CapId]
    (store : CapStore CapId) (target identity : CapId)
    (liveAfter : store.remove target identity = true) :
    store identity = true := by
  by_cases same : identity = target
  · subst identity
    simp at liveAfter
  · simpa [CapStore.remove, same] using liveAfter

/-- No executable action can duplicate or fabricate provider authority. Every
    live identity after a step was live before it, and its multiplicity remains
    at most one. -/
theorem no_duplication_of_affine_capability
    {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] (action : Action signature CapId)
    (config : Config signature CapId) (outcome : StepOutcome signature CapId)
    (step : action.Steps config outcome) (identity : CapId) :
    outcome.next.caps.multiplicity identity ≤ 1 ∧
    (outcome.next.caps identity = true → config.caps identity = true) := by
  cases step
  constructor
  · exact capability_store_affine _ _
  · cases effect : action.capability with
    | preserve =>
        simp [Action.execute, Action.nextCaps, effect]
    | consume target =>
        intro live
        exact remove_no_fabrication config.caps target identity
          (by simpa [Action.execute, Action.nextCaps, effect] using live)
    | discard target =>
        intro live
        exact remove_no_fabrication config.caps target identity
          (by simpa [Action.execute, Action.nextCaps, effect] using live)

/-- Action preservation for the intrinsic core: state inhabitants keep the
    same system signature, and a typed capability transition realizes its
    exact inferred output context. -/
theorem action_preservation
    {system : Nat} {signature : Signature system} {CapId : Type}
    [DecidableEq CapId] {input output : CapStore CapId}
    (action : Action signature CapId) (config : Config signature CapId)
    (outcome : StepOutcome signature CapId)
    (typing : ActionTyping input output action)
    (realizes : config.Realizes input)
    (step : action.Steps config outcome) :
    StateTyping outcome.next.state ∧ outcome.next.Realizes output := by
  cases step
  constructor
  · exact stateTyping_intro _
  · intro identity
    cases effect : action.capability with
    | preserve =>
        calc
          (action.execute config).next.caps identity = config.caps identity := by
            simp [Action.execute, Action.nextCaps, effect]
          _ = input identity := realizes identity
          _ = output identity := by
            symm
            calc
              output identity = (action.capability.output input) identity :=
                typing.outputExact identity
              _ = input identity := by simp [CapEffect.output, effect]
    | consume target =>
        by_cases same : identity = target
        · subst identity
          have outputFalse : output target = false := by
            calc
              output target = (action.capability.output input) target := typing.outputExact target
              _ = false := by simp [CapEffect.output, effect]
          simp [Action.execute, Action.nextCaps, effect, outputFalse]
        · calc
            (action.execute config).next.caps identity = config.caps identity := by
              simp [Action.execute, Action.nextCaps, effect, CapStore.remove, same]
            _ = input identity := realizes identity
            _ = output identity := by
              rw [typing.outputExact identity]
              simp [CapEffect.output, effect, CapStore.remove, same]
    | discard target =>
        by_cases same : identity = target
        · subst identity
          have outputFalse : output target = false := by
            calc
              output target = (action.capability.output input) target := typing.outputExact target
              _ = false := by simp [CapEffect.output, effect]
          simp [Action.execute, Action.nextCaps, effect, outputFalse]
        · calc
            (action.execute config).next.caps identity = config.caps identity := by
              simp [Action.execute, Action.nextCaps, effect, CapStore.remove, same]
            _ = input identity := realizes identity
            _ = output identity := by
              rw [typing.outputExact identity]
              simp [CapEffect.output, effect, CapStore.remove, same]

/-- Property checking preserves its exact system index by construction. -/
theorem property_system_indexing
    {system : Nat} {signature : Signature system} {CapId : Type}
    (property : Property system signature) (config : Config signature CapId) :
    property.check config = property.predicate config.state := rfl

theorem property_transport_refl
    {system : Nat} {signature : Signature system}
    (property : Property system signature) :
    property.transport rfl = property := rfl

#print axioms applyUpdates_frame
#print axioms frame_soundness
#print axioms progress_or_blocked
#print axioms blocked_iff_no_step
#print axioms blocking_reason_iff
#print axioms typed_blocked_only_false_guard
#print axioms no_duplication_of_affine_capability
#print axioms action_preservation
#print axioms property_system_indexing
#print axioms property_transport_refl

end NMLT

namespace NMLT.Behavior.WorkflowSessionBridge

/-!
Multi-slot bounded job-session lifecycle bridge matching `crates/nmlt-runtime/src/lifecycle.rs`.

Models bounded slots, per-slot monotonic generations, work-budget reservation and
monotonic charging, transfer, dispatch, cancellation, timeout, uncertain recovery,
explicit reconciliation, and terminal collection.

All theorems are proved constructively under Lean's unchanged `propext`/`Quot.sound`
focused allowlist (without `Classical.choice`). This module is a pure transition
specification; differential parity against the Rust runtime is checked separately
by `tools/check_lifecycle_parity.sh`.
-/

structure Limits where
  slots : Nat
  generationsPerSlot : Nat
  maxAttempts : Nat
  maxEvents : Nat
  workBudget : Nat
deriving DecidableEq, Repr

inductive Outcome where
  | completed (tag : Nat)
  | failed (code : Nat)
  | cancelled
deriving DecidableEq, Repr

inductive Phase where
  | reserved
  | running
  | cancelRequested
  | uncertain (cancellationRequested : Bool)
  | finished (outcome : Outcome)
  | collected (outcome : Outcome)
deriving DecidableEq, Repr

structure Control where
  slot : Nat
  generation : Nat
  owner : Nat
  revision : Nat
deriving DecidableEq, Repr

structure Attempt where
  control : Control
  reservedWork : Nat
  phase : Phase
  dispatched : Bool
deriving DecidableEq, Repr

structure SessionState where
  limits : Limits
  attempts : List Attempt
  generations : List Nat
  slots : List (Option Nat)
  eventCount : Nat
deriving DecidableEq, Repr

inductive ResponseOutcome where
  | completed (tag : Nat)
  | failed (code : Nat)
  | cancelled
deriving DecidableEq, Repr

structure Response where
  slot : Nat
  generation : Nat
  outcome : ResponseOutcome
deriving DecidableEq, Repr

inductive Command where
  | reserve (owner : Nat) (reservedWork : Nat)
  | transfer (control : Control) (toOwner : Nat)
  | dispatch (control : Control)
  | cancel (control : Control)
  | timeout (control : Control)
  | deliver (response : Response)
  | reconcile (control : Control) (response : Response)
  | collect (control : Control)
  | recover
deriving DecidableEq, Repr

inductive Receipt where
  | control (c : Control)
  | dispatched (c : Control)
  | ignored
  | collected (slot : Nat) (generation : Nat) (outcome : Outcome)
  | recovered (uncertainAttempts : Nat)
deriving DecidableEq, Repr

def getAt : List α → Nat → Option α
  | [], _ => none
  | x :: _, 0 => some x
  | _ :: xs, n + 1 => getAt xs n

def setAt : List α → Nat → α → List α
  | [], _, _ => []
  | _ :: xs, 0, v => v :: xs
  | x :: xs, n + 1, v => x :: setAt xs n v

def getAtNat (xs : List Nat) (n : Nat) : Nat :=
  match getAt xs n with
  | some v => v
  | none => 0

def replicateList (n : Nat) (v : α) : List α :=
  match n with
  | 0 => []
  | k + 1 => v :: replicateList k v

def SessionState.init (limits : Limits) : SessionState where
  limits := limits
  attempts := []
  generations := replicateList limits.slots 0
  slots := replicateList limits.slots none
  eventCount := 0

def attemptCharged (a : Attempt) : Nat :=
  if a.dispatched then a.reservedWork else 0

def attemptReserved (a : Attempt) : Nat :=
  if a.dispatched then 0 else
    match a.phase with
    | .reserved => a.reservedWork
    | _ => 0

def attemptUsed (a : Attempt) : Nat :=
  attemptCharged a + attemptReserved a

def chargedWork : List Attempt → Nat
  | [] => 0
  | a :: rest => attemptCharged a + chargedWork rest

def reservedWork : List Attempt → Nat
  | [] => 0
  | a :: rest => attemptReserved a + reservedWork rest

def usedWork : List Attempt → Nat
  | [] => 0
  | a :: rest => attemptUsed a + usedWork rest

def availableWork (s : SessionState) : Nat :=
  s.limits.workBudget - usedWork s.attempts

def findFreeSlotAux : List (Option Nat) → List Nat → Nat → Nat → Option Nat
  | [], _, _, _ => none
  | _, [], _, _ => none
  | s :: ss, g :: gs, maxGen, idx =>
      match s with
      | none =>
          if g < maxGen then some idx
          else findFreeSlotAux ss gs maxGen (idx + 1)
      | some _ => findFreeSlotAux ss gs maxGen (idx + 1)

def findFreeSlot (s : SessionState) : Option Nat :=
  findFreeSlotAux s.slots s.generations s.limits.generationsPerSlot 0

def controlled (s : SessionState) (c : Control) : Option (Nat × Attempt) :=
  match getAt s.slots c.slot with
  | some (some idx) =>
      match getAt s.attempts idx with
      | some a => if a.control = c then some (idx, a) else none
      | none => none
  | _ => none

def recoverAttempt (a : Attempt) : Attempt :=
  match a.phase with
  | .collected _ => a
  | .running =>
      { a with
        phase := .uncertain false
        control := { a.control with revision := a.control.revision + 1 } }
  | .cancelRequested =>
      { a with
        phase := .uncertain true
        control := { a.control with revision := a.control.revision + 1 } }
  | _ =>
      { a with
        control := { a.control with revision := a.control.revision + 1 } }

def isUncertain : Phase → Bool
  | .uncertain _ => true
  | _ => false

def countUncertain : List Attempt → Nat
  | [] => 0
  | a :: rest => (if isUncertain a.phase then 1 else 0) + countUncertain rest

def applyReserve (s : SessionState) (owner : Nat) (work : Nat) :
    Except String (SessionState × Receipt) :=
  if work = 0 then
    .error "work reservation must be positive"
  else if s.limits.maxAttempts ≤ s.attempts.length then
    .error "attempt allocation limit exhausted"
  else if s.limits.workBudget < usedWork s.attempts + work then
    .error "work reservation exceeds available budget"
  else
    match findFreeSlot s with
    | none => .error "no free slot with an unused generation"
    | some slot =>
        let nextGen := getAtNat s.generations slot + 1
        let ctrl : Control :=
          { slot := slot, generation := nextGen, owner := owner, revision := 0 }
        let att : Attempt :=
          { control := ctrl, reservedWork := work, phase := .reserved, dispatched := false }
        let next : SessionState :=
          { s with
            attempts := s.attempts ++ [att]
            generations := setAt s.generations slot nextGen
            slots := setAt s.slots slot (some s.attempts.length) }
        .ok (next, .control ctrl)

def applyTransfer (s : SessionState) (c : Control) (toOwner : Nat) :
    Except String (SessionState × Receipt) :=
  match controlled s c with
  | none => .error "stale attempt, owner, or control revision"
  | some (idx, a) =>
      if toOwner = c.owner then
        .error "transfer requires a different owner"
      else
        match a.phase with
        | .reserved =>
            let ctrl' : Control :=
              { a.control with owner := toOwner, revision := a.control.revision + 1 }
            let a' : Attempt := { a with control := ctrl' }
            .ok ({ s with attempts := setAt s.attempts idx a' }, .control ctrl')
        | _ => .error "transfer requires a reserved attempt"

def applyDispatch (s : SessionState) (c : Control) :
    Except String (SessionState × Receipt) :=
  match controlled s c with
  | none => .error "stale attempt, owner, or control revision"
  | some (idx, a) =>
      match a.phase with
      | .reserved =>
          let ctrl' : Control := { a.control with revision := a.control.revision + 1 }
          let a' : Attempt :=
            { a with control := ctrl', phase := .running, dispatched := true }
          .ok ({ s with attempts := setAt s.attempts idx a' }, .dispatched ctrl')
      | _ => .error "dispatch requires unused reserved control"

def applyCancel (s : SessionState) (c : Control) :
    Except String (SessionState × Receipt) :=
  match controlled s c with
  | none => .error "stale attempt, owner, or control revision"
  | some (idx, a) =>
      let nextPhase : Option Phase :=
        match a.phase with
        | .reserved => some (.finished .cancelled)
        | .running => some .cancelRequested
        | .uncertain false => some (.uncertain true)
        | _ => none
      match nextPhase with
      | none => .error "attempt cannot request cancellation in this phase"
      | some phase' =>
          let ctrl' : Control := { a.control with revision := a.control.revision + 1 }
          let a' : Attempt := { a with control := ctrl', phase := phase' }
          .ok ({ s with attempts := setAt s.attempts idx a' }, .control ctrl')

def applyTimeout (s : SessionState) (c : Control) :
    Except String (SessionState × Receipt) :=
  match controlled s c with
  | none => .error "stale attempt, owner, or control revision"
  | some (idx, a) =>
      let cancelledOpt : Option Bool :=
        match a.phase with
        | .running => some false
        | .cancelRequested => some true
        | _ => none
      match cancelledOpt with
      | none => .error "timeout requires in-flight work"
      | some cancelled =>
          let ctrl' : Control := { a.control with revision := a.control.revision + 1 }
          let a' : Attempt :=
            { a with control := ctrl', phase := .uncertain cancelled }
          .ok ({ s with attempts := setAt s.attempts idx a' }, .control ctrl')

def applyResponse (s : SessionState) (resp : Response) (ctrl : Option Control) :
    Except String (SessionState × Receipt) :=
  let checkedCtrl : Except String (Option Nat) :=
    match ctrl with
    | none => .ok none
    | some c =>
        match controlled s c with
        | none => .error "stale attempt, owner, or control revision"
        | some (idx, _) => .ok (some idx)
  match checkedCtrl with
  | .error msg => .error msg
  | .ok ctrlIdx =>
      match getAt s.slots resp.slot with
      | some (some idx) =>
          match ctrlIdx with
          | some expectedIdx =>
              if expectedIdx ≠ idx then
                .error "reconciliation control names another attempt"
              else
                match getAt s.attempts idx with
                | none => .ok (s, .ignored)
                | some a =>
                    if a.dispatched = false ∨ a.control.slot ≠ resp.slot ∨
                        a.control.generation ≠ resp.generation then
                      .ok (s, .ignored)
                    else
                      let cancelledOpt : Option Bool :=
                        match a.phase with
                        | .cancelRequested => some true
                        | .uncertain cr => some cr
                        | _ => none
                      match cancelledOpt with
                      | none => .ok (s, .ignored)
                      | some true =>
                          let ctrl' : Control :=
                            { a.control with revision := a.control.revision + 1 }
                          let a' : Attempt :=
                            { a with control := ctrl', phase := .finished .cancelled }
                          .ok ({ s with attempts := setAt s.attempts idx a' }, .control ctrl')
                      | some false =>
                          match resp.outcome with
                          | .completed tag =>
                              let ctrl' : Control :=
                                { a.control with revision := a.control.revision + 1 }
                              let a' : Attempt :=
                                { a with control := ctrl', phase := .finished (.completed tag) }
                              .ok ({ s with attempts := setAt s.attempts idx a' }, .control ctrl')
                          | .failed code =>
                              let ctrl' : Control :=
                                { a.control with revision := a.control.revision + 1 }
                              let a' : Attempt :=
                                { a with control := ctrl', phase := .finished (.failed code) }
                              .ok ({ s with attempts := setAt s.attempts idx a' }, .control ctrl')
                          | .cancelled =>
                              .error "unsolicited cancellation acknowledgement"
          | none =>
              match getAt s.attempts idx with
              | none => .ok (s, .ignored)
              | some a =>
                  if a.dispatched = false ∨ a.control.slot ≠ resp.slot ∨
                      a.control.generation ≠ resp.generation then
                    .ok (s, .ignored)
                  else
                    let cancelledOpt : Option Bool :=
                      match a.phase with
                      | .running => some false
                      | .cancelRequested => some true
                      | _ => none
                    match cancelledOpt with
                    | none => .ok (s, .ignored)
                    | some true =>
                        match resp.outcome with
                        | .cancelled =>
                            let ctrl' : Control :=
                              { a.control with revision := a.control.revision + 1 }
                            let a' : Attempt :=
                              { a with control := ctrl', phase := .finished .cancelled }
                            .ok ({ s with attempts := setAt s.attempts idx a' }, .control ctrl')
                        | _ => .ok (s, .ignored)
                    | some false =>
                        match resp.outcome with
                        | .completed tag =>
                            let ctrl' : Control :=
                              { a.control with revision := a.control.revision + 1 }
                            let a' : Attempt :=
                              { a with control := ctrl', phase := .finished (.completed tag) }
                            .ok ({ s with attempts := setAt s.attempts idx a' }, .control ctrl')
                        | .failed code =>
                            let ctrl' : Control :=
                              { a.control with revision := a.control.revision + 1 }
                            let a' : Attempt :=
                              { a with control := ctrl', phase := .finished (.failed code) }
                            .ok ({ s with attempts := setAt s.attempts idx a' }, .control ctrl')
                        | .cancelled =>
                            .error "unsolicited cancellation acknowledgement"
      | _ => .ok (s, .ignored)

def applyCollect (s : SessionState) (c : Control) :
    Except String (SessionState × Receipt) :=
  match controlled s c with
  | none => .error "stale attempt, owner, or control revision"
  | some (idx, a) =>
      match a.phase with
      | .finished outcome =>
          let ctrl' : Control := { a.control with revision := a.control.revision + 1 }
          let a' : Attempt := { a with control := ctrl', phase := .collected outcome }
          let next : SessionState :=
            { s with
              attempts := setAt s.attempts idx a'
              slots := setAt s.slots c.slot none }
          .ok (next, .collected c.slot c.generation outcome)
      | _ => .error "collection requires a settled terminal outcome"

def applyRecover (s : SessionState) : Except String (SessionState × Receipt) :=
  let updated := s.attempts.map recoverAttempt
  .ok ({ s with attempts := updated }, .recovered (countUncertain updated))

def applyCommand (s : SessionState) : Command → Except String (SessionState × Receipt)
  | .reserve owner work => applyReserve s owner work
  | .transfer c toOwner => applyTransfer s c toOwner
  | .dispatch c => applyDispatch s c
  | .cancel c => applyCancel s c
  | .timeout c => applyTimeout s c
  | .deliver resp => applyResponse s resp none
  | .reconcile c resp => applyResponse s resp (some c)
  | .collect c => applyCollect s c
  | .recover => applyRecover s

def step (s : SessionState) (cmd : Command) : Except String (SessionState × Receipt) :=
  if s.limits.maxEvents ≤ s.eventCount then
    .error "event limit exhausted"
  else
    match applyCommand s cmd with
    | .error msg => .error msg
    | .ok (next, receipt) => .ok ({ next with eventCount := next.eventCount + 1 }, receipt)

/-- Reachability over finite sequences of accepted lifecycle commands. -/
inductive Reachable (s0 : SessionState) : SessionState → Prop where
  | refl : Reachable s0 s0
  | step {s s' : SessionState} {cmd : Command} {r : Receipt} :
      Reachable s0 s →
      step s cmd = .ok (s', r) →
      Reachable s0 s'

-- Helper arithmetic and list-preservation lemmas.

theorem usedWork_eq_charged_add_reserved :
    ∀ xs : List Attempt, usedWork xs = chargedWork xs + reservedWork xs
  | [] => rfl
  | a :: rest => by
      have ih := usedWork_eq_charged_add_reserved rest
      simp [usedWork, chargedWork, reservedWork, attemptUsed]
      omega

theorem usedWork_append (xs ys : List Attempt) :
    usedWork (xs ++ ys) = usedWork xs + usedWork ys := by
  induction xs with
  | nil => simp [usedWork]
  | cons a rest ih => simp [usedWork, ih]; omega

theorem chargedWork_append (xs ys : List Attempt) :
    chargedWork (xs ++ ys) = chargedWork xs + chargedWork ys := by
  induction xs with
  | nil => simp [chargedWork]
  | cons a rest ih => simp [chargedWork, ih]; omega

theorem usedWork_setAt_le :
    ∀ (xs : List Attempt) (idx : Nat) (a' : Attempt),
      (∀ a, getAt xs idx = some a → attemptUsed a' ≤ attemptUsed a) →
      usedWork (setAt xs idx a') ≤ usedWork xs
  | [], _, _, _ => by simp [setAt, usedWork]
  | a :: rest, 0, a', h => by
      have ha : attemptUsed a' ≤ attemptUsed a := h a rfl
      simp [setAt, usedWork]
      omega
  | a :: rest, idx + 1, a', h => by
      have ih := usedWork_setAt_le rest idx a' (fun x hx => h x hx)
      simp [setAt, usedWork]
      omega

theorem chargedWork_setAt_ge :
    ∀ (xs : List Attempt) (idx : Nat) (a' : Attempt),
      (∀ a, getAt xs idx = some a → attemptCharged a ≤ attemptCharged a') →
      chargedWork xs ≤ chargedWork (setAt xs idx a')
  | [], _, _, _ => by simp [setAt, chargedWork]
  | a :: rest, 0, a', h => by
      have ha : attemptCharged a ≤ attemptCharged a' := h a rfl
      simp [setAt, chargedWork]
      omega
  | a :: rest, idx + 1, a', h => by
      have ih := chargedWork_setAt_ge rest idx a' (fun x hx => h x hx)
      simp [setAt, chargedWork]
      omega

theorem usedWork_map_le (f : Attempt → Attempt)
    (hf : ∀ a, attemptUsed (f a) ≤ attemptUsed a) :
    ∀ xs : List Attempt, usedWork (xs.map f) ≤ usedWork xs
  | [] => by simp [usedWork]
  | a :: rest => by
      have h1 := hf a
      have h2 := usedWork_map_le f hf rest
      simp [usedWork]
      omega

theorem chargedWork_map_ge (f : Attempt → Attempt)
    (hf : ∀ a, attemptCharged a ≤ attemptCharged (f a)) :
    ∀ xs : List Attempt, chargedWork xs ≤ chargedWork (xs.map f)
  | [] => by simp [chargedWork]
  | a :: rest => by
      have h1 := hf a
      have h2 := chargedWork_map_ge f hf rest
      simp [chargedWork]
      omega

theorem getAtNat_setAt_ge :
    ∀ (xs : List Nat) (idx k v : Nat),
      getAtNat xs idx ≤ v →
      getAtNat xs k ≤ getAtNat (setAt xs idx v) k
  | [], _, _, _, _ => by simp [setAt, getAtNat, getAt]
  | y :: ys, 0, 0, v, hv => by
      simp [setAt, getAtNat, getAt] at hv ⊢
      exact hv
  | y :: ys, 0, k + 1, v, _ => by
      simp [setAt, getAtNat, getAt]
  | y :: ys, i + 1, 0, v, _ => by
      simp [setAt, getAtNat, getAt]
  | y :: ys, i + 1, k + 1, v, hv => by
      simp [setAt, getAtNat, getAt] at hv ⊢
      exact getAtNat_setAt_ge ys i k v hv

theorem controlled_getAt {s : SessionState} {c : Control} {idx : Nat} {a : Attempt}
    (h : controlled s c = some (idx, a)) :
    getAt s.attempts idx = some a ∧ a.control = c := by
  unfold controlled at h
  cases hslot : getAt s.slots c.slot with
  | none => simp [hslot] at h
  | some opt =>
      cases opt with
      | none => simp [hslot] at h
      | some i =>
          cases hatt : getAt s.attempts i with
          | none => simp [hslot, hatt] at h
          | some att =>
              by_cases hctrl : att.control = c
              · simp [hslot, hatt, hctrl] at h
                rcases h with ⟨rfl, rfl⟩
                exact ⟨hatt, hctrl⟩
              · simp [hslot, hatt, hctrl] at h

theorem recoverAttempt_used_eq (a : Attempt) :
    attemptUsed (recoverAttempt a) = attemptUsed a := by
  cases hp : a.phase <;> cases hd : a.dispatched <;>
    simp [recoverAttempt, attemptUsed, attemptCharged, attemptReserved, hp, hd]

theorem recoverAttempt_charged_eq (a : Attempt) :
    attemptCharged (recoverAttempt a) = attemptCharged a := by
  cases hp : a.phase <;> cases hd : a.dispatched <;>
    simp [recoverAttempt, attemptCharged, hp, hd]

-- Per-command invariants.

private theorem applyReserve_invariants {s s' : SessionState} {owner work : Nat} {r : Receipt}
    (h : applyReserve s owner work = .ok (s', r)) :
    s'.limits = s.limits ∧
    usedWork s'.attempts ≤ s.limits.workBudget ∧
    chargedWork s.attempts ≤ chargedWork s'.attempts ∧
    (∀ k, getAtNat s.generations k ≤ getAtNat s'.generations k) := by
  unfold applyReserve at h
  by_cases h0 : work = 0
  · simp [h0] at h
  · simp [h0] at h
    by_cases hmax : s.limits.maxAttempts ≤ s.attempts.length
    · simp [hmax] at h
    · simp [hmax] at h
      by_cases hbud : s.limits.workBudget < usedWork s.attempts + work
      · simp [hbud] at h
      · simp [hbud] at h
        cases hslot : findFreeSlot s with
        | none => simp [hslot] at h
        | some slot =>
            simp [hslot] at h
            rcases h with ⟨rfl, rfl⟩
            refine ⟨rfl, ?_, ?_, ?_⟩
            · rw [usedWork_append]
              simp [usedWork, attemptUsed, attemptCharged, attemptReserved]
              omega
            · rw [chargedWork_append]
              simp [chargedWork, attemptCharged]
            · intro k
              exact getAtNat_setAt_ge s.generations slot k (getAtNat s.generations slot + 1)
                (Nat.le_succ _)

private theorem applyTransfer_invariants {s s' : SessionState} {c : Control} {toOwner : Nat}
    {r : Receipt} (h : applyTransfer s c toOwner = .ok (s', r)) :
    s'.limits = s.limits ∧
    usedWork s'.attempts ≤ usedWork s.attempts ∧
    chargedWork s.attempts ≤ chargedWork s'.attempts ∧
    s'.generations = s.generations := by
  unfold applyTransfer at h
  cases hc : controlled s c with
  | none => simp [hc] at h
  | some p =>
      rcases p with ⟨idx, a⟩
      have ⟨hatt, _⟩ := controlled_getAt hc
      simp [hc] at h
      by_cases hsame : toOwner = c.owner
      · simp [hsame] at h
      · simp [hsame] at h
        cases hp : a.phase <;> simp [hp] at h
        rcases h with ⟨rfl, rfl⟩
        refine ⟨rfl, ?_, ?_, rfl⟩
        · apply usedWork_setAt_le
          intro x hx
          have : x = a := Option.some.inj (hx.symm.trans hatt)
          subst this
          simp [attemptUsed, attemptCharged, attemptReserved]
        · apply chargedWork_setAt_ge
          intro x hx
          have : x = a := Option.some.inj (hx.symm.trans hatt)
          subst this
          simp [attemptCharged]

private theorem applyDispatch_invariants {s s' : SessionState} {c : Control} {r : Receipt}
    (h : applyDispatch s c = .ok (s', r)) :
    s'.limits = s.limits ∧
    usedWork s'.attempts ≤ usedWork s.attempts ∧
    chargedWork s.attempts ≤ chargedWork s'.attempts ∧
    s'.generations = s.generations := by
  unfold applyDispatch at h
  cases hc : controlled s c with
  | none => simp [hc] at h
  | some p =>
      rcases p with ⟨idx, a⟩
      have ⟨hatt, _⟩ := controlled_getAt hc
      simp [hc] at h
      cases hp : a.phase <;> simp [hp] at h
      rcases h with ⟨rfl, rfl⟩
      refine ⟨rfl, ?_, ?_, rfl⟩
      · apply usedWork_setAt_le
        intro x hx
        have : x = a := Option.some.inj (hx.symm.trans hatt)
        subst this
        cases hd : a.dispatched <;>
          simp [attemptUsed, attemptCharged, attemptReserved, hp, hd]
      · apply chargedWork_setAt_ge
        intro x hx
        have : x = a := Option.some.inj (hx.symm.trans hatt)
        subst this
        cases hd : a.dispatched <;> simp [attemptCharged, hd]

private theorem applyCancel_invariants {s s' : SessionState} {c : Control} {r : Receipt}
    (h : applyCancel s c = .ok (s', r)) :
    s'.limits = s.limits ∧
    usedWork s'.attempts ≤ usedWork s.attempts ∧
    chargedWork s.attempts ≤ chargedWork s'.attempts ∧
    s'.generations = s.generations := by
  unfold applyCancel at h
  cases hc : controlled s c with
  | none => simp [hc] at h
  | some p =>
      rcases p with ⟨idx, a⟩
      have ⟨hatt, _⟩ := controlled_getAt hc
      simp [hc] at h
      cases hp : a.phase with
      | reserved =>
          simp [hp] at h
          rcases h with ⟨rfl, rfl⟩
          refine ⟨rfl, ?_, ?_, rfl⟩
          · apply usedWork_setAt_le
            intro x hx
            have : x = a := Option.some.inj (hx.symm.trans hatt)
            subst this
            cases hd : a.dispatched <;>
              simp [attemptUsed, attemptCharged, attemptReserved, hp, hd]
          · apply chargedWork_setAt_ge
            intro x hx
            have : x = a := Option.some.inj (hx.symm.trans hatt)
            subst this
            simp [attemptCharged]
      | running =>
          simp [hp] at h
          rcases h with ⟨rfl, rfl⟩
          refine ⟨rfl, ?_, ?_, rfl⟩
          · apply usedWork_setAt_le
            intro x hx
            have : x = a := Option.some.inj (hx.symm.trans hatt)
            subst this
            cases hd : a.dispatched <;>
              simp [attemptUsed, attemptCharged, attemptReserved, hp, hd]
          · apply chargedWork_setAt_ge
            intro x hx
            have : x = a := Option.some.inj (hx.symm.trans hatt)
            subst this
            simp [attemptCharged]
      | cancelRequested => simp [hp] at h
      | uncertain cr =>
          cases cr with
          | true => simp [hp] at h
          | false =>
              simp [hp] at h
              rcases h with ⟨rfl, rfl⟩
              refine ⟨rfl, ?_, ?_, rfl⟩
              · apply usedWork_setAt_le
                intro x hx
                have : x = a := Option.some.inj (hx.symm.trans hatt)
                subst this
                cases hd : a.dispatched <;>
                  simp [attemptUsed, attemptCharged, attemptReserved, hp, hd]
              · apply chargedWork_setAt_ge
                intro x hx
                have : x = a := Option.some.inj (hx.symm.trans hatt)
                subst this
                simp [attemptCharged]
      | finished _ => simp [hp] at h
      | collected _ => simp [hp] at h

private theorem applyTimeout_invariants {s s' : SessionState} {c : Control} {r : Receipt}
    (h : applyTimeout s c = .ok (s', r)) :
    s'.limits = s.limits ∧
    usedWork s'.attempts ≤ usedWork s.attempts ∧
    chargedWork s.attempts ≤ chargedWork s'.attempts ∧
    s'.generations = s.generations := by
  unfold applyTimeout at h
  cases hc : controlled s c with
  | none => simp [hc] at h
  | some p =>
      rcases p with ⟨idx, a⟩
      have ⟨hatt, _⟩ := controlled_getAt hc
      simp [hc] at h
      cases hp : a.phase with
      | running =>
          simp [hp] at h
          rcases h with ⟨rfl, rfl⟩
          refine ⟨rfl, ?_, ?_, rfl⟩
          · apply usedWork_setAt_le
            intro x hx
            have : x = a := Option.some.inj (hx.symm.trans hatt)
            subst this
            cases hd : a.dispatched <;>
              simp [attemptUsed, attemptCharged, attemptReserved, hp, hd]
          · apply chargedWork_setAt_ge
            intro x hx
            have : x = a := Option.some.inj (hx.symm.trans hatt)
            subst this
            simp [attemptCharged]
      | cancelRequested =>
          simp [hp] at h
          rcases h with ⟨rfl, rfl⟩
          refine ⟨rfl, ?_, ?_, rfl⟩
          · apply usedWork_setAt_le
            intro x hx
            have : x = a := Option.some.inj (hx.symm.trans hatt)
            subst this
            cases hd : a.dispatched <;>
              simp [attemptUsed, attemptCharged, attemptReserved, hp, hd]
          · apply chargedWork_setAt_ge
            intro x hx
            have : x = a := Option.some.inj (hx.symm.trans hatt)
            subst this
            simp [attemptCharged]
      | reserved => simp [hp] at h
      | uncertain _ => simp [hp] at h
      | finished _ => simp [hp] at h
      | collected _ => simp [hp] at h

private theorem finish_attempt_invariants {s : SessionState} {idx : Nat} {a : Attempt}
    (hatt : getAt s.attempts idx = some a) (hdisp : a.dispatched = true) (out : Outcome) :
    let a' : Attempt :=
      { a with
        control := { a.control with revision := a.control.revision + 1 }
        phase := .finished out }
    usedWork (setAt s.attempts idx a') ≤ usedWork s.attempts ∧
    chargedWork s.attempts ≤ chargedWork (setAt s.attempts idx a') := by
  dsimp only
  constructor
  · apply usedWork_setAt_le
    intro x hx
    have : x = a := Option.some.inj (hx.symm.trans hatt)
    subst this
    simp [attemptUsed, attemptCharged, attemptReserved, hdisp]
  · apply chargedWork_setAt_ge
    intro x hx
    have : x = a := Option.some.inj (hx.symm.trans hatt)
    subst this
    simp [attemptCharged, hdisp]

private theorem applyResponse_invariants {s s' : SessionState} {resp : Response}
    {ctrl : Option Control} {r : Receipt} (h : applyResponse s resp ctrl = .ok (s', r)) :
    s'.limits = s.limits ∧
    usedWork s'.attempts ≤ usedWork s.attempts ∧
    chargedWork s.attempts ≤ chargedWork s'.attempts ∧
    s'.generations = s.generations := by
  unfold applyResponse at h
  cases ctrl with
  | none =>
      simp only [] at h
      cases hslot : getAt s.slots resp.slot with
      | none =>
          simp [hslot] at h
          rcases h with ⟨rfl, rfl⟩
          exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
      | some opt =>
          cases opt with
          | none =>
              simp [hslot] at h
              rcases h with ⟨rfl, rfl⟩
              exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
          | some idx =>
              cases hatt : getAt s.attempts idx with
              | none =>
                  simp [hslot, hatt] at h
                  rcases h with ⟨rfl, rfl⟩
                  exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
              | some a =>
                  simp [hslot, hatt] at h
                  by_cases hguard : a.dispatched = false ∨ a.control.slot ≠ resp.slot ∨
                      a.control.generation ≠ resp.generation
                  · simp [hguard] at h
                    rcases h with ⟨rfl, rfl⟩
                    exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
                  · have hdisp : a.dispatched = true := by
                      cases hd : a.dispatched
                      · exfalso; exact hguard (Or.inl hd)
                      · rfl
                    simp [hguard] at h
                    cases hp : a.phase with
                    | running =>
                        simp [hp] at h
                        cases hout : resp.outcome with
                        | completed tag =>
                            simp [hout] at h
                            rcases h with ⟨rfl, rfl⟩
                            have ⟨hu, hc⟩ := finish_attempt_invariants hatt hdisp (.completed tag)
                            exact ⟨rfl, hu, hc, rfl⟩
                        | failed code =>
                            simp [hout] at h
                            rcases h with ⟨rfl, rfl⟩
                            have ⟨hu, hc⟩ := finish_attempt_invariants hatt hdisp (.failed code)
                            exact ⟨rfl, hu, hc, rfl⟩
                        | cancelled => simp [hout] at h
                    | cancelRequested =>
                        simp [hp] at h
                        cases hout : resp.outcome with
                        | completed _ =>
                            simp [hout] at h
                            rcases h with ⟨rfl, rfl⟩
                            exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
                        | failed _ =>
                            simp [hout] at h
                            rcases h with ⟨rfl, rfl⟩
                            exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
                        | cancelled =>
                            simp [hout] at h
                            rcases h with ⟨rfl, rfl⟩
                            have ⟨hu, hc⟩ := finish_attempt_invariants hatt hdisp .cancelled
                            exact ⟨rfl, hu, hc, rfl⟩
                    | reserved =>
                        simp [hp] at h
                        rcases h with ⟨rfl, rfl⟩
                        exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
                    | uncertain _ =>
                        simp [hp] at h
                        rcases h with ⟨rfl, rfl⟩
                        exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
                    | finished _ =>
                        simp [hp] at h
                        rcases h with ⟨rfl, rfl⟩
                        exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
                    | collected _ =>
                        simp [hp] at h
                        rcases h with ⟨rfl, rfl⟩
                        exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
  | some c =>
      cases hc : controlled s c with
      | none => simp [hc] at h
      | some p =>
          rcases p with ⟨cidx, ca⟩
          simp [hc] at h
          cases hslot : getAt s.slots resp.slot with
          | none =>
              simp [hslot] at h
              rcases h with ⟨rfl, rfl⟩
              exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
          | some opt =>
              cases opt with
              | none =>
                  simp [hslot] at h
                  rcases h with ⟨rfl, rfl⟩
                  exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
              | some idx =>
                  simp [hslot] at h
                  by_cases hne : cidx ≠ idx
                  · simp [hne] at h
                  · simp [hne] at h
                    cases hatt : getAt s.attempts idx with
                    | none =>
                        simp [hatt] at h
                        rcases h with ⟨rfl, rfl⟩
                        exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
                    | some a =>
                        simp [hatt] at h
                        by_cases hguard : a.dispatched = false ∨ a.control.slot ≠ resp.slot ∨
                            a.control.generation ≠ resp.generation
                        · simp [hguard] at h
                          rcases h with ⟨rfl, rfl⟩
                          exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
                        · have hdisp : a.dispatched = true := by
                            cases hd : a.dispatched
                            · exfalso; exact hguard (Or.inl hd)
                            · rfl
                          simp [hguard] at h
                          cases hp : a.phase with
                          | cancelRequested =>
                              simp [hp] at h
                              rcases h with ⟨rfl, rfl⟩
                              have ⟨hu, hc'⟩ := finish_attempt_invariants hatt hdisp .cancelled
                              exact ⟨rfl, hu, hc', rfl⟩
                          | uncertain cr =>
                              cases cr with
                              | true =>
                                  simp [hp] at h
                                  rcases h with ⟨rfl, rfl⟩
                                  have ⟨hu, hc'⟩ := finish_attempt_invariants hatt hdisp .cancelled
                                  exact ⟨rfl, hu, hc', rfl⟩
                              | false =>
                                  simp [hp] at h
                                  cases hout : resp.outcome with
                                  | completed tag =>
                                      simp [hout] at h
                                      rcases h with ⟨rfl, rfl⟩
                                      have ⟨hu, hc'⟩ :=
                                        finish_attempt_invariants hatt hdisp (.completed tag)
                                      exact ⟨rfl, hu, hc', rfl⟩
                                  | failed code =>
                                      simp [hout] at h
                                      rcases h with ⟨rfl, rfl⟩
                                      have ⟨hu, hc'⟩ :=
                                        finish_attempt_invariants hatt hdisp (.failed code)
                                      exact ⟨rfl, hu, hc', rfl⟩
                                  | cancelled => simp [hout] at h
                          | reserved =>
                              simp [hp] at h
                              rcases h with ⟨rfl, rfl⟩
                              exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
                          | running =>
                              simp [hp] at h
                              rcases h with ⟨rfl, rfl⟩
                              exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
                          | finished _ =>
                              simp [hp] at h
                              rcases h with ⟨rfl, rfl⟩
                              exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩
                          | collected _ =>
                              simp [hp] at h
                              rcases h with ⟨rfl, rfl⟩
                              exact ⟨rfl, Nat.le_refl _, Nat.le_refl _, rfl⟩

private theorem applyCollect_invariants {s s' : SessionState} {c : Control} {r : Receipt}
    (h : applyCollect s c = .ok (s', r)) :
    s'.limits = s.limits ∧
    usedWork s'.attempts ≤ usedWork s.attempts ∧
    chargedWork s.attempts ≤ chargedWork s'.attempts ∧
    s'.generations = s.generations := by
  unfold applyCollect at h
  cases hc : controlled s c with
  | none => simp [hc] at h
  | some p =>
      rcases p with ⟨idx, a⟩
      have ⟨hatt, _⟩ := controlled_getAt hc
      simp [hc] at h
      cases hp : a.phase <;> simp [hp] at h
      rcases h with ⟨rfl, rfl⟩
      refine ⟨rfl, ?_, ?_, rfl⟩
      · apply usedWork_setAt_le
        intro x hx
        have : x = a := Option.some.inj (hx.symm.trans hatt)
        subst this
        cases hd : a.dispatched <;>
          simp [attemptUsed, attemptCharged, attemptReserved, hp, hd]
      · apply chargedWork_setAt_ge
        intro x hx
        have : x = a := Option.some.inj (hx.symm.trans hatt)
        subst this
        simp [attemptCharged]

private theorem applyRecover_invariants {s s' : SessionState} {r : Receipt}
    (h : applyRecover s = .ok (s', r)) :
    s'.limits = s.limits ∧
    usedWork s'.attempts ≤ usedWork s.attempts ∧
    chargedWork s.attempts ≤ chargedWork s'.attempts ∧
    s'.generations = s.generations := by
  unfold applyRecover at h
  simp at h
  rcases h with ⟨rfl, rfl⟩
  refine ⟨rfl, ?_, ?_, rfl⟩
  · exact usedWork_map_le recoverAttempt
      (fun a => Nat.le_of_eq (recoverAttempt_used_eq a)) s.attempts
  · exact chargedWork_map_ge recoverAttempt
      (fun a => Nat.le_of_eq (recoverAttempt_charged_eq a).symm) s.attempts

private theorem step_unwrap {s s' : SessionState} {cmd : Command} {r : Receipt}
    (h : step s cmd = .ok (s', r)) :
    ∃ mid : SessionState,
      applyCommand s cmd = .ok (mid, r) ∧
      s' = { mid with eventCount := mid.eventCount + 1 } := by
  unfold step at h
  by_cases hlim : s.limits.maxEvents ≤ s.eventCount
  · simp [hlim] at h
  · simp [hlim] at h
    cases happly : applyCommand s cmd with
    | error msg => simp [happly] at h
    | ok pair =>
        rcases pair with ⟨mid, r'⟩
        simp [happly] at h
        rcases h with ⟨rfl, rfl⟩
        exact ⟨mid, happly, rfl⟩

-- Headline safety theorems audited under the focused axiom allowlist.

/-- Every accepted lifecycle step preserves the work-budget ceiling and limits. -/
theorem step_preserves_budget {s s' : SessionState} {cmd : Command} {r : Receipt}
    (hbud : usedWork s.attempts ≤ s.limits.workBudget)
    (hstep : step s cmd = .ok (s', r)) :
    s'.limits = s.limits ∧ usedWork s'.attempts ≤ s'.limits.workBudget := by
  rcases step_unwrap hstep with ⟨mid, happly, rfl⟩
  cases cmd with
  | reserve owner work =>
      have ⟨hl, hu, _, _⟩ := applyReserve_invariants happly
      exact ⟨hl, by rw [hl]; exact hu⟩
  | transfer c toOwner =>
      have ⟨hl, hu, _, _⟩ := applyTransfer_invariants happly
      exact ⟨hl, by rw [hl]; exact Nat.le_trans hu hbud⟩
  | dispatch c =>
      have ⟨hl, hu, _, _⟩ := applyDispatch_invariants happly
      exact ⟨hl, by rw [hl]; exact Nat.le_trans hu hbud⟩
  | cancel c =>
      have ⟨hl, hu, _, _⟩ := applyCancel_invariants happly
      exact ⟨hl, by rw [hl]; exact Nat.le_trans hu hbud⟩
  | timeout c =>
      have ⟨hl, hu, _, _⟩ := applyTimeout_invariants happly
      exact ⟨hl, by rw [hl]; exact Nat.le_trans hu hbud⟩
  | deliver resp =>
      have ⟨hl, hu, _, _⟩ := applyResponse_invariants happly
      exact ⟨hl, by rw [hl]; exact Nat.le_trans hu hbud⟩
  | reconcile c resp =>
      have ⟨hl, hu, _, _⟩ := applyResponse_invariants happly
      exact ⟨hl, by rw [hl]; exact Nat.le_trans hu hbud⟩
  | collect c =>
      have ⟨hl, hu, _, _⟩ := applyCollect_invariants happly
      exact ⟨hl, by rw [hl]; exact Nat.le_trans hu hbud⟩
  | recover =>
      have ⟨hl, hu, _, _⟩ := applyRecover_invariants happly
      exact ⟨hl, by rw [hl]; exact Nat.le_trans hu hbud⟩

/-- Across every reachable session state from `SessionState.init limits`, total
charged plus reserved work never exceeds `limits.workBudget`. -/
theorem reachable_budget_non_exceeding {limits : Limits} {s : SessionState}
    (hreach : Reachable (SessionState.init limits) s) :
    s.limits = limits ∧ chargedWork s.attempts + reservedWork s.attempts ≤ limits.workBudget := by
  have hmain : s.limits = limits ∧ usedWork s.attempts ≤ s.limits.workBudget := by
    induction hreach with
    | refl =>
        exact ⟨rfl, by simp [SessionState.init, usedWork]⟩
    | @step a b cmd r _ hstep ih =>
        have ⟨hlim, hbud⟩ := step_preserves_budget ih.2 hstep
        exact ⟨hlim.trans ih.1, hbud⟩
  refine ⟨hmain.1, ?_⟩
  rw [← usedWork_eq_charged_add_reserved, ← hmain.1]
  exact hmain.2

/-- Dispatched work is monotonically charged: no lifecycle transition (including
cancel, timeout, recover, reconcile, or collect) can ever reduce `chargedWork`. -/
theorem step_preserves_charged_monotonic {s s' : SessionState} {cmd : Command} {r : Receipt}
    (hstep : step s cmd = .ok (s', r)) :
    chargedWork s.attempts ≤ chargedWork s'.attempts := by
  rcases step_unwrap hstep with ⟨mid, happly, rfl⟩
  cases cmd with
  | reserve owner work => exact (applyReserve_invariants happly).2.2.1
  | transfer c toOwner => exact (applyTransfer_invariants happly).2.2.1
  | dispatch c => exact (applyDispatch_invariants happly).2.2.1
  | cancel c => exact (applyCancel_invariants happly).2.2.1
  | timeout c => exact (applyTimeout_invariants happly).2.2.1
  | deliver resp => exact (applyResponse_invariants happly).2.2.1
  | reconcile c resp => exact (applyResponse_invariants happly).2.2.1
  | collect c => exact (applyCollect_invariants happly).2.2.1
  | recover => exact (applyRecover_invariants happly).2.2.1

/-- Per-slot generation counters never decrease across any accepted lifecycle step. -/
theorem step_preserves_generations_monotonic {s s' : SessionState} {cmd : Command} {r : Receipt}
    (hstep : step s cmd = .ok (s', r)) (k : Nat) :
    getAtNat s.generations k ≤ getAtNat s'.generations k := by
  rcases step_unwrap hstep with ⟨mid, happly, rfl⟩
  cases cmd with
  | reserve owner work => exact (applyReserve_invariants happly).2.2.2 k
  | transfer c toOwner =>
      have hg := (applyTransfer_invariants happly).2.2.2
      simp [hg]
  | dispatch c =>
      have hg := (applyDispatch_invariants happly).2.2.2
      simp [hg]
  | cancel c =>
      have hg := (applyCancel_invariants happly).2.2.2
      simp [hg]
  | timeout c =>
      have hg := (applyTimeout_invariants happly).2.2.2
      simp [hg]
  | deliver resp =>
      have hg := (applyResponse_invariants happly).2.2.2
      simp [hg]
  | reconcile c resp =>
      have hg := (applyResponse_invariants happly).2.2.2
      simp [hg]
  | collect c =>
      have hg := (applyCollect_invariants happly).2.2.2
      simp [hg]
  | recover =>
      have hg := (applyRecover_invariants happly).2.2.2
      simp [hg]

/-- Reserving a slot strictly advances that slot's generation counter by 1. -/
theorem reserve_increments_generation {s s' : SessionState} {owner work : Nat} {c : Control}
    (hstep : step s (.reserve owner work) = .ok (s', .control c)) :
    c.generation = getAtNat s.generations c.slot + 1 := by
  rcases step_unwrap hstep with ⟨mid, happly, _⟩
  unfold applyCommand applyReserve at happly
  by_cases h0 : work = 0
  · simp [h0] at happly
  · simp [h0] at happly
    by_cases hmax : s.limits.maxAttempts ≤ s.attempts.length
    · simp [hmax] at happly
    · simp [hmax] at happly
      by_cases hbud : s.limits.workBudget < usedWork s.attempts + work
      · simp [hbud] at happly
      · simp [hbud] at happly
        cases hslot : findFreeSlot s with
        | none => simp [hslot] at happly
        | some slot =>
            simp [hslot] at happly
            rcases happly with ⟨_, hctrl⟩
            cases hctrl
            rfl

/-- Delivering a response with a stale generation leaves attempts, generations,
and slot occupancy completely unchanged and returns `Receipt.ignored`. -/
theorem stale_generation_delivery_rejected {s s' : SessionState} {resp : Response} {r : Receipt}
    {idx : Nat} {a : Attempt}
    (hslot : getAt s.slots resp.slot = some (some idx))
    (hatt : getAt s.attempts idx = some a)
    (hne : a.control.generation ≠ resp.generation)
    (hstep : step s (.deliver resp) = .ok (s', r)) :
    r = .ignored ∧ s'.attempts = s.attempts ∧ s'.slots = s.slots := by
  rcases step_unwrap hstep with ⟨mid, happly, rfl⟩
  unfold applyCommand applyResponse at happly
  simp [hslot, hatt, hne] at happly
  rcases happly with ⟨rfl, rfl⟩
  exact ⟨rfl, rfl, rfl⟩

/-- Recovery transitions in-flight (`running` / `cancelRequested`) attempts into
`uncertain` while preserving `dispatched` and `reservedWork`. -/
theorem recover_marks_inflight_uncertain (a : Attempt) :
    (a.phase = .running →
      (recoverAttempt a).phase = .uncertain false ∧
      (recoverAttempt a).dispatched = a.dispatched ∧
      (recoverAttempt a).reservedWork = a.reservedWork) ∧
    (a.phase = .cancelRequested →
      (recoverAttempt a).phase = .uncertain true ∧
      (recoverAttempt a).dispatched = a.dispatched ∧
      (recoverAttempt a).reservedWork = a.reservedWork) := by
  constructor <;> intro hp <;> simp [recoverAttempt, hp]

/-- Collecting an attempt in `Phase.uncertain` is always rejected until explicit
reconciliation transitions it to `Phase.finished`. -/
theorem collect_rejects_uncertain {s : SessionState} {c : Control} {idx : Nat} {a : Attempt}
    {cr : Bool}
    (hctrl : controlled s c = some (idx, a))
    (hunc : a.phase = .uncertain cr) :
    ∀ s' r, step s (.collect c) ≠ .ok (s', r) := by
  intro s' r hstep
  rcases step_unwrap hstep with ⟨mid, happly, _⟩
  unfold applyCommand applyCollect at happly
  simp [hctrl, hunc] at happly

/-- Double collection fails closed: once a slot is vacant (`none`), collecting
any control naming that slot is always rejected. -/
theorem double_collect_rejected {s : SessionState} {c : Control}
    (hvacant : getAt s.slots c.slot = some none) :
    ∀ s' r, step s (.collect c) ≠ .ok (s', r) := by
  intro s' r hstep
  rcases step_unwrap hstep with ⟨mid, happly, _⟩
  unfold applyCommand applyCollect controlled at happly
  simp [hvacant] at happly

end NMLT.Behavior.WorkflowSessionBridge

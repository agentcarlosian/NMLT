import NMLT.Behavior.WorkflowSessionBridge

open NMLT.Behavior.WorkflowSessionBridge

private def phaseTag : Phase → String
  | .reserved => "reserved"
  | .running => "running"
  | .cancelRequested => "cancel_requested"
  | .uncertain false => "uncertain(false)"
  | .uncertain true => "uncertain(true)"
  | .finished (.completed t) => s!"finished(ok:{t})"
  | .finished (.failed c) => s!"finished(err:{c})"
  | .finished .cancelled => "finished(cancelled)"
  | .collected (.completed t) => s!"collected(ok:{t})"
  | .collected (.failed c) => s!"collected(err:{c})"
  | .collected .cancelled => "collected(cancelled)"

private def receiptTag : Receipt → String
  | .control c => s!"control({c.slot},{c.generation},{c.owner},{c.revision})"
  | .dispatched c => s!"dispatch({c.slot},{c.generation},{c.owner},{c.revision})"
  | .ignored => "ignored"
  | .collected s g (.completed t) => s!"collected({s},{g},ok:{t})"
  | .collected s g (.failed code) => s!"collected({s},{g},err:{code})"
  | .collected s g .cancelled => s!"collected({s},{g},cancelled)"
  | .recovered u => s!"recovered({u})"

private def slotsSummary (s : SessionState) : String :=
  let entries := s.slots.map fun opt =>
    match opt with
    | none => "_"
    | some idx =>
        match getAt s.attempts idx with
        | none => "?"
        | some a => s!"g{a.control.generation}:{phaseTag a.phase}"
  String.intercalate "," entries

private def runStep (idx : Nat) (s : SessionState) (cmd : Command) : IO SessionState := do
  match step s cmd with
  | .ok (next, receipt) =>
      IO.println s!"STEP|{idx}|OK|{receiptTag receipt}|{chargedWork next.attempts}|{reservedWork next.attempts}|{availableWork next}|{slotsSummary next}"
      pure next
  | .error _ =>
      IO.println s!"STEP|{idx}|ERR|-|{chargedWork s.attempts}|{reservedWork s.attempts}|{availableWork s}|{slotsSummary s}"
      pure s

def main : IO UInt32 := do
  let limits : Limits :=
    { slots := 2, generationsPerSlot := 2, maxAttempts := 4, maxEvents := 32, workBudget := 5 }
  let mut s := SessionState.init limits

  -- 1. Reserve slot 0 (owner 1, work 2) -> gen 1, rev 0
  s ← runStep 1 s (.reserve 1 2)
  let c0_r0 : Control := { slot := 0, generation := 1, owner := 1, revision := 0 }

  -- 2. Transfer slot 0 from owner 1 to owner 2 -> rev 1
  s ← runStep 2 s (.transfer c0_r0 2)
  let c0_r1 : Control := { slot := 0, generation := 1, owner := 2, revision := 1 }

  -- 3. Dispatch slot 0 -> running, rev 2, charged=2, reserved=0
  s ← runStep 3 s (.dispatch c0_r1)
  let c0_r2 : Control := { slot := 0, generation := 1, owner := 2, revision := 2 }

  -- 4. Reserve slot 1 (owner 1, work 2) -> gen 1, rev 0, avail=1
  s ← runStep 4 s (.reserve 1 2)
  let c1_r0 : Control := { slot := 1, generation := 1, owner := 1, revision := 0 }

  -- 5. Reserve exceeding budget (work 2 > avail 1) -> ERR
  s ← runStep 5 s (.reserve 1 2)

  -- 6. Deliver stale generation (slot 0, gen 2) -> IGNORED
  s ← runStep 6 s (.deliver { slot := 0, generation := 2, outcome := .completed 9 })

  -- 7. Dispatch slot 1 -> running, rev 1, charged=4
  s ← runStep 7 s (.dispatch c1_r0)
  let c1_r1 : Control := { slot := 1, generation := 1, owner := 1, revision := 1 }

  -- 8. Cancel slot 1 -> cancel_requested, rev 2
  s ← runStep 8 s (.cancel c1_r1)

  -- 9. Timeout slot 0 -> uncertain(false), rev 3
  s ← runStep 9 s (.timeout c0_r2)
  let c0_r3 : Control := { slot := 0, generation := 1, owner := 2, revision := 3 }

  -- 10. Collect slot 0 while uncertain -> ERR
  s ← runStep 10 s (.collect c0_r3)

  -- 11. Reconcile slot 0 with completed(42) -> finished(ok:42), rev 4
  s ← runStep 11 s (.reconcile c0_r3 { slot := 0, generation := 1, outcome := .completed 42 })
  let c0_r4 : Control := { slot := 0, generation := 1, owner := 2, revision := 4 }

  -- 12. Collect slot 0 -> collected, slot 0 freed
  s ← runStep 12 s (.collect c0_r4)

  -- 13. Double collect slot 0 -> ERR
  s ← runStep 13 s (.collect c0_r4)

  -- 14. Recover -> marks slot 1 (cancel_requested) uncertain(true), bumps rev to 3
  s ← runStep 14 s .recover
  let c1_r3 : Control := { slot := 1, generation := 1, owner := 1, revision := 3 }

  -- 15. Reconcile slot 1 with completed(7) (cancellation requested -> settles as cancelled) -> rev 4
  s ← runStep 15 s (.reconcile c1_r3 { slot := 1, generation := 1, outcome := .completed 7 })
  let c1_r4 : Control := { slot := 1, generation := 1, owner := 1, revision := 4 }

  -- 16. Collect slot 1 -> collected(cancelled), slot 1 freed
  s ← runStep 16 s (.collect c1_r4)

  -- 17. Reserve slot 0 again (work 1) -> reuses slot 0 at generation 2!
  s ← runStep 17 s (.reserve 1 1)
  let c0_g2_r0 : Control := { slot := 0, generation := 2, owner := 1, revision := 0 }

  -- 18. Cancel undispatched reserved slot 0 -> releases reserved work back to avail=1
  s ← runStep 18 s (.cancel c0_g2_r0)
  let c0_g2_r1 : Control := { slot := 0, generation := 2, owner := 1, revision := 1 }

  -- 19. Collect cancelled slot 0 -> slot 0 freed
  let _ ← runStep 19 s (.collect c0_g2_r1)
  pure 0

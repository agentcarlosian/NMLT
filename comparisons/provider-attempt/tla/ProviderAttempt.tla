------------------------- MODULE ProviderAttempt -------------------------
EXTENDS Integers, TLC

Phases == {"proposed", "authorized", "dispatched", "selected",
           "rejected", "indeterminate"}

VARIABLES phase, bound, armed, dispatchCount, responseIntact,
          evaluationPassed

vars == <<phase, bound, armed, dispatchCount, responseIntact,
          evaluationPassed>>

Init ==
  /\ phase = "proposed"
  /\ bound = FALSE
  /\ armed = FALSE
  /\ dispatchCount = 0
  /\ responseIntact = FALSE
  /\ evaluationPassed = FALSE

Bind ==
  /\ phase = "proposed"
  /\ phase' = "authorized"
  /\ bound' = TRUE
  /\ UNCHANGED <<armed, dispatchCount, responseIntact, evaluationPassed>>

Arm ==
  /\ phase = "authorized"
  /\ bound
  /\ armed' = TRUE
  /\ UNCHANGED <<phase, bound, dispatchCount, responseIntact,
                 evaluationPassed>>

Dispatch ==
  /\ phase = "authorized"
  /\ bound /\ armed
  /\ dispatchCount = 0
  /\ phase' = "dispatched"
  /\ dispatchCount' = 1
  /\ UNCHANGED <<bound, armed, responseIntact, evaluationPassed>>

ConfirmPass ==
  /\ phase = "dispatched"
  /\ phase' = "selected"
  /\ responseIntact' = TRUE
  /\ evaluationPassed' = TRUE
  /\ UNCHANGED <<bound, armed, dispatchCount>>

ConfirmFail ==
  /\ phase = "dispatched"
  /\ phase' = "rejected"
  /\ responseIntact' = TRUE
  /\ evaluationPassed' = FALSE
  /\ UNCHANGED <<bound, armed, dispatchCount>>

LoseResponse ==
  /\ phase = "dispatched"
  /\ phase' = "indeterminate"
  /\ UNCHANGED <<bound, armed, dispatchCount, responseIntact,
                 evaluationPassed>>

Next == Bind \/ Arm \/ Dispatch \/ ConfirmPass \/ ConfirmFail \/ LoseResponse

Spec == Init /\ [][Next]_vars

TypeOK ==
  /\ phase \in Phases
  /\ bound \in BOOLEAN
  /\ armed \in BOOLEAN
  /\ dispatchCount \in 0..1
  /\ responseIntact \in BOOLEAN
  /\ evaluationPassed \in BOOLEAN

DispatchRequiresArm == dispatchCount > 0 => bound /\ armed
SelectionRequiresPassingEvidence ==
  phase = "selected" => responseIntact /\ evaluationPassed
AtMostOneDispatch == dispatchCount <= 1
NoBlindReplay == phase = "indeterminate" => ~ENABLED Dispatch

=============================================================================

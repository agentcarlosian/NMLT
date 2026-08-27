import NMLT.Counterexamples.CompositionCongruence
import NMLT.Core.FiniteObservationTrace

namespace NMLT.Counterexamples.HiddenDivergence

open NMLT
open NMLT.Counterexamples.CompositionCongruence

/-!
  First C1 I-FAIR *negative* slice.

  RFC 0008: I-FAIR is liveness; hidden divergence must be excluded or
  discharged separately. RFC 0007 R-DIVERGENCE: no fair concrete execution
  may take infinitely many hidden steps with only finitely many matching
  abstract steps, unless that is separately justified. WeakRefines (T1)
  does not discharge this. Finite observation-trace inclusion (T5
  corollary) is stutter-expansion of *finite* traces only.

  ConcreteSender can take hidden `ping` forever and stay at the unique
  abstract state. That is permitted by `hiddenStep` (one-step state
  equality). T5 / `weakRefines_finite_observation_trace_inclusion` still
  hold on every finite prefix, so they do not exclude the infinite hidden
  path.

  Stream-free: an infinite path is `Nat → state`. No Mathlib, no LTL,
  no infinite observation words as a theory, no fairness transport
  through compose. Not a liveness theorem.
-/

/-- Stream-free infinite hidden path. Not a coinductive word theory and
    not Mathlib `Stream`. -/
structure InfiniteHiddenPath {ConcreteLabel Obs : Type}
    (concrete : LTS ConcreteLabel Obs) (hidden : ConcreteLabel → Bool) where
  state : Nat → concrete.State
  label : Nat → ConcreteLabel
  steps : ∀ n, concrete.step (state n) (label n) (state (n + 1))
  isHidden : ∀ n, hidden (label n) = true

/-- `WeakRefines.hiddenStep` iterates: every state on an infinite hidden
    path maps to the same abstract state. This is why observational
    refinement permits hidden divergence. -/
theorem infiniteHiddenPath_stays
    {ConcreteLabel AbstractLabel Obs : Type}
    {concrete : LTS ConcreteLabel Obs} {abstract : LTS AbstractLabel Obs}
    {hidden : ConcreteLabel → Bool} {mapLabel : ConcreteLabel → AbstractLabel}
    (R : WeakRefines concrete abstract hidden mapLabel)
    (p : InfiniteHiddenPath concrete hidden) :
    ∀ n, R.mapState (p.state n) = R.mapState (p.state 0) := by
  intro n
  induction n with
  | zero => rfl
  | succ n ih =>
    have hμ := R.hiddenStep (p.steps n) (p.isHidden n)
    exact hμ.symm.trans ih

/-- Constant path on the unique concrete sender state. -/
def hiddenPingPath : Nat → concreteSender.State :=
  fun _ => ()

def hiddenPingInfinite : InfiniteHiddenPath concreteSender senderHidden where
  state := hiddenPingPath
  label := fun _ => SenderLabel.ping
  steps := fun _ => rfl
  isHidden := fun _ => rfl

theorem hiddenPing_still_senderRefinement :
    Nonempty (WeakRefines concreteSender abstractSender senderHidden id) :=
  ⟨senderRefinement⟩

theorem hiddenPingPath_steps :
    ∀ n, concreteSender.step (hiddenPingPath n) SenderLabel.ping
      (hiddenPingPath (n + 1)) :=
  hiddenPingInfinite.steps

theorem hiddenPingPath_stays :
    ∀ n, senderRefinement.mapState (hiddenPingPath n) =
      senderRefinement.mapState (hiddenPingPath 0) :=
  infiniteHiddenPath_stays senderRefinement hiddenPingInfinite

/-- Every finite prefix of the infinite hidden-ping path is a legal
    observation trace: `n` pings produce `n+1` copies of `false`. -/
theorem hiddenPing_producesObs :
    ∀ n, ProducesObs concreteSender (hiddenPingPath 0)
      (List.replicate (n + 1) false) := by
  intro n
  induction n with
  | zero =>
    exact ProducesObs.refl (lts := concreteSender) (hiddenPingPath 0)
  | succ n ih =>
    have hstep :
        concreteSender.step (hiddenPingPath 0) SenderLabel.ping
          (hiddenPingPath 0) :=
      rfl
    exact ProducesObs.cons (lts := concreteSender)
      (hiddenPingPath 0) (hiddenPingPath 0) SenderLabel.ping
      (List.replicate (n + 1) false) hstep ih

/-- Finite observation-trace inclusion (T5 corollary) still holds on every
    finite prefix of the diverging path. That is why T5 does not discharge
    I-FAIR / R-DIVERGENCE. -/
theorem hiddenPing_finite_prefixes_included :
    ∀ n, ∃ aobs,
      ProducesObs abstractSender
        (senderRefinement.mapState (hiddenPingPath 0)) aobs ∧
      StutterExpands aobs (List.replicate (n + 1) false) :=
  fun n =>
    weakRefines_finite_observation_trace_inclusion senderRefinement
      (hiddenPing_producesObs n)

/-- Observational `WeakRefines` (T1) permits hidden divergence.
    ConcreteSender takes hidden `ping` at every `Nat` index and stays at
    the unique abstract state. Finite observation-trace inclusion still
    accepts every finite prefix. Not WF/SF transport, not LTL, not an
    I-FAIR lift through synchronization. Case 7 compose resource
    homomorphism remains open. -/
theorem hiddenPing_divergence_not_discharged_by_weakRefines :
    Nonempty (WeakRefines concreteSender abstractSender senderHidden id) ∧
      (∀ n, concreteSender.step (hiddenPingPath n) SenderLabel.ping
          (hiddenPingPath (n + 1))) ∧
      (∀ n, senderRefinement.mapState (hiddenPingPath n) =
          senderRefinement.mapState (hiddenPingPath 0)) ∧
      (∀ n, ∃ aobs,
        ProducesObs abstractSender
          (senderRefinement.mapState (hiddenPingPath 0)) aobs ∧
        StutterExpands aobs (List.replicate (n + 1) false)) :=
  ⟨hiddenPing_still_senderRefinement, hiddenPingPath_steps, hiddenPingPath_stays,
    hiddenPing_finite_prefixes_included⟩

#print axioms hiddenPing_still_senderRefinement
#print axioms infiniteHiddenPath_stays
#print axioms hiddenPingPath_stays
#print axioms hiddenPing_producesObs
#print axioms hiddenPing_finite_prefixes_included
#print axioms hiddenPing_divergence_not_discharged_by_weakRefines

end NMLT.Counterexamples.HiddenDivergence

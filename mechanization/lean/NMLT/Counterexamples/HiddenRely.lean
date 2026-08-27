import NMLT.Counterexamples.CompositionCongruence
import NMLT.Behavior.OpenResourceCongruence

namespace NMLT.Counterexamples.HiddenRely

open NMLT
open NMLT.Counterexamples.CompositionCongruence
open NMLT.Behavior.OpenResourceCongruence
open NMLT.Grades

/-- Nominal environmental fact assumed by the hidden ping in this C1 slice. -/
inductive EnvFact
  | ready
  deriving DecidableEq

/-- Concrete hidden ping relies on `ready`. Capability, consume, transfer,
    and grade fields stay inert so the failure isolates I-RELY. -/
def pingRelyAction : ActionResources Empty EnvFact where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun f => f = .ready
  guarantees := fun _ => False

/-- Abstract stutter profile on the same ping *name*: does not rely on
    `ready`. `ResourceRefinement.rely` forbids widening assumptions. -/
def pingNoRelyAction : ActionResources Empty EnvFact where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

/-- Resource decoration of `concreteSender`. LTS steps are unchanged. -/
def concretePingRely : SystemResources SenderLabel Empty EnvFact where
  owned := fun _ => False
  action := fun
    | .ping => pingRelyAction

/-- Resource decoration of `abstractSender`. The abstract LTS still has no
    ping *step*; `SenderLabel` is a singleton, so every `mapAction` lands on
    `.ping`, whose profile does not rely on `ready`. -/
def abstractNoRely : SystemResources SenderLabel Empty EnvFact where
  owned := fun _ => False
  action := fun
    | .ping => pingNoRelyAction

/-- Observational `WeakRefines` (T1 / Paper 1 small model) does not imply
    rely/assumption preservation (I-RELY). Hidden ping still
    `senderRefinement`s the step-free abstract sender. Decorating that ping
    with a `ready` rely unmatched on the abstract stutter profile makes
    `ResourceRefinement` fail on the `rely` clause.

    `ResourceRefinement` is action-indexed independently of `WeakRefines`;
    this file does not extend `LTS.step` with facts. Hidden ping has no
    abstract *step*, so `WeakRefines` uses `hiddenStep` (state equality)
    rather than `mapLabel`. Resource comparison still needs a total
    `mapAction`. Inventing a dummy name is unnecessary: `SenderLabel` has
    only `.ping`, so the map `fun _ => .ping` (equal to `id`) compares ping
    to ping, and that image does not rely on `ready`.

    I-RELY here is that rely clause (refinement must not widen assumptions).
    Not a positive I-RELY lift, not I-CAP / I-GRADE / I-FAIR, not a
    rely discharge through compose, and T4 does not imply this. -/

theorem hiddenPing_still_senderRefinement :
    Nonempty (WeakRefines concreteSender abstractSender senderHidden id) :=
  ⟨senderRefinement⟩

def mapPing : SenderLabel → SenderLabel :=
  fun _ => SenderLabel.ping

theorem concrete_ping_relies_on_ready :
    (concretePingRely.action SenderLabel.ping).rely EnvFact.ready :=
  rfl

theorem abstract_ping_does_not_rely_ready :
    ¬ (abstractNoRely.action SenderLabel.ping).rely EnvFact.ready :=
  fun h => h

/-- T1 holds on the LTS, and the extra `ready` rely decoration still
    falsifies `ResourceRefinement` along `mapPing` (the only `SenderLabel`
    map). -/
theorem hiddenPing_rely_breaks_resourceRefinement :
    Nonempty (WeakRefines concreteSender abstractSender senderHidden id) ∧
      ¬ ResourceRefinement concretePingRely abstractNoRely mapPing :=
  ⟨hiddenPing_still_senderRefinement, fun refinement =>
    abstract_ping_does_not_rely_ready
      ((refinement.rely SenderLabel.ping EnvFact.ready)
        concrete_ping_relies_on_ready)⟩

#print axioms hiddenPing_still_senderRefinement
#print axioms hiddenPing_rely_breaks_resourceRefinement

end NMLT.Counterexamples.HiddenRely

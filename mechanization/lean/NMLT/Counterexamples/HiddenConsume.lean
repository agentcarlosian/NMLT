import NMLT.Counterexamples.CompositionCongruence
import NMLT.Behavior.OpenResourceCongruence

namespace NMLT.Counterexamples.HiddenConsume

open NMLT
open NMLT.Counterexamples.CompositionCongruence
open NMLT.Behavior.OpenResourceCongruence
open NMLT.Grades

/-- Nominal capability consumed by the hidden ping in this C1 slice. -/
inductive TokenCap
  | token
  deriving DecidableEq

/-- Concrete hidden ping consumes `token` and requires it. Other resource
    fields stay inert so the failure isolates I-CAP consume. -/
def pingConsumeAction : ActionResources TokenCap Empty where
  requires := fun c => c = .token
  consumes := fun c => c = .token
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

/-- Abstract stutter profile on the same ping *name*: requires `token` so
    `ResourceRefinement.requires` is not the breaking clause, but does not
    consume it. -/
def pingNoConsumeAction : ActionResources TokenCap Empty where
  requires := fun c => c = .token
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

/-- Resource decoration of `concreteSender`. LTS steps are unchanged. -/
def concretePingConsume : SystemResources SenderLabel TokenCap Empty where
  owned := fun c => c = .token
  action := fun
    | .ping => pingConsumeAction

/-- Resource decoration of `abstractSender`. The abstract LTS still has no
    ping *step*; `SenderLabel` is a singleton, so every `mapAction` lands on
    `.ping`, whose profile does not consume `token`. -/
def abstractNoConsume : SystemResources SenderLabel TokenCap Empty where
  owned := fun c => c = .token
  action := fun
    | .ping => pingNoConsumeAction

/-- Observational `WeakRefines` (T1 / Paper 1 small model) does not imply
    resource/capability preservation (I-CAP). Hidden ping still
    `senderRefinement`s the step-free abstract sender. Decorating that ping
    with a `token` consume unmatched on the abstract profile makes
    `ResourceRefinement` fail.

    `ResourceRefinement` is action-indexed independently of `WeakRefines`;
    this file does not extend `LTS.step` with capabilities. Hidden ping has
    no abstract *step*, so `WeakRefines` uses `hiddenStep` (state equality)
    rather than `mapLabel`. Resource comparison still needs a total
    `mapAction`. Inventing a dummy name is unnecessary: `SenderLabel` has
    only `.ping`, so the map `fun _ => .ping` (equal to `id`) compares ping
    to ping, and that image does not consume `token`.

    I-CAP here is that consume clause (hidden steps may not use unmatched
    authority). Not a positive I-CAP lift, not I-GRADE / I-FAIR, and T4
    does not imply this. -/

theorem hiddenPing_still_senderRefinement :
    Nonempty (WeakRefines concreteSender abstractSender senderHidden id) :=
  ⟨senderRefinement⟩

def mapPing : SenderLabel → SenderLabel :=
  fun _ => SenderLabel.ping

theorem concrete_ping_consumes_token :
    (concretePingConsume.action SenderLabel.ping).consumes TokenCap.token :=
  rfl

theorem abstract_ping_does_not_consume_token :
    ¬ (abstractNoConsume.action SenderLabel.ping).consumes TokenCap.token :=
  fun h => h

/-- T1 holds on the LTS, and the `token` consume decoration still falsifies
    `ResourceRefinement` along `mapPing` (the only `SenderLabel` map). -/
theorem hiddenPing_consume_breaks_resourceRefinement :
    Nonempty (WeakRefines concreteSender abstractSender senderHidden id) ∧
      ¬ ResourceRefinement concretePingConsume abstractNoConsume mapPing :=
  ⟨hiddenPing_still_senderRefinement, fun refinement =>
    abstract_ping_does_not_consume_token
      ((refinement.consumes SenderLabel.ping TokenCap.token).mp
        concrete_ping_consumes_token)⟩

#print axioms hiddenPing_still_senderRefinement
#print axioms hiddenPing_consume_breaks_resourceRefinement

end NMLT.Counterexamples.HiddenConsume

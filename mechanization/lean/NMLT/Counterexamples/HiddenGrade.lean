import NMLT.Counterexamples.CompositionCongruence
import NMLT.Behavior.OpenResourceCongruence

namespace NMLT.Counterexamples.HiddenGrade

open NMLT
open NMLT.Counterexamples.CompositionCongruence
open NMLT.Behavior.OpenResourceCongruence
open NMLT.Grades

/-- Positive cost atom on the hidden ping. Other grade coordinates stay zero
    so the failure isolates I-GRADE. `zero` is the epsilon / stutter grade. -/
def pingCost : Grade where
  cost := 1
  privacy := 0
  energy := 0
  uncertainty := zeroPpm

/-- Concrete hidden ping carries `pingCost`. Capability, consume, transfer,
    and rely/guarantee fields stay inert so the failure isolates I-GRADE. -/
def pingCostAction : ActionResources Empty Empty where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := pingCost
  rely := fun _ => False
  guarantees := fun _ => False

/-- Abstract stutter profile on the same ping *name*: epsilon (`zero`) grade.
    `ResourceRefinement.grade` requires `Le concrete.grade abstract.grade`. -/
def pingEpsilonAction : ActionResources Empty Empty where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

/-- Resource decoration of `concreteSender`. LTS steps are unchanged. -/
def concretePingCost : SystemResources SenderLabel Empty Empty where
  owned := fun _ => False
  action := fun
    | .ping => pingCostAction

/-- Resource decoration of `abstractSender`. The abstract LTS still has no
    ping *step*; `SenderLabel` is a singleton, so every `mapAction` lands on
    `.ping`, whose profile is epsilon. -/
def abstractEpsilon : SystemResources SenderLabel Empty Empty where
  owned := fun _ => False
  action := fun
    | .ping => pingEpsilonAction

/-- Observational `WeakRefines` (T1 / Paper 1 small model) does not imply
    grade preservation (I-GRADE). Hidden ping still `senderRefinement`s the
    step-free abstract sender. Decorating that ping with a positive cost
    unmatched by the abstract epsilon profile makes `ResourceRefinement`
    fail on the `grade` clause.

    `ResourceRefinement` is action-indexed independently of `WeakRefines`;
    this file does not extend `LTS.step` with grades. Hidden ping has no
    abstract *step*, so `WeakRefines` uses `hiddenStep` (state equality)
    rather than `mapLabel`. Resource comparison still needs a total
    `mapAction`. Inventing a dummy name is unnecessary: `SenderLabel` has
    only `.ping`, so the map `fun _ => .ping` (equal to `id`) compares ping
    to ping, and that image is epsilon.

    I-GRADE here is that grade clause (hidden steps may not carry a worse
    bound than the mapped stutter). Not a positive I-GRADE lift, not
    I-CAP / I-FAIR / I-RELY, not a grade homomorphism through compose,
    and T4 does not imply this. -/

theorem hiddenPing_still_senderRefinement :
    Nonempty (WeakRefines concreteSender abstractSender senderHidden id) :=
  ⟨senderRefinement⟩

def mapPing : SenderLabel → SenderLabel :=
  fun _ => SenderLabel.ping

theorem concrete_ping_grade_is_cost :
    (concretePingCost.action SenderLabel.ping).grade = pingCost :=
  rfl

theorem abstract_ping_grade_is_epsilon :
    (abstractEpsilon.action SenderLabel.ping).grade = zero :=
  rfl

theorem pingCost_not_le_zero : ¬ Le pingCost zero := by
  decide

/-- T1 holds on the LTS, and the positive cost decoration still falsifies
    `ResourceRefinement` along `mapPing` (the only `SenderLabel` map). -/
theorem hiddenPing_grade_breaks_resourceRefinement :
    Nonempty (WeakRefines concreteSender abstractSender senderHidden id) ∧
      ¬ ResourceRefinement concretePingCost abstractEpsilon mapPing :=
  ⟨hiddenPing_still_senderRefinement, fun refinement =>
    pingCost_not_le_zero (refinement.grade SenderLabel.ping)⟩

#print axioms hiddenPing_still_senderRefinement
#print axioms hiddenPing_grade_breaks_resourceRefinement

end NMLT.Counterexamples.HiddenGrade

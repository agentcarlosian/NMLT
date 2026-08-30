import NMLT.Counterexamples.CompositionCongruence
import NMLT.Behavior.OpenResourceCongruence

namespace NMLT.Counterexamples.HiddenInert

open NMLT
open NMLT.Counterexamples.CompositionCongruence
open NMLT.Behavior.OpenResourceCongruence
open NMLT.Grades

/-- Shared stutter profile: no consume, epsilon grade, no rely. Matching this
    on both sides is the positive dual of the three C1 negatives. -/
def pingInertAction : ActionResources Empty Empty where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := zero
  rely := fun _ => False
  guarantees := fun _ => False

/-- Resource decoration of `concreteSender`. LTS steps are unchanged. -/
def concretePingInert : SystemResources SenderLabel Empty Empty where
  owned := fun _ => False
  action := fun _ => pingInertAction

/-- Resource decoration of `abstractSender`. The abstract LTS still has no
    ping *step*; `SenderLabel` is a singleton, so every `mapAction` lands on
    `.ping`, whose profile is the same stutter. -/
def abstractInert : SystemResources SenderLabel Empty Empty where
  owned := fun _ => False
  action := fun _ => pingInertAction

/-- Observational `WeakRefines` (T1 / Paper 1 small model) is compatible with
    resource preservation when the hidden ping does not add consume, cost, or
    rely. Hidden ping still `senderRefinement`s the step-free abstract sender.
    Decorating that ping with the same inert stutter profile as the abstract
    side witnesses `ResourceRefinement` along `mapPing`.

    `ResourceRefinement` is action-indexed independently of `WeakRefines`;
    this file does not extend `LTS.step` with capabilities. Hidden ping has
    no abstract *step*, so `WeakRefines` uses `hiddenStep` (state equality)
    rather than `mapLabel`. Resource comparison still needs a total
    `mapAction`. Inventing a dummy name is unnecessary: `SenderLabel` has
    only `.ping`, so the map `fun _ => .ping` (equal to `id`) compares ping
    to ping, and that image matches.

    This is the matching-profile dual of the three C1 negatives
    (`HiddenConsume`, `HiddenGrade`, `HiddenRely`). Those fails are
    independent of this case. Not a product/compose resource homomorphism
    (RFC 0008 Case 7 stays open), not I-FAIR, and T4 does not imply this. -/

theorem hiddenPing_still_senderRefinement :
    Nonempty (WeakRefines concreteSender abstractSender senderHidden id) :=
  ⟨senderRefinement⟩

def mapPing : SenderLabel → SenderLabel :=
  fun _ => SenderLabel.ping

/-- Direct witness: every `ResourceRefinement` clause holds because both
    profiles are the inert stutter. Proofs stay primitive (`id` / `Nat.le_refl`)
    so this can stay axiom-free. -/
theorem inertPingRefinement :
    ResourceRefinement concretePingInert abstractInert mapPing := by
  refine ⟨?owned, ?requires, ?consumes, ?transfers, ?receives, ?grade, ?rely, ?guarantees⟩
  · intro _ h
    exact h
  · intro _ _ h
    exact h
  · intro _ _
    exact Iff.intro id id
  · intro _ _
    exact Iff.intro id id
  · intro _ _
    exact Iff.intro id id
  · intro _
    exact ⟨Nat.le_refl 0, Nat.le_refl 0, Nat.le_refl 0, Nat.le_refl 0⟩
  · intro _ _ h
    exact h
  · intro _ _ h
    exact h

/-- T1 holds on the LTS, and matching inert profiles (no extra consume,
    epsilon grade, no extra rely) witness `ResourceRefinement` along
    `mapPing` (the only `SenderLabel` map). -/
theorem hiddenPing_inert_resourceRefinement :
    Nonempty (WeakRefines concreteSender abstractSender senderHidden id) ∧
      ResourceRefinement concretePingInert abstractInert mapPing :=
  ⟨hiddenPing_still_senderRefinement, inertPingRefinement⟩

#print axioms hiddenPing_still_senderRefinement
#print axioms inertPingRefinement
#print axioms hiddenPing_inert_resourceRefinement

end NMLT.Counterexamples.HiddenInert

/-
  Finite observation-trace inclusion for `WeakRefines`.

  Paper 1 Corollary (cor:obs-trace): a one-step refinement mapping with
  stuttering lifts to finite observation-trace inclusion. This file is
  that corollary, not LTL, infinite traces, fairness, or liveness.

  Precise statement: if `R : WeakRefines C A h m` and `C` produces a
  finite observation list `obs` from `s`, then `A` produces some finite
  observation list `aobs` from `R.mapState s` such that `obs` is a
  stutter-expansion of `aobs` (hidden concrete steps repeat the current
  abstract observation; visible steps take exactly one abstract step).
-/
import NMLT.Core.Transition

namespace NMLT

/-- A finite observation trace of `lts` starting at `s`. The list is
    never empty: it always begins with `lts.observe s`. -/
inductive ProducesObs {Label Obs : Type} (lts : LTS Label Obs) :
    lts.State -> List Obs -> Prop where
  | refl (s : lts.State) : ProducesObs lts s [lts.observe s]
  | cons (s t : lts.State) (label : Label) (rest : List Obs) :
      lts.step s label t ->
      ProducesObs lts t rest ->
      ProducesObs lts s (lts.observe s :: rest)

/-- `StutterExpands short long`: `long` is `short` with adjacent copies
    inserted. Each element of `short` appears as a non-empty block. -/
inductive StutterExpands {α : Type} : List α -> List α -> Prop where
  | nil : StutterExpands [] []
  | one (x : α) {xs ys : List α} :
      StutterExpands xs ys -> StutterExpands (x :: xs) (x :: ys)
  | extra {x : α} {xs ys : List α} :
      StutterExpands (x :: xs) ys -> StutterExpands (x :: xs) (x :: ys)

theorem producesObs_observe {Label Obs : Type}
    {lts : LTS Label Obs} {s : lts.State} {obs : List Obs} :
    ProducesObs lts s obs ->
      ∃ rest, obs = lts.observe s :: rest := by
  intro h
  cases h with
  | refl s => exact ⟨[], rfl⟩
  | cons s t label rest hstep hrest => exact ⟨rest, rfl⟩

/-- Hidden concrete steps preserve the concrete observation, because they
    fix the abstract state and `WeakRefines.observe` commutes. -/
theorem weakRefines_hidden_preserves_observe
    {ConcreteLabel AbstractLabel Obs : Type}
    {concrete : LTS ConcreteLabel Obs} {abstract : LTS AbstractLabel Obs}
    {hidden : ConcreteLabel -> Bool}
    {mapLabel : ConcreteLabel -> AbstractLabel}
    (R : WeakRefines concrete abstract hidden mapLabel)
    {s t : concrete.State} {label : ConcreteLabel}
    (hstep : concrete.step s label t)
    (hhid : hidden label = true) :
    concrete.observe s = concrete.observe t :=
  (R.observe s).trans
    ((congrArg abstract.observe (R.hiddenStep hstep hhid)).trans
      (R.observe t).symm)

/-- Finite observation-trace inclusion for the one-step mapping with
    stuttering. Not LTL, not infinite traces, not fairness, not liveness. -/
theorem weakRefines_finite_observation_trace_inclusion
    {ConcreteLabel AbstractLabel Obs : Type}
    {concrete : LTS ConcreteLabel Obs} {abstract : LTS AbstractLabel Obs}
    {hidden : ConcreteLabel -> Bool}
    {mapLabel : ConcreteLabel -> AbstractLabel}
    (R : WeakRefines concrete abstract hidden mapLabel)
    {s : concrete.State} {obs : List Obs}
    (prod : ProducesObs concrete s obs) :
    ∃ aobs,
      ProducesObs abstract (R.mapState s) aobs ∧
      StutterExpands aobs obs := by
  induction prod with
  | refl s =>
      refine ⟨[abstract.observe (R.mapState s)], ProducesObs.refl _, ?_⟩
      rw [← R.observe s]
      exact StutterExpands.one (concrete.observe s) StutterExpands.nil
  | cons s t label rest hstep hrest ih =>
      rcases ih with ⟨aobs, ha, hexp⟩
      cases hhid : hidden label with
      | true =>
          have hμ : R.mapState s = R.mapState t := R.hiddenStep hstep hhid
          rcases producesObs_observe ha with ⟨atail, haeq⟩
          have hhead :
              abstract.observe (R.mapState t) = concrete.observe s :=
            (congrArg abstract.observe hμ.symm).trans (R.observe s).symm
          refine ⟨aobs, hμ.symm ▸ ha, ?_⟩
          rw [haeq, hhead]
          rw [haeq, hhead] at hexp
          exact StutterExpands.extra hexp
      | false =>
          have astep := R.visibleStep hstep hhid
          refine ⟨abstract.observe (R.mapState s) :: aobs,
            ProducesObs.cons (R.mapState s) (R.mapState t) (mapLabel label)
              aobs astep ha, ?_⟩
          rw [← R.observe s]
          exact StutterExpands.one (concrete.observe s) hexp

/-- Same inclusion, specialized to runs that start at an initial state
    (the paper corollary's wording). Initials map by `WeakRefines.init`. -/
theorem weakRefines_finite_observation_trace_inclusion_from_init
    {ConcreteLabel AbstractLabel Obs : Type}
    {concrete : LTS ConcreteLabel Obs} {abstract : LTS AbstractLabel Obs}
    {hidden : ConcreteLabel -> Bool}
    {mapLabel : ConcreteLabel -> AbstractLabel}
    (R : WeakRefines concrete abstract hidden mapLabel)
    {s : concrete.State} {obs : List Obs}
    (hinit : concrete.init s)
    (prod : ProducesObs concrete s obs) :
    ∃ aobs,
      abstract.init (R.mapState s) ∧
      ProducesObs abstract (R.mapState s) aobs ∧
      StutterExpands aobs obs :=
  match weakRefines_finite_observation_trace_inclusion R prod with
  | ⟨aobs, hprod, hexp⟩ => ⟨aobs, R.init hinit, hprod, hexp⟩

#print axioms weakRefines_finite_observation_trace_inclusion
#print axioms weakRefines_finite_observation_trace_inclusion_from_init
#print axioms weakRefines_hidden_preserves_observe

end NMLT

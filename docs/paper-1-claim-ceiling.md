# Paper 1 claim ceiling

**Working title:** Hidden Actions Are Not Contextual Silence  
**Subtitle (optional):** A mechanized counterexample to unconditional refinement congruence under synchronization, and a restricted repair  
**Date:** 2026-08-27  
**Repo snapshot:** local NMLT @ `451001b` (ahead of GitHub main `0417f6e`; not pushed)  
**Status:** draft claim ceiling — freeze before expanding proofs or prose

This document is the authority on what Paper 1 may and may not claim. If the
TeX draft or a talk slide exceeds this ceiling, the ceiling wins until it is
explicitly revised.

---

## 1. One-sentence contribution

We mechanize a counterexample showing that a one-step refinement mapping with
stuttering (not conventional weak simulation) is not congruent under synchronized
parallel composition when a locally hidden concrete action remains connected to a
peer, and we prove a restricted positive product-refinement theorem under named
interface conditions (sharp in the hypothesis-independence sense for the default lift).

---

## 2. Theorems we claim (positive)

### T1 — Standalone weak refinement of the sender
**Lean:** `NMLT.Counterexamples.CompositionCongruence.senderRefinement`  
**Claim:** The concrete sender with hidden `ping` weakly refines the
step-free abstract sender under the candidate local rules (hidden step maps to
abstract state equality; observations preserved).

### T2 — Concrete synchronization exists
**Lean:** `concreteSynchronization`  
**Claim:** In the composite with the receiver, the synchronized
`ping`/`receive` step is enabled from `((), false)` to `((), true)`.

### T3 — No observation-preserving weak refinement of the composites
**Lean:** `noCompositeRefinement` (via `compositeRefinementImpossible`)  
**Claim:** There is no weak refinement of the same shape relating
`C || D` to `A || D` when the synchronized label is classified hidden.
Equivalently: unconditional congruence of the candidate weak simulation under
this synchronization pattern is false.

**Visible dual:** `visibleClassification_fails_on_hidden_ping_wire` (via
`abstractSyncImpossible`; default-lift restatement
`visibleClassification_fails_default_lift`). Same systems with the sync
classified visible; still no product `WeakRefines`, because `A` has no
matching step. Axiom-free. Distinct from T5a `VisibleSync` (honest visible
ping with a different abstract sender). Local dual:
`visiblePing_breaks_senderRefinement`.

### T4 — Exact-action / strong product congruence (restricted repair)
**Lean:** `NMLT.Behavior.OpenComposition.StrongRefinement.compositionCongruence`
(and related receptiveness / wiring lemmas cited in the paper body)  
**Claim:** For the *exact-action* / strong refinement model with whole-wiring
equivalence and the stated composability/receptiveness premises, left-component
refinement lifts through parallel composition.

**Ceiling on T4:** The paper may call this a “restricted repair” or “strong
exact-action fragment.” It must not be described as a proof of the full weak
conditional congruence conjecture.

---

## 3. Conjectures we may state (not theorems)

### T5 — Small-model weak safety lift (Transition.lean LTS)
**Lean:** `NMLT.Behavior.WeakConditionalCongruence.weakConditionalCongruence_safety`
(+ `weakConditionalCongruence_safety_nonempty`)

**Claim:** In the small `LTS` / `WeakRefines` / `parallel` model of
`Core/Transition.lean`, if `InterfaceCompatible` holds — i.e.
`I-NO-HIDDEN-BOUNDARY`, whole-wiring `WiringCovered`, and
`IsolationReflects` (the I-CONNECT domain fragment: an unconnected concrete
label stays unconnected after `mapLabel`) — then
`liftMap R.mapState` is a `WeakRefines` of the products under the default
composite hidden classifier and label map.

**Axioms:** `[propext]` only.

**Ceiling:** This is the one-step observation-safety fragment of the original C1
*for this small model* (finite observation-trace inclusion is the named Lean theorem
`weakRefines_finite_observation_trace_inclusion`: a finite concrete
observation trace is a stutter-expansion of a finite abstract observation
trace; not LTL / infinite traces / fairness / liveness). It does not prove resource/grade/fairness congruence,
OpenComposition port systems, or IsolationReflects from `WiringCovered` alone
(that needs injective `mapLabel`, which is a separate lemma). Do not claim T5
closes full RFC 0008 CONDITIONAL-CONGRUENCE. Relative to published GitHub
`main` @ `0417f6e`, the T5 module may be local-only until pushed/tagged
(`artifact-SHA` placeholder in the paper).

### T5a — Visible ping/receive positive control
**Lean:** `VisibleSync.visibleSync_productRefinement`
Same wiring as T3, but ping is visible and the abstract sender can take it.
Product weak refinement exists. Axioms: `[propext]`.

### T5b — Empty-wiring hidden tau positive control
**Lean:** `EmptyWiringTau.emptyWiring_productRefinement`
Hidden left tau with empty connection; product weak refinement exists.
Axioms: `[propext]`.

### C1 — Residual weak congruence (still open)
The remaining conjecture is anything beyond T5:

- IsolationReflects without assuming it (or injectivity) as an extra field;
- `I-CAP` / `I-GRADE` / `I-FAIR` / `I-RELY`;
- the OpenComposition port/assumption model of T4;
- a composite hidden/map other than the defaults `compositeHiddenOf` /
  `compositeMapOf`.

**Status:** Open. Do not describe T5 as “RFC 0008 CONDITIONAL-CONGRUENCE.”

**First C1 slice (negative; not a lift):** I-CAP is independent of T5. Named lemma `hiddenPing_consume_breaks_resourceRefinement` (`[propext]` only): observational `WeakRefines` (T1 / T5 small model) still holds for hidden ping, but a `token` consume unmatched on the abstract profile falsifies `ResourceRefinement`. Not a positive I-CAP/I-GRADE/I-FAIR theorem; T4 does not imply this.

**Second C1 slice (negative; not a lift):** I-GRADE is independent of T5. Named lemma `hiddenPing_grade_breaks_resourceRefinement` (`[propext]` only): observational `WeakRefines` still holds for hidden ping, but a positive cost atom unmatched by the abstract epsilon (`zero`) profile falsifies `ResourceRefinement.grade`. Not a positive I-GRADE/I-FAIR/I-RELY theorem, not a grade homomorphism through compose; T4 does not imply this.

### C2 — Hypothesis-independence of individual premises
Each named premise of T5 is independently indispensable for the *default* lift
(`compositeHiddenOf` / `compositeMapOf`) in the hypothesis-independence sense.
Do not claim necessity in a stronger, model-universal sense.

**Status (checked, local 2026-08-27; may be ahead of GitHub `main` @ `0417f6e`):**
- Zero-premise / hidden-connected failure: T3 (`noCompositeRefinement`).
- Visible-classification dual of T3 (same systems, sync classified visible):
  `visibleClassification_fails_on_hidden_ping_wire` (axioms: none).
- `I-NO-HIDDEN-BOUNDARY` fails on ping/receive:
  `pingReceive_violates_noHiddenBoundary`.
- `IsolationReflects` independently indispensable for the default T5 lift even when NHB +
  `WiringCovered` hold: `CollidedAbstractWire.isolation_necessary_for_default_lift`
  (non-injective `mapLabel` collapses independent `tau` onto connected `abs`;
  product one-step refinement impossible; axioms: none).
- `WiringCovered` independently indispensable:
  `ExtraAbstractWire.wiring_necessary_for_default_lift`
  (NHB and IsolationReflects hold; `WiringCovered` fails; axioms: none).

---

## 4. Explicit non-claims (must appear in the paper)

The paper **does not** claim:

1. Congruence of the original RFC 0001 compose clause (“hidden sync maps to
   composite silent”) — that clause is withdrawn.
2. Full weak + resource (`I-CAP`) + grade (`I-GRADE`) + fairness (`I-FAIR`)
   congruence.
3. Liveness or divergence transport.
4. Rust implementation correspondence, Charon/Aeneas extraction soundness, or
   verified compilation of the NMLT frontend.
5. Novelty of I/O automata, TLA stuttering, or refinement mappings *in
   isolation*. Novelty is the mechanized collision between local hiding and
   synchronized ports under the stated candidate rules.
6. That NMLT is a complete verified programming language.
7. Any result about C/Rust source equivalence or automatic extraction of
   models from code.
8. That the strong theorem (T4) implies residual C1.
9. That T5 is the full RFC 0008 CONDITIONAL-CONGRUENCE (resources,
   grades, fairness, OpenComposition ports).
10. That the Lean Paper 1 `receiver` is input-receptive, or that the
    OpenSystem-receptive dual (`examples/paper1/receptive_receiver.nmlt`)
    is the paper small model. Lean receive is enabled only at `bit=false`;
    the dual is a distinct executable fixture.

---

## 5. Trusted computing base for claimed theorems

For T1–T5 (and T4 as cited):

- Lean 4.30.0 kernel / elaborator as pinned in `mechanization/lean/lean-toolchain`
- Hand-written definitions in the cited modules
- Foundational axioms reported by `#print axioms` (expect Lean’s standard
  `propext` where applicable; **no** project `sorry`, `sorryAx`, or custom axiom)
- **Recorded 2026-08-27:** T1/T2/T4 axiom-free; T3/T5/T5a/T5b depend on
  `[propext]` only; visible-classification dual of T3 is axiom-free
  (see `papers/hidden-silence/axiom-audit-2026-08-27.md`)
- Standard library only (no Mathlib) for these artifacts

Paper must include an axiom-audit sentence and a reproducibility appendix
(`lake build` from a clean checkout of a tagged commit).

---

## 6. Figures (required)

| ID | Content | Notes |
|---|---|---|
| Fig 1 | Ping/receive systems: A, C, D, wiring | States + labels; show obs bit |
| Fig 2 | Local refinement C ≼ A vs failed composite | Side-by-side |
| Fig 3 | Premise checklist for conditional lift | `I-NO-HIDDEN-BOUNDARY` highlighted |
| Fig 4 (optional) | Exact-action positive instance | Only if T4 is developed in the body |

---

## 7. Related-work posture

**Align with / credit:** Lamport (stuttering, refinement mappings), Lynch et al.
(I/O automata implementation relations and compatibility), Abadi–Lamport,
Peled–Wilke (stutter-invariant LTL / no-`next`), recent MPST liveness
mechanizations (as motivation to keep fairness separate).

**Differentiate:** We do not claim a new automaton model. We claim that a
*specific, tempting rule* combining local weak hiding with synchronized
composition is false, and that the failure is machine-checked. Related systems
often assume interface conditions; the contribution is making the failure and
the minimal-looking repair obligations explicit and mechanized for this
candidate.

---

## 8. Venue posture

| Venue class | Fit | Notes |
|---|---|---|
| CONCUR / EXPRESS/SOS-style | Strong | Compositionality audience |
| CPP / ITP | Strong | Mechanization-first |
| CAV (short / tool+) | Medium | Need clearer “what breaks in tools” hook |
| POPL/PLDI | Weak for v1 | Wait for language slice + weak theorem |

**Recommendation:** Aim CONCUR or CPP first; keep a 12–15 page core with
appendix for Lean listings.

---

## 9. Artifact checklist before submission

- [ ] Tag `paper-hidden-silence-v1` (or equivalent) on the commit that matches
      the paper’s Lean citations
- [ ] RFC 0001 §4 matches this ceiling (silent-sync clause withdrawn)
- [ ] `#print axioms` dumps attached or summarized for T1–T4
- [ ] Ping/receive ASCII or TikZ matches Lean definitions exactly (label names,
      obs values)
- [ ] Every use of “proved” in the PDF points at T1–T4 only
- [ ] C1 clearly labeled conjecture / future work
- [ ] No Build Week / judge-demo framing in the abstract

---

## 10. Revision log

| Date | Change |
|---|---|
| 2026-08-27 | Initial ceiling drafted from Lean counterexample + OpenComposition strong theorem + RFC 0008 |
| 2026-08-27 | Sol 5.6 paper sync: one-step mapping wording; hypothesis-independence; local-vs-0417f6e honesty |
| 2026-08-27 | Named T3 visible dual: `visibleClassification_fails_on_hidden_ping_wire` |
| 2026-08-27 | Named finite obs-trace inclusion: `weakRefines_finite_observation_trace_inclusion` |
| 2026-08-27 | Named Lean vs OpenSystem receiver split (dual fixture is not the small model) |
| 2026-08-27 | First C1 negative slice: `hiddenPing_consume_breaks_resourceRefinement` (I-CAP independent of T5) |
| 2026-08-27 | Second C1 negative slice: `hiddenPing_grade_breaks_resourceRefinement` (I-GRADE independent of T5) |

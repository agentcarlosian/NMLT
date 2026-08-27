# Paper 1 claim ceiling

**Working title:** Hidden Actions Are Not Contextual Silence  
**Subtitle (optional):** A mechanized counterexample to unconditional refinement congruence under synchronization, and a restricted repair  
**Date:** 2026-08-27  
**Repo snapshot:** local NMLT @ `0417f6e` (+ RFC 0001 repair of 2026-08-27)  
**Status:** draft claim ceiling — freeze before expanding proofs or prose

This document is the authority on what Paper 1 may and may not claim. If the
TeX draft or a talk slide exceeds this ceiling, the ceiling wins until it is
explicitly revised.

---

## 1. One-sentence contribution

We mechanize a counterexample showing that weak forward simulation is not
congruent under synchronized parallel composition when a locally hidden
concrete action remains connected to a peer, and we state interface conditions
under which a restricted positive product-refinement theorem holds.

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

### C1 — Weak conditional safety congruence
If `R : C` weakly refines `A` and `interfaceCompatible(R, D, K_C, K_A)` holds,
including at least:

- `I-NO-HIDDEN-BOUNDARY` (no hidden label is connected),
- whole-wiring coverage (`I-CONNECT`),
- observation / sync commutation premises,

then `lift(R, id)` is a weak refinement of the products.

**Status:** Open. Outline in RFC 0008 §5–6. Paper may include the statement
and proof sketch as future work, not as a checked result.

### C2 — Necessity of individual premises
Omitting `I-NO-HIDDEN-BOUNDARY`, or omitting whole-wiring coverage, admits a
counterexample (ping/receive, or the extra-abstract-wire peer-only block).

**Status:** Ping/receive necessity for “no interface conditions” is T3.
Dedicated “omit one named premise” variants may be cited if already in Lean;
otherwise mark as planned negative controls, not theorems.

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
8. That the strong theorem (T4) implies the weak conjecture (C1).

---

## 5. Trusted computing base for claimed theorems

For T1–T3 (and T4 as cited):

- Lean 4.30.0 kernel / elaborator as pinned in `mechanization/lean/lean-toolchain`
- Hand-written definitions in the cited modules
- Foundational axioms reported by `#print axioms` (expect Lean’s standard
  `propext` where applicable; **no** project `sorry`, `sorryAx`, or custom axiom)
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

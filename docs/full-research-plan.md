# NMLT Full Research Plan

> Dated transition document based on the Build Week repository snapshot. It is
> retained for research provenance and does not override the active
> [`Plan.md`](../Plan.md), [roadmap](roadmap.md), or architecture.

**Date:** 2026-08-27
**Repo snapshot:** `0417f6e` (main; Build Week judge sprint merged)
**Author of plan:** research collaborator notes for Carlosian / sus magi
**Intent:** Map the full potential of NMLT as a formal language and mathematical research program, with a publishable near-term paper and a multi-year horizon. This is not a Build Week redo plan.

---


## Freeze note (2026-08-27)

Orbit A / Paper 1 **mechanization core is frozen enough to write against**:
T1–T5, VisibleSync, EmptyWiringTau, CollidedAbstractWire (isolation necessity).
Immediate priority shifts to paper readability + PDF, not residual C1
(resources/grades/fairness) or new big Lean theorems. See
`docs/paper-1-in-plain-english.md`.

Local surface slice: Paper 1 boolean finite-graph sketch in `nmlt-core`
(reachable Bool assignments only; not source-to-LTS). M9 still fail-closes
full compile.

## 0. Frame

NMLT is three coupled bets:

1. **New mathematics** — a compositional account of behaviors, authority, grades, refinement, and evidence, mechanized hard enough that false rules die in Lean.
2. **New languages** — one surface and one typed core in which those objects are first-class, not bolted-on annotations.
3. **New techniques** — evidence-directed development: mutants, fail-closed claim classes, small kernels deciding what automation proposes.

The Build Week Rust verifier and C-to-Rust vignette were a *submission wrapper* around (3), built after the fact so judges had something to click. They are not the program. The program is (1)+(2), with (3) as method.

Working taste for the next year: **one falsifiable theorem at a time, one language slice that can state it, one paper that does not overclaim.** The three-day assisted research sprint built an institution. The next phase is slower on purpose.

---

## 1. Inventory — what already exists (as of `0417f6e`)

### 1.1 Mechanized mathematics (hand-reviewed Lean, excl. Aeneas dump)

About **21** hand-written Lean files, ~5k lines excluding `OpenKernelGenerated/Funs.lean` (~155 KB generated).

| Result | Location | Claim strength |
|---|---|---|
| Unconditional weak-refinement congruence is **false** | `Counterexamples/CompositionCongruence.lean` — `noCompositeRefinement` | Kernel-checked **refutation** (the prize) |
| Exact-action strong product congruence + receptiveness | `Behavior/OpenComposition.lean` — `compositionCongruence` | Bounded positive theorem; no weak hiding |
| Two-sided / mapped / resource-aware congruence seeds | `OpenMappedCongruence.lean`, `OpenResourceCongruence.lean` | Finite safety; axiom audits clean |
| Encoding / readback / kernel-execution contracts | `OpenEncodingCorrespondence.lean`, `OpenKernel*`, `OpenSourceReadback.lean` | Certificate boundary; **not** verified encoder extraction |
| Behavior-indexed temporal typing + constructive evidence erasure soundness | `TemporalTyping.lean` | Seed; implication/negation/until open |
| Directed refinement identity/composition + invariant transport | `Refinement.lean` | Finite stuttering; no fairness |
| Compatible up-to coinduction seed | `Coinductive.lean` — `compatibleUpToSound` | Safety simulation only |
| Graded product algebra laws | `Grades/Algebra.lean` | Math carrier; not Rust/analyzer correspondence |
| Provider / M9 extrinsic correspondence | `Core/*`, `Correspondence/M9Kernel.lean`, `Metatheory/Soundness.lean` | Shared vectors; not compiler correctness |

### 1.2 Language and executable spine

- Lossless frontend (`nmlt-core`), HIR resolver, elaborator, sealed `CheckedProgram` kernel, BFS engine, CLI.
- **M9 supported fragment** is narrow: enums, Bool/Nat/Int, systems, scalar state, guards, simultaneous updates, `Once<T>`, observations, safety/temporal ASTs.
- Design fixtures (`examples/technicus/provider_attempt.nmlt`, ports in `trust_chain.nmlt`, resources in `token_bucket.nmlt`) are **richer than what typechecks/model-checks end-to-end**.
- Zero crates.io dependencies; `unsafe_code = forbid`.
- Evidence vocabulary is real: `model_checked` / `refuted` / `unknown` / scoped `proved`; stale bindings fail closed.

### 1.3 Known inconsistency to fix immediately

RFC 0001 §4 still states a compose rule whose “hidden synchronization → silent observation” clause is exactly what `noCompositeRefinement` kills. **Candidate rules must not contradict checked theorems.** First hygiene task before any new math.

### 1.4 What is *not* yet a breakthrough

- Four-coordinate grade product (RFC 0012): conservative engineering.
- Rzk/Segal seed: outside TCB (`U : U`); fashion risk.
- Charon/Aeneas kernel translation: impressive systems work; paper only if tied to a semantic theorem people care about.
- Provider-attempt 9-state checker: evaluation fixture, not novelty.
- 13 RFCs + governance: process scaffolding, not results.

---

## 2. Novelty thesis (what “full potential” means)

The field already has:

- TLA+ / Quint — stuttering, refinement maps, executable specs
- I/O automata — receptiveness, compositional traces
- Linear / affine types — non-duplicable authority
- Graded modal types (Granule et al.) — resource algebras
- Session types — protocol composition and (increasingly) liveness
- Small checkers / proof-carrying code — untrusted search, trusted kernel

**Combining them is not automatically new mathematics.** The combination pays rent only where the ingredients *disagree*.

### The disagreement you already own

TLA-style hiding treats a local identity step as stutter. Port-style composition lets that same label synchronize with a peer. Affine authority adds a third failure: a “hidden” step that spends a peer-owned capability. RFC 0001’s candidate allowed the first two to collide. Lean killed it.

**Central thesis for the next 1–2 years:**

> Refinement hiding is not contextual silence. Compositional refinement for open, authority-sensitive, graded behaviors requires interface, resource, and (later) fairness premises that are necessary as well as sufficient — and those premises must be stated in the *type* of a behavior, not recovered after the fact by a model checker.

That is the spine. Everything else is either support (language, evidence) or a later orbit (constructive temporal evidence, statistical grades, hybrid dynamics).

---

## 3. Architecture of the full program

Think in three layers and five research orbits. Do not try to advance all orbits at once.

```text
                    ┌─────────────────────────────┐
                    │  Evidence calculus (W4)      │
                    │  claim classes, identities,  │
                    │  fail-closed readback        │
                    └──────────────▲───────────────┘
                                   │
 ┌─────────────────┐    ┌──────────┴──────────┐    ┌─────────────────┐
 │ Surface language│───▶│ Typed behavioral    │───▶│ Engines / Lean  │
 │ .nmlt (W2)      │    │ core + open systems │    │ (W1, W3)        │
 └─────────────────┘    └──────────┬──────────┘    └─────────────────┘
                                   │
                    ┌──────────────▼───────────────┐
                    │ Techniques (W5–W6): mutants, │
                    │ repair authority, runtime    │
                    │ journals (method, not thesis)│
                    └──────────────────────────────┘
```

### Orbit A — Compositional refinement (PRIMARY)
Weak/interface-aware congruence; necessity of premises via negative controls; eventual fairness/divergence.

### Orbit B — Language = mathematics
Surface constructs for ports, capabilities, compose, refine elaborate into the objects Orbit A proves about. Close the design-fixture / executable-fragment gap.

### Orbit C — Evidence as mathematics
Not just JSON manifests: a small theory of claim constructors, composition of evidence, and when `unknown` is forced. This is NMLT’s “Technicae” contribution and supports every paper.

### Orbit D — Constructive temporal evidence (SECONDARY, deep)
Separate classical truth from composable witnesses; implication, negation, until, productivity without smuggling LEM into infinite traces.

### Orbit E — Quantitative grades on behaviors (TERTIARY)
Only after grades survive hiding and composition in a way that is not “Granule on the side.” Privacy/energy claims need real semantics or stay quarantined.

**Quarantined until someone forces them:** hybrid dynamics, cubical/HoTT as core, probability model checking, Rzk in the TCB, production attestation, codegen, “NMLT verifies C↔Rust.”

---

## 4. Paper track (near-term, from what exists)

### Paper 1 — “Hidden Actions Are Not Contextual Silence”
**Status:** Writable now; strengthen while writing.
**Venue shape:** CONCUR / CAV (tools+theory short) / CPP / ITP / JLAMP-style journal later. Start as a tight conference paper.

**Contribution triad:**
1. Mechanized counterexample to unconditional weak forward-simulation congruence under synchronization (`noCompositeRefinement`).
2. Diagnosis: local state-map equality ≠ contextual silence when labels are connected.
3. Sufficient interface conditions (`I-NO-HIDDEN-BOUNDARY`, whole-wiring coverage, …) and a **restricted** positive theorem (exact-action / strong refinement product congruence), with negative controls showing dropped premises fail.

**Explicit non-claims:** no full weak+resource+fairness theorem yet; no Rust↔Lean compiler correspondence; no novelty of I/O automata or TLA alone.

**Artifacts to ship with the paper:**
- Lean development (counterexample + positive theorem + axiom audit).
- Minimal ASCII diagram of ping/receive.
- Diff: RFC 0001 compose rule **before/after** repair (shows intellectual honesty).
- Optional: 2-page comparison table vs I/O automata implementation relations and TLA refinement mappings.

**Writing order (do not wait for M11-001c extraction):**
1. Fix RFC 0001 so it no longer states the refuted rule.
2. Freeze theorem statements and example in a `papers/hidden-silence/` (or Overleaf) with claim ceilings.
3. Prove or clearly demarcate remaining weak-case obligations as “conjecture / future work.”
4. Related work pass: Lynch I/O automata, Lamport stuttering, Abadi–Lamport refinement mappings, recent MPST liveness in Rocq.
5. Submit when the counterexample + strong repair + necessity story is tight — even if weak congruence is still open.

**Success:** reviewers can re-check Lean; they understand why the tempting rule fails; they see what is still open.

### Paper 2 — “Evidence Objects for Bounded Behavioral Claims” (6–12 months)
Typed result classes, source/engine identity, mutant oracles, fail-closed stale binding. Provider suite as evaluation. This is the techniques paper. Can cite Paper 1 for why composition needs explicit interfaces in the claim.

### Paper 3 — “Behavior-Indexed Types with Affine Authority” (12–24 months)
Only after Orbit B makes `capability` / `compose` executable and Orbit A’s weak theorem (or a clean fragment) lands. Language paper with mechanized metatheory and a second case study beyond provider-attempt.

### Paper 4 (optional, opportunistic) — Constructive temporal evidence
Only if Orbit D produces a crisp theorem (e.g. erasure soundness for a usable fragment including implication/until). Do not schedule this as the next milestone.

---

## 5. Mathematical roadmap (Orbit A in detail)

### Phase M-A0 — Hygiene (1–2 weeks)
- [ ] Revise RFC 0001 composition/hiding to match RFC 0008 diagnosis.
- [ ] Add a permanent “refuted candidate” subsection pointing at `noCompositeRefinement`.
- [ ] Ensure README/Plan do not describe unconditional congruence as a goal without the interface premises.
- [ ] Tag a research snapshot: `research-composition-v0` (Lean + RFCs only; ignore judge demo).

### Phase M-A1 — Weak safety congruence (3–6 months) ★ main quest
Prove the conditional theorem from RFC 0008 §5 for **weak** refinement:

```text
R : C refines A
interfaceCompatible(R, D, K_C, K_A)   // includes I-NO-HIDDEN-BOUNDARY, whole wiring
resourceCompatible(R, D)              // may be postponed to M-A2
relyCompatible(R, D)
------------------------------------------------
lift(R, id) : (C || D) refines (A || D)
```

Order inside M-A1:
1. Encode ports, directions, connection maps, `I-NO-HIDDEN-BOUNDARY` on the *weak* model (not only strong/exact-action).
2. Case split on hidden left / visible left / peer / sync (outline already in RFC 0008 §6).
3. Necessity suite: omit each premise → counterexample (extend `Counterexamples/`).
4. Optional grades/capabilities **after** the pure interface theorem is stable.

**Do not** block M-A1 on Charon equality-soundness or verified encoder extraction. That is systems debt from the M11-001c branch, not the semantic theorem.

### Phase M-A2 — Authority and grades through composition (after M-A1)
- Affine capability partition and transfer under sync (`I-CAP`).
- Grade homomorphism (`I-GRADE`).
- Negative controls: shared capability; nonmonotone grade map.
- Connect to RFC 0006 / 0012 only as much as the theorem needs.

### Phase M-A3 — Fairness and divergence (after M-A2)
- Hidden infinite stutter vs progress.
- Fairness transport obligations (`I-FAIR`).
- Keep liveness out of safety papers. Study Rocq MPST liveness mechanization as methodology, not as a reason to switch provers unless Lean coinduction becomes the bottleneck.

### Phase M-A4 — Correspondence (selective)
Only prove Rust↔Lean correspondence for definitions that Paper 1–3 cite. Prefer **translation validation** or shared executable oracles over full verified compilation. M11-001c encoder extraction is one path, not the only path, and not the critical path for Paper 1.

---

## 6. Language roadmap (Orbit B)

Goal: **the language you mean is the language the checker and Lean talk about.**

### L0 — Single source of truth for the fragment (now)
- Document the M9 supported fragment on one page (steal from RFC 0013 §“Initial supported fragment”).
- Mark every example as `executable` | `syntax-only` | `aspirational`.
- Fix stale comments (e.g. mutant files saying result is `unknown` when `model-check` refutes).

### L1 — Promote design fixtures through M9 (1–3 months)
Make these elaborate + typecheck + (where finite) model-check:
- `capability` / `consume` / `Once<_>` as in `provider_attempt.nmlt`
- `observe`
- richer `Phase` enums already used in fixtures

Until `examples/technicus/provider_attempt.nmlt` is executable, the flagship example is a poster.

### L2 — Open systems in the surface (parallel to M-A1)
- `port`, `assume`, `guarantee`, `compose` (see `trust_chain.nmlt` sketch).
- Elaboration into the open-system structures Lean already has.
- A surface rendering of the ping/receive counterexample as regression syntax.

### L3 — `refine` and observation maps (after L2)
- Refinement declarations that produce checkable witnesses (even if only strong/finite first).
- Property transport only with explicit maps (RFC 0001 already requires this).

### L4 — Usability without abandoning honesty
- `nmlt fmt`, basic LSP later.
- Diagnostics that point at failed interface premises (“hidden label `ping` is connected”) — this turns the math into a language feature.
- Still no claim of C/Rust parsing.

**Language success metric:** a second person can write an open system in `.nmlt`, compose it, and either get a counterexample in the style of Paper 1 or a bounded refinement certificate — without reading Plan.md.

---

## 7. Evidence / techniques roadmap (Orbit C)

Keep this thin but sharp.

- Freeze an **Evidence Kernel** note: constructors, promotion vetoes, identity tuple (source, ruleset, engine, bounds).
- Every theorem in Orbit A gets an evidence manifest (you already do this for M11 — generalize the pattern, reduce the markdown sprawl).
- Mutant discipline stays mandatory for any “validated slice.”
- Agentic repair (`nmlt-agent`) stays a **method demo**, not a research headline, until Orbit B is real.

---

## 8. Full potential map (5-year horizon, deliberately sparse)

| Horizon | If it works, the world gets… | Depends on |
|---|---|---|
| 0–1 yr | A cited theorem: hidden≠silence; repaired conditional congruence; honest RFC | M-A0, M-A1, Paper 1 |
| 1–2 yr | A small open-system language people can write; affine authority through compose; evidence paper | L1–L3, M-A2, Paper 2–3 |
| 2–3 yr | Fairness/divergence story; maybe constructive temporal fragment; second domain case study (consensus, MCP tool auth, webhook replay) | M-A3, Orbit D seed |
| 3–5 yr | Graded behaviors with *semantic* grades (not annotation products); optional runtime monitors; external users | Orbit E only if A+B solid |

**Full potential is not “replace TLA+ and Lean.”**
Full potential is: **the first practical type theory where open behavioral refinement, linear authority, and typed evidence are one judgment family, and false composition principles are kernel-rejected.**

---

## 9. What to stop doing

| Stop / defer | Why |
|---|---|
| M11-001c verified encoder extraction as main quest | Systems side-quest; does not unlock Paper 1 |
| New RFCs before revising 0001 | Process without truth maintenance |
| Rzk/Segal deepening | Unsound assistant; dilutes focus |
| Expanding grade coordinates / DP claims | RFC 0012 already says annotations ≠ privacy theorems |
| Judge-demo / Devpost energy | Costume complete |
| Milestone theater (M12, M13, …) | Replace with theorem-driven phases above |
| Kitchen-sink extensions (hybrid, cubical core, probabilistic MC) | Your own Phase 7 quarantine was correct |

---

## 10. 30 / 90 / 365 day plan

### Next 30 days
1. RFC 0001 repair + changelog entry.
2. Paper 1 outline + theorem/claim ceiling doc (2–3 pages).
3. Label all examples executable vs syntax-only.
4. Re-check Lean metatheory gate on a clean machine; pin axiom dumps for the counterexample and `compositionCongruence`.
5. Decide venue target and deadline.
6. Optional: surface syntax sketch of ping/receive as `.nmlt` regression (even if not fully elaborated yet).

### Next 90 days
1. Paper 1 first complete draft.
2. M-A1 underway: weak model + `I-NO-HIDDEN-BOUNDARY` encoded; at least one new necessity counterexample.
3. L1 started: `provider_attempt.nmlt` on the checked pipeline (or a documented subset with an explicit feature ladder).
4. Collapse stale handoffs into one `docs/RESEARCH_STATUS.md` pointing at this plan.

### Next 365 days
1. Paper 1 submitted and revised.
2. M-A1 complete or honestly reduced with a new checked obstruction (either outcome is research progress).
3. Paper 2 drafted.
4. Language slice L2 usable for open composition examples.
5. One external case study chosen and begun (not provider-attempt).

---

## 11. Operating method (keep what worked in 3 days, drop what did not)

**Keep:**
- Intent capsule → reference → mutants → witness → independent replay.
- `#print axioms` / no `sorry` policy.
- Negative controls as permanent artifacts.
- Explicit residual gaps in every claim.

**Drop:**
- Parallel invention of seven “complete” phases in one weekend.
- Latin/manifesto-first public framing when talking to mathematicians (use it in the thesis statement, not as the abstract).
- Treating generated output as architecture; it remains a search tool.

**Cadence:**
- Weekly: one theorem obligation or one language obligation, not both as “almost done.”
- Monthly: rewrite the claim ceiling for Paper 1.
- After any failed proof: either a counterexample file or a revised RFC — never an axiom.

---

## 12. Decision record — recommended defaults

| Question | Recommendation |
|---|---|
| Primary goal for 12 months? | Paper 1 + M-A1 weak safety congruence |
| Continue M11-001c equality soundness now? | Background / opportunistic only |
| Expand the executable language or freeze it? | Expand **only** along L1–L2 (capabilities, ports, compose) |
| Second case study domain? | Pick after Paper 1 draft; prefer agent tool-auth or webhook/idempotency protocols |
| Switch to Rocq for coinduction? | Only if M-A3 stalls hard on Lean infrastructure |
| Is the Build Week verifier part of the research story? | Footnote / related artifact; not the contribution |

---

## 13. Success criteria

You are succeeding if, in twelve months:

1. A stranger can re-check a Lean counterexample that changes how they write hiding+composition rules.
2. RFC 0001 no longer contradicts the kernel.
3. At least one positive conditional congruence theorem is stated at the strength you claim, no more.
4. The flagship `.nmlt` example is executable, not a poster.
5. You have not accumulated three new “bounded seeds” that nobody outside the repo can name.

You are failing if:

- Plan.md grows and theorem count for Orbit A does not.
- Evidence JSON outpaces proofs.
- The next public artifact is another demo of a 9-state missing guard.

---

## 14. Immediate next artifact

**Do this next, before more code:**

1. Patch RFC 0001 composition section.
2. Create `docs/paper-1-claim-ceiling.md` with the exact theorems, non-theorems, and figure list for “Hidden Actions Are Not Contextual Silence.”
3. Start the paper TeX with the counterexample as Section 2 (not the related work as Section 2).

The full potential is large. The way to reach it is to let this one disagreement with the literature finish becoming a theorem, a language construct, and a citation — in that order.

---

## Appendix A — Key paths in this checkout

```text
mechanization/lean/NMLT/Counterexamples/CompositionCongruence.lean
mechanization/lean/NMLT/Behavior/OpenComposition.lean
rfcs/0001-behavior-types.md
rfcs/0008-mechanization-and-compositional-refinement.md
rfcs/0013-source-to-typed-core.md
docs/metatheory/phase-1-mathematical-core.md
docs/core-calculus.md
docs/manifesto.md
examples/technicus/provider_attempt.nmlt
docs/reboot-handoff-2026-07-20.md   # M11-001c systems continuation; not Paper 1 critical path
```

## Appendix B — Relationship to existing Plan.md

`Plan.md` remains a useful lab notebook of what was built in July 2026. This document **supersedes it as the forward research strategy**. Where they conflict (e.g. “active focus = M11-001c full congruence extraction”), prefer this plan’s Orbit A / Paper 1 priority unless you explicitly re-choose systems correspondence as the main quest.

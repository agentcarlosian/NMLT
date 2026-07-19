# RFC 0012: Conservative graded-resource modalities

- Status: Experimental
- Authors: Carlosian <carlosian@agentmail.to>
- Created: 2026-07-18
- Phase: 7 independent research extension

## Summary

Add an independent, explicitly bounded resource-analysis extension whose
grades are products of four upper-bound coordinates:

```text
Grade = CostTicks × PrivacyMicroEpsilon × EnergyMicrojoules × UncertaintyPpm
```

Sequential work composes by checked componentwise addition, alternatives by
componentwise maximum, and parallel work by the same conservative addition.
Uncertainty addition saturates at one million parts per million. Unbounded
iteration, arithmetic overflow, malformed grades, or an unknown child makes a
budget claim `unknown`; it never becomes `within_budget`.

The implemented checker validates declared annotations. It does not infer
them from arbitrary code and does not prove that a privacy annotation belongs
to a differentially private mechanism, that an energy annotation matches a
physical device, or that uncertainty is a calibrated probability.

## Research basis

### In the archive

The requested `search-the-archives` collector was run on 2026-07-18 with
focused queries for graded monads and quantitative types, graded effect
systems and resource semirings, quantitative/resource modalities, coeffects,
linear resource analysis, and privacy type systems. No relevant graded-type,
graded-effect, or quantitative-resource work was surfaced in the local JSON
archive. Broad `linear types` and `differential privacy type system` queries
returned recent lexical false positives about transformer linearization,
generic privacy systems, and unrelated applications; they were excluded.

The collector's live arXiv calls were rate-limited or timed out for several
queries, and its Hugging Face feed had no relevant result. This RFC therefore
does not attribute a design decision to a work previously captured in the
archive.

### New/current primary sources

- Orchard, Liepelt, and Eades' Granule paper,
  [*Quantitative Program Reasoning with Graded Modal Types*](https://www.cs.kent.ac.uk/people/staff/dao7/publ/granule-icfp19.pdf),
  demonstrates multiple user-facing graded modalities built over algebraic
  grade structures. NMLT adopts the separation between a generic algebraic
  interface and specific grades, but not Granule's full type theory.
- Moon, Eades, and Orchard's
  [*Graded Modal Dependent Type Theory*](https://arxiv.org/abs/2010.13163)
  studies parameterized semiring-like quantitative data-flow analyses in a
  dependent setting. It motivates keeping the algebra explicit and treating
  richer dependent grades as later work.
- Gordon's
  [*A Generic Approach to Flow-Sensitive Polymorphic Effects*](https://arxiv.org/abs/1705.02264)
  gives an algebraic account of sequential effects using effect quantales. It
  motivates stating the exact law profile rather than calling every grade
  structure a semiring. NMLT's selected profile is commutative; other
  order-sensitive extensions may validly be noncommutative.
- Gaboardi et al.'s
  [*Linear Dependent Types for Differential Privacy*](https://cs-people.bu.edu/gaboardi/publication/GaboardiEtAll13popl.pdf)
  connects sensitivity typing and randomized computation to differential
  privacy. NMLT does **not** implement that proof chain; a numeric privacy
  coordinate alone is insufficient.
- Kairouz, Oh, and Viswanath's
  [*The Composition Theorem for Differential Privacy*](https://arxiv.org/abs/1311.0776)
  studies privacy loss under sequential composition, while Smith et al.'s
  [parallel-composition analysis](https://arxiv.org/abs/2109.09078) makes data
  overlap central to stronger parallel bounds. NMLT therefore adds privacy in
  parallel unless a future extension checks disjointness evidence.
- Kahn and Hoffmann's
  [*Exponential Automatic Amortized Resource Analysis*](https://arxiv.org/abs/2002.09519)
  shows how a type-based analysis can derive concrete resource bounds and
  preserve compositional inference. The present backend is much narrower: it
  checks annotations and does not perform amortized inference.
- Kura, Gaboardi, Sekiyama, and Unno's 2026 preprint
  [*A Category-Theoretic Framework for Dependent Effect Systems*](https://arxiv.org/abs/2601.14846)
  covers indexed graded monads for value-dependent effects, including cost and
  probability/expectation bounds. It is a current lead for symbolic grades,
  not evidence that this prototype implements dependent effects.
- Abel, Danielsson, and Eriksson's 2026 preprint
  [*A Graded Modal Dependent Type Theory with Erasure, Formalized*](https://arxiv.org/abs/2603.29716)
  reports an Agda formalization parameterized by a partially ordered
  semiring-like modality structure. Its mechanization standard informs the
  open proof obligations below; NMLT has not reproduced those theorems.

No on-point primary source found in this search establishes one common
cost/privacy/energy/uncertainty product with NMLT's exact operations. That
product is a conservative engineering proposal and is evaluated as such.

## Motivation

NMLT needs a way to state claims such as “every bounded execution of this
plan stays below all four declared limits” without collapsing unlike
quantities into one score. A product keeps the units distinct, makes budget
violations local, and lets an independent extension fail closed without
changing the alpha language's trusted semantics.

The extension also tests a wider thesis: formal languages should carry the
conditions under which a quantitative claim is meaningful. In particular,
`privacy = 500000` is not accepted as a privacy theorem merely because the
arithmetic fits under a budget.

## Goals

- Define a deterministic product grade with explicit units and order.
- Give compositional rules for atoms, sequence, choice, parallel work, and
  finite repetition.
- Preserve `unknown` for missing bounds and overflow.
- Produce dimension-specific budget counterexamples.
- Bind the reference, claim, implementation, executable, controls, and
  evidence to exact identities.
- Include an invalid-algebra control and source-level semantic controls.
- Bind finite regression sampling and the separate Lean algebra proof without
  conflating either with Rust extraction or program-analysis soundness.

## Non-goals

- Inferring cost, privacy, energy, or uncertainty annotations.
- Proving differential privacy, sensitivity, calibration, independence, or a
  physical energy model.
- Modeling latency; `cost_ticks` means total abstract work, not wall time.
- Applying the max privacy parallel-composition theorem without checked data
  disjointness.
- Supporting real-valued, symbolic, dependent, amortized, or asymptotic
  grades in this prototype.
- Claiming a certified compiler, verified Rust extraction, typed-term
  preservation, or program-analysis soundness from the algebra capsule.
- Claiming universal algebra laws from finite tests; the universal laws listed
  below instead have a separate Lean proof over their mathematical carrier.

## Surface language

The extension is an isolated S-expression pilot:

```text
PROGRAM ::= program NAME budget NAT NAT NAT PPM plan PLAN
PLAN    ::= (atom NAME NAT NAT NAT PPM)
          | (seq PLAN*)
          | (choice PLAN+)
          | (par PLAN*)
          | (repeat BOUND PLAN)
BOUND   ::= NAT | ?
```

The four numbers always appear in this order: cost ticks, privacy
micro-epsilon, energy microjoules, and uncertainty ppm. Comments begin with
`#`. `?` is an explicit unknown bound, not an inferred value. An empty
sequence or parallel group has grade zero; an empty choice is invalid because
there is no executable alternative whose bound could be selected.

## Mathematical grade structure

Let `M = 1,000,000`, `U = {0, ..., M}`, and define the mathematical exact
carrier:

```text
G = Nat × Nat × Nat × U
0 = (0, 0, 0, 0)
```

The order is componentwise. For `g = (c,p,e,u)` and
`h = (c',p',e',u')`:

```text
g then h = (c+c', p+p', e+e', min(M, u+u'))
g join h = (max(c,c'), max(p,p'), max(e,e'), max(u,u'))
g par  h = g then h
```

This selected profile has the following mathematical laws over unbounded
naturals:

1. `then` and `par` are commutative monoids with identity `0`;
2. `join` is a commutative, associative, idempotent operation with identity
   `0`, inducing the componentwise order;
3. `then` is monotone and distributes over binary, hence nonempty finite,
   `join`;
4. `0` is the least grade.

This is an ordered, join-enriched commutative resource algebra. The RFC does
not call it a semiring: numeric zero is both the sequencing identity and the
least alternative, so the usual annihilating-zero semiring axiom is not the
selected interface.

The proof is coordinatewise. Natural addition is associative, commutative,
monotone, and distributes over maximum. `min(M, x+y)` retains those monoid and
distributivity laws on `U` because truncation is monotone and preserves
maximum. Finite products preserve the component laws. The binary
distributivity statement extends to nonempty finite choices. It deliberately
does not include the empty join: `0` is also the sequencing identity, not an
annihilator.

[`NMLT/Grades/Algebra.lean`](../mechanization/lean/NMLT/Grades/Algebra.lean)
mechanizes this exact mathematical product in Lean 4.30.0. It proves saturated
uncertainty addition and the product identity, associativity, commutativity,
choice, order, monotonicity, binary/nonempty-finite distributivity, and Boolean
budget-order theorems. The file is imported by the Lean root and the axiom audit
reports only Lean's standard `propext` and `Quot.sound`; it contains no
`sorry`, `admit`, project axiom, or `native_decide`.

Rust uses three `u64` coordinates rather than mathematical naturals. Checked
overflow is lifted to `unknown`, making the evidence layer total and
fail-closed; it is not wraparound and is not a new exact grade.

## Static analysis rules

Write `P ⇓ g` for exact analysis, `P ⇓ ?(d)` for unknown with diagnostic `d`,
and `g ≤ b` for componentwise budget order.

```text
                         each Pi ⇓ gi
------------------       --------------------------
atom(a,g) ⇓ g             seq(P1..Pn) ⇓ g1 then ... then gn

each Pi ⇓ gi, n>0         each Pi ⇓ gi
------------------       -------------------------
choice(P1..Pn) ⇓ join gi  par(P1..Pn) ⇓ g1 par ... par gn

P ⇓ g                     P ⇓ g
--------------------      -------------------------------
repeat(0,P) ⇓ 0           repeat(n+1,P) ⇓ g then repeat(n,P)

---------------------------------------------
repeat(?,P) ⇓ ?(unknown iteration bound)
```

An unknown premise makes the enclosing expression unknown. A checked
arithmetic overflow also produces unknown. The budget rules are:

```text
P ⇓ g    g ≤ b                 P ⇓ g    not (g ≤ b)
-------------------------      ----------------------------
check(P,b) = within_budget      check(P,b) = exceeded(dims)

P ⇓ ?(d)
--------------------
check(P,b) = unknown(d)
```

There is deliberately no rule from `unknown` to `within_budget`.

## Composition interpretations

- **Sequence:** cost, energy, and basic privacy loss add. Uncertainty uses a
  saturated union-bound abstraction.
- **Choice:** at most one alternative executes; the checker selects a
  componentwise worst case. A componentwise join need not correspond to one
  concrete branch, but it is an upper bound for every branch.
- **Parallel:** total work, total energy, and uncertainty add conservatively.
  Privacy also adds because the prototype carries no data-domain proof.
  Latency and disjoint-data privacy would require separate coordinates and
  evidence rules.
- **Repeat:** only a syntactically finite count gives an exact result. Fast
  exponentiation changes evaluation cost, not the defined repeated sum.

## Backend and evidence classes

`nmlt-grades` parses the pilot language into a typed plan AST, computes the
grade, and compares it with the product budget. It can return only:

- `within_budget`: exact annotated composition is componentwise within budget;
- `exceeded`: exact annotated composition exceeds named coordinates;
- `unknown`: a required bound is absent, arithmetic overflowed, or a child is
  unknown;
- parse rejection: the source is malformed or a grade is invalid.

Even `within_budget` is not `proved` in the repository-wide evidence
vocabulary. The strongest exact claim is “the executable checker evaluated
these bound annotations under RFC 0012's finite rules.”

The evidence harness independently compiles twice, requires identical
executables, executes twice, requires identical JSON, validates the schema,
recomputes normative source and source-set IDs, binds all controls, and
recomputes the RFC 8785-compatible integer/string-subset evidence ID. It also
binds the Lean entry point, root import, grade build/toolchain source set,
checker, and exact theorem handles. Lean kernel checking remains a separate
pinned CI gate; the Python evidence harness does not impersonate the kernel. The claim
identity is extension-local until a canonical graded core IR is frozen.

## Negative controls

The extension is not accepted unless all controls behave as follows:

| Control | Required result | Failure guarded against |
| --- | --- | --- |
| privacy use `500001`, budget `400000` | `exceeded(privacy)` | accepting a component that exceeds its product budget |
| `repeat ?` | `unknown` | treating an absent loop bound as zero or success |
| uncertainty `1000001 ppm` | parse rejection | accepting a value outside the declared scale |
| concatenating word algebra | commutativity-law witness | accepting an algebra that violates this extension's declared profile |
| `u64::MAX + 1` | `unknown` | wraparound or silent saturation of exact cost |
| empty choice | `unknown` | treating no alternative as a zero-cost execution |
| forged `proved` evidence | schema rejection | assurance inflation |
| stale source identity | binding rejection | replaying a result for different bytes |
| forged verified-Rust-extraction flag | schema rejection | laundering a standalone algebra theorem into compiler correctness |
| missing Lean theorem handle | schema rejection | weakening the bound metatheory after evidence generation |

The word-algebra control does not say noncommutative effects are invalid in
general. It says they cannot masquerade as RFC 0012's commutative product.

## Trusted computing base and threat model

The claim trusts:

- all atom annotations, units, and the assumption that each is an upper bound;
- the plan's control-flow abstraction, especially exclusive choice and finite
  repeat counts;
- the Rust parser, grade operations, analyzer, and budget comparator;
- the Rust compiler and host used to build/run the checker;
- for the mathematical algebra theorems only, the bound Lean sources, Lean
  4.30.0 elaborator/kernel, standard `omega` tactic proof generation, project
  build configuration, and metatheory gate;
- the Python evidence harness, JSON parser, local schema validator,
  canonicalization subset, SHA-256 implementation, and filesystem reads;
- human interpretation that the narrow claim matches the intended system.

Important threats and responses:

- A false or stale annotation can produce a numerically valid but materially
  false result. Response: no promotion beyond annotation checking; future
  backends must bind measurement or proof provenance.
- Unit confusion can make products meaningless. Response: coordinate names
  and integer scales are fixed in syntax, schema, and claim identity.
- Applying a DP parallel maximum without disjointness is unsound. Response:
  the prototype always adds privacy in parallel.
- Correlated or non-probabilistic “uncertainty” may invalidate a probabilistic
  reading. Response: the coordinate is explicitly abstract and only the
  saturated-bound arithmetic is claimed.
- Overflow, unbounded iteration, malformed input, stale identities, forged
  assurance, or missing controls could launder a result. Response: each fails
  closed and is exercised by the evidence harness.
- SHA-256 identities establish byte equality, not truth, authorship, or
  physical calibration.

## Compatibility and extension boundary

This pilot uses `.nmltg`, a separate parser, and a separate evidence schema.
It changes no alpha NMLT syntax or existing model-checking claim. Promotion
into the main language requires a later RFC that connects graded terms to the
typed core, defines elaboration and substitution, and updates canonical
semantic identity.

## Alternatives

- **One scalar score:** rejected because addition across unlike units is not
  meaningful and hides which budget failed.
- **Floating-point grades:** rejected for the first evidence format because
  canonical identity and comparison become more complicated. Fixed integer
  units are explicit and deterministic.
- **Parallel privacy as maximum:** postponed until disjointness/overlap is an
  explicit checked premise.
- **Infer annotations with AARA or dependent effects:** compelling future
  work, but substantially larger than an independent executable pilot.
- **Accept arbitrary user-defined grade code:** postponed until termination,
  law checking, identity, and trust rules for grade implementations exist.

## Proof and implementation obligations

Complete in this pilot:

- deterministic parser and typed plan AST;
- checked four-coordinate algebra and compositional analyzer;
- budget backend with exact violations and explicit unknown;
- finite regression law checker and noncommutative control;
- reference benchmark, source controls, schema, exact identities, and
  reproducible executable evidence;
- pinned Lean proofs for the mathematical product algebra, nonempty-finite
  choice distribution, componentwise order, and executable Boolean budget
  predicate;
- unit tests for operations, bounds, parser failures, overflow, and controls.

Open before language promotion:

- mechanize substitution/grade-preservation rules and the plan analyzer; the
  standalone mathematical grade laws are complete for this profile;
- prove correspondence between the Rust `u64`/overflow implementation and the
  Lean `Nat` model, or produce the Rust checker from a verified definition;
- define an operational cost/energy semantics and prove annotation soundness;
- connect privacy grades to sensitivity and randomized mechanisms;
- give uncertainty a named semantics (probability, imprecision, belief, or
  another calculus) rather than a generic label;
- support symbolic/dependent bounds without general proof search in routine
  type checking;
- prove elaboration and compiler correspondence to the main NMLT core.

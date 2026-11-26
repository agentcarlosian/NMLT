# R4 discovery workflows: bounded engineering evidence

This increment extends the [first wiring control](r4-composition-premise.md).
It adds a complete fixed-premise experiment, a finite worker-protocol search,
and checked finite graph objects. It does not establish human usefulness,
mathematical novelty, host-process refinement, or general liveness.

## Evidence states

| State | Meaning here |
|---|---|
| Conjecture | A statement or candidate before checking. |
| Bounded computational evidence | Exhaustive evaluation inside declared finite bounds; `assurance: none`. |
| Checked refutation | A Lean proof of the negation of the fixed universal statement, with retained-premise and enabledness witnesses. |
| Lean theorem | A closed declaration checked by Lean; independent acceptance additionally requires the current NanoDA gate. |
| Novelty/usefulness assessment | A separate human/domain judgment; not supplied by proof acceptance. |

JSON reports from the experimental Python tools are inventories and diagnostics,
not new accepted proof artifacts. They cannot promote an unchecked run. Only
the existing Lean/NanoDA and finite-path checking routes supply their existing
assurance. Timeouts, missing tools, and failed proof search remain unknown;
rejection of a candidate does not refute its proposition.

## Composition premises

[CompositionPremises.lean](../mechanization/lean/NMLT/Discovery/CompositionPremises.lean)
fixes the model class in `Problem` and the conclusion in `Problem.conclusion`:
existence of the original composite `ResourceWeakRefinement`, allowing any
state map. The four whole premises are component refinement `R`, equivalent
wiring `W`, concrete product formation `C`, and abstract product formation `A`.
This is the static `ResourceBehavior` layer. It does not classify the premises
of the separate unified `ResourceDynamics` theorem.

All 16 subsets are classified. Universal sufficiency in this fixed family is
exactly `R && W && (C || A)`; this is not a claim that every particular refining
pair must have those premises.

| Mask (bits R,W,C,A) | Retained premises | Result |
|---|---|---|
| 00 | none | Refuted: missing R |
| 01 | R | Refuted: missing W |
| 02 | W | Refuted: missing R |
| 03 | R,W | Refuted: missing formation |
| 04 | C | Refuted: missing R |
| 05 | R,C | Refuted: missing W |
| 06 | W,C | Refuted: missing R |
| 07 | R,W,C | Sufficient |
| 08 | A | Refuted: missing R |
| 09 | R,A | Refuted: missing W |
| 10 | W,A | Refuted: missing R |
| 11 | R,W,A | Sufficient |
| 12 | C,A | Refuted: missing R |
| 13 | R,C,A | Refuted: missing W |
| 14 | W,C,A | Refuted: missing R |
| 15 | R,W,C,A | Sufficient |

The original theorem supplies a known-result calibration. The additive
`liftWithVisibleConnections` construction isolates the connected-left
visibility actually used by its proof. Concrete formation supplies that fact
directly; abstract formation supplies it through equivalent wiring and hidden
action preservation. No existing theorem, decoder requirement, formation
policy, source syntax, or axiom allowance is changed.

For missing R, the concrete synchronization has no corresponding abstract
synchronization, although W/C/A hold. For missing W, the existing disconnected
wiring control retains R/C/A. For missing C and A together, a connected hidden
concrete step can stutter under the component refinement but becomes a visible
product synchronization with no abstract match. Each refutation rules out
**every** composite state map. Both products have admitted initial states and
an enabled initial step. In the new fixtures the abstract left component is
disabled, but its product has an isolated peer `tick`; this is initial product
enabledness, not per-component enabledness or deadlock freedom.

The module, all 16 cases, and the separate enabledness witnesses are imported
by `NMLT`, explicitly probed in the axiom audit, and included in its independent
export. The probes are necessary because an enabledness fact need not occur
in the dependency closure of a negative theorem.

`make r4-discovery` reproduces three descriptive controls: deterministic
classification of 16 masks, the original four-premise theorem applied with
ordinary Lean tactics (1/16 successful applications), and 18 recorded developer
candidates (16 classification proofs plus two invalid terms). For 13 masks
the recorded proof targets a negation while the baseline attempts positive
sufficiency. These are **not matched automation-performance measurements**.
The proofs are exposed, and there are no live model calls.

## Worker-protocol search

The [frozen contract](../examples/discovery/contract.json) binds an independently
enumerated input/environment/required-outcome inventory. One or two generations,
three typed input outcomes, three final cancellation modes, and all delivery
words of length 0–3 give 441 cases. Old generations and duplicates can appear
even when a candidate would prefer to remove their actions.

[laboratory.py](../tools/experiments/laboratory.py) checks all 64 policies against
all 441 cases before exposing a score: 28,224 candidate/case checks. Observable
requirements include exact successful values or typed failure, accepted
generation, at-most-once completion, ignored responses, dispatch charges,
cancellation, and enabled work when pending. Dropping all work and replacing
success with cheap failure are explicit rejected candidates.

The six switches are handoff fusion, accepting stale generations, accepting
duplicate completions, ignoring cancellation, dropping work, and immediate
cheap failure. Only policies 0 and 1 satisfy every case. Policy 0 executes
`reserve` then `publish` for each dispatch; policy 1 executes `reserve_publish`.
Both mutate the modeled reservation/publication sets, and scores count their
executed bookkeeping traces: 1,398 versus 699 setup transitions.

This improvement is restricted to a model whose environment begins delivery
**after setup**, with no interleaving between reservation and publication.
The operation cost model is declared, not measured CPU or host work. The
production lifecycle is unchanged, and no refinement from these Python states
to the host implementation is claimed. The search checks all policies in this
declared family, not every possible worker protocol. Bounded pending/readiness
checks do not establish fairness or unbounded completion.

## Finite constructions and prior art

The exact graph predicate is simple, undirected, connected by bounded finite
reachability, triangle-free, and degree two. The candidate family consists of
one-step circulants: `a != b` and either `a + step = b` or `b + step = a` modulo
the size. It does not enumerate all graphs. Sizes 4 and 5 are development
inputs; 6 and 7 are size holdouts. All are developer-exposed, not a blind test.

There are 18 candidate rows, 14 accepted rows, seven distinct labeled adjacency
matrices, and four isomorphism classes. Steps `k` and `size-k` may describe the
same matrix. Every accepted row has an explicit edge-preserving and reflecting
permutation `i ↦ i*k mod size` to the cycle. Thus rediscovery is counted, not
presented as novelty. The Lean
[finite object theorem](../mechanization/lean/NMLT/Discovery/FiniteConstructions.lean)
checks every row and permutation by kernel reduction. Python/Lean parity is
a separate executable check; it is not verified translation. No theorem about
all sizes is asserted.

Prior-art check, 2026-10-01: the primary Mathlib documentation already defines
[circulant graphs and their cycle instance](https://leanprover-community.github.io/mathlib4_docs/Mathlib/Combinatorics/SimpleGraph/Circulant.html#SimpleGraph.cycleGraph_eq_circulantGraph).
Our explicit isomorphisms are sufficient to reject a novelty claim for these
objects. This scoped lookup is not an exhaustive literature survey. External mathematical usefulness/novelty assessment has not been obtained.
It is optional feedback. The required solo R4 gate instead needs an executed
second use and a written local value/limits assessment; see [solo validation](solo-validation.md).

## Reproduction and remaining gates

The [nested reuse example](../mechanization/lean/NMLT/Discovery/NestedReuse.lean)
applies `liftWithVisibleConnections` at two nested boundaries, exhibits an
initialized synchronized step, and compares declared work 3 with bound 4.
It refutes an undersized bound and proves that the outer product fails the
existing direction-formation policy. This is a second static mathematical
question, not a source-admissible product, dynamic authority result or novelty
claim. `make r5-solo` executes its controls alongside changed-context native
proof and recovery workflows; full independent checking remains required.

Run `make experiment-tests`, `make metatheory`, `make r4-discovery`, and
`make nanoda`; `make reproduce` includes them and the R5 checks. Each execution
keeps a fresh ignored directory under `target/`, source/tool identities,
candidate diagnostics, bounds, and actual denominators. Existing evidence is
not overwritten. Final-tree results are recorded separately in the
[continuation verification record](r4-r5-verification.md).

Under [CONTRIBUTING](../CONTRIBUTING.md), these additive research declarations,
fixtures, and harnesses require no new language/trust RFC. Experimental JSON
is not accepted by the production proof checkers. Separate code/proof critique is engineering evidence, not independent human
assessment. Current solo and publication gates follow [Plan.md](../Plan.md);
external mathematical/user/cross-model feedback is optional, while kernel,
independent checker and negative-control requirements remain in force.

# Design principles

## 1. Behavior before implementation

Describe allowed state changes and observations before choosing an execution
strategy. Imperative or process syntax is elaborated into the same behavioral
core rather than defining a second semantics.

## 2. State-based reasoning with behavioral semantics

Use states and actions to construct and check invariants. Interpret complete
systems as behaviors so safety, liveness, fairness, refinement, hiding, and
stuttering remain first-class.

## 3. One semantic core, multiple views

Mathematical definitions, executable models, process notation, proof
obligations, model-based tests, and monitors must elaborate through a shared
typed intermediate representation.

## 4. Evidence is typed

Different verification methods justify different claim classes. A bounded
model check never masquerades as an unbounded proof. A parser pass never
masquerades as semantic validation.

## 5. Unknown is safe

Failure to decide is not success. Ambiguous external effects produce
`indeterminate`, not automatic retry. Unknown or blocked assurance dimensions
prevent promotion when they are required.

## 6. Counterexamples are values

State traces, assignments, failing goals, event histories, and violated grades
are machine-readable language values with stable schemas.

## 7. Resources and authority are explicit

Effects, capabilities, trust, privacy exposure, calls, time, and money should be
tracked with linear or graded structure where feasible. Authority may not widen
silently through composition.

## 8. Refinement connects models to reality

An implementation claim identifies observations, hidden state, stuttering,
environment assumptions, and a concrete trace or simulation mapping.

## 9. Semantic strength is challenged

A specification must reject meaningful non-equivalent mutants. Vacuous or
weakened properties are failures even when a solver reports success.

## 10. The trusted core stays small

Parsers, elaborators, solvers, tactics, code generators, and AI assistants may
be complex. Positive proof claims should end in a small checker or a precisely
scoped external trust statement.

## 11. Research features are quarantined

Experimental type formers, solvers, and semantics live behind explicit feature
stages. They enter the stable core only with metatheory, implementation,
negative controls, and representative benchmarks.

## 12. Human meaning remains a boundary

No formal system can mechanically prove that a formalization captures an
unstated human intention. NMLT should make the intent boundary reviewable and
challengeable, not claim to eliminate it.

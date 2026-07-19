# Research method

The NMLT research program uses evidence-directed development for the flagship
language, its mathematical and evidence languages, and programs written in
them.

## Cycle

1. State an intent capsule with examples and non-goals.
2. Write an obvious reference definition before optimizing it.
3. Identify the claim class and cheapest applicable sound oracle.
4. Construct semantic mutants and negative controls.
5. Run the checker and retain its structured witness.
6. Repair the smallest failed action, lemma, mapping, or precondition.
7. Replay independently from exact source and tool identities.
8. Connect the accepted model to implementation traces.
9. Publish assumptions and residual gaps with the result.

## Research claim levels

- **Hypothesis:** plausible direction without implemented evidence.
- **Prototype:** implementation demonstrates feasibility on selected examples.
- **Validated slice:** preregistered properties and negative controls pass on a
  frozen benchmark.
- **Replicated result:** an independent implementation or review reproduces the
  claim.
- **Stable feature:** metatheory, implementation, compatibility, and user
  evidence support inclusion in the stable language.

## Specification strength

A property that accepts both a correct implementation and meaningful
non-equivalent mutants is too weak for its intended role. Each benchmark should
include changes such as removed actions, weakened guards, widened constants,
corrupt bindings, hidden replay, and altered observations.

## AI participation

AI systems may search the design space and localize repairs. Evaluation must
distinguish syntax, type correctness, bounded semantic correctness, proof, and
human intent agreement. No AI-authored assertion is trusted because of its
origin or rhetorical confidence.

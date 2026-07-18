# Related systems and negative space

This matrix identifies lessons and boundaries, not winners. It should be
reviewed before public claims because tools evolve.

| System | Primary strength | NMLT lesson | NMLT must not merely duplicate |
|---|---|---|---|
| TLA+ / TLC / TLAPS | Mathematical behavioral specification, explicit-state checking, temporal proof | Preserve actions, behaviors, safety/liveness, fairness, stuttering, and refinement | TLA+ with alternate punctuation |
| Quint | Typed, executable, approachable TLA+-style specifications with model-checker integration | Treat usability, execution, traces, and tooling as core product concerns | A typed executable TLA+ frontend |
| P | Communicating state-machine programming, systematic testing, monitors, runtime observation | Keep executable process views close to implementations and preserve safety plus liveness | An actor language with a checker |
| Apalache | Symbolic analysis of TLA+ through SMT | Separate typed frontend semantics from multiple exploration engines | A single SMT translation pipeline |
| Lean | Small proof kernel, dependent types, metaprogrammable automation | Mechanize the calculus and keep proof acceptance small | A general theorem prover with temporal libraries |
| Dafny / F* / Verus | Verification-aware programming with contracts and SMT-backed proof | Make function contracts and executable code practical | Another pre/postcondition language |
| Granule | Graded modal, linear, and indexed reasoning about resources and effects | Treat cost, authority, use, and privacy as algebraic grades | A graded functional language without behavioral semantics |
| KeYmaera X | Hybrid programs and differential dynamic logic | Add continuous dynamics only through a principled extension | A hybrid-systems-specific prover |

## Proposed negative space

NMLT aims to combine behavior types, temporal action semantics, graded linear
authority, compositional open systems, proof-relevant refinement, typed
evidence, semantic mutation, and runtime trace conformance in one semantic
architecture.

No novelty claim is established merely because this combination is written
down. The claim becomes credible only after the calculus, vertical slice, and
comparative benchmarks exist.

## Primary references

- TLA+: <https://lamport.azurewebsites.net/tla/tla.html>
- Quint: <https://quint.sh/>
- P: <https://p-org.github.io/P/>
- Apalache: <https://apalache-mc.org/>
- Lean: <https://lean-lang.org/>
- Dafny: <https://dafny.org/>
- Granule: <https://granule-project.github.io/>
- KeYmaera X: <https://keymaerax.org/>

# Glossary

**NMLT research program**
New Mathematics, Languages, and Techniques: the umbrella project investigating
mathematical foundations, formal languages, and evidence-directed methods for
trustworthy computation.

**NMLT language**
The program's first flagship language: a behavior-first, evidence-carrying
programming language. After first mention, “NMLT” may name the language where
the technical context is unambiguous.

**Action**
A relation between pre-state and post-state, potentially indexed by inputs,
outputs, capabilities, and resource grades.

**Assumption boundary**
The conditions and trusted components outside which an evidence claim does not
apply.

**Behavior**
A finite or infinite state sequence satisfying initialization and transition
rules, including permitted stuttering.

**Capability**
Typed authority to perform an operation. It may be affine, linear, scoped, or
quantitatively graded.

**Counterexample**
A structured witness refuting a claim, such as a state trace, assignment,
failing proof goal, or effect history.

**Evidence**
A typed artifact connecting a claim to a verification method, scope,
assumptions, result, and supporting certificate or witness.

**Grade**
An algebraic annotation describing resource or effect information such as use
count, cost, latency, privacy exposure, or trust provenance.

**Indeterminate**
A result used when an effect or verification may have occurred but terminal
evidence is insufficient. It is distinct from unknown and failure.

**Observation**
The externally meaningful projection of internal state or events.

**Refinement**
A justified relation showing that one behavior implements another under an
observation mapping, assumptions, hiding, and stuttering policy.

**Safety**
A property refutable by a finite bad prefix.

**Liveness**
A property requiring eventual progress and not generally refutable by a finite
prefix alone.

**Semantic mutant**
An intentionally non-equivalent variant used to test whether a specification
is strong enough to reject meaningful defects.

**Stuttering**
A step that leaves the selected observations unchanged. Stuttering invariance
allows implementation detail to vary without changing abstract behavior.

**Trusted computing base**
The components whose correctness must be trusted for an evidence claim to hold.

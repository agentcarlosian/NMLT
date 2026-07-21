# Inspiration

AI-assisted development can produce a plausible change faster than a reviewer
can explore all of its behavior. A stateful migration may compile, pass
ordinary tests, and still lose one authorization or sequencing guard.

NMLT explores a narrower, inspectable approach. A developer manually abstracts
the critical workflow into a finite behavior model. NMLT then exhausts its
reachable state space within explicit bounds and returns either a bounded result
or the exact transition that violated a property.

# What it does

NMLT is a pre-alpha behavior-modeling language and deterministic verification
CLI for developers reviewing agentic workflows, stateful backends, distributed
protocols, and safety-sensitive changes.

The Build Week judge demo uses one concrete migration-review story. A C provider
dispatch function requires the attempt to be authorized and armed. A proposed
Rust port drops the armed guard. The snippets make the review problem concrete,
and their relevant behavior is manually represented by two NMLT models that
differ by exactly one requirement.

For the preserved workflow, NMLT completes finite state-space exploration and
reports four properties as model_checked. For the dropped-guard model, it
reports refuted and returns a structured witness: the attempt begins unarmed,
is authorized, and then dispatches while armed is still false.

The same command saves a deterministic report, changes the exact NMLT model
bytes, and attempts readback. The source binding changes, so the demo refuses
to apply the prior result as current evidence.

NMLT does not currently parse C or Rust, translate between them, prove native
memory safety, or establish source-code equivalence. Fidelity between the source
snippets and the manually authored model is a review obligation. model_checked
means complete exploration of the reported finite model within its displayed
bounds, not an unbounded theorem about native code.

# How we built it

I built NMLT with Sol, GPT-5.6 running inside the OpenAI Codex CLI. I supplied
the product thesis, behavioral constraints, architecture, semantic decisions,
trust assumptions, and assurance ceilings. Codex accelerated implementation,
debugging, recovery, and documentation across Rust, Python, Lean, TLA+, Quint,
and P.

The runnable path is a Rust CLI with a lossless parser, recovering syntax tree,
resolver, typed HIR, explicit core, deterministic finite-state exploration,
structured witnesses, and source-bound semantic identities. Python and JSON
Schema checkers exercise persisted evidence and adversarial controls. Separate
Lean work studies a narrow Rust validation kernel and explicitly records the
remaining correspondence boundary.

Codex output was treated as proposed work rather than trusted truth.
Deterministic checkers, regression tests, and seeded negative controls decided
what was accepted. NMLT itself has no runtime LLM dependency.

# Challenges we ran into

The hardest problem was claim integrity across layers. A finite model result
must expose its bounds. A generated Lean representation cannot inherit a native
Rust claim without a checked correspondence path. Evidence from one source or
checker configuration must not silently apply after those inputs change.

The C-to-Rust vignette required the same discipline. It is a useful developer
story only when presented as a manual behavioral abstraction, not as automatic
translation or equivalence verification.

# Accomplishments that we are proud of

The no-build release gives judges one command and three visible outcomes:

- A preserved workflow with complete bounded exploration
- A one-line dropped-guard model refuted with an exact counterexample
- An earlier report rejected after its bound model source changes

That is NMLT's core product loop: state the behavior, check the claim, inspect
the witness, and keep the result tied to what was actually checked.

The broader repository also contains deterministic evidence manifests,
independent readback scripts, seeded defects, open-system composition research,
comparison models, and a separate Lean metatheory path. Those layers support
the research direction, but the judge demo does not require them.

# What we learned

A structured counterexample is more actionable than a generic failing test.
An honest bounded result is more useful than an overstated proof claim. AI
assistance is strongest when generated work must clear deterministic,
replayable checks and when negative controls remain visible.

We also learned that the product experience matters as much as the underlying
research. A judge should reach the useful result in one command before being
asked to understand the architecture.

# What is next for NMLT

Future work includes verified source-to-model adapters, broader
source-to-certificate correspondence, richer agent and CI integrations, and
additional finite and infinite-state techniques.

Automatic C-to-Rust translation and source-equivalence proof are not current
features. They are possible future integrations on top of the behavior-checking
foundation demonstrated here.

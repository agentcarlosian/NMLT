# Devpost Submission Text

## Project tagline

Model finite software behavior, check bounded safety properties, and get
source-bound results with structured counterexamples when a property fails.

## Inspiration

Software claims can outrun their evidence, especially when an AI system can
produce code faster than a person can review its behavior. Tests show selected
examples; they do not necessarily expose unsafe reachable states or explain the
exact transition that violated an invariant.

NMLT explores a behavior-first alternative. A developer states a finite system
and its properties, then deterministic tools either check the reachable state
space within explicit bounds or return a reproducible counterexample. The goal
is not to trust an AI-generated assertion. The goal is to make a concrete claim
inspectable, bounded, and independently replayable.

## What it does

NMLT is a pre-alpha research language, CLI, and verification laboratory for
formal-methods researchers and developers evaluating safety-sensitive workflow
or agent-system behavior.

Today, a user can:

- parse and inspect lossless `.nmlt` syntax;
- resolve and type-check the supported executable core;
- exhaust a finite reachable state space within declared limits;
- receive JSON results and step-by-step counterexample witnesses;
- replay claim-specific, source-bound evidence experiments; and
- compare frozen provider-attempt models in NMLT, TLA+, Quint, and P.

The submission demo contrasts a reference provider workflow with a one-line
seeded defect. The reference completes bounded exploration with its properties
reported as `model_checked`. The mutant is `refuted` and includes a witness in
which `dispatch` occurs while authorization is false.

`model_checked` is a bounded result, not an unbounded theorem. The structural
`evidence` command reports `unknown`, and NMLT does not claim to be a
general-purpose verified programming language.

## How we built it

The Rust workspace contains the lossless frontend, resolver, typed HIR,
explicit core, deterministic breadth-first model checker, evidence tooling, and
CLI. Python and JSON Schema checks validate frozen identities, manifests,
negative controls, and readback paths.

A separate Lean 4 package contains mechanized, claim-specific results and a
bounded Rust validation path translated with pinned Charon/Aeneas. Active work
is still closing the correspondence between generated structural equality and
native Lean equality layer by layer. Rich source-to-certificate encoding and
general source correspondence remain outside verified extraction.

The repository also carries comparison models in TLA+, Quint, and P. `make ci`
runs the Rust workspace, evidence checks, and the pinned Quint typecheck. TLC
runs only when `TLA2TOOLS_JAR` is supplied, and P runs only when P 3.1.0 is
installed. Lean is a separate GitHub Actions job and is included in the local
`make reproduce` gate. The corrected P model remains explicitly unvalidated
when P is unavailable.

GPT-5.6, running as Sol inside the Codex CLI, accelerated implementation,
cross-language recovery, debugging, negative-control construction, and
documentation across Rust, Lean, Python, TLA+, Quint, and P. The human author
directed the architecture, semantics, trust boundaries, and which proof claims
were acceptable. NMLT has no runtime LLM dependency; judges run deterministic
local tools.

## Challenges we ran into

- Keeping state exploration finite and making every bounded result state its
  limits.
- Connecting a translated Rust execution path to Lean without treating
  generated structural equality as native equality before that implication is
  proved.
- Keeping results from different formal systems separate unless a checked
  composition path justifies combining them.
- Designing negative controls that reject stale, forged, mismatched, or
  assurance-laundered artifacts.

## Accomplishments that we are proud of

- An end-to-end supported slice from `.nmlt` source through typed core to a
  deterministic bounded result.
- Structured counterexamples that show the exact transition and state that
  refuted a property.
- Claim-specific source bindings and adversarial controls that fail closed on
  the mismatches they are designed to detect.
- A public CI workflow split into a Rust/evidence job and a separate Lean
  metatheory job, without representing optional TLC or P execution as complete.
- Explicit `unknown`, `indeterminate`, and bounded-result semantics instead of
  promoting incomplete evidence to proof.

## What we learned

- AI assistance is most useful when its output must pass deterministic,
  independently replayable checks.
- Counterexamples communicate a failed safety claim more effectively than a
  generic test failure.
- Formal evidence needs a stated trust boundary and result ceiling, not just a
  green badge.
- Cross-language verification is only as strong as the checked correspondence
  between the languages.

## What is next for NMLT

- Complete the remaining generated-to-native Lean equality layers.
- Extend verified source-to-certificate correspondence beyond the current
  bounded slices.
- Package the CLI for easier evaluation and integrate bounded checking into
  developer and agent workflows.
- Research fairness transport, infinite-state techniques, signatures, and
  runtime attestation without weakening the current result boundaries.

## Built with

Rust, Lean 4, Python, JSON Schema, TLA+, Quint, P, Charon, Aeneas, GNU Make,
and the OpenAI Codex CLI with GPT-5.6.

## Links

- Demo: https://www.youtube.com/watch?v=-PbhZ9me46Y
- Repository: https://github.com/agentcarlosian/NMLT
- Submission release: https://github.com/agentcarlosian/NMLT/releases/tag/build-week-submission-2026
- Judge instructions: `docs/devpost_judge_instructions.md`


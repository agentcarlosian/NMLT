# Authority-bounded repair benchmark

This pilot evaluates whether structured feedback plus localized edit authority
can improve completion without giving a repair assistant authority over the
claim. It contains one held-out syntax task, one held-out type task, and one
held-out semantic task. All three baseline candidates fail their target stage;
the deterministic protocol assistant completes all three in one feedback round
while all frozen identities and negative controls remain intact.

That `0/3 -> 3/3` result demonstrates this tiny deterministic workflow only. It
is not an LLM result, a statistical generalization claim, or evidence that the
NMLT language is sound.

## Isolation and authority

The assistant interface exposes exactly four things: task ID, candidate source,
editable spans, and one structured checker result. Trusted intent, property,
oracle implementation, expected outcome, and expected patch are withheld. No
gold or expected patch is stored anywhere in the suite; the evaluator checks
the repaired semantics instead of comparing text.

The evaluator:

1. freezes intent, property, and oracle bytes by SHA-256;
2. checks the baseline and emits typed feedback;
3. passes only the bounded interface to the deterministic assistant;
4. rejects non-candidate paths, protected spans, whole-file replacements, and
   forged result claims;
5. applies a localized proposal in memory;
6. reruns syntax, typing, semantics, and a task-specific negative control;
7. verifies trusted identities and serializes the complete artifact graph;
8. reads the graph back and compares it structurally.

The semantic task deliberately includes a reachable authorization transition.
A guard that merely makes dispatch unreachable therefore fails the
anti-vacuity control. Deleting the repaired guard must also recreate the
counterexample.

## Published result

[`evaluation.json`](evaluation.json) reports:

| Measure | Unassisted | Assisted |
| --- | ---: | ---: |
| Target-stage completion | 0/3 | 3/3 |
| Syntax task | 0/1 | 1/1 |
| Type task | 0/1 | 1/1 |
| Semantic task | 0/1 | 1/1 |

The same run rejects 21/21 integrity probes, retains and kills 3/3 negative
controls, and promotes 0 unknown or conflict results. The 21 probes are seven
classes applied to each task: property weakening, oracle editing, path
traversal, symlink-like traversal syntax, whole-file replacement, forged
result, and dropped control identity.

[`suite.json`](suite.json) binds all candidate and trusted digests. The JSON
schemas are `schemas/agentic-task-suite-v1.schema.json`,
`schemas/agentic-evaluation-v1.schema.json`, and
`schemas/agentic-artifact-graph-v1.schema.json`.

## Search-the-archives research record

Search date: 2026-07-18 CDT (collector timestamps 2026-07-19 UTC). Queries
covered verifier-guided repair, specification gaming, counterexample-guided
repair, and semantic mutation testing.

### Surfaced in the local archive

- [AxDafny: Agentic Verified Code Generation in
  Dafny](https://arxiv.org/abs/2606.32007) was the strongest formal-verification
  match. It motivates measuring executable and proof-artifact completion, but
  it does not establish NMLT's authority protocol.
- [To Run or Not to Run: Analyzing the Cost-Effectiveness of Code Execution in
  LLM-Based Program Repair](https://arxiv.org/abs/2606.26978) studies the
  generate-run-revise pattern. NMLT keeps re-execution, but places a digest and
  edit-policy gate before it.
- [Form, Not Content? A Preregistered, Placebo-Controlled Evaluation of Learned
  Error-Conditioned Self-Repair](https://arxiv.org/abs/2607.12962) reinforces
  the need for explicit baselines and controls when attributing gains to
  feedback.

These were adjacent results from lexical retrieval. The archive surfaced no
close match for a repair interface that simultaneously withholds trusted
claims, rejects result forgery, preserves controls, and fails closed on unknown
or conflicting evidence. That absence is not a novelty claim.

### New/current leads

- [Specification-Guided Repair of Arithmetic Errors in Dafny Programs using
  LLMs](https://arxiv.org/abs/2507.03659) supports verifier-local feedback and
  isolated rechecking. NMLT makes the assumed specification boundary an
  explicit immutable role.
- [Property-Based Mutation Testing](https://arxiv.org/abs/2301.13615) motivates
  property-linked mutation operators and separate treatment of malformed,
  equivalent, killed, and out-of-scope mutants.
- [A Case Study of LLM for Automated Vulnerability
  Repair](https://arxiv.org/abs/2405.15690) supports iterative validation while
  underscoring that plausible output is not checked evidence.
- [Verified VCG and Verified Compiler for
  Dafny](https://arxiv.org/abs/2512.05262) motivates keeping the assistant and
  unverified checker plumbing outside the trusted kernel.

The live arXiv endpoint returned HTTP 429 and timeout errors during several
collector queries. The RFC-linked primary pages above were therefore retained
as current leads; the benchmark makes no completeness claim about the live
literature search.

### Design implications

- Feedback may focus search but cannot widen authority.
- Mutation score is indexed by a frozen property and named operator.
- A repaired parse cannot promote a remaining semantic counterexample.
- Unknown and backend conflict are terminal non-success states for repair.
- Every claimed gain is paired with a no-feedback baseline, integrity probes,
  and anti-vacuity controls.

## Schema and integration notes

The evaluation schema references the artifact-graph schema externally by its
relative URI. Offline validators must register
`agentic-artifact-graph-v1.schema.json` under its `$id` before validating the
evaluation; they must not fetch `nmlt.dev`. The corpus was validated with a
Draft 2020-12 validator and a local `$id` registry. Rust tests independently
bind every fixture digest into both `suite.json` and the checked-in evaluation
graph, then require byte-for-byte reproduction of `evaluation.json`.

This crate covers the RFC 0011 repair/authority workflow only. Runtime monitor
and drift-event production remains in the temporal/runtime layer. Root
integration must connect those monitor result identities to exact model and
implementation identities in the artifact graph; no drift evidence is claimed
by this pilot.

# Research note: source-to-typed-core integration and project identity

- Search date: 2026-07-19
- Researcher: Carlosian <carlosian@agentmail.to>, with AI-assisted retrieval
  and synthesis
- Method: local JSON research archive, current arXiv/Hugging Face paper search,
  primary paper pages, official tool documentation, and a preliminary exact-name
  collision search
- Archive queries: `verified compiler`, `type elaboration`, `temporal types`,
  and `proof carrying code`

## Questions

1. What should NMLT require of a trustworthy surface-to-typed-core boundary?
2. Which deeper mathematical directions should follow that integration work?
3. Does “New Mathematics, Languages, and Techniques” accurately fit NMLT?
4. Should a Latin expansion replace the English name or serve another role?

## Retrieval limits

The local archive search is lexical over previously collected metadata and
notes. It surfaced useful adjacent work but did not return a close match for
bidirectional elaboration or temporal type theory under the short queries.
That absence is a retrieval fact, not evidence that NMLT is novel. Current
paper search was therefore used to fill the identified gaps. Paper inclusion
below means “relevant design evidence,” not endorsement or independent
replication.

## In the local archive

### Verifier-guided generation and repair

[AxDafny: Agentic Verified Code Generation in Dafny](https://arxiv.org/abs/2606.32007)
was the strongest directly useful archive hit. It separates executable code
from proof artifacts and uses verifier feedback to iteratively generate
implementations, invariants, assertions, and termination arguments. It also
reports verification and runtime tests as different measurements. For NMLT,
the important architectural lesson is that generated work remains a proposal;
acceptance comes from a named checker over an exact artifact. This supports the
existing authority-bounded repair direction, but does not solve source-to-core
correctness.

### Compiler feedback during generation

[Generative Compilation: On-the-Fly Compiler Feedback as AI Generates
Code](https://arxiv.org/abs/2607.13921) was also present in the archive. It
turns partial programs into diagnosable complete programs, proves key
completion properties for a core calculus in Lean, and uses early compiler
feedback to avoid dead ends during generation. The NMLT implication is to make
stable, stage-specific diagnostics available from projection, resolution,
elaboration, and kernel checking. It does not justify accepting a recovered or
sealed program as semantically equivalent to the user's incomplete source.

The archive did not surface a close, direct treatment of NMLT's specific
surface-to-behavior-core contract. That became the target of current search.

## New/current leads

### Correct-by-construction elaboration

[Bidirectional Elaborators à la Carte](https://arxiv.org/abs/2607.09564)
describes elaboration as translation from implicit surface syntax to explicit
core syntax and presents a dependently typed DSL in which the elaboration
cannot produce ill-typed terms and is stable under substitution. NMLT should
not copy its much richer dependent setting wholesale. The actionable ideas are
to make synthesis/checking judgments explicit, expose dependencies on
conversion, and treat the elaboration derivation as a first-class artifact.

For M9 this leads to bidirectional judgments of the form:

```text
Gamma; Sigma; Delta; B |- e => A ~> t ; D
Gamma; Sigma; Delta; B |- e <= A ~> t ; D
```

where `B` is the behavior/system index, `t` is explicit core, and `D` is a
checkable derivation. The elaborator's boolean “success” is not the evidence.

### Verified source semantics, VCG, and compiler

[Verified VCG and Verified Compiler for Dafny](https://arxiv.org/abs/2512.05262)
starts from a functional big-step semantics for a meaningful Dafny subset and
mechanizes a verified VCG and compiler to CakeML in HOL4. Its relevance is
structural: a compiler-correctness claim needs explicit source semantics,
explicit target semantics, and a preservation theorem across the translation.
NMLT should first complete one narrow vertical slice rather than attach proof
language to a pipeline that silently skips unsupported constructs.

### Trusted-computing-base realism

[The Trusted Computing Base of the CompCert Verified
Compiler](https://arxiv.org/abs/2201.10280) catalogues loopholes that can remain
around a verified compiler, including source/target modeling and external
algorithms. This prevents a common overclaim: a Lean theorem about a small core
does not establish that the Rust parser, source-set resolver, elaborator,
runtime representation, or backend invocation implements the same subject.
M9 evidence must name those boundaries and reduce trust only when a receiver
can independently check the relevant artifact.

### Receiver-checkable evidence

[A Proof Carrying Code Framework for Inlined Reference Monitors in Java
Bytecode](https://arxiv.org/abs/1012.2995) shows a receiver supplying a trusted
ghost monitor and checking verification conditions attached to delivered
code. The direct NMLT analogy is not bytecode monitoring; it is the separation
between an untrusted elaborator and a smaller receiver-side kernel that checks
the exact HIR/core/ruleset derivation. The checker must bind the program and
policy rather than accept an unattached proof status.

### Behavior types and temporal mathematics

[Temporal Type Theory: A topos-theoretic approach to systems and
behavior](https://arxiv.org/abs/1710.10258) explicitly uses behavior types,
embeds LTL and MTL, and gives a sheaf/topos semantics broad enough for hybrid
dynamical systems. It is directly aligned with NMLT's aspiration, but its own
axiomatic and semantic commitments are much larger than the current kernel.
It should be treated as a comparison and research source after the M9 semantic
spine exists—not as a bag of axioms to import into pre-alpha typing rules.

[Parallel Complexity Analysis with Temporal Session
Types](https://arxiv.org/abs/1804.06013) combines linear-logic-based session
types with next, always, and eventually modalities, then establishes progress
and preservation over timed multiset rewriting. It demonstrates that temporal
modalities can support compositional, local quantitative reasoning when the
operational semantics and metatheorems are precise. For NMLT, this supports a
post-M9 program investigating behavior-indexed temporal types together with
affine capabilities and grades; it does not establish that these systems can
simply be merged.

## Synthesis for M9

The literature supports the following sequence:

1. **Close the semantic gap first.** Exact source membership, complete surface
   projection, deterministic resolution, and explicit core are prerequisites
   for attaching meaningful proofs to the flagship language.
2. **Use an independently checked derivation.** The elaborator may remain a
   convenient producer, while a smaller kernel validates the exact core,
   contexts, and ruleset before engines see `CheckedProgram`.
3. **Require two-way action correspondence.** A one-way type-preservation
   result is insufficient for behavior: emitted core must neither lose
   permitted source steps nor introduce new ones in the supported fragment.
4. **Keep evidence identity transitive and exact.** A model result must bind
   source set, HIR, core, certificate, kernel, engine, and configuration. A
   valid proof for different imports or core bytes is stale.
5. **Preserve explicit uncertainty.** Unsupported syntax and incomplete
   certificates fail with stage-specific diagnostics; they do not disappear
   or become an open symbol.

This is captured in [RFC 0013](../../rfcs/0013-source-to-typed-core.md) and the
M9 section of [Plan.md](../../Plan.md).

## Deeper mathematics and verification after M9

Once NMLT has one trustworthy semantic subject, the next research program is:

1. behavior-indexed temporal propositions with explicit finite/infinite trace,
   stuttering, fairness, and observation semantics;
2. proof-relevant refinement whose evidence records maps, hidden state,
   assumptions, and correspondence witnesses;
3. compositional open-system rules joining assumptions, guarantees, authority,
   and observations;
4. affine/linear capability protocols combined with temporal modalities;
5. quantitative grade algebras whose source annotations, operational costs,
   and Lean model are connected rather than merely manually aligned;
6. verified lowering from checked core to transition graphs and verification
   conditions, with independently checked backend certificates;
7. only then, guarded experiments in cubical equality, hybrid dynamics, and
   probabilistic behavior, each isolated behind explicit axioms and promotion
   tests.

The intended novelty is the tested integration of these subjects in one small,
evidence-carrying semantic foundation. None of the cited ingredients is claimed
as an NMLT invention.

## Project-name decision

### English expansion

**New Mathematics, Languages, and Techniques** fits the work better than the
singular phrase. “Mathematics” covers candidate formal foundations;
“Languages” covers the flagship NMLT language and its explicit core, evidence,
observation, and experimental extension languages; “Techniques” covers
evidence-directed development, mutation, independent checking, refinement,
and runtime conformance. The plurals also prevent one surface syntax or one
workflow from being mistaken for the entire program.

The necessary distinction is:

- **NMLT research program:** the umbrella;
- **NMLT language:** the first flagship product.

This decision is recorded in
[ADR 0003](../decisions/0003-project-identity.md).

### Latin candidate

*Nova Mathematica Lingua Testimonii* is a plausible acronym-preserving epithet,
approximately “a new mathematical language of evidence.” `nova` and
`mathematica` modify feminine singular `lingua`; `testimonii` is a genitive of
`testimonium`, a word used for evidence, testimony, or witness. The lexical
anchors are Lewis and Short's entry for
[`lingua`](https://classics.andrewgadsden.com/lewisandshort/entry/n26692) and
the Computational Historical Semantics entry for
[`testimonium`](https://cld.bbaw.de/lemma/lat/testimonium).

It is better as an epithet or component codename than as the official program
expansion: it describes one language and omits techniques; *testimonium* can
also carry legal or religious connotations. `Testimonium` is a particularly
strong future candidate for the evidence/certificate kernel, subject to a
classicist review and a fuller name/trademark/package collision search.

### Preliminary collision check

Exact searches found no established programming language using the full
English expansion, the Latin phrase, or NMLT as its primary language name.
They did find unrelated expansions of the initials in industrial and
organizational contexts. This is sufficient to keep the repository name during
research, but it is not legal or trademark clearance and should be repeated
across package registries, domains, and relevant jurisdictions before a public
release.

## Decision

- Official expansion: **NMLT — New Mathematics, Languages, and Techniques**.
- Repository and flagship-language name: **NMLT**.
- Near-term technical focus: **M9 source-to-typed-core integration**.
- Next research focus: temporal/refinement/compositional mathematics and
  verified lowering, in that order.
- Latin: retained as a candidate epithet or kernel codename, not adopted as an
  alternate official expansion.

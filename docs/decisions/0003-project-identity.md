# ADR 0003: NMLT project identity

- Status: Accepted
- Date: 2026-07-19
- Decider: Carlosian <carlosian@agentmail.to>

## Context

The initials NMLT began as the repository and language name, while early prose
used the singular thesis “new mathematics, new language, and new technique.”
The work has since separated several coordinated artifacts: a human-facing
language, explicit core and evidence languages, mathematical models, and
development and verification methods. The project needs one stable expansion
that describes this scope without renaming technical identifiers or implying
that the current prototype is a complete verified language.

## Decision

The official expanded name is **NMLT — New Mathematics, Languages, and
Techniques**.

NMLT is the umbrella research program. Its first flagship programming language
is called the **NMLT language**. After a qualified first mention, “NMLT” may
refer to the language in a clearly technical context.

The canonical thesis is:

> To truly progress, humanity needs new mathematics, new languages, and new
> techniques.

Repository, package, schema, evidence-domain, Lean-namespace, file-extension,
and CLI identifiers remain unchanged. This decision changes public identity
and prose, not source or artifact identity algorithms.

## Consequences

- Project introductions distinguish the research program from its flagship
  language.
- “Languages” includes surface, core, evidence, observation, and experimental
  extension languages only where they are governed by explicit semantic
  connections; it is not permission to accumulate unrelated syntaxes.
- “Techniques” names a family of evidence-directed practices rather than a
  claim that a single workflow solves trustworthy programming.
- “New mathematics” remains a research thesis. Novelty claims require
  comparison, formalization, mechanization, and evidence.
- Existing `.nmlt`, `nmlt-*`, `NMLT.*`, and evidence identity domains remain
  stable.

## Alternatives considered

### Singular English expansion

“New Mathematics, Language, and Technique” understates the umbrella scope and
blurs the difference between the program, the flagship language, and the
methods and formal artifact languages it develops.

### Latin expansion

*Nova Mathematica Lingua Testimonii* is a defensible candidate epithet for the
flagship language, approximately “a new mathematical language of evidence.” It
captures the evidence thesis but denotes one language and does not express the
program's techniques. It is therefore not the official expansion. A future RFC
or branding decision may use *Testimonium* as a component codename after a
collision and specialist language review.

### Renaming the repository or flagship language

A rename would create migration work without resolving a technical problem.
NMLT remains concise, already binds checked artifacts, and has no identified
programming-language collision in the preliminary search. This is not a
trademark clearance.

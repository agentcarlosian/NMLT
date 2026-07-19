# Contributing to NMLT

NMLT separates research claims, language decisions, and implementation work so
that an attractive prototype cannot silently outrun its evidence.

## Contribution license

NMLT is licensed under the Apache License, Version 2.0. Unless you explicitly
mark a communication as "Not a Contribution," any contribution intentionally
submitted for inclusion is offered under the same license, as described by
section 5 of `LICENSE`. By submitting, you represent that you have the right to
do so. The project does not require a contributor license agreement at this
stage.

## Before changing the language

Use an RFC for any change to syntax, typing, semantics, trusted components,
evidence classes, backend interpretation, or compatibility. Small corrections,
tests, documentation fixes, and implementation work under an accepted RFC do
not require a new RFC.

1. Copy `rfcs/0000-template.md` to the next available number.
2. State the problem and non-goals before proposing syntax.
3. Define static and dynamic semantics, verification consequences, failure
   modes, and migration impact.
4. Include at least one positive example and one semantic negative control.
5. Record unresolved questions instead of hiding them in implementation notes.

## Development workflow

```bash
make fmt
make ci
```

Changes should preserve these rules:

- parsing is not verification;
- bounded checking states its bounds;
- `unknown` and `indeterminate` are never promoted to success;
- counterexamples remain structured data;
- trusted specifications are not weakened merely to make an implementation
  pass;
- generated code, proof automation, and LLM output remain outside the trusted
  kernel unless independently checked.

## Pull requests

Keep each pull request focused. Describe the claim being changed, the evidence
used to evaluate it, negative controls, residual gaps, and relevant RFC or
decision record. Do not combine a semantics change with an unrelated refactor.

## Commits

Use clear imperative subjects. Generated artifacts should be reproducible and
must not be edited by hand when a checked-in source generates them.

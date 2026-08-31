# Contributing to NMLT

NMLT — New Mathematics, Languages, and Techniques — separates research claims,
language decisions, checked mathematics, and implementation work so that an
attractive prototype cannot silently outrun its definitions and validation.

## Contribution license

NMLT is licensed under the Apache License, Version 2.0. Unless you explicitly
mark a communication as "Not a Contribution," any contribution intentionally
submitted for inclusion is offered under the same license, as described by
section 5 of `LICENSE`. By submitting, you represent that you have the right to
do so. The project does not require a contributor license agreement at this
stage.

## Before changing the language

Use an RFC for any change to syntax, typing, semantics, trusted components,
artifact interpretation, or compatibility. Small corrections,
tests, documentation fixes, and implementation work under an accepted RFC do
not require a new RFC.

1. Copy `rfcs/0000-template.md` to the next available number.
2. State the problem and non-goals before proposing syntax.
3. Define static and dynamic semantics, theorem consequences, failure
   modes, and migration impact.
4. Include at least one positive example and one semantic negative control.
5. Record unresolved questions instead of hiding them in implementation notes.

## Development workflow

```bash
make fmt
make reproduce
```

`make reproduce` runs the Rust language gate and Lean metatheory gate.
Contributors changing only Rust may use `make ci` while iterating, but a
semantic or publication-ready change must pass the complete gate.

Changes should preserve these rules:

- parsing is not verification;
- Rust acceptance is not reported as Lean acceptance;
- bounded exploration always reports `assurance: none`;
- source identification is not described as verified translation;
- product-formation policy is distinguished from demonstrated theorem
  dependencies;
- trusted specifications are not weakened merely to make an implementation
  pass;
- generated code, proof automation, and LLM output remain outside the trusted
  kernel unless independently checked.

## Pull requests

Keep each pull request focused. Describe the definition or claim being changed,
the validation used to evaluate it, negative controls, residual gaps, and
relevant RFC or decision record. Do not combine a semantics change with an
unrelated refactor.

Changes to the Lean behavior core or artifact trust boundary should receive an
independent adversarial review. Publication stacks should also receive a
cross-family review before push; if that review is unavailable, record it as an
open gate rather than silently omitting it.

## Commits

Use clear imperative subjects. Generated artifacts should be reproducible and
must not be edited by hand when a checked-in source generates them.

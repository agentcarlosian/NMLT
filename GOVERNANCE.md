# Governance

NMLT is currently an early research project with a maintainer-led governance
model. This document is expected to evolve before the first public release.

## License stewardship

The project is distributed under Apache License 2.0. Maintainers must preserve
required notices and provenance, may not relicense accepted contributions
without the rights and recorded decision to do so, and must review imported
code or data for license compatibility before inclusion.

## Roles

- **Maintainers** merge changes, accept RFCs, define releases, and protect the
  trusted computing base.
- **RFC shepherds** guide a proposal through review and ensure objections and
  alternatives are recorded.
- **Contributors** propose research, implementation, documentation, examples,
  and negative controls.
- **Reviewers** may approve a scoped area after demonstrating sustained
  expertise in that area.

## Decision rules

Implementation convenience cannot by itself override semantic soundness. A
language decision should record:

1. the claim being made;
2. its mathematical or operational interpretation;
3. trusted assumptions and components;
4. known counterexamples and negative controls;
5. compatibility and migration consequences;
6. unresolved gaps.

Maintainers may accept, reject, postpone, or request revision of an RFC. A
postponed RFC remains a research lead and is not an implied commitment.

## Changes to the trusted core

Changes to type checking, proof checking, evidence classification, refinement,
or backend result interpretation require two independent reviews once the
project has more than one maintainer. Until then, they require an RFC plus a
written decision record.

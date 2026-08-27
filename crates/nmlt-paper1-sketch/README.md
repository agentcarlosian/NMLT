# `nmlt-paper1-sketch`

Adapter from `nmlt-core`'s Paper 1 boolean finite-graph *sketch* onto
`nmlt-temporal`'s `FiniteGraph` / `OpenSystem` so the existing
`HiddenConnectedAction` checker can be driven from the Paper 1 fixture.

This crate exists because `nmlt-compile` does not depend on `nmlt-temporal`
and `nmlt-temporal` must not depend on `nmlt-core`. It is a sketch fragment
plus a finite instance check. It is **not** a verified compiler and **not**
source-to-LTS in general. M9 still fail-closes full compose elaboration.

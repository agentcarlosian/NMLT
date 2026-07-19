# `nmlt-hir`

`nmlt-hir` is NMLT's deterministic M9 module and name-resolution boundary.
`project_source_module` passes exact bytes through `nmlt-core`'s lossless parser
and complete surface projection; `resolve_modules` then validates a closed
source set, acyclic direct imports, typed definition paths, source spans, and
accepted resource bounds before assigning content-derived identities.
The projected input and resolved artifact internals are opaque outside the
crate, so callers cannot edit projected metadata independently of its exact
bytes or mutate a resolved table while retaining a stale identity.

The crate deliberately fails on recovery-dependent, uncensused, unsupported,
ambiguous, shadowed, cyclic, or out-of-policy input. Its `ResolvedProgram`
parses every admitted raw type/expression, assigns owner-derived local binders,
contains a span-preserving HIR node graph, and emits a canonical
`ResolutionMap` in bijection with textual reference nodes. Construction runs a
separate candidate-replay/readback pass over exact source spellings and graph
closure. It is not a typed term representation, a proof certificate, a
`CheckedProgram`, or permission to execute source. `nmlt-ir` owns M9-004's
explicit typed core; `nmlt-elaborate` emits M9-005 derivations, and M9-006 adds
independent checking.

Anonymous observations and action-local binders are not global definitions;
the resolver assigns stable owner-derived node/local identities. Named enums, constructors, systems,
states, capabilities, actions, and properties receive full typed `DefPath`s.

The normative contract and accepted encodings are in
[RFC 0013](../../rfcs/0013-source-to-typed-core.md).

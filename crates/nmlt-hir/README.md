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
ambiguous, shadowed, cyclic, or out-of-policy input. Its `ResolvedProgram` is
currently a module/declaration resolver index, not RFC 0013's final
all-reference HIR: raw type/expression references and local binders still need
source-derived resolution-map coverage. It is not a typed term representation,
a proof certificate, a `CheckedProgram`, or permission to execute source.
M9-004 adds the explicit typed core after that reference map; M9-005 and M9-006
add elaboration and independent checking.

Anonymous observations and action-local binders are retained by the complete
surface projection but are not global definitions: later substages assign
owner-derived node/local identities. Named enums, constructors, systems,
states, capabilities, actions, and properties receive full typed `DefPath`s.

The normative contract and accepted encodings are in
[RFC 0013](../../rfcs/0013-source-to-typed-core.md).

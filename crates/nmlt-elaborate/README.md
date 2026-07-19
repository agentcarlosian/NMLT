# `nmlt-elaborate`

`nmlt-elaborate` implements the M9-005 resolved-HIR-to-explicit-core boundary.
It checks or synthesizes scalar terms, forms system-indexed state and temporal
propositions, computes exact action frames and affine capability consumption,
and emits a canonical proof-relevant derivation DAG.

An `ElaborationArtifact` binds the exact source set, module map, canonical
surface program, resolved HIR, typed core, ruleset bundle, resource policy,
required obligation roots, and complete derivation graph. Construction rejects
missing root coverage, uncovered HIR nodes, unreachable derivations, type or
system-index mismatches, malformed action targets, duplicate capability use,
and resource excess. Its IDs are deterministic content identities, not
authority or provenance signatures.

This crate is still a producer and remains in the M9-005 trusted boundary. Its
success does **not** create `CheckedProgram`: M9-006 must independently
reconstruct every judgment from the exact HIR and core and reject forged,
stale, cyclic, or noncanonical certificates.

The normative contract is [RFC 0013](../../rfcs/0013-source-to-typed-core.md).

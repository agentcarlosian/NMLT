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

This crate is a producer and remains trusted only for the M9-005 producer
profile. Its success does **not** create `CheckedProgram`: M9-006 now
independently reconstructs every judgment from exact HIR and core and rejects
forged, stale, cyclic, or noncanonical certificates. Certificate syntax lives
in `nmlt-certificate`; the receiver does not reuse this producer's identity
calculation.

The normative contract is [RFC 0013](../../rfcs/0013-source-to-typed-core.md).

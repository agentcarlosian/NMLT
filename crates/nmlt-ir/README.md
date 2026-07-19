# `nmlt-ir`

`nmlt-ir` owns the explicit typed core accepted at the M9-004 boundary. Every
term carries a closed `CoreType`; temporal formulas carry their system index;
actions carry exact update/frame partitions; and ordinary string-named calls
do not exist.

`CoreNodeId` derives from a resolved-HIR `NodeId` plus a bounded canonical
insertion path. This allows elaboration to insert explicit coercion or
state-predicate nodes without losing the exact source origin. `CoreProgramId`
binds the resolved-HIR identity and the canonical span-free core encoding.

`CoreProgram::new` checks structural typing, graph closure and reachability,
owner and system agreement, integer canonicality, action frames, local scope,
and resource ceilings. It does **not** certify that the core faithfully
elaborates its HIR. M9-005 supplies derivations and M9-006 independently checks
that correspondence before producing `CheckedProgram`.

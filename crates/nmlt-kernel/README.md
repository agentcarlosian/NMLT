# `nmlt-kernel`

`nmlt-kernel` is the independent M9-006 receiver-side checker. It accepts an
exact resolved HIR, explicit core, and untrusted certificate; selects the
frozen M9-v1 ruleset and resource policy; independently recomputes identities;
checks ordering, uniqueness, coverage, reachability, depth, and limits; and
reconstructs each typing/formation judgment and aggregate core record.

Only successful `check` can construct `CheckedProgram`. The core, certificate
digest, and kernel-profile identity are owned behind private fields. Stable
`NMLT_KERNEL_*` diagnostics fail closed and never denote property refutation.

This boundary establishes type and formation acceptance for the exact M9-v1
fragment. It does not establish temporal truth, engine correspondence, or a
verified source-to-core compiler theorem. Persisted certificate decoding and
evidence-manifest binding remain M9-008.

The normative contract is [RFC 0013](../../rfcs/0013-source-to-typed-core.md).

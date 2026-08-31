# `nmlt-certificate`

`nmlt-certificate` defines the neutral M9 elaboration-certificate vocabulary:
obligations, frozen rule tags, conclusions, witnesses, derivation nodes, and
the producer artifact. It also contains producer-side canonical identity
construction.

The data is not proof by construction. It is copied into freely mutable
`RawCertificate` input and accepted only after independent replay by
`nmlt-kernel`. Producer identity functions are deliberately not reused by the
kernel.

These domain-separated elaboration-certificate identities are distinct from
the bare `source_sha256` field used by `behavior-core-v1` for source-byte
identification.

The normative contract is [RFC 0013](../../rfcs/0013-source-to-typed-core.md).

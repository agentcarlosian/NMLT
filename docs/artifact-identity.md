# Canonical source and evidence identity

Status: normative for identity version 1. Cryptographic identity binds exact
bytes and declared meaning; filesystem paths and display names are metadata.

## Encoding primitives

- Hash algorithm: SHA-256.
- Digest text: 64 lowercase hexadecimal characters.
- Length prefix: unsigned 64-bit big-endian byte length.
- Text entering an identity preimage: UTF-8 without a byte-order mark.
- Domain separators are the exact ASCII byte strings shown below.
- Producers must reject overflow, duplicate JSON object names, invalid UTF-8,
  and malformed identity strings. They must not repair inputs before hashing.

## Source identity

For exact source bytes `b`:

```text
source_digest = SHA256("NMLT-SOURCE\0v1\0" || u64be(len(b)) || b)
source_id     = "nmlt-source-v1:sha256:" || hex(source_digest)
```

Line endings, Unicode normalization, comments, and trailing whitespace are
meaningful bytes and therefore change the identity. Repository text policy is
UTF-8 and LF, but identity calculation does not normalize a violating file.

Portable paths use `/`, are relative, and contain no empty, `.` or `..`
segment. A path is not part of `source_id`, so moving an unchanged file retains
its source identity.

## Source-set identity

A module graph or corpus must bind more than a root file. Sort entries by the
UTF-8 bytes of portable path. For each entry append `u64be(path length)`, path
bytes, and the 32 raw bytes of its source digest. Hash the count and entries:

```text
SHA256("NMLT-SOURCE-SET\0v1\0" || u64be(entry_count) || entries)
```

The textual prefix is `nmlt-source-set-v1:sha256:`. Duplicate paths are an
error. Symlink traversal and generated/imported sources must be resolved and
declared before this identity is computed.

## Semantic and claim identity

Once elaboration exists, canonical core IR uses a separately versioned binary
encoding with explicit tags and lengths. Its identity prefix is
`nmlt-semantic-v1:sha256:`. A canonical claim binds:

```text
<semantic identity, fully qualified behavior name,
 fully qualified claim name, property-core identity,
 observation-contract identity>
```

under domain `NMLT-CLAIM\0v1\0`. Human-readable names such as `C04.NoBlindReplay`
are provisional handles, not cryptographic claim identities. Phase 0 examples
therefore freeze handles and source IDs while leaving canonical claim IDs
unassigned until the typed core encoding is accepted.

## Evidence identity

Evidence JSON must first satisfy the evidence schema and the I-JSON constraints
used by RFC 8785. Producers then:

1. parse while rejecting duplicate object names, non-I-JSON values, and
   negative zero;
2. remove the top-level `manifest_id` and `signatures` members;
3. serialize the remaining value with RFC 8785 JSON Canonicalization Scheme;
4. hash the canonical bytes as:

```text
evidence_digest = SHA256(
  "NMLT-EVIDENCE\0v1\0" || u64be(len(jcs_bytes)) || jcs_bytes
)
manifest_id = "nmlt-evidence-v1:sha256:" || hex(evidence_digest)
```

Removing signatures prevents circularity. A signature must sign the
`manifest_id` together with its signature-suite identifier and key identity.
Pretty-printed and canonical representations may coexist, but consumers must
recompute rather than trust the supplied ID.

## Required bindings

A verification evidence object is incomplete unless it binds:

- exact source or source-set identity;
- semantic and claim identity when semantic elaboration exists;
- verification configuration identity, including bounds and fairness;
- engine name, version, executable digest, and backend digest;
- trusted-component identities;
- result class and method;
- every witness, certificate, and negative-control artifact by digest;
- assumptions and residual gaps.

An engine identity is not merely a version string. The canonical engine record
contains the executable SHA-256 digest, build/version output, platform target,
and recursively relevant backend identities. Configuration uses JCS plus the
domain `NMLT-CONFIG\0v1\0`.

## Verification rules

- Consumers recompute all reachable identities before promotion.
- A missing, malformed, stale, or conflicting binding fails closed.
- The same manifest ID with different bytes is a critical integrity failure.
- A valid digest establishes byte equality, not truth, safety, authorship, or
  fitness for purpose.
- SHA-256 agility requires a new identity version and migration RFC; prefixes
  must never silently change meaning.

The reference source-ID calculator is `tools/canonical_examples.py`. Evidence
JCS support is intentionally not claimed by the pre-alpha CLI until RFC 8785
conformance vectors are present.

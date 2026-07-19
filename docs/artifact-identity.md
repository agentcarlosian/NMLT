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

## M9 module-map and HIR identity

RFC 0004's path-sorted `SourceSetId` remains unchanged. Logical module naming
is separate semantic input and is bound by RFC 0013. In the first M9 profile a
logical module is one ASCII NMLT identifier. The mapping is a bijection over
the source set, sorted by logical-module ASCII bytes. With
`lp(x) = u64be(len(x)) || x`:

```text
module_map_digest = SHA256(
  "NMLT-MODULE-MAP\0v1\0" || raw(source_set_id) || u64be(entry_count) ||
  concat(lp(logical_module) || lp(portable_path))
)
module_map_id = "nmlt-module-map-v1:sha256:" || hex(module_map_digest)
```

Changing a logical-name/path assignment changes `module_map_id` even when all
source bytes remain unchanged. `ModuleId` then binds this exact map:

```text
module_digest = SHA256(
  "NMLT-MODULE\0v1\0" || raw(module_map_id) || lp(logical_module)
)
module_id = "nmlt-module-v1:sha256:" || hex(module_digest)
```

Named HIR definitions use a full typed path. Its encoding begins with the
`u64be` segment count and then repeats `u8 kind_tag || lp(ASCII name)`. Tags
`01`–`0a` respectively mean type, constructor, constant/value, system, state,
action, system input, capability, property, and observation contract.

```text
def_digest = SHA256(
  "NMLT-DEF\0v1\0" || raw(module_id) || lp(def_path_encoding)
)
def_id = "nmlt-def-v1:sha256:" || hex(def_digest)
```

The full parent path prevents same-named members of different systems from
colliding. Local binders derive from their owning semantic node rather than
receiving top-level `DefId`s. Semantic `NodeId` uses
`NMLT-NODE\0v1\0`, the raw owner `DefId`, and a length-prefixed canonical
semantic-path encoding. Spans and arena allocation order are forbidden inputs.
Golden encodings live beside `nmlt-hir`; changing any accepted encoding or tag
requires a new identity version.

The completed all-reference HIR additionally assigns local binders and the
resolved artifact itself:

```text
local_digest = SHA256("NMLT-LOCAL\0v1\0" || raw(binder_node_id))
local_id     = "nmlt-local-v1:sha256:" || hex(local_digest)

resolution_digest = SHA256(
  "NMLT-HIR-RESOLUTION\0v2\0" || raw(source_set_id) ||
  raw(module_map_id) || lp(canonical_hir_bytes)
)
resolution_id = "nmlt-hir-resolution-v2:sha256:" || hex(resolution_digest)
```

HIR v2 canonically binds imports, declarations, local binders, semantic roots,
all HIR nodes, and the all-reference `ResolutionMap`; diagnostic spans are
excluded. The v1 resolution prefix is not accepted as a v2 identity.

## M9 typed-core identity

Core nodes retain an exact HIR origin while allowing deterministic
type-directed insertion. A canonical insertion path is at most 32 unsigned
32-bit big-endian segments; an empty path denotes direct translation:

```text
core_node_digest = SHA256(
  "NMLT-CORE-NODE\0v1\0" || raw(hir_origin_node_id) ||
  u64be(segment_count) || concat(u32be(segment))
)
core_node_id = "nmlt-core-node-v1:sha256:" || hex(core_node_digest)
```

`CoreProgram` uses fixed constructor/type/operator tags, `u64be` collection and
byte lengths, raw identity digests, and raw-digest map/set order. It contains no
spans or unresolved strings. Its identity binds the exact resolved HIR and the
complete canonical core encoding:

```text
core_program_digest = SHA256(
  "NMLT-CORE-PROGRAM\0v1\0" || raw(resolution_id) || canonical_core
)
core_program_id = "nmlt-core-program-v1:sha256:" || hex(core_program_digest)
```

This identity means “these structurally validated core bytes tied to this HIR
identity.” It does not mean that elaboration correspondence has been checked;
that requires M9-005's derivation and M9-006's independent kernel.

## Canonical-example registry identity

The Phase 0 registry freezes more than source bytes. Each example entry binds
its handle, path, source ID, prose intent, provisional claim handles, negative
control, and intended evidence classes. Remove `entry_id`, encode the remaining
string/array/object JSON subset as UTF-8 with object keys sorted, no insignificant
whitespace, and no ASCII escaping, then hash:

```text
SHA256("NMLT-CANONICAL-EXAMPLE\0v1\0" || u64be(len(bytes)) || bytes)
```

The prefix is `nmlt-canonical-example-v1:sha256:`. The top-level `corpus_id`
uses the same encoding over the entire registry with only `corpus_id` removed,
the domain `NMLT-CANONICAL-CORPUS\0v1\0`, and prefix
`nmlt-canonical-corpus-v1:sha256:`. Thus a change to an intended claim or
negative control cannot retain the same entry or corpus identity even when the
source file is unchanged. This is a registry freeze, not a semantic proof or a
replacement for the typed claim identity below.

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
are provisional handles, not cryptographic claim identities. M9-004 assigns
`CoreProgramId`, but `nmlt-semantic-v1` and canonical claim identity remain
unassigned until M9-008 fixes their checked-core, property, and observation
bindings.

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

Removing a future signatures member prevents circularity. If a signature suite
is later specified, it must sign the `manifest_id` together with its suite and
key identity. No signature or transparency-log format is implemented today.
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

An engine identity is not merely a version string. Generic assurance manifests
record `engine`, `engine_version`, `engine_source_set_id`, and
`engine_executable_sha256`; method-specific records additionally bind build
output, platform target, and recursively relevant backend identities.
`trusted_components` is nonempty and contains `{id, identity}` objects, where
`identity` is a raw `sha256:<digest>` or a versioned NMLT domain-separated
content identity. Configuration uses canonical JSON plus the domain
`NMLT-CONFIG\0v1\0`.

Proof-certificate `reference` in the generic schema is the raw content address
`sha256:<64 lowercase hex>`. The current local schema also requires a display
path for `proved`; it is not authoritative, so the checker confines it to the
repository, requires a regular file, and recomputes the reference from its
bytes. A future remote store needs a separate retrieval/readback protocol.
Witness references likewise begin with a content identity and may add a
fragment selecting an item inside the addressed artifact.

## Verification rules

- Consumers recompute all reachable identities before promotion.
- A missing, malformed, stale, or conflicting binding fails closed.
- The same manifest ID with different bytes is a critical integrity failure.
- A valid digest establishes byte equality, not truth, safety, authorship, or
  fitness for purpose.
- SHA-256 agility requires a new identity version and migration RFC; prefixes
  must never silently change meaning.
- A digest proves byte equality under its encoding rules, not existence in an
  artifact store. Local assurance checkers must also resolve/read the artifact;
  remote stores require an independently specified retrieval/readback policy.
- A digest is not a signature, authorship statement, timestamp, or
  transparency proof.

The reference source-ID calculator is `tools/canonical_examples.py`.
`tools/check_evidence.py` implements the repository's integer/string/Boolean/
null canonical JSON subset and duplicate-key rejection. Full RFC 8785 numeric
conformance is intentionally not claimed by the pre-alpha CLI or checker until
the required conformance vectors and number serialization are implemented.

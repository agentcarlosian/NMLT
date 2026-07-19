# RFC 0004: Canonical artifact identity

- Status: Accepted
- Authors: Carlosian <carlosian@agentmail.to>
- Created: 2026-07-18

## Decision

Adopt the versioned, domain-separated SHA-256 source, source-set, semantic,
claim, configuration, engine, and evidence identities specified in
`docs/artifact-identity.md`. Evidence JSON is canonicalized with RFC 8785 after
removing identity and signature fields.

## Rationale

Hashing an ordinary path fails under moves; hashing loosely serialized JSON
fails across producers; and hashing a source file alone fails to bind imports,
the property, finite bounds, or the engine. Separate domains prevent one
digest class from being interpreted as another and allow independent version
migration.

RFC 8785 is selected instead of an NMLT-specific JSON printer because it gives
independent test vectors and cross-language rules. Verified errata, including
the negative-zero ambiguity, are adopted by the input rejection rules.

## Negative controls

- CRLF and LF sources have different source IDs.
- Moving identical bytes preserves source ID but changes source-set entries.
- Reordered JSON object members preserve evidence ID.
- Reordered arrays change evidence ID.
- Duplicate JSON keys, negative zero, missing bounds, and stale engine digests
  are rejected.
- Changing only `manifest_id` cannot make a manifest valid.

## Compatibility

The current `structural:*` scaffold IDs are legacy placeholders and are not
canonical evidence IDs. Consumers must not promote them. Version 1 identity
prefixes are permanently reserved for these exact rules.

## Implementation gate

The source identity calculator and corpus checks land in Phase 0. Evidence ID
generation lands only after RFC 8785 conformance and schema tests exist.

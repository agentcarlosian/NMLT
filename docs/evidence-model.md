# Evidence model

NMLT treats verification results as typed artifacts rather than presentation
badges.

## Result classes

- `proved`: an accepted proof object covers the stated unbounded claim.
- `model_checked`: the property held within recorded finite bounds.
- `tested`: recorded concrete executions passed a stated test strategy.
- `monitored`: observed traces satisfied a monitor over a stated interval.
- `refuted`: a witness violates the claim.
- `unknown`: no accepted method has established or refuted the claim.
- `indeterminate`: verification or an external effect may have occurred, but
  authoritative terminal evidence is unavailable.

## Required fields

An evidence manifest records:

- schema version and manifest identity;
- artifact and claim identity;
- canonical verification-configuration identity;
- result class and verification method;
- exactly one source or source-set identity for every assurance-bearing result;
- engine name, version, source-set identity, and executable SHA-256;
- scope and finite bounds;
- assumptions and a nonempty list of content-addressed trusted components;
- content-addressed structured witness or proof-certificate references;
- negative controls exercised;
- residual gaps.

The normative machine contract is `schemas/evidence-manifest.schema.json`.

`proved`, `model_checked`, `tested`, `monitored`, and `refuted` are
assurance-bearing. All five require the exact artifact, engine, and TCB
bindings above. `proved` additionally requires a locally resolvable certificate
path whose bytes match its `sha256:<64 lowercase hex>` reference; other positive
classes do not invent a proof certificate and retain their method-specific
scope. `refuted` requires a resolvable content-addressed witness.

Each trusted component is an object with a stable role `id` and an `identity`
that is either a raw SHA-256 content address or a versioned NMLT
domain-separated identity. A label such as `"trust-me"` is neither an identity
nor a valid component record.

## Validation and readback

Schema validation establishes required shape, not truth or filesystem
existence. `tools/check_evidence.py` independently recomputes the canonical
manifest ID, confines local repository paths, verifies the exact source,
resolves current claim and configuration IDs, rebuilds and identifies the
engine, resolves every trusted component, negative control, and witness, and
checks local certificate bytes. An artifact source set is rejected until a
membership resolver exists. Self-test mode uses current provider inputs but
does not trust or read the checked-in evidence manifests; it exercises stale,
missing, syntactically valid-but-unresolvable, and trust-me forgeries.

The M11 metatheory has a separate claim-specific readback artifact at
`benchmarks/results/open-composition/m11-001a-evidence.json`. Its checker binds
the exact Lean sources, theorem and control handles, pinned toolchain, schema,
checkers, trusted-component inventory, and audited `#print axioms` output. It
is not an `evidence-manifest.schema.json` provider result and explicitly records
that no Rust/Lean correspondence has been verified.

## Promotion

Promotion is a vector of required evidence dimensions, not an average score.
If a required dimension is unknown, blocked, refuted, or indeterminate, the
artifact does not receive that promotion level.

## Structural evidence scaffolds

The current CLI can emit a manifest after parsing a file. Its result is always
`unknown`, its method is `structural_check`, and its residual gaps explicitly
state that semantic verification has not run. Legacy `structural:*` manifests
remain schema-valid without semantic source/engine/TCB bindings only because
their result ceiling is `unknown`; they are noncanonical and cannot be
promoted.

NMLT does not currently sign manifests or publish them to a transparency log.
A valid content digest binds bytes but does not establish authorship,
provenance, freshness, or independent publication.

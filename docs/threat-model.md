# NMLT trusted-computing-base threat model

- Scope: repository-wide pre-alpha NMLT frontend, M9 surface/resolution
  boundary, typed/bounded engines, temporal/refinement and resource extensions,
  agentic evaluation, artifacts, and CI
- Model version: 1.3
- Reviewed: 2026-07-19
- Security policy: `SECURITY.md`
- Trusted-component manifest: `security/trusted-components.toml`

## Overview

The NMLT research program's flagship language and tooling intend to turn
source, temporal claims, verification results, and runtime observations into
evidence-carrying artifacts. Their central security property is **claim
integrity**: no input, backend, cache, contributor, or renderer may cause a
result to be interpreted as stronger, newer, broader, or better bound than the
evidence establishes.

The repository now contains a lossless frontend, a deliberately narrow typed
executable fragment with deterministic finite exploration, finite temporal and
refinement checks, a finite verification-condition/certificate checker,
authority-bounded repair evaluation, and a graded-resource prototype. These
components establish only their documented finite or statement-local claims.
They do not form a verified compiler, do not authenticate runtime events, and
do not authorize external effects.

### Assets

1. Exact NMLT source and imported-module bytes.
2. The meaning and identity of behavior, property, observation, and refinement
   claims.
3. Result classification (`proved`, `model_checked`, `tested`, `monitored`,
   `refuted`, `unknown`, `indeterminate`) and its scope.
4. Bounds, assumptions, trusted-component lists, configurations, certificates,
   counterexamples, and runtime traces.
5. Compiler, checker, schema, release, and CI integrity.
6. Maintainer review authority and private vulnerability reports.
7. Availability for bounded checking; resource exhaustion must not become a
   false success or an unsafe automatic retry.

Credentials and private source may pass through future adapters, but NMLT must
not store them in evidence, diagnostics, traces, or fixtures.

### Security objectives

- Parse and hash exact bytes without hidden normalization.
- Fail closed on stale, missing, malformed, conflicting, or unverifiable
  evidence.
- Preserve `unknown` and `indeterminate` through every API and renderer.
- Keep untrusted source, solvers, model checkers, proof search, generated code,
  AI output, witnesses, and runtime traces outside the claim-acceptance kernel.
- Make every accepted claim reproducible from exact source, configuration,
  engine, and trusted-component identities.
- Never let a verification result directly authorize a critical external
  effect in pre-alpha software.

## Threat Model, Trust Boundaries, and Assumptions

### Actors

- **Specification author:** may make mistakes or intentionally submit hostile
  syntax, imports, properties, paths, or oversized finite domains.
- **Contributor/dependency attacker:** may attempt to weaken a checker, schema,
  negative control, CI gate, or trusted-component declaration.
- **Backend attacker:** controls solver/model-checker output, exit status,
  certificate, trace, cache, or version text.
- **Artifact attacker:** can replay, truncate, reorder, substitute, or combine
  manifests and witnesses from different revisions.
- **Runtime/environment attacker:** can forge, omit, reorder, or duplicate
  implementation events and exploit ambiguous external-effect outcomes.
- **Maintainer or CI compromise:** can sign or publish malicious code and is
  not solved by language metatheory alone.

### Trust boundaries

```text
untrusted source/imports
        |
        v
[B1 byte reader + lossless lexer + parser]
        | CST, declarations, diagnostics
        v
[B2 resolver + type/elaboration/behavior kernel]
        | typed core + obligations
        v
[B3 untrusted search engines/backends]
        | raw result/certificate/witness
        v
[B4 narrow result/proof checker + classifier]
        | canonical evidence
        v
[B5 artifact store, CI, release, renderers]
        |
        v
[B6 runtime adapters and effect controllers]   (never implicit)
```

- **B1:** OS filesystem and UTF-8 decoding enter the Rust frontend. Source,
  paths, comments, strings, and sizes are untrusted. Frontend success means
  only structural acceptance.
- **B2:** The M9 surface projector, all-reference resolver, resolution readback,
  typed-core structural validator, and M9-005 elaboration/certificate producer
  enter the TCB for projection, resolved HIR, structurally validated core, and
  elaborator-produced derivation claims. The current
  executable-fragment parser,
  contextual elaborator, type checker, frame/capability rules,
  simultaneous-update evaluator, and property indexing additionally enter the
  TCB for typed finite results. Unsupported language forms must be rejected,
  not approximated as success. Candidate replay now detects missing, stale, or
  relabelled resolution entries, but it shares the Rust crate and lookup model;
  the resolver therefore remains trusted. The typed-core validator checks
  closure and annotations, not faithfulness to HIR. M9-005 makes the claimed
  translation explicit but remains trusted; only M9-006 independent replay can
  remove the producer from acceptance of that correspondence.
- **B3:** TLC, Quint, P, SMT solvers, proof search, AI systems, and generated
  code may discover evidence but are untrusted unless a narrower checker
  validates a certificate under the exact obligation.
- **B4:** The explicit-state readback, temporal/refinement readback,
  finite-invariant certificate checker, evidence normalizer, agentic authority
  evaluator, and graded checker may construct only their method-specific result
  classes. Timeouts, missing terminal output, unchecked certificates, and
  engine disagreement map to `unknown` or `indeterminate`, never success.
- **B5:** Duplicate-rejecting JSON parsing, the documented canonical JSON
  subset, SHA-256, schema validation, checkers, caches, CI actions, registries,
  and release tooling protect artifact identity. Human-facing renderers do not
  determine result class. NMLT has no signature or transparency-log
  implementation today.
- **B6:** Runtime traces are untrusted observations. A trace adapter must bind
  clock/order assumptions and may monitor only declared observations. No
  pre-alpha evidence permits automatic safety-critical effects.

### Current claim-specific TCB

The normative inventory is `security/trusted-components.toml`.

For structural acceptance, trust is limited to hardware/OS process and
filesystem behavior, the pinned Rust declaration and resulting compiler and
standard library, the NMLT frontend/diagnostic code, `nmlt-cli`, and the
build/CI procedure. The example body is not semantically interpreted by this
claim profile.

For canonical source IDs produced by the Phase 0 reference tool, Python 3,
`pathlib`, `hashlib` and its SHA-256 implementation, the identity algorithm,
and filesystem bytes are additionally trusted. This reference is not yet a
proof-producing kernel.

For the current M9 source-to-elaborated-core boundary, the lossless parser, total
origin-censused projector, exact source/source-set and module-map identity
encoders, portable-path policy, import-graph checker, namespace resolver, and
pinned Rust build are trusted. This profile can establish deterministic closed
module and named-declaration tables plus lookup behavior for the exact input.
The resolver now emits and replays the complete source-derived `ResolutionMap`.
The trusted M9-005 elaborator checks/synthesizes the supported fragment and
emits a fully reachable, identity-bound derivation DAG, but no independent
kernel accepts it yet. This profile cannot establish temporal truth, execution
safety, a verified compiler theorem, or removal of the resolver/elaborator from
the TCB.

For a typed bounded result, the parser/elaborator/type checker, operational
semantics, deterministic explorer, report/evidence checkers, Rust build output,
and their exact identities are additionally trusted. Temporal, refinement,
runtime-trace, agentic-evaluation, graded-resource, multi-engine, and Lean
claims each have narrower profiles in the inventory. Trust is not transitive
across profiles: for example, a Lean proof does not establish a Rust execution
claim without the missing correspondence theorem.

The following are explicitly outside the TCB: documentation prose, examples,
comparison models, benchmark expectations, AI repair candidates, fuzzers,
search/proof strategies, unchecked solver/model-checker returns,
pretty-printers, dashboards, and test counts. They improve assurance but cannot
construct a stronger result by themselves.

Every non-structural assurance manifest must name exactly one artifact
source/source-set identity, an engine name/version/source-set/executable digest,
and a nonempty list of content-addressed trusted components. Generic assurance
evidence must include the exact `security/trusted-components.toml` digest. The
graded metatheory evidence additionally checks that the
`nmlt_lean.statements` inventory is exactly the repository-local import closure
of `NMLT.lean`, and binds that source set. `proved` also requires a local path
and raw-SHA-256 certificate reference. JSON Schema checks this shape;
`tools/check_evidence.py` confines paths, recomputes exact source and manifest
IDs, resolves the claim, configuration, engine, trusted components, negative
controls, and witness against current repository artifacts, and verifies local
certificate bytes. Generic source sets fail closed until membership readback
exists. A syntactically valid digest by itself is never evidence that the named
bytes exist or that a claim is true.

### Assumptions

- The host kernel, CPU, memory, and cryptographic primitives behave as
  specified; a fully compromised host can forge all local evidence.
- Maintainer keys and private reporting channels are protected outside NMLT.
- SHA-256 collision and second-preimage resistance remain adequate for v1.
- The checked-out toolchain and source correspond to their recorded identities.
- GitHub Actions `checkout` and the Lean build action execute the exact commits
  recorded in CI; the `ubuntu-latest` image, network delivery, rustup/elan
  bootstrap binaries, Python runtime, linker, and system libraries remain
  residual trust rather than reproducibly attested inputs.
- External tool documentation accurately describes the invoked version only;
  backend truth is not assumed from branding or exit code.
- Human intent cannot be derived from source alone. Intent capsules, semantic
  mutants, and review are required but remain fallible.

### Security invariants

1. Structural success is never serialized or displayed as semantic success.
2. A result class cannot exceed its method (`model_checked` is bounded;
   `proved` requires an accepted certificate).
3. Evidence is invalid if any required source, claim, configuration, engine,
   TCB, witness, or negative-control identity is stale or absent.
4. Property and benchmark edits invalidate prior results.
5. Untrusted backends cannot directly mutate trusted source, claims, schemas,
   or result classification.
6. Indeterminate external effects cannot be blindly retried.
7. Hidden state and trace events cannot be omitted from a refinement without a
   declared observation/hiding map.
8. Digest-only evidence must not be presented as signed, authenticated, or
   transparency-logged provenance.

## Attack Surface, Mitigations, and Attacker Stories

| Surface / attacker story | Impact | Existing mitigation | Required or residual work |
|---|---|---|---|
| A source uses comments, strings, Unicode, or delimiter recovery to smuggle a fake declaration. | Wrong program is analyzed. | RFC 0003 lossless tokens; immutable lossless CST; recovery diagnostics; canonical/provider round-trip and malformed-input tests. | Differential fuzzing and import-resolution adversarial tests remain. |
| A huge file, identifier, comment, or finite domain exhausts memory/time. | Denial of service; possible lost terminal evidence. | Lexer is linear; explicit-state bounds return `unknown` instead of success. | Add wall-time, memory, input, and output limits; never retry effects from a resource-limit exit. |
| CRLF, Unicode normalization, symlinks, or path moves make evidence appear current. | Stale or substituted source. | RFC 0004 exact-byte hashes; provider evidence checker confines repository-relative paths and recomputes `source_id`. | Complete import/source-set membership and symlink-race handling remain. |
| An attacker edits `manifest_id` or reorders JSON to forge a digest. | Evidence substitution. | Canonical subset encoding excludes identity/signature fields, recomputes the digest, rejects duplicate keys, and tests member-order invariance. | Full RFC 8785 numeric conformance and a versioned migration remain. `structural:*` is deliberately noncanonical/unknown. |
| A bounded run is labelled `proved`, or a timeout is treated as a pass. | False assurance. | Schema/checker enforce bounds and frontier completion; the bounded producer refuses `proved`; adversarial promotion tests run independently of provider artifacts. | Do not generalize the finite-VC proof class to unbounded language semantics. |
| A malicious backend returns `sat/unsat`, a corrupt proof, or a witness for another claim. | False result or misleading diagnosis. | Backends are outside TCB; nmlt-verify checks a narrow finite certificate and binds the exact VC; plain status/stdout remains unknown. | Independent review and scalable certificate formats remain. |
| A counterexample/certificate reference uses traversal, symlink swap, or decompression bomb. | File disclosure, substitution, or DoS. | Generic references are content-addressed; local checker paths are confined and regular-file/exact-byte checked. | General artifact-store loaders still need size limits, race-safe opens, and no implicit execution/decompression. |
| An AI or repair loop weakens the property, benchmark, bounds, or negative control. | Reward hacking and false validation. | Protected spans/digests and explicit edit authority separate claims/controls from repair candidates; evaluation is only `tested`. | Human review authority, hosted branch protection, and authenticated approvals remain. No signature claim is made. |
| CI or a dependency is replaced; mutable action tags fetch new code. | Compromised releases/checkers. | Least-privilege `contents: read`; checkout credentials are not persisted; checkout v4.2.2 and Lean action v1.5.0 are commit-pinned; lean4export and nanoda sources are exact-SHA pinned; language versions are exact. | Runner image, network delivery, rustup/elan downloads, locally built binary digests, Python, linker/system libraries, provenance/SBOM, signing, and transparency remain unattested or unimplemented. |
| A malicious contributor changes checker and tests together. | Backdoored TCB with passing CI. | RFC/ADR process and planned independent TCB review. | Enforce protected reviews/CODEOWNERS once hosted; use semantic mutants and independent implementations. |
| Runtime events are omitted, reordered, duplicated, or forged. | Incorrect conformance or unsafe replay. | Finite runtime adapter declares observations, preserves three-valued outcomes, and localizes contradictions. | Authenticate event sources and bind order/clock assumptions before deployment; the adapter does not authorize effects. |
| Diagnostics, traces, or evidence capture credentials/private source. | Confidentiality breach. | Reporting policy forbids secrets; no network or telemetry in current frontend. | Add redaction policy and fixture tests before external adapters/log ingestion. |
| A renderer displays a green badge while machine data says unknown. | Human false assurance. | Machine result is authoritative; current CLI prints explicit structural-only note. | Add presentation conformance tests and never infer class from color/text. |

### High-value attacker stories

1. **Stale proof replay:** prove claim `P` for source `S`, weaken `P` or change an
   import, then reuse the old manifest. Recalculated source-set, semantic,
   claim, and configuration identities must reject it.
2. **Backend impersonation:** place a fake `z3`, `tlc`, or proof checker earlier
   on `PATH`. Engine identity must include the executable digest and bootstrap
   provenance, not version output alone.
3. **Ambiguous effect replay:** crash after provider dispatch but before receipt
   persistence, then coerce `indeterminate` into `failed` and retry. The state
   machine and evidence classifier must retain ambiguity and consume the
   one-shot authority.
4. **Vacuous verification:** replace a meaningful invariant with `true` while
   preserving its display name. Claim semantic identity and frozen negative
   controls must change/fail.
5. **Checked-code bypass:** return a valid certificate for a different formula
   or theory version. The certificate checker must reconstruct the exact
   obligation and bind checker/theory identities.
6. **Trust-me manifest:** label a nonexistent path `proved`, name an arbitrary
   engine/version, use `trusted_components: ["trust-me"]`, and point at a
   non-content-addressed certificate. The generic schema rejects the missing
   exact engine/TCB/certificate shapes, and independent readback rejects the
   nonexistent or source-mismatched artifact even if all supplied digest text
   is syntactically well formed.

## Severity Calibration (Critical, High, Medium, Low)

Severity combines attainable impact with the pre-alpha deployment boundary.
The absence of production support lowers current exposure, not the conceptual
severity of false-assurance paths in a future release.

### Critical

- A remotely or commonly reachable path produces `proved`/accepted refinement
  for false attacker-chosen source and can authorize safety-critical,
  financial, security, or irreversible effects.
- Release/signature compromise allows undetectable substitution of the trusted
  kernel or canonical evidence across users.
- A memory-safety or command-execution flaw crosses from untrusted source or
  artifact into the build/check host with broad impact.

There is no supported critical-effect integration today, so such paths are
out-of-contract and must remain disabled.

### High

- False semantic success, stale evidence acceptance, property/claim mismatch,
  or backend spoofing affects ordinary repository workflows.
- Sandbox escape, arbitrary file read/write, or credential disclosure from a
  crafted source, witness, certificate, or runtime trace.
- Systematic misclassification of `indeterminate` as retryable failure.

### Medium

- Reliable resource exhaustion within documented limits or CI that does not
  yield false success.
- Misleading diagnostics/counterexamples that materially obstruct review but
  leave machine result class correct.
- Integrity failures requiring local write access and producing a detectable
  identity mismatch.

### Low

- Cosmetic rendering, documentation, or diagnostic-location errors with no
  claim-class, identity, confidentiality, or availability consequence.
- Non-sensitive crash on malformed input with bounded impact and no evidence
  artifact emitted.

### Out of scope for repository controls

- A fully compromised host, CPU, compiler, maintainer identity, or signing key.
- Errors in a user's physical-system model or undisclosed environment facts.
- Authorization decisions that ignore the explicit pre-alpha prohibition.

These are not dismissed: releases must document them as assumptions, and a
future production profile requires reproducible builds, key governance,
independent kernel review, sandboxing, and deployment-specific hazard analysis.

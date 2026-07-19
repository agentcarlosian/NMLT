# NMLT trusted-computing-base threat model

- Scope: repository-wide pre-alpha NMLT frontend, artifacts, CI, and planned
  verification boundaries
- Model version: 1.0
- Reviewed: 2026-07-18
- Security policy: `SECURITY.md`
- Trusted-component manifest: `security/trusted-components.toml`

## Overview

NMLT is research software that intends to turn source, temporal claims,
verification results, and runtime observations into evidence-carrying
artifacts. Its central security property is **claim integrity**: no input,
backend, cache, contributor, or renderer may cause a result to be interpreted
as stronger, newer, broader, or better bound than the evidence establishes.

Today the executable code lexes losslessly, recognizes structural `system`
declarations, renders diagnostics, and emits an `unknown` evidence scaffold. It
does not type-check, execute behavior semantics, model-check, prove, refine, or
authorize effects. The threat model includes planned boundaries so new code
cannot silently inherit more trust than this baseline.

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
        | syntax/diagnostics only
        v
[B2 resolver + type/elaboration kernel]        (planned)
        | typed core + obligations
        v
[B3 untrusted search engines/backends]
        | raw result/certificate/witness
        v
[B4 narrow result/proof checker + classifier]  (planned)
        | canonical evidence
        v
[B5 artifact store, CI, release, renderers]
        |
        v
[B6 runtime adapters and effect controllers]   (planned, never implicit)
```

- **B1:** OS filesystem and UTF-8 decoding enter the current Rust frontend.
  Source, paths, comments, strings, and sizes are untrusted. Success means only
  structural acceptance.
- **B2:** Future name resolution, type checking, frame elaboration, behavior
  semantics, and canonical core encoding enter the semantic TCB.
- **B3:** TLC, Quint, P, SMT solvers, proof search, AI systems, and generated
  code may discover evidence but are untrusted unless a narrower checker
  validates a certificate under the exact obligation.
- **B4:** Result adapters and proof checkers may construct claim classes only
  when their method-specific requirements hold. Timeouts and missing terminal
  output map to `unknown` or `indeterminate`, never success.
- **B5:** JSON parsing, RFC 8785 canonicalization, SHA-256, signatures, schema
  validation, caches, CI actions, registries, and release tooling protect
  artifact identity. Human-facing renderers do not determine result class.
- **B6:** Runtime traces are untrusted observations. A trace adapter must bind
  clock/order assumptions and may monitor only declared observations. No
  pre-alpha evidence permits automatic safety-critical effects.

### Current claim-specific TCB

The normative inventory is `security/trusted-components.toml`.

For current structural acceptance, trust is limited to hardware/OS process and
filesystem behavior, the pinned Rust compiler and standard library,
`nmlt-core::lexer`, `nmlt-core::syntax`, diagnostic rendering, `nmlt-cli`, and
the build/release procedure. The example body is not semantically interpreted.

For canonical source IDs produced by the Phase 0 reference tool, Python 3,
`pathlib`, `hashlib` and its SHA-256 implementation, the identity algorithm,
and filesystem bytes are additionally trusted. This reference is not yet a
proof-producing kernel.

The following are explicitly outside the TCB: documentation prose, examples,
comparison models, benchmark expectations, AI output, fuzzers, search
strategies, model checkers without checked certificates, pretty-printers,
dashboards, and test counts. They improve assurance but cannot construct a
stronger result by themselves.

### Assumptions

- The host kernel, CPU, memory, and cryptographic primitives behave as
  specified; a fully compromised host can forge all local evidence.
- Maintainer keys and private reporting channels are protected outside NMLT.
- SHA-256 collision and second-preimage resistance remain adequate for v1.
- The checked-out toolchain and source correspond to their recorded identities.
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

## Attack Surface, Mitigations, and Attacker Stories

| Surface / attacker story | Impact | Existing mitigation | Required or residual work |
|---|---|---|---|
| A source uses comments, strings, Unicode, or delimiter recovery to smuggle a fake declaration. | Wrong program is analyzed. | RFC 0003 lossless tokens; keywords arise only from identifier tokens; byte spans round-trip. | Full error-recovering parser and differential/fuzz tests are Phase 1 work. |
| A huge file, identifier, comment, or finite domain exhausts memory/time. | Denial of service; possible lost terminal evidence. | Current lexer is linear and contains no unsafe code. | Add input, state-space, time, memory, and output limits; classify limit exits as `unknown`, never retry effects. |
| CRLF, Unicode normalization, symlinks, or path moves make evidence appear current. | Stale or substituted source. | RFC 0004 hashes exact bytes and separately binds portable source-set paths. | Integrate IDs into CLI; define symlink/import resolution before semantic evidence. |
| An attacker edits `manifest_id` or reorders JSON to forge a digest. | Evidence substitution. | RFC 8785 canonicalization excludes the identity/signature fields and requires recomputation. | Implement JCS conformance vectors; current `structural:*` IDs remain noncanonical placeholders. |
| A bounded run is labelled `proved`, or a timeout is treated as a pass. | False assurance. | Closed result enum, schema conditions, explicit residual gaps, governance rule. | Method-specific constructors and independent readback are not implemented yet. |
| A malicious backend returns `sat/unsat`, a corrupt proof, or a witness for another claim. | False result or misleading diagnosis. | Backends are outside TCB by policy; identities must bind raw outputs. | Add narrow proof/result checkers and adversarial adapter tests before semantic claims. |
| A counterexample/certificate reference uses traversal, symlink swap, or decompression bomb. | File disclosure, substitution, or DoS. | No artifact loader exists today. | Future loaders require content-addressed references, root confinement, size limits, and no implicit execution. |
| An AI or repair loop weakens the property, benchmark, bounds, or negative control. | Reward hacking and false validation. | Trusted intent/claims are separated from generated implementation; source IDs invalidate results. | Enforce edit authority and review signatures before agentic workflows. |
| CI or a dependency is replaced; mutable action tags fetch new code. | Compromised releases/checkers. | Minimal dependency-free Rust code; least-privilege `contents: read`. | Pin CI actions by digest, add release provenance/SBOM, review lockfile and tool bootstrap hashes. |
| A malicious contributor changes checker and tests together. | Backdoored TCB with passing CI. | RFC/ADR process and planned independent TCB review. | Enforce protected reviews/CODEOWNERS once hosted; use semantic mutants and independent implementations. |
| Runtime events are omitted, reordered, duplicated, or forged. | Incorrect conformance or unsafe replay. | Proposed typed trace/refinement contract and durable examples. | Authenticate event sources, declare order/clock assumptions, preserve unknown cells; runtime adapters are not implemented. |
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

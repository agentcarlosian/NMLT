# NMLT active threat model

- Scope: current language-and-mathematics pivot
- Model version: 2.0
- Reviewed: 2026-08-31
- Security policy: [`SECURITY.md`](../SECURITY.md)
- Component inventory:
  [`security/trusted-components.toml`](../security/trusted-components.toml)

## Security objective

The primary security property is **claim integrity**: no source, artifact,
backend, cache, contributor change, CLI wording, or documentation page may
cause a result to be interpreted as stronger, broader, newer, or more
authoritative than the checked semantics establish.

NMLT is not a production authorization system. Its current results are
language-implementation checks and Lean theorems about explicit definitions.

## Assets

1. Exact source and artifact bytes.
2. The meaning of the current behavior, resource profile, product, and
   refinement definitions.
3. The distinction between Rust acceptance/exploration and Lean theorem
   acceptance.
4. Product-formation assumptions, state/refinement maps, authority ownership,
   action effects, grades, and rely/guarantee facts.
5. The theorem source set, toolchain pins, axiom report, negative controls, and
   CI configuration.
6. Maintainer review authority and private vulnerability reports.

## Trust boundaries

```text
untrusted .nmlt bytes
        │
        ▼
[Rust lossless frontend, resolver, typed elaboration]
        │ ordinary checked core / behavior-core-v1
        ▼
[artifact bytes + separately supplied source bytes]
        │
        ├──────────────► [Rust explorer: assurance none]
        │
        ▼
[Lean decoder and finite semantic construction]
        │
        ▼
[Lean kernel + focused axiom and mutation gates]
```

### Rust frontend boundary

Source, paths, identifiers, comments, delimiters, expressions, finite-domain
sizes, and all user-declared resource data are untrusted. Recovery-dependent or
unsupported input must fail before semantic artifact construction.

The retained `nmlt-kernel` checks the ordinary typed-elaboration
certificate. It does not define behavioral semantics and cannot confer a
behavior theorem.

### Executable source job boundary

The executable-only [scoped job profile](r2-source-async.md) adds a distinct host
boundary. Source cannot construct or copy session controls; a checked lexical
handle must be collected once on every normal path. The CLI binds every source
operation to its exact attempt and journal transitions, and replay validates
captured observations before returning recorded polling/cancellation decisions.
This prevents inconsistent source/session records from being accepted, but does
not authenticate physical observations, fence copied journals, restore handles
after restart, or contain process trees. The configured worker/Lean installation
and its libraries remain trusted local dependencies.

### Artifact boundary

Artifact JSON is untrusted. Canonical decoding must reject duplicate
identities, reordered/noncanonical data, malformed finite terms, inconsistent
payload/resource profiles, invalid wiring, partial refinement maps, and stale
source digests.

The digest check only compares the artifact's asserted digest with the supplied
source bytes. It does not run the compiler. CI separately regenerates the
committed primary artifact and byte-compares it, but arbitrary artifacts do not
inherit that stronger reproducibility fact.

### Lean boundary

Lean's decoder and semantic-closure code are trusted definitions for the
statements they construct. The Lean kernel, pinned toolchain, standard
foundational axioms listed in `AXIOMS.md`, and independently pinned
NanoDA path are part of the theorem trust boundary.

A checked conditional theorem does not establish:

- that a concrete step exists;
- that an initial state reaches it;
- that Rust faithfully translated source;
- that every product-formation condition is logically necessary; or
- that a formal model captures unstated human intent.

### Explorer boundary

`nmlt-eval` accepts canonical Rust-decoded artifact data and performs
bounded operational inspection. Its output is debugging information with
`assurance: none`. It is not a proof or model checker.

## Attacker stories and controls

| Story | Impact | Current control | Residual work |
|---|---|---|---|
| Malformed or recovery-dependent source is treated as a program | Wrong source meaning | Lossless CST, explicit diagnostics, projection coverage tests | Fuzzing and stronger resource limits |
| An artifact copies a legitimate source digest but contains different valid semantics | False translation impression | Documentation limits digest to identification; primary fixture is regenerated and compared | Translation validation or verified compiler |
| Duplicate or malformed capability/resource identities enter the artifact | Authority ambiguity or fabrication | Canonical Rust decoder, independently derived v2 maps, and actual-step/world controls | Extend coverage as the language expands |
| A hidden action changes control or authority but is called stutter | Unsound weak matching | Full-state agreement in unified lifting; hidden-world mutation and consumption controls | General trace transport remains future work |
| Product formation drops a wire, mismatches ports, shares ownership, or leaves reliance undischarged | Invalid composition | Formation judgment and boundary-specific controls | Countermodels for claims of theorem-premise necessity |
| Retained v1 evidence is presented as covering the new execution model | Claim inflation | Versioned entry points and distinct v1/v2 claim profiles | Preserve boundaries during future migrations |
| A conditional lifting function is presented as an executed path | Reachability overclaim | V1 wording stays conditional; v2 checks the initializer and every unified step and derives finite reachability | Per-invocation proof export remains future work |
| Rust output is presented as Lean acceptance | Trust-boundary confusion | CLI explicitly says when Lean was not invoked; explorer says assurance none | Stable output tests for every command |
| A proof placeholder or unexpected theorem dependency enters Lean | False theorem confidence | no-`sorry` scan, focused `#print axioms` audit, NanoDA pass | Exact per-theorem allowlist enforcement |
| CI actions, toolchain downloads, dependencies, or host are compromised | Arbitrary false results | Pinned language versions and action commits, least-privilege checkout | Reproducible bootstrap, SBOM, signing, isolated builders |
| A maintainer changes theorem and controls together | Backdoored green gate | RFC/ADR process and independent review | Branch protection, CODEOWNERS, multiple maintainers |

## Security invariants

1. Structural or Rust type acceptance is never displayed as Lean semantic
   acceptance.
2. Exploration never emits proof or verification authority.
3. Unsupported syntax and malformed artifacts fail closed.
4. Hidden stuttering cannot erase capability effects, nonzero grades, or extra
   reliance.
5. One capability has at most one dynamic owner.
6. Synchronized transfer changes ownership exactly once and cannot be retained
   by the sender.
7. Documentation distinguishes product-formation policy from demonstrated
   theorem dependencies.
8. The active trusted-component inventory contains only existing paths.
9. Dated historical records cannot override current claim ceilings.

## Local job and recovery boundary

Source jobs record a versioned process/resource policy and complete pinned Lean
installation identities. Closed Init term inputs cannot insert arbitrary Lean
commands or tactics. The empty-axiom report is checked as captured process
evidence, with the pinned checker and host still trusted. It is not a runtime
attestation or independent audit of each generated proof.

The source decision journal is flushed around each effect. Resumption preserves
prior decisions and charged attempts; unresolved effects require explicit
operator failure acknowledgement. Old dispatched work is never relaunched by
recovery. Lost collect replies reuse the existing result. Explicit incomplete-tail
repair preserves the removed suffix and never discards a corrupt complete row.
Copied or rolled-back stores remain outside local lock fencing, and coherent
forgeries are not authenticated. See [RFC 0030](../rfcs/0030-durable-source-resumption.md).

The [platform process contract](../rfcs/0028-contained-process-lifecycle.md) is
not filesystem/network isolation. Windows whole-job resource/parent-death
coverage and Unix process-group fallback limits must not be presented as equal
OS guarantees. Current native containment execution evidence is Windows;
Linux compilation alone is not runtime validation.

## Residual trust

- host CPU, memory, kernel, filesystem, process execution, and SHA-256;
- Rust and Lean bootstrap/download infrastructure;
- Cargo and Lake dependency delivery;
- GitHub-hosted runner images and network delivery;
- linker and system libraries;
- correctness of hand-written Lean definitions relative to intended NMLT
  meaning; and
- maintainer judgment about human intent and publication wording.

NMLT has no signing, transparency log, reproducible bootstrap, verified
compiler, authenticated runtime-event channel, or production deployment
profile.

## Severity

- **Critical:** pre-alpha output can directly authorize an attacker-chosen
  irreversible effect, or remote input compromises the checking host.
- **High:** ordinary repository workflows accept false Lean semantics, forged
  translation claims, or attacker-chosen artifact substitution.
- **Medium:** bounded denial of service, materially misleading diagnostics, or
  local claim drift that does not pass the checked boundary.
- **Low:** cosmetic or documentation defects with no effect on machine results.

The lack of production integration reduces current exposure, not the importance
of preventing false-assurance paths.

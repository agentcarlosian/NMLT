# Security policy

NMLT is pre-alpha programming-language and mathematics research. It has no
supported production release and must not be used to authorize
safety-critical, security-critical, financial, or irreversible effects.

## Reporting a vulnerability

Do not disclose an exploitable vulnerability in a public issue.

1. Prefer a private GitHub security advisory for the repository.
2. If that is unavailable, email the maintainer at
   [carlosian@agentmail.to](mailto:carlosian@agentmail.to).

Include the affected revision, impact, minimal reproduction conditions, the
trust boundary crossed, and whether the result is proven, observed, suspected,
or indeterminate. Never send credentials, private source, or destructive
payloads.

## Current security boundary

The active repository contains:

- a Rust lossless frontend, resolver, typed IR, elaborator, and retained
  typed-elaboration validator;
- a Rust producer for canonical `behavior-core-v1` and `behavior-core-v2`
  artifacts;
- Lean definitions, artifact decoding, finite semantic construction, finite
  safety-invariant checking, and conditional composition/refinement theorems;
- a Rust bounded explorer and finite interpreter (`nmlt-eval`) with no
  verification authority;
- an executable-only pure workflow engine (`nmlt-workflow`) and bounded local
  job runtime (`nmlt-runtime`) with durable journals, vendored ProcessKit
  (`3.3.4-nmlt.1`) console-free Windows process supervision, and pinned `Init`
  proof-term checking;
- bound local Lean/Lake task workflows (`nmlt lean-task`) with
  candidate-selected fixed tactic imports and independent NanoDA proof checking
  over retained proof artifacts; and
- CI gates for Rust, Windows contained workers, Lean, NanoDA, artifact
  mutations, unchecked proof placeholders, and focused theorem dependencies.

It does not contain the former active temporal checker, finite VC verifier,
agentic repair evaluator, standalone grade analyzer, evidence-manifest system,
or contest release workflow. Those systems are historical and recoverable from
the `build-week-judge-demo-2026` tag.

The current trusted-component inventory is
[`security/trusted-components.toml`](security/trusted-components.toml).
The active attacker stories and residual trust are documented in
[`docs/threat-model.md`](docs/threat-model.md).

## Important limitations

- Rust-to-Lean translation is not verified.
- A source digest identifies supplied source bytes; it does not prove that an
  arbitrary artifact was compiled from them.
- The Rust explorer cannot issue a proof, model-check, evidence, or runtime
  authorization claim.
- The dynamic Lean result is conditional one-step lifting, not reachability or
  liveness.
- Local workflows, durable job journals, and ProcessKit resource supervision
  are executable-only controls: they do not provide complete OS sandboxing,
  filesystem/network isolation, rollback-resistant storage, or exactly-once
  external execution guarantees.
- Experimental discovery, evaluation, and solo-validation JSON summaries carry
  `assurance: none` and are not proof-acceptance records.
- Digests provide byte identity, not authorship, freshness, signing, or
  transparency.
- The host, toolchain bootstrap, dependencies, runner image, linker, and
  filesystem remain trusted.

Any report that raises a result above those limits is a security-relevant claim
integrity issue even when it does not create conventional memory corruption.

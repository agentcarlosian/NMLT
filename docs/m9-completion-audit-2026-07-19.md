# M9 Completion Audit — 2026-07-19

M9 closes the narrow source-to-typed-core vertical slice. It does not claim a
verified Rust compiler, temporal truth, unbounded verification, or production
runtime safety.

## Promoted route

```text
exact source modules
  → lossless projection and explicit feature boundary
  → closed resolution and readback
  → bidirectional elaboration and certificate DAG
  → independent kernel acceptance
  → opaque CheckedProgram
  → checked-only finite engine adapter
  → identity-bound model report
```

The former `nmlt-engine` parser is deleted. `TypedModel` cannot be constructed
outside the engine crate, and public execution starts at `from_checked`.

## Frozen canonical outcomes

| Examples | Outcome |
|---|---|
| `boolean_toggle`, `durable_controller`, `provider_attempt` | `type_checked` through the kernel |
| `trust_chain` | explicit port boundary |
| `two_process_mutex` | explicit selected-update boundary |
| `two_phase_commit` | explicit system-constant boundary |
| `one_bit_clock` | explicit unsupported ordinary-call resolution boundary |
| `euclid` | explicit system-input boundary |
| `bounded_channel` | explicit hiding boundary |
| `token_bucket` | explicit action-grade boundary |

`tools/check_m9_vertical_slice.py` binds this table to the frozen ten-example
registry and fails on membership, outcome, diagnostic, engine API, schema, or
negative-control drift.

## Evidence and readback

Every promoted model report binds source set, module map, surface program,
resolved HIR, typed core, ruleset bundle, resource policy, certificate, and
kernel profile. Benchmark envelopes additionally bind separate elaborator,
kernel, complete engine source-set, and executable identities. Python readback
independently recomputes source/module identity and reruns the checked pipeline.

Canonical v1 certificate bytes now round-trip through a bounded untrusted
decoder. Domain, tag, Boolean/sign canonicality, count, length, trailing-byte,
premise-edge, magnitude, and total-size checks fail before semantic acceptance;
only `nmlt-kernel::check` constructs `CheckedProgram`.

## Mechanization boundary

`NMLT/Correspondence/M9Kernel.lean` defines malformed extrinsic terms,
certificates, an executable reference checker, and `check_sound`. It states
coverage, resolution, initializer preservation, forward/backward action
simulation, exact framing, affine no-duplication preservation, and property
index/denotation preservation. Shared accept, missing-frame, and bad-rule-tag
vectors are exercised on the Lean and Rust sides.

This is a mathematical reference boundary, not a proof that the Rust parser,
resolver, decoder, identity code, or checker implements the Lean definitions.
Those links remain explicit in `security/trusted-components.toml`.

## Reproduction gates

- `make ci`: Rust formatting, checking, linting, tests, corpus, benchmark and
  evidence readback, canonical examples, comparisons, correspondence vectors,
  and the M9 audit.
- `make reproduce`: `make ci` plus the pinned Lean/NanoDA metatheory policy.
- The final commit is additionally tested from a clean private-repository clone
  before the completion record is finalized.

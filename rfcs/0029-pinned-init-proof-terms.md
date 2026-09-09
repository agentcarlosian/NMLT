# RFC 0029: Pinned Init proof terms

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-08

`job_start_lean_check(statement: Text, proof: Text) -> Job<Text>` accepts ordinary
typed inputs and function results. It uses the same affine handles, attempt
budget, process supervisor, cancellation, collection and captured replay as
the existing Lean template route. Both routes use the `lean-init` adapter,
version 1, and the explicitly versioned `nmlt-lean-request-v2` input envelope.

The input language is a closed expression grammar over the pinned `Init`
environment: qualified constant references, decimal naturals, application,
explicit `@` application, single `fun`/`forall` binders, type annotations,
arrows, equality and Nat `+`/`*`. A bounded parser fully parenthesizes its output.
It does not accept arbitrary files, commands, tactics, quotations, comments,
custom imports or metaprograms. Each input is at most 4096 ASCII bytes, 512
tokens and depth 48; the complete canonical request must also fit the existing
4096-byte protocol bound. Unsupported syntax fails before job allocation.

The generated file defines the requested statement with type `Prop`, checks
the candidate against that definition, and prints the checked theorem's
transitive axioms. Exit zero, empty stderr, and the exact empty-axiom report
are required for acceptance. Type rejection produces a failed job, not a
refutation. An axiom dependency produces policy failure. Success returns the
generated file's SHA-256; it is a receipt for this local check, not a theorem
about the host or an independent checker result.

All Lean requests bind a bounded sorted manifest of every `bin` and `lib` file
in the direct executable's installation. The tree is captured at preflight and
checked again before dispatch. Project locks retain those exact identities.
The OS loader, system libraries, pinned Lean implementation and concurrent
filesystem stability remain trusted; content hashes do not authenticate a
coherently forged local record. Replay checks captured consistency and does
not rerun Lean. Existing project/target import support remains R3.

Validation covers dynamic source arguments, transfer through functions,
incorrect proof/fallback, distinct arithmetic and propositional statements,
axiom-policy rejection, grammar bounds, identity mutations and real replay.

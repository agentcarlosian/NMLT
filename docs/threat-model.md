# Threat model

## Protected claims

The current checkpoint protects narrow statements about exact source bytes,
Rust formation/type acceptance, deterministic artifact encoding, Lean-checked
static theorem declarations, conditional affine authority-world step lifting,
and non-authoritative finite exploration.

It does not protect a claim that Rust elaboration is verified, that a decoded
artifact was correctly produced from its named source, that the conditional
dynamic theorem proves step existence or reachability, or that finite
exploration proves unbounded behavior.

## Primary attacker stories

- A malformed source attempts to reach typed core through parser recovery or
  incomplete projection.
- A stale or noncanonical artifact attempts to pass decoding or source-digest
  comparison.
- Duplicate capabilities or malformed resource profiles attempt to bypass
  affine and composition checks.
- A hidden resource-bearing transition attempts to masquerade as abstract
  stutter.
- A Rust exploration result is presented as a Lean proof or production
  authorization.
- Unchecked Lean placeholders or unexpected axioms weaken the advertised
  theorem boundary.

## Controls

The repository uses lossless parsing, fail-closed typed diagnostics,
deterministic artifact snapshots, decoder mutation tests, a no-`sorry` policy,
focused theorem axiom audits, independent declaration checking, and explicit
`assurance: none` evaluator output.

Residual trust includes the host, toolchain bootstrap, hashing implementation,
Rust compiler, Lean kernel and runtime, filesystem, and CI environment.

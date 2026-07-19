# M9 independent-kernel research synthesis — 2026-07-19

## Question

What minimum receiver-side boundary can accept NMLT's typed-core elaboration
without trusting the elaborator that produced it?

## Archive result

The local research archive was searched for proof-carrying code, small proof
checkers, independent typed-IR checking, canonical proof DAGs, translation
validation, certifying compilers, and trusted-computing-base certificate
checking. It contained no strong direct match. One “certifying compiler” hit
was a lexical false positive about quantum compilation and was excluded. This
absence is recorded rather than treating current web results as if they had
already appeared in the archive.

## Current primary sources

Necula and Lee's proof-carrying-code architecture separates a potentially
expensive proof producer from a receiver that validates evidence under the
receiver's safety policy. The receiver need not trust the producer or its
proof-generation process. This is the most direct precedent for making the
NMLT elaborator untrusted after an independent checker accepts its artifact:

- George C. Necula and Peter Lee, [Safe Kernel Extensions Without Run-Time
  Checking](https://www.cs.cmu.edu/afs/cs/project/pop-10/member/petel/www/publications/oakland97.pdf).
- George C. Necula, [Compiling with Proofs](https://www.cs.cmu.edu/~rwh/students/necula.pdf).
- CMU, [Proof-Carrying Code bibliography](https://www.cs.cmu.edu/~fox/pcc-bib.html).

CompCert provides a useful contrast. Its strongest guarantee is stated in
terms of proved semantic preservation for its compiler passes, while its
documentation also makes the exact supported language and compiler boundary
explicit. NMLT M9-006 does not claim such a compiler-correctness theorem; it
checks a supplied derivation against exact resolved HIR and explicit core:

- CompCert, [The CompCert verified compiler](https://compcert.org/).
- CompCert, [Compiler overview and guarantees](https://compcert.org/man/manual001.html).

Certificate-enhanced data-flow analysis supplies another receiver-side
pattern: an analyzer may emit certificates so a separate checker can validate
results with less work. It supports the architectural choice, but its results
do not prove NMLT's rules or implementation:

- Zhao et al., [Certificate Enhanced Data-Flow Analysis](https://arxiv.org/abs/1808.01246).

## Adopted consequences

1. The producer and checker are separate crates with a neutral certificate
   vocabulary between them. The checker does not call the elaborator.
2. The checker independently implements canonical derivation, certificate,
   ruleset, and policy encodings. Agreement with producer hashes is tested but
   not assumed.
3. The checker selects its accepted ruleset and resource policy. Certificate
   claims cannot weaken them.
4. Acceptance binds exact source-set, module-map, surface, resolved-HIR, and
   core identities before semantic replay.
5. The entire required-root DAG must be ordered, unique, closed, acyclic,
   bounded, reachable, and exactly cover HIR origins and core terms.
6. Every frozen rule and aggregate module/system/action record is reconstructed
   from HIR and compared to core. A test reseals a semantic forgery with valid
   fresh hashes and demonstrates rejection by rule replay.
7. `CheckedProgram` has no public constructor or fields. The sole construction
   path is successful kernel replay.

## Claim boundary and remaining work

M9-006 can establish that an exact explicit core is well typed and
well formed for the frozen M9-v1 fragment relative to the trusted parser,
projector, resolver, HIR/core representations, certificate model, kernel,
toolchain, host, and SHA-256. It does not establish temporal truth, operational
engine correspondence, full source-to-core semantic preservation, provenance,
or freshness.

The current boundary consumes an in-memory raw certificate. Persisted binary or
schema decoding, readback, and evidence-manifest identity binding are M9-008.
M9-007 must next make the provider engine consume `CheckedProgram` exclusively
and delete its second parser before the integrated execution claim can advance.

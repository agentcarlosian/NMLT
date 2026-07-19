# Independent reproduction record — 2026-07-18

- Result: passed at the bounded pre-alpha scope
- Reproduced implementation revision: `e3f7ec6ae2d14ade78183ff78d58f7198cb76858`
- Checkout method: `git clone --no-local` into a newly created temporary
  directory; no repository `target/`, Lean `.lake/`, or generated evidence was
  copied into the clone
- Working tree after the gate: clean
- Release decision: do not create a `0.1.0` tag yet

## One-command gate

After installing the pinned Rust and Lean toolchains and setting
`TLA2TOOLS_JAR` to the byte-identical TLC jar, the repository-level invocation
is:

```bash
ELAN_HOME=/tmp/nmlt-elan \
PATH=/tmp/nmlt-elan/bin:$PATH \
TLA2TOOLS_JAR=/absolute/path/to/tla2tools.jar \
make reproduce
```

`make reproduce` runs `make ci` and the pinned Lean metatheory gate. The
comparison step always checks exact TLA+/Quint/P source hashes; it executes TLC
when `TLA2TOOLS_JAR` is set and P when P 3.1.0 is installed.

## Environment

| Component | Reproduced value |
|---|---|
| Host | Ubuntu 24.04; Linux 6.17.0-35-generic; x86_64 |
| Rust | rustc 1.94.0 (`4a4ef493e`); Cargo 1.94.0 |
| Python | 3.12.3 |
| Shell/build | GNU Bash 5.2.21; GNU Make 4.3 |
| Node/npm runner | Node 25.8.1; npx 11.11.1 |
| Quint | 0.32.0 |
| Java | OpenJDK 21.0.11 |
| TLC | 2.19, revision `5a47802`; jar SHA-256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88` |
| Lean | 4.30.0, commit `d024af099ca4bf2c86f649261ebf59565dc8c622` |

The host, bootstrap downloads, npm registry/cache, linker, standard libraries,
and installed tool binaries remain residual trust as stated in the threat
model. Exact version strings and a TLC jar hash are not a signed supply-chain
attestation.

## Observed outcomes

- Rust formatting, workspace checking, Clippy with warnings denied, and all
  129 Rust tests passed from fresh build directories.
- All ten canonical examples retained exact source, intent, claim, control,
  entry, source-set, and corpus identities.
- Provider suite v2 reproduced one complete bounded `model_checked` reference
  and four deterministic refutations with the intended structured witnesses;
  benchmark mutation and assurance-laundering controls passed.
- Phase 4 temporal/lasso/refinement/runtime evidence, Phase 5 two-route finite
  VC evidence, Phase 6 authority/runtime graph, Phase 7 graded evidence, and
  all eight generic assurance manifests reproduced exactly.
- Lean built all ten jobs, including the complete eight-file NMLT statement
  import closure. Reported foundational dependencies remain the standard
  `propext` and `Quot.sound`; no project `sorry`, `sorryAx`, `admit`, custom
  axiom, or `native_decide` was accepted.
- The NMLT comparison exhausted five states and six transitions. TLC exhausted
  seven distinct states with no error. Quint 0.32.0 typechecked its source.
- P was not run because P/.NET was unavailable. Its corrected four-file source
  set is byte-bound, and the earlier pre-correction run is explicitly excluded
  as evidence for the current bytes.

## Promotion boundary

This record establishes reproducibility for the exact finite artifacts above.
It does not establish a stable language release, unbounded verification,
verified surface-to-core compilation, compiler-derived source-to-temporal or
source-to-VC mappings, deployed runtime attestation, or current P execution.
Those gaps are why the repository remains version 0.0.1 pre-alpha and no
`0.1.0` tag was created.

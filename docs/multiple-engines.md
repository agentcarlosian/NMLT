# Multiple verification engines

`nmlt-verify` is the Phase 5 boundary between a semantic claim, heterogeneous
verification engines, and evidence that a consumer may safely interpret.

The implementation is intentionally finite and exact. It supports Boolean
transition systems with state invariants. It does not claim that arbitrary
NMLT programs can already be translated into this IR soundly.

## Data flow

```text
typed model + claim + configuration identities
                    |
                    v
          canonical finite-safety VC
             /              \
 deterministic BFS     inductiveness enumeration
       |                       |
 trace or coverage     finite invariant certificate
    certificate
             \              /
              checked normalizer
                     |
       raw-preserving evidence composition
```

SMT-LIB, Lean, and generated-test adapters attach at the same VC boundary. An
adapter never assigns its own final assurance class.

## Constructing a VC

The ordered variables determine bit positions in finite valuations. For
example, bit zero can represent `authorized` and bit one `dispatched`:

```rust
use nmlt_verify::{BoolExpr, FiniteSafetyVc, VerificationConfig,
    VerificationIdentity, Sha256Id};

let config = VerificationConfig {
    finite_domain: true,
    terminal_stutter: true,
    assumptions: vec!["closed-system".into()],
};
let identity = VerificationIdentity {
    model: Sha256Id::digest(b"canonical model bytes"),
    claim: Sha256Id::digest(b"dispatch implies authorized"),
    configuration: config.identity(),
};
let property = BoolExpr::implies(
    BoolExpr::current(1),
    BoolExpr::current(0),
);
```

`FiniteSafetyVc::validate` checks variable names and indices, forbids
next-state references in initialization and properties, caps enumeration at
twenty Boolean variables, and recomputes the configuration identity.
`FiniteSafetyVc::digest` then binds all identities, ordered variables, and
formulas.

## Engine semantics

`ReachabilityEngine` performs deterministic breadth-first search. It returns:

- `refuted` candidate plus a shortest state sequence when it reaches a bad
  state;
- a `model_checked` candidate plus the sorted reachable-set coverage
  certificate after complete queue exhaustion;
- `unknown` when a declared state or depth limit truncates exploration.

`InductiveEngine` enumerates the entire valuation relation and asks whether
the property itself contains every initial state and is transition-closed. It
returns:

- a real initial-state refutation when initiation fails;
- `unknown` when consecution fails, because the offending predecessor may be
  unreachable;
- a finite-invariant certificate when both obligations hold.

These engines do not call one another and do not share result logic. Their
different positive classes are compatible: complete reachability gives
finite model-checking evidence, while an accepted induction certificate gives
proof of the exact finite VC.

## Checked artifacts

A refutation witness is accepted only after replay:

1. the first valuation satisfies initialization;
2. every adjacent pair satisfies the transition relation;
3. the final valuation falsifies the property.

The same narrow finite-invariant artifact is used as an independently checked
coverage certificate for reachability and as an inductive certificate for the
proof route. It is accepted only when its exact bindings and recorded state
count match and its sorted state set:

1. contains all initial valuations;
2. contains no property violation;
3. is closed under every transition.

Neither the producing engine's success code, `CompleteFinite` label, state
count, nor prose is trusted. Bare bounded or complete-finite `Holds` output
without a valid certificate normalizes to `unknown`.

## External backends

`encode_inductiveness_query` emits SMT-LIB 2.7 and requests a proof and solver
version. A plain `unsat` response remains `unknown`: only a returned certificate
under an accepted format can become `proved`. `sat` means the property is not
inductive; it is not automatically a reachable counterexample.

`export_lean4_inductiveness` emits finite definitions and a closed
`NMLTInductiveObligation` proposition. The export contains no proof hole or
axiom. `normalize_proof_assistant_return` treats text-only success as
`unknown`; the current checked return is the same finite-invariant
certificate.

The exact backend record includes name, version, executable/build digest,
protocol, and at least one trusted component with name, version, digest, and
role. The literal value `unknown` is invalid for an exact version field.

## Testing hook

`run_model_based_tests` uses a recorded seed, case count, and step bound to
generate reproducible traces. Passing produces only `tested`. A generated bad
trace can become `refuted` after the same independent replay used for an
explicit-engine witness.

This distinction matters: a high test count is not a finite-state proof and a
finite bound is not an unbounded theorem.

## Composition behavior

The composer retains every `RawEngineResult`, including stdout and stderr,
beside its `NormalizedResult`. It fails closed when:

- an identity is stale;
- a backend identity or TCB record is inexact;
- a witness or certificate is rejected;
- a backend fails or reports unknown;
- raw or accepted results disagree about hold versus refutation;
- no result is supplied.

The aggregate can be `proved` only if an accepted proof certificate exists and
every supplied positive scope is complete finite. A `Bounded` or `Sampled`
result cannot be promoted to `proved`, even if it carries a syntactically valid
certificate. A proof class applies to the exact VC, not automatically to the
NMLT source named by the caller: source-to-VC correctness is a separate
obligation.

## Running the Phase 5 boundary

The crate is part of the repository workspace:

```sh
cargo fmt --all --check
cargo clippy -p nmlt-verify --all-targets -- -D warnings
cargo test -p nmlt-verify --all-targets
python3 tools/check_multi_engine_evidence.py
```

The integration tests cover the two-engine success path and negative controls
for a forged certificate, disagreement, stale configuration, bounded-to-proved
promotion, inexact backend versions, absent certificates,
bounded/uncertified/sampled-to-model-checked promotion, and a
non-inductive-but-initially-true claim.

The persisted provider fixture checks the frozen
`ProviderAttempt.DispatchRequiresArm` formula through deterministic
reachability and independent finite inductiveness enumeration. It retains both
raw results, checks the finite invariant certificate, demonstrates a reachable
dispatch anti-vacuity trace, and exercises fail-closed disagreement and
bounded-proof-laundering controls. Its `proved` class is explicitly scoped to
the hand-constructed two-observable finite VC. The artifact records that the
full compiler translation is not yet verified and therefore does not promote
that result to a proof of the complete provider source.

## Research provenance

Focused archive searches on 2026-07-18 found no relevant local items, and live
arXiv collection was partially unavailable. The design was checked against
newly consulted primary sources: the official
[SMT-LIB 2.7 standard](https://smt-lib.org/papers/smt-lib-reference-v2.7-r2025-02-05.pdf),
[SMTCoq](https://arxiv.org/abs/1606.05947),
[DRAT-trim](https://www.cs.utexas.edu/~marijn/drat-trim/), the original
[SAT-based symbolic model-checking paper](https://doi.org/10.1007/3-540-49059-0_14),
and [QuickCheck](https://doi.org/10.1145/351240.351266).

The resulting NMLT-specific decision is conservative: external search is
useful, but only identity-bound artifacts checked by a narrow local boundary
may raise assurance.

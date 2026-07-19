# Canonical example corpus v1

Status: non-authoritative human-readable summary of the corpus frozen on
2026-07-18. The sole machine-readable authority is
[`canonical-v1.json`](canonical-v1.json), currently identified as
`nmlt-canonical-corpus-v1:sha256:7e57ecaa99607ff5fd292e7499d82b9bb38354cb77371b440018993301e63999`.
It fixes the exact paths, intent strings, claim handles, structured negative
controls, ordering, entry identities, and source-set identity. The table below
paraphrases some fields for readers and must never be used to calculate or
validate the corpus. Changing an authoritative field or source requires the
identity/version procedure defined by that JSON record.

These files are design fixtures. Until a later phase supplies the stated
oracle, their intended result is `unknown`; structural acceptance never
establishes the intended claim.

| ID | Source | Intent capsule | Intended claims | Required negative control | Intended oracle |
|---|---|---|---|---|---|
| C01 | `basics/boolean_toggle.nmlt` | Toggle one Boolean and remain in the Boolean state space. | `BooleanClosure` | An action assigns a non-Boolean value. | Type checker plus invariant checker |
| C02 | `hyperbook/one_bit_clock.nmlt` | Alternate a visible bit forever under declared fairness. | `BooleanState`, `KeepsTicking` | `tick` leaves the bit unchanged. | Invariant plus temporal checker |
| C03 | `math/euclid.nmlt` | Compute a GCD by terminating, invariant-preserving subtraction. | `GcdPreserved`, `Terminates` | A reduction subtracts the larger value from the smaller. | Function proof plus termination checker |
| C04 | `technicus/provider_attempt.nmlt` | Dispatch one authorized external effect and preserve ambiguity without replay. | `DispatchRequiresArm`, `SelectionRequiresPassingEvidence`, `NoBlindReplay` | Remove authorization, replay after ambiguity, corrupt response binding, or select failed evidence. | Type/resource checker plus model checker |
| C05 | `concurrency/two_process_mutex.nmlt` | Permit either process to enter while never admitting both. | `MutualExclusion`, `NoStarvation` | Remove the peer-critical guard. | Invariant plus fairness checker |
| C06 | `refinement/bounded_channel.nmlt` | Implement an abstract FIFO stream through a hidden bounded buffer. | `CapacityBound`, `FifoPrefix`, `Delivery` | Receive from the tail instead of the head. | Refinement plus temporal checker |
| C07 | `agents/trust_chain.nmlt` | Allow effects only when authority provenance reaches a human grant. | `NoSilentAuthorityWidening` | Treat agent attestation as a human grant. | Information-flow and capability checker |
| C08 | `runtime/durable_controller.nmlt` | Relate a crash-safe journal to an at-most-once external effect. | `JournalBeforeEffect`, `AtMostOnceEffect`, `NoBlindReplay` | Retry an indeterminate dispatch. | Model checker plus concrete trace refinement |
| C09 | `distributed/two_phase_commit.nmlt` | Reach one terminal decision and teach participants without disagreement. | `Agreement`, `CommitRequiresPrepared` | Commit before every participant prepares. | Distributed-state model checker |
| C10 | `resources/token_bucket.nmlt` | Admit work only within an explicit quantitative budget. | `Capacity`, `NoOverdraft`, `Accounted` | Admit a cost larger than the token balance. | Graded type checker plus invariant checker |

Source identities use the rules in `docs/artifact-identity.md`; the
machine-readable corpus is regenerated or verified with
`tools/canonical_examples.py`.

## Plain-language witness obligations

Every negative control must eventually produce a structured witness explaining
the first semantic divergence. For temporal claims, a finite prefix alone may
refute safety but may not establish unbounded liveness. For refinement claims,
the witness must include both the concrete step and the failed abstract match.

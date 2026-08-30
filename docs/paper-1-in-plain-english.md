# Paper 1 in plain English

**For:** sus magi  
**Date:** 2026-08-27  
**Status:** Orbit A core frozen enough to write against; residual math deferred

## What this project is trying to say

NMLT treats programs as *behaviors* you can compose. Refinement means “the concrete thing is a valid stand-in for the abstract thing.” Composition means “plug two behaviors together on matching ports.”

The hard question: if A is a good stand-in for B *alone*, is “A plugged into D” still a good stand-in for “B plugged into D”?

If the answer were always yes, composition would be easy. It isn’t always yes.

## The bug we checked (the punchline)

Imagine a sender that can `ping`, and a receiver that flips an observable bit when it gets the ping.

Locally, you can classify `ping` as “hidden” (internal noise) under a one-step refinement mapping with stuttering (not classical weak simulation) and still say the sender refines a do-nothing abstract sender.

After you wire them together, that “hidden” ping is no longer private: it forces the receiver to move, and the *pair* changes what you can observe. So the composed concrete system is *not* a refinement of the composed abstract system.

**Moral:** something you hide locally is not automatically silent in a larger system if it’s still connected to a peer.

That’s Theorem T3 in Lean. It’s the paper’s main negative result.

## The repair (what we’re allowed to claim positively)

For the small mathematical model in the Lean core files, we proved a conditional positive theorem (T5):

If you refuse to hide any connected action, the wirings match, and label renaming doesn’t smash a free action onto a busy name, then left-side refinement *does* lift through composition (observation-safety / finite observation-trace inclusion). Those three premises are independently indispensable for the default lift—not “necessary” in a stronger universal sense.

We also have:

- a positive example where `ping` is visible (same wiring, lift works)
- a positive example with a private internal step and *no* wires (lift works)
- a negative example where label renaming collides names (lift fails even though wires “match”)
- a negative example with an extra abstract wire (wiring coverage fails; lift fails)

There’s also an older *strong* / exact-action congruence theorem (T4) in a richer OpenComposition model. It’s real, but it’s a different setting—not residual C1. It does not secretly prove the full weak story with resources and fairness, and T5 does not close full RFC 0008.

## Residual C1, first slice (authority is not silent either)

The same hidden ping still observation-refines the do-nothing sender (T1). If that ping consumes a capability `token` the abstract profile does not, resource refinement fails (`hiddenPing_consume_breaks_resourceRefinement`). Hiding is not contextual silence for authority. That is I-CAP independence from T5, not a full resource/grade/fairness lift. The same shape holds for grades: a hidden ping with a real cost against a silent abstract stutter fails `ResourceRefinement.grade` (`hiddenPing_grade_breaks_resourceRefinement`). That is I-GRADE independence from T5, not a positive grade lift and not I-FAIR / I-RELY. The same shape holds for assumptions: a hidden ping that relies on an extra `ready` fact the silent abstract stutter does not rely on fails `ResourceRefinement.rely` (`hiddenPing_rely_breaks_resourceRefinement`). That is I-RELY independence from T5, not a positive rely lift and not a discharge through compose. If instead the hidden ping matches the silent abstract stutter (no extra consume, no extra cost, no extra rely), resource refinement holds (`hiddenPing_inert_resourceRefinement`); the three fails are independent of that matching case.

Observation lift through compose is not resource lift through compose. T5 already lifts the empty-wiring hidden tau as an observation (`emptyWiring_productRefinement`). If that tau is decorated with a `token` consume the abstract stutter does not have, the *product* still observation-refines, but product resource refinement fails (`emptyWiring_hiddenConsume_breaks_productResourceRefinement`). The same shape holds for grades: a hidden left tau with a real cost against a silent abstract stutter fails product `ResourceRefinement.grade` (`emptyWiring_hiddenGrade_breaks_productResourceRefinement`). The same shape holds for assumptions: a hidden left tau that relies on an extra `ready` fact the silent abstract stutter does not rely on fails product `ResourceRefinement.rely` (`emptyWiring_hiddenRely_breaks_productResourceRefinement`). If both sides use the matching inert stutter, the product resource profile does lift (`emptyWiring_inert_productResourceRefinement`) — the first compose resource homomorphism on this small model. That product pack (inert positive + consume/grade/rely negatives) is frozen. That is not I-FAIR, not hidden-connected ping, not OpenComposition, and not the full RFC 0008 CONDITIONAL-CONGRUENCE story.

Visible sync is a different independence. T5 already lifts VisibleSync as an observation (`visibleSync_productRefinement`: visible ping wired to Lean `receive`). If that ping transfers a `token` the receiver does not receive, `SynchronizationCompatible` fails (`visibleSync_unmatchedTransfer_not_synchronizationCompatible`) even though the product still observation-refines *and* product resource refinement still holds — because the sync action zeros transfer/receive. Matching transfer is the extra I-CAP premise (`visibleSync_matchedTransfer_synchronizationCompatible`); T5 does not imply it. Shared ownership of the same `token` independently fails `CapabilityPartition` (`visibleSync_sharedOwnership_not_capabilityPartition`). Grades through *sync* are independent too: a visible ping with a real cost against silent receive/abstract epsilon fails product `ResourceRefinement.grade` at the `.sync ping receive` label (`visibleSync_hiddenGrade_breaks_productResourceRefinement`); matching epsilon grades already lift that sync inequality (`visibleSync_matchedTransfer_syncGrade`). Fairness through sync remains open.

The same hidden ping can also run forever. Weak refinement only asks that a hidden step stay at the same abstract state, so ConcreteSender can `ping` at every step of a `Nat → state` path and never leave the unique abstract state. Every finite prefix still looks like a stutter-expansion of the abstract observation (T5's finite-trace corollary), which is why finite inclusion does not rule out hidden divergence. That is the first I-FAIR negative (`hiddenPing_divergence_not_discharged_by_weakRefines`): liveness still needs a separate divergence/fairness obligation. It is not a liveness theorem and not fairness transport through composition.

## What we are *not* claiming yet

- Full “everything in RFC 0008” (capabilities, grades, fairness, liveness)
- That the contest demo / Rust verifier is the research contribution
- That we’ve finished a journal paper ready to submit without more writing

## Two receivers, two models (do not mix them)

The Lean paper uses a small receiver: `receive` is on only while the bit is `false`, then it flips to `true` and stays off. That is the math. The executable OpenSystem checker is stricter: every input has to stay enabled in *every* local state (I/O-automata-style receptiveness). So the Lean receiver is right for the paper and *rejected* by the OpenSystem checker after the bit flips.

There is a second fixture, not a replacement: `examples/paper1/receptive_receiver.nmlt` still accepts `receive` after the bit is already true (`set bit = true` with no `require`, so true stays true). That dual is what the executable VisibleSync path can accept. It is not the Lean small model. Accepting it does not mean the Lean receiver is receptive, and OpenSystem is not the paper small model.

## Local pipeline (2026-08-27)

Parse of `examples/paper1/hidden_ping_receive.nmlt` → boolean sketch → `OpenSystem` now drives two finite checks: InvalidHiddenPing is rejected with `HiddenConnectedAction("ping")`; VisibleSync (visible ping, no hide, wired to Lean `receive`) has an accepted local identity refinement and an accepted one-wire product refinement (peer `bit` false→true on both products). The OpenSystem congruence checker still does not accept that Lean product: sketched `receive` is not enabled after the bit flips, and CompatibilityChecker requires inputs in every local state (receptiveness). The OpenSystem-receptive dual is accepted with visible ping. That is not M9 compile, not general LTS, and not residual C1.

## Artifact note (2026-08-27)

T5 and the independence lemmas may still be local-only relative to GitHub `main` @ `0417f6e`. Frozen paper SHA stays a placeholder until you approve push/tag.

## Decision (2026-08-27)

**Freeze** the Orbit A mechanization core as the mathematical spine of Paper 1.

**Next work** should make the paper readable and self-contained (prose, figures, PDF), not invent new big theorems tonight.

Later research map (not blocked, just not the immediate push): resource-aware congruence, language surface = Lean theorems, evidence calculus thickness.

## If you only remember three sentences

1. Local hiding + live connection can break composition; we proved that.
2. Under clear interface rules, a small-model repair works; we proved that too.
3. The rest of the research program is real, but Paper 1 should ship this spine cleanly rather than swallowing the whole map.

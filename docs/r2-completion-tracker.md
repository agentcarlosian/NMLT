# R2 completion audit

**Completed on 2026-09-09 at local, finite, pre-alpha scope.** This covers the
full R2 requirement set in `Plan.md` and `docs/practical-language-plan.md`.
See the [validation record](reviews/r2-completion-2026-09-09.md) for the complete
reproduction, final follow-up CI and retained evidence. RFC acceptance,
publication and independent human review remain separate.

| Requirement | Implementation and validation |
|---|---|
| Common parsing, resolution, spans and semantic disposition | Canonical lossless projection reused; finite/workflow routes and their authority are explicit; frontend, package and diagnostic regressions pass |
| Shared interpreter/explorer steps | Common finite initializer/successors; unchanged Rust/Lean graphs of 4 states/10 transitions and 8 states/12 transitions |
| Entries, modules, functions, values and iteration | Acyclic parameterized functions, imports, records, outcomes, matching and bounded lists/folds; finite enums are supported by the behavior profile |
| Typed Lean and worker controls | Start/poll/cancel/collect; variable closed Init statements/proofs; actual rejection/fallback, axiom policy and replay pass |
| Affine ownership and bounded identities | Moves through functions, branches and folds; private one-time restored handles; bounded slots/generations and stale-result controls pass |
| Budgets and settlement | Interrupted work stays charged; no reset or duplicate dispatch/collection; never-dispatched reservations cancel without a child or charge |
| Recovery and explicit uncertainty | Durable intents/replies and observations; source/project resume; explicit operator failure acknowledgement; nine saved boundaries, timeout/retry, tail quarantine and tamper controls pass |
| Process containment and dependencies | Recorded ProcessKit tree/resource policy; native Windows cleanup, parent-death and memory/process tests pass; every Lean request binds complete bin/lib identities |
| User-defined safety invariant | Exact predicate/declaration binding; proved enumeration completeness and initialization/preservation; actual invariant/counterexamples and 15 independent Lean rejection controls pass |
| Project loop | Init/lock/run/test/replay/resume/fmt/check-project; real input changes, fallback/reuse, source snapshots and 14,972-file Lean installation lock tested |
| Structured diagnostics | Shared CLI envelope, located workflow type errors, expected/actual types and related declarations; unsupported profile forms remain explicit errors |
| Real-input acceptance workflow | Complete worker and Lean project runs, tests, replay and saved-result resumption pass without JSON editing |
| Completion evidence | Integrated reproduction exited 0; final follow-up CI exited 0 with 345 Rust tests and 14 Python tests; NanoDA checked 9,004 declarations without errors |

The integrated proof/Lean gates preceded the final restored-reservation
cancellation guard; the full Rust CI and Linux compile check were rerun after
that guard. The [validation record](reviews/r2-completion-2026-09-09.md) identifies
both revisions' evidence precisely. Linux compilation is not a claim of Linux
runtime execution. Workflow effects remain executable-only, and model safety
does not establish host correspondence or exactly-once external effects.

Supported use: [source jobs](r2-source-async.md), [proof terms](r2-lean-terms.md),
[recovery](r2-recovery.md), [projects](r2-projects.md) and
[safety invariants](r2-safety-invariants.md).

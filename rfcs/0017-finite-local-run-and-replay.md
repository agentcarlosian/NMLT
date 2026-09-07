# RFC 0017: Finite local run and replay

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-06
- Tracking issue: not assigned
- Design disposition: experimental R2 first increment; acceptance pending

## Problem and scope

R1's `trace` first enumerates a graph and then selects a path from that graph.
Start R2 with direct bounded execution from `.nmlt` source, sharing the existing
initializer and successor calculation with exploration. Record and replay the
complete finite run without asking users to construct core JSON or path JSON.

This implements a small executable-only policy layer over the existing v2
finite operations. It does not accept the broader
[workflow profile](0014-executable-workflow-profile.md), change ADR 0004's
normative semantics, or complete R2. Host jobs, modules, general values,
functions, invariants, project locks, and crash recovery remain separate work.

## Interface and source interpretation

```text
nmlt run <source.nmlt> --behavior <name> --max-steps <n> --emit-run <new.json>
         [--actions <comma-separated labels>]
nmlt replay <record.json> --source <source.nmlt>
```

Options follow the source and may appear in any order. Each option appears at
most once. Required bounds are 1–10,000 steps; an explicit action list is also
limited to 10,000 entries. An empty list records initialization alone; empty
entries inside a nonempty list are errors. Bounds include self-loops.

| Construct | Source route | Interpretation or validation |
|---|---|---|
| Existing declarations, finite expressions, ports, ownership, grades, hiding, binary connections | `nmlt-core` lossless syntax/projection → `nmlt-compile::compile_behavior_v2` → `nmlt-ir::BehaviorCoreProgram` | Existing v2 finite interpretation in Lean; this command does not invoke Lean |
| Run entry | CLI `--behavior`, resolved to one leaf or a pair of distinct named leaves | Rust executable-only selection; no new source entry declaration |
| Scheduler and step limit | `nmlt-eval::execute` over shared initialization/successors | Executable-only policy; no fairness or liveness theorem |
| Run record, executable identity, replay | CLI `runtime` module, typed Serde data | Same-implementation consistency; no proof artifact |
| Unsupported source constructs | Existing behavioral compiler rejection | No fallback to the retained ordinary typed-core route |

Common parsing here means reuse of the existing finite route, not completion of
the broader source-route consolidation. No new grammar or type rules are added.

## Step and stop rules

Without `--actions`, select the lexicographically first enabled action label.
Connected labels retain the compiler's canonical endpoint order. With an action
list, select its next exact label. Duplicate identical edges follow exploration's
deduplication policy; an ambiguous selected label is an evaluator error.

Initialization and all enabled-step operations are shared with exploration:
guards, simultaneous field updates, finite value typing, local consumption,
synchronized transfer, reception freshness, and v2 unmatched-transfer rejection.
The runner does not enumerate reachable states to obtain a successor.

Each recorded step contains the label, resulting full state and authority,
model grade, and transfer descriptions. Hidden steps remain in this diagnostic
trace and count against the step bound. This is not an observation quotient.
An absent authority map entry means unowned; it does not distinguish consumed
from never allocated. Full state and declared observations are distinct.

Stop rules are evaluated in this order:

1. An exhausted explicit action list yields `schedule_complete`.
2. Compute enabled successors. Automatic scheduling with no successor yields
   `quiescent`, including exactly at the step bound.
3. At the step bound, yield `step_limit` before another requested action.
4. An unavailable requested action yields `action_unavailable`, retaining its
   name, currently enabled labels, and the already executed prefix.
5. If adding the next step's grades would overflow `u64`, yield
   `grade_overflow` before committing that step or any partial grade sum.
6. Commit the selected finite transition and continue.

The sum is a modeled annotation, not observed host expenditure, reservation,
refund, or enforced work quota. Existing per-step composition overflow and other
evaluator errors still fail the command without a record. The record does not
claim to be a write-ahead journal.

`run` writes a structured result to stdout and a newly created file.
`quiescent` and `schedule_complete` return exit zero; the other recorded outcomes
return nonzero with a partial trace. Quiescence only means no enabled step, and
schedule completion only means the requested prefix ran. Neither establishes
that useful work succeeded. Invalid options or failed compilation produce no
run record. Existing output files are never overwritten. An interrupted or failed
file write may leave an incomplete new file; replay must reject incomplete JSON.

## Record and replay contract

`nmlt-finite-run-v1` carries the `finite-v2-local-v1` profile, `assurance: none`,
SHA-256 of the current executable, logical source path, exact source-byte hash,
canonical v2 artifact hash, behavior name, scheduler/bound configuration,
initial state, steps, cumulative model grade, and final outcome. The typed
definitions live in the [CLI runtime](../crates/nmlt-cli/src/runtime.rs) and
[interpreter](../crates/nmlt-eval/src/interpreter.rs). A separate interoperable
JSON Schema is deferred until the experimental record shape is reviewed.

Replay requires the exact executable bytes and exact source bytes. It compiles
the explicit local source using the recorded logical source name, reruns the
recorded policy, and compares the complete typed record, including identity,
states, authority, steps, grades, and outcome. Moving identical source is allowed;
the record cannot direct the CLI to open an implicit source path. Unsupported
versions/profiles, unrecognized fields, and any inconsistent reconstructed data
are errors. Whitespace and JSON object ordering are not significant.

Replay exits zero when an incomplete run is accurately reproduced, preserving
that incomplete outcome in its own JSON response. Its `matched` field means
record consistency, not successful execution. Changing executable builds,
platforms, or source requires a new run; no migration is implied.

Hashes identify bytes; they do not authenticate who recorded them or establish
compiler correctness. Consistently rewriting an entire unsigned record to
describe another valid run is not prevented. Binary identity also does not
pin shared OS libraries or establish a full project/toolchain lock. Retain the
executable and its compatible environment for replay. No host effect is repeated,
and no theorem, finite model-check result, or runtime authorization is issued.

## Examples and controls

The [finite retry example](../examples/pivot/finite_retry.nmlt) transfers a permit,
takes a simulated failure and one retry, consumes the permit on completion, and
copies a persistent Boolean result after authority is gone. Changing the initial
Boolean skips failure and retry. These are finite state transitions, not a host
worker adapter or a completed R2 workflow.

Tests cover both input paths, blocked early use, use after transfer, repeated
consumption, retransfer, self-loops, all 22 edges in the frozen value/resource
graphs, grade overflow, invalid bounds, stale source/executable identities,
changed artifact/state/authority/grade/outcome, unknown fields, incomplete runs,
and output collision. Existing v1 artifacts and v2 path fixtures remain unchanged.
The existing Rust/Lean graph comparisons must pass after the shared-step refactor.

## Alternatives and remaining gates

Keeping graph-first `trace` alone cannot execute a path without paying for
exploration. A separate interpreter implementation would duplicate the most
sensitive finite operations. General workflow syntax and host effects now would
combine several still unresolved semantics in one change.

Before any stronger runtime claim: specify bounded job identities and settlement,
add adapters and their recovery controls, extend the source and value profile
with explicit lowering, implement user predicates, and establish the applicable
abstraction/validation boundary. RFC acceptance and independent publication review
remain separate gates; this increment makes no new Lean theorem claim.

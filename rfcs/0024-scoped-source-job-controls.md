# RFC 0024: Scoped asynchronous source job controls

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-07
- Design disposition: executable-only R2 increment; acceptance pending

## Problem and scope

RFC 0023 exposes concurrent worker and Lean jobs only to Rust callers. Source
workflows need the same explicit start, observation, cancellation, and collection
operations, with static rejection of reused job authority and replay of the
actual control decisions. This increment exposes lexically scoped handles.
Transferring handles through functions, aggregates, return values, or loop
accumulators, general Lean proof input, process-tree containment, dependency
locks, and automatic source resumption remain separate work.

## Source and typing

`let h = job_start_square(input); body` introduces `h: Job<Int>`.
`let h = job_start_lean("existing_lemma"); body` introduces `h: Job<Text>`.
Lean strategies are the four literal names `wrong_term`, `existing_lemma`,
`induction`, and `admitted`, referring to the unchanged RFC 0023 templates.
Unknown or computed strategies are rejected during compilation.

`job_poll(h): Bool` borrows the handle and reports whether it is ready.
`job_cancel(h): Bool` borrows it and returns whether cancellation won; already
settled results win the race. `job_collect(h): Outcome<T>` waits within the
configured host deadline, then consumes the handle and collects exactly once.
Confirmed cancellation is an ordinary `Err("job cancelled")`. Uncertain host
outcomes stop evaluation and preserve the runtime's charged, unresolved work.

Handles must be introduced directly by a let-bound start call and supplied
directly by name to a control operation. They cannot be copied, compared,
serialized, returned, supplied as entry inputs, stored inside other types, or
passed to functions. Ordinary collected values remain freely reusable. All
normal paths must collect each scoped handle; early evaluator stops retain the
session for bounded cleanup and recovery inspection. This collection discipline
is deliberately stronger than affine weakening.

The checker tracks available/consumed handle state at each lexical local index.
Both branches must leave identical states. Short-circuit right operands must
leave outer handle states unchanged. Fold bodies must preserve outer states
across iterations; they may create and collect iteration-local jobs. These
checks apply to unused functions and branches as well. Existing transitive
job-effect inference includes the new operations.

## Lowering and execution

The canonical declaration projection remains the parser entry point. The typed
workflow tree gains start/control nodes; scoped handles have separate evaluator
bindings and never enter the serializable `Value` type. A bounded explicit host
owns the actual non-cloneable session handles. Source positions accompany every
operation. The CLI requires explicit slots, attempts, timeout, and new evidence
paths; Lean jobs additionally require an explicit pinned Lean executable.

The existing durable session remains responsible for reservation, dispatch,
observation validation, late-result rejection, cancellation, slot generations,
collection, timeout, and independent direct-child deadlines. Source control
does not strengthen its process-isolation or exactly-once claims.

## Evidence and compatibility

The new asynchronous source record has its own versioned schema. It binds source
and import bytes, typed program, entry and inputs, exact CLI and tool identities,
bounds, ordered source control requests/results, and a session snapshot. Replay
validates the snapshot and each operation's journal position and result, then
re-evaluates the source against those captured operations without launching jobs.
Pending polls and completion/cancellation races are recorded observations, not
deterministic timing claims. Extra, missing, reordered, misbound, or inconsistent
operations fail replay. Recovery classifies the saved session without restoring
source handles or redispatching.

The earlier finite, pure, and synchronous source routes keep their formats.
The added reserved built-in identifiers and `Job` type are a source compatibility
change. Retain the original executable for older exact-executable records.
This entire extension has `assurance: none`; it adds no Lean interpretation of
source workflows and changes no behavioral artifact or theorem. A Lean receipt
identifies a submitted fixed template, not a source-language proof object.

## Validation and open gates

Positive cases start two jobs before collection, poll, handle failure and bounded
fallback, reuse outputs, cancel, and reuse a collected slot. Negative controls
cover duplicate collection, use after collection, branch and short-circuit
imbalances, outer-handle consumption in folds, aliasing, aggregate escape,
forged inputs, missing host configuration, exhausted bounds, and altered replay
requests, outcomes, journal positions, source identities, and snapshots.

The Rust gates and real worker exercise are required. Real pinned Lean execution
must be reported separately from mock-host tests. Broader R2 invariant, project,
recovery, and Lean-input work remains open; this RFC does not complete R2.

## Native Windows validation tooling

The full gate also needs small host adaptations on native Windows: Make invokes
shell scripts through Bash, Python can be selected with `PYTHON`, executable
discovery handles `.exe`, and the execution-control harness passes forward-slash
paths to the existing digest checker. Rejection verdicts and proof inputs remain
unchanged.

The complete Lean declaration list exceeds Windows' command-line limit.
`tools/ExportFromFile.lean` imports the pinned exporter's unchanged `Main` and
calls its entry point with the same ordered roots read from a file. Before
exporting the complete set, the gate compares this adapter's output byte for
byte with the upstream executable on eight actual roots. The adapter streams
stdout directly to the export file, avoiding the elaborator's message buffer.
The adapter belongs to
the existing kernel-audit tooling trust boundary; no exporter algorithm, checker
pin, axiom policy, or NMLT Lean definition changes. Optional successful evidence
retains the adapter and parity inputs/outputs alongside the full export.

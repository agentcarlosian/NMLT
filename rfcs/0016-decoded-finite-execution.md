# RFC 0016: Decoded finite execution

- Status: Under review
- Created: 2026-09-06
- Scope: R1/M2–M3, opt-in finite binary execution

## Contract

`behavior-core-v2` retains the v1 finite control AST and introduces two required
maps: `known_capabilities` maps each system to its nominal capability types;
`initial_authority` maps each system/composition to a complete capability-to-owner
map, using JSON null for vacancy. Both maps must equal their independently
derived values. The capability universe is the union of all known names.

A system's known names are exactly its initial `capabilities` declarations plus
the affine parameters of its input actions. Repeated names must have the same
type, including across systems. A known name grants no ownership. Later actions
may consume or output it, but an actual step requires the actor to own it.
An input may reacquire initially owned authority after an earlier transfer;
reception while already owning it is dynamically disabled.

Initial ownership for a selected behavior comes only from its actual leaves.
Alternative systems elsewhere in the file contribute names/types but no owners.
This version supports a system or a binary composition of distinct named systems;
repeated instances and nested source compositions remain unsupported.

The default compiler and v1 decoder keep their existing rules and exact bytes.
V2 requires an explicit compiler option and decoder entry point. Migrating a v1
source means recompiling it with that option; changing its schema string alone
does not supply the required maps. Unknown versions fail explicitly.

## Execution witness

`behavior-execution-v1` is a separate untrusted finite path containing
`artifact_sha256`, `behavior`, `states`, and `actions`. There is one more state
than action and at least one state. Each state has `left` and `right` control
indices and a complete `authority` map. Each action has nullable `left` and
`right` action names; both present means synchronization, one means an isolated
step, and neither is invalid. This witness profile selects a binary composition.

Control indices enumerate the decoded finite domains in field-name order:
Bool false/true, Unit, or enum constructors in lexical order, with the last
field varying fastest. Indices outside the actual domain are rejected; the
decoder's sentinel is never a valid witness state or action. Owner names must
be the selected left/right systems. All capability names must be present,
including vacant names; extra names and owners are rejected.

Lean constructs the unified `ResourceDynamics.parallel` behavior, checks its
formation, the actual initializer, and every complete control/world step, and
retains proofs of the resulting finite path and reachability. Claimed state
updates and ownership changes are not trusted. Paths are independent of any
refinement pair; optional refinement statements retain their separate meaning.

Declared refinement applications are checked separately. If the selected path
starts with synchronization and has a matching concrete application,
`ExecutionLift` constructs an initialized abstract step through the unified
lifting theorem. This first bridge requires component/peer order to match both
compositions, equal initial worlds under that explicit owner correspondence,
and exact agreement with the decoded application wiring. It does not implement
general owner renaming or transport the entire supplied path.

Source and artifact SHA-256 comparisons bind supplied bytes; neither establishes
correct compilation. Rust produces witnesses from reference exploration and
continues to report `assurance: none`. Open transfers/receives cannot execute
without a peer in the v2 completed-step explorer. V1 exploration is retained
under its existing non-authoritative interpretation.

The compiled Lean JSON decoder and decision procedures remain trusted at runtime.
NanoDA checks the exported package declarations, including the equivalence
between the finite step test and the unified relation. The executable does not
export a separate kernel-checkable proof artifact for each accepted path.

## Required evidence

Reproduce v1 and v2 fixtures, check decoded synchronized transfer, receive then
consume, and receive then retransfer paths. Include rejected initial-world,
control-index, unknown-owner, wrong-action, early-use, repeat-use, retained-owner,
fabrication, stale-artifact, and unsupported-version cases. Compare the bounded
Rust reachable graph with Lean's actual relation, including local, synchronized,
and hidden steps. Prove finite-path vacancy preservation and explained authority
changes using the same unified step relation. No runtime, liveness, or general
compiler correspondence claim follows.

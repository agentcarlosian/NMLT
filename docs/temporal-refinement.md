# Temporal, refinement, and runtime checker

`nmlt-temporal` is the executable Phase 4 candidate for finite temporal graphs.
It provides deterministic lasso witnesses, explicit action fairness, finite
forward-simulation diagnostics, and three-valued runtime trace conformance.

It is deliberately independent of parsing and elaboration. A caller must build
a `FiniteGraph`, bind predicates and mappings, and preserve those identities in
any evidence envelope. See [RFC 0009](../rfcs/0009-temporal-refinement-runtime.md)
for the formal scope and research basis.

## Graphs and identity stutter

```rust
use nmlt_temporal::{FiniteGraph, ModelState, Transition, Value};
use std::collections::BTreeMap;

fn state(done: bool) -> ModelState {
    BTreeMap::from([("done".into(), Value::Bool(done))])
}

let graph = FiniteGraph::new(
    vec![state(false), state(true)],
    vec![0],
    vec![Transition::action(0, "finish", 1)],
)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Construction validates indices, sorts and deduplicates transitions, and keeps
stable numeric identifiers. Temporal checking automatically gives every state
an action-free identity self-loop, including states with enabled actions. This
means progress requires an explicit fairness assumption. A declared internal
action is never an identity stutter merely because public observations do not
change.

## Eventuality and fairness

```rust
use nmlt_temporal::{Fairness, FairnessSet, TemporalChecker, Value};

let checker = TemporalChecker::new(
    &graph,
    FairnessSet::new(vec![Fairness::weak("finish")]),
);
let report = checker.eventually(|state| {
    state.get("done") == Some(&Value::Bool(true))
});
assert!(report.holds());
```

Use `Fairness::weak(a)` when continuous enabledness of `a` must imply eventual
occurrence. Use `Fairness::strong(a)` when infinitely recurring enabledness must
imply occurrence. Do not add either assumption unless the scheduler or
environment contract justifies it.

`eventually` and `leads_to` return `CheckOutcome::Violated` with a `Lasso` when
they find a fair avoiding behavior. The lasso contains both state and transition
identifiers. Call `is_well_formed` before rendering or replaying a witness.

## Observation and refinement

```rust
use nmlt_temporal::{ActionHiding, ObservationMap, RefinementChecker, RefinementSpec};

let spec = RefinementSpec {
    state_map: vec![0, 1],
    concrete_observation: ObservationMap::identity(["done"]),
    abstract_observation: ObservationMap::identity(["done"]),
    actions: ActionHiding::new([("finish", Some("commit"))]),
};
let report = RefinementChecker::check(&concrete, &abstract_graph, &spec);
```

Every concrete action must be explicitly mapped to an abstract action or to
`None` (hidden). A missing entry is an error. A hidden step must map both
endpoints to the same abstract state. Observation equality alone does not make
it hidden.

An accepted report means that initial states, observations, and one-step
forward simulation hold for these finite graphs. It does not establish fairness
transport, absence of hidden divergence, resource preservation, or
compositional refinement.

For finite snapshot words, `stutter_project` removes adjacent duplicate
observations, and `stutter_equivalent` compares the results. Keep the original
intensional action/event/resource trace alongside any projection.

## Finite open-system composition

The `open` module adds the first M11 safety-only profile. An `OpenSystem`
bundles a finite graph with a total action interface and exact symbolic safety
claims. `ActionSignature` classifies every graph action as input, output, or
internal; boundary actions also name an opaque channel identity.

`CompatibilityChecker` requires every declared input to be enabled at every
model state. Each explicit connection must pair complementary input/output
actions on the same channel and must be one-to-one. The current strict contract
profile also requires every local assumption identifier to be discharged
exactly once by the other component's identically named guarantee, whose
provider must have no assumptions of its own. Mutual or otherwise conditional
symbolic discharge is rejected. Identifier equality is only a declared link,
not a proof of logical implication.

`compose_open_systems` constructs the cartesian finite product. Connected
actions synchronize and become internal; they cannot also interleave.
Unconnected actions and state fields receive deterministic `left::` or
`right::` namespaces. The construction preserves remaining external actions
but the strict executable profile closes all symbolic assumptions. Checked
arithmetic and default caps of 100,000 states and 1,000,000 generated
transition candidates make oversized products fail explicitly. A conservative
50,000,000 work-item preflight bounds the module's own compatibility and
product-enumeration loops; it is not a wall-clock, byte, allocator, or
standard-library comparison/sort budget.
`compose_open_systems_with_limits` accepts smaller caller-selected bounds.

`OpenRefinementCongruenceChecker` first checks the local finite refinement,
both compositions, polarity/channel preservation, the no-hidden-boundary
condition, injectivity of visible boundary mappings, and exact connection
preservation in both directions. Bidirectional coverage is essential: an extra
abstract connection could otherwise block a peer-only action that remains
independent in the concrete product. Only after those checks pass does it build
and check the lifted product refinement.

Lean separately proves structural product congruence for an abstract
exact-action, state-surjective profile from `StrongRefinement` and equality of
the complete wiring relations. Predicate-contract compatibility and global
input receptiveness establish a separate composability result and product
receptiveness; they are not premises needed by the structural step-lifting
half. Lean wiring may be an arbitrary relation, whereas the Rust profile is
one-to-one. The Rust and Lean representations are not yet connected by a
correspondence theorem. Neither artifact covers temporal contract satisfaction,
payload subtyping, affine capabilities, grades, fairness, hidden divergence, or
liveness transport. The claim-specific
[M11 evidence manifest](../benchmarks/results/open-composition/m11-001a-evidence.json)
binds the frozen theorem handles, exact positive and negative controls, source
set, Lean toolchain, checkers, and audited axiom sets. See the
[M11 research note](research-notes/m11-open-system-refinement-2026-07-19.md)
and [RFC 0008](../rfcs/0008-mechanization-and-compositional-refinement.md).

## Runtime journals

```rust
use nmlt_temporal::{
    JournalAction, JournalRecord, JournalValue, RuntimeMapping,
    RuntimeTraceAdapter, RuntimeVerdict,
};

let mapping = RuntimeMapping::identity(["done"]);
let journal = vec![JournalRecord {
    sequence: 42,
    action: JournalAction::Initial,
    observations: BTreeMap::from([
        ("done".into(), JournalValue::Known(Value::Bool(false))),
    ]),
}];
let report = RuntimeTraceAdapter::new(&graph, &mapping).check(&journal);
assert_eq!(report.verdict, RuntimeVerdict::Accepted);
```

Record zero must use `Initial`; each later sequence number must increase by one
and names the exact intervening action, an explicit identity stutter, or
`Unknown`. Required mapped observations may be `Known`, `Unknown`, or absent.

- `Accepted`: complete known data has a matching path.
- `Rejected`: known data or journal structure contradicts every path.
- `Unknown`: compatible paths remain, but required action or state data is
  unobserved.

A known contradiction wins over unrelated uncertainty. Rejections localize the
first failing record and list the candidate states immediately before it.
Runtime acceptance is finite-prefix conformance; it is not an eventuality result
or an attestation that logging was complete.

## Evidence checklist

Before treating a report as durable evidence, bind the canonical identities of
the graph, predicates, fairness set, observation/action/state maps, checker
artifact, and (for runtime checking) journal plus completeness assumptions.
The current closure-based predicate API has no portable canonical identity, so
raw library output should remain diagnostic until the engine supplies one.

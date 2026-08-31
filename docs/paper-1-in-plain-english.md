# Paper 1 in plain English

NMLT's first active theorem asks when a component can be replaced by a refined
implementation without changing the meaning of a larger connected program.

That replacement is unsafe if we look only at visible state. An action can be
hidden and still consume authority, transfer a capability, spend a quantitative
budget, or assume a fact supplied by its environment. Hiding changes what an
observer sees; it does not erase those effects.

The Lean model therefore attaches the complete resource profile directly to
each transition. Two components may synchronize only when their ports agree,
their capability ownership is disjoint, authority is transferred exactly once,
and each side supplies the facts on which the other relies. Product grades add.

For products admitted by those formation rules, complete wiring preservation
and a resource-aware weak refinement let Lean lift replacement through the
current binary product. The proof analyzes the product's actual left, right,
and synchronized steps. Some formation fields are deliberately carried at the
language boundary even though the present proof does not consume each field;
the development does not claim that the theorem's premise bundle is minimal.

The source example is
`examples/pivot/visible_resource_sync.nmlt`. A concrete sender transfers one
`permit` to a receiver. Their `Ready` and `Authorized` contracts discharge one
another, and their grades add to `work=3`. Rust can explore the two resulting
states. Lean decodes that emitted artifact into the finite static behavior
model, checks the theorem's premises, and supplies the static theorem instance;
Rust does neither. A separate dynamic resource-world layer proves how a
supplied product step moves authority. The current certificate does not prove
that such a step is reachable or even exists from the decoded initial state.

Seven permanent controls break one formation or refinement obligation at a
time. They show distinct rejected boundaries; they are not countermodels
proving that every field is logically necessary for every possible lifting
theorem. Separate source fixtures make sure the compiler reports those as
distinct failures instead of collapsing them into a generic hidden-action
error.

This is not yet a liveness or fairness result. The earlier hidden-divergence
idea remains quarantined until the safety/resource semantics and language
pipeline are stable.

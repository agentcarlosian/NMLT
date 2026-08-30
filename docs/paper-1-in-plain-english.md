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

Under those conditions, plus complete wiring preservation and a genuine
resource-aware weak refinement, Lean proves that replacement lifts through the
binary product. The proof analyzes the product's actual left, right, and
synchronized steps.

The source example is
`examples/pivot/visible_resource_sync.nmlt`. A concrete sender transfers one
`permit` to a receiver. Their `Ready` and `Authorized` contracts discharge one
another, and their grades add to `work=3`. Rust can explore the two resulting
states, but only Lean supplies the theorem.

Seven permanent counterexamples explain the theorem's boundary by breaking one
premise at a time. Separate source fixtures make sure the compiler reports those
as distinct failures instead of collapsing them into a generic hidden-action
error.

This is not yet a liveness or fairness result. The earlier hidden-divergence
idea remains quarantined until the safety/resource semantics and language
pipeline are stable.

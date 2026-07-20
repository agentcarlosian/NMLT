# M11 two-sided open-congruence implementation note

Date: 2026-07-19

Scope: the finite safety core implemented toward M11-001c.

## Executable relation

`TwoSidedCongruenceChecker` accepts four open systems and two independently
checkable M11-001b refinement witnesses. It fails closed unless:

1. both local label-aware refinements pass;
2. both concrete and abstract compositions pass the existing receptiveness,
   exact-payload, noncircular-discharge, and resource checks;
3. applying both local label maps gives a bijection between the complete
   concrete and abstract connection lists;
4. both finite products construct within the same checked limits;
5. the mechanically constructed product state/action map passes
   `OpenRefinementChecker`, including assumption contravariance and guarantee
   covariance on every exposed product boundary; and
6. when supplied, a canonical Boolean table is total over the abstract product
   state space, holds on all abstract reachable states, and also holds after
   pulling back along every reachable concrete product state.

The edge-bijection check deliberately consumes each abstract edge once. This
prevents a duplicate abstract connection over already-matched endpoints from
being mistaken for reflected wiring.

## Mechanized core

Lean's `StrongRefinement` now has identity and composition, right-component
step lifting, right-side composability preservation, a two-sided composition
theorem, exact reachability, and contravariant invariant transport.
`OpenMappedCongruence.lean` then strengthens the boundary to complete typed
port bijections, mapped whole wiring, direction preservation, contravariant
assumptions, covariant guarantees, product step lifting, and invariant
transport. Its positive instance uses distinct concrete and abstract port
types on both sides and performs a real synchronization. The pinned Lean
kernel reports no axioms for these declarations.

The resource continuation adds `OpenResourceCongruence.lean`: concrete
authority and rely facts may not widen, consumption/transfer is exact, grades
improve, abstract guarantees persist, disjoint partitions pull back, and a
compatible synchronization discharges its rely set. `ResourceAwareMappedRefinement`
ties that witness to the operational/contract map, and
`liftResourceAwareParallel` covers both internal interleavings, all four
exposed input/output forms, and both synchronization directions. The Rust
checker enforces the corresponding finite profile in the same acceptance path.

## Correspondence boundary

`OpenEncodingCorrespondence.lean` now checks a raw finite representation with
natural state/action maps, Boolean predicates, nominal resource atoms, grades,
and wiring tables. Acceptance requires supplied typed `Fin` maps to decode
exactly, table coverage/injectivity, contract/resource variance, and whole
wiring. Rust emits that profile only after requiring one exact nominal payload
universe, total visible action maps, and surjective state maps, then revalidates
the isolated canonical certificate with the normalized predicate. Lean's
`accepted_implementation_contract` quantifies over every accepted certificate
and exports common payload identity, surjective typed maps, pointwise contract
and resource variance, authority narrowing, and whole wiring. The eleven shared
controls bind the executable checks and Lean handles. This is a proof-carrying
canonical-certificate boundary, but the Rust encoder and validator are not
verified extraction. The
[claim-specific evidence](../../benchmarks/results/open-congruence/m11-001c-evidence.json)
records `verified_implementation_theorem = false` for that reason.

## Negative controls

- a concrete connection whose two endpoints do not map to one abstract edge;
- an extra abstract edge outside the consumed concrete image;
- an invariant table with a stale state-space domain; and
- a nonmonotone concrete grade;
- an input rely fact absent from the peer guarantee;
- one affine capability owned by both components;
- a nonuniform nominal payload universe at the table boundary; and
- a post-encoding duplicate action-map mutation; and
- the retained Lean extra-wiring counterexample that blocks an otherwise
  independent peer step.

## Executed bounded-kernel correspondence

`nmlt-open-kernel` is a dependency-free, `no_std`, fixed-capacity Rust checker
used authoritatively by the canonical validator. Charon commit
`40ee060a8df43f4e7e0842d3f05387b0a4426aaf` and Aeneas commit
`45061fa1a5b4bad876f17c03d3a5544d818622e6` translate its actual control flow
into `OpenKernelGenerated`. Lean theorem `check_accepts_implies_contract`
projects successful execution into payload identity/uniqueness/capacity, both
refinement checks, and whole-wiring acceptance. The kernel supports at most
four states, four actions, four payload variants, eight atoms, and four
connections; larger certificates fail closed before execution.

The kernel envelope now retains the sorted, duplicate-free atom dictionary.
Before executing the kernel, Rust independently reads every active system,
action, predicate, resource table, refinement map, and wiring endpoint back
against the canonical certificate. Controls reject reordered dictionaries,
numeric atom substitution, active-action omission, and capacity overflow.
`OpenKernelReadback.lean` specifies functional/injective dictionary decoding,
complete coverage of referenced numeric IDs, and composition of that readback
condition with translated kernel acceptance.

The rich-to-canonical boundary now has a second independent readback. Rust
compares payload identity and variants, every system and ordered action,
predicates, resources and grades, refinement maps, and both wiring tables back
to the original rich source. Substitution controls cover action names,
resources, and wiring. `OpenSourceReadback.lean` proves that exact readback
transports an accepted canonical implementation contract to the source and
that exact readback is functional.

The authoritative bounded kernel now exposes `check_bound(expected, raw)`,
which executes structural equality before the validation decision. The
temporal validator supplies a separately reconstructed expected numeric
certificate, and a substitution control mutates the executed certificate.
Pinned Charon/Aeneas translate this path; Lean proves successful bound
execution exposes both the structural-readback decision and the complete
execution contract. Construction of the expected numeric certificate remains
ordinary Rust and is therefore still inside the residual correspondence gap.

## Remaining M11-001c gate

M11-001c is not promoted complete. The structural resource theorem, general
accepted-certificate semantic contract, and normalized-kernel execution proof
are now present. Promotion still requires verified extraction, or an equivalent
execution proof, for the rich `OpenSystem`-to-canonical encoder and the two
Rust readback implementations themselves. Fairness and
hidden-divergence transport remain M11-005 rather than being inferred from this
safety result.

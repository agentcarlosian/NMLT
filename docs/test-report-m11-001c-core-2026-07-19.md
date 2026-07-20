# M11-001c finite-core implementation and test report

Date: 2026-07-19

## Outcome

The finite safety core toward M11-001c is implemented and passes its local
verification gates. The full M11-001c promotion gate remains open.

## Implemented

- `TwoSidedCongruenceChecker` checks two independent M11-001b component
  refinements and the concrete and abstract compositions.
- Complete wiring is transported through both label maps with one-to-one edge
  consumption. Extra or duplicate abstract edges are not treated as reflected.
- The checker builds both bounded products and mechanically constructs the
  product state and action map.
- The lifted product is checked with `OpenRefinementChecker`, including
  remaining exposed assumptions and guarantees. The positive fixture retains
  a real exposed output contract after synchronization.
- Optional finite abstract invariants are checked over reachable product states
  and pulled back along the accepted lifted state map. Stale truth-table
  domains fail closed.
- Required resource profiles check affine ownership partitioning, exact atomic
  transfer, action-grade covariance, rely contravariance, peer-guarantee
  discharge, and the lifted product resource relation.
- The canonical encoding gate requires one exact nominal payload universe,
  total visible action maps, and surjective state maps, then emits sorted
  dictionaries with natural maps and Boolean predicate tables.
- The isolated canonical certificate is revalidated without consulting the
  richer source systems. Duplicate/out-of-range maps, predicate/resource
  variance failures, authority widening, and wiring drift fail closed.
- Lean now proves strong-refinement identity/composition, symmetric right-side
  lifting, two-sided composition congruence, composability preservation,
  reachability mapping, and contravariant invariant transport.
- A claim-specific schema, exact evidence manifest, shared Rust/Lean vectors,
  correspondence drift checker, CI target, and TCB profile were added.
- Lean bundles behavioral, contract, and resource refinement across all eight
  structural product-action constructors. Its executable certificate checker
  proves a general semantic contract for every accepted raw table and supplied
  typed `Fin` map.

## Controls

The Rust suite includes:

- a positive two-sided, contract-sound product and invariant instance;
- rejection when both mapped connection endpoints do not match;
- rejection of an abstract connection outside the consumed concrete image;
- rejection of an incomplete component boundary map;
- rejection of a stale invariant domain; and
- rejection of a nonmonotone grade, undischarged rely, shared affine
  capability, and nonuniform nominal payload universe; and
- rejection of a duplicate action map introduced after canonical encoding; and
- a source binding from the Rust controls to the shared correspondence vector.

Lean retains the extra-wiring counterexample and adds positive two-sided and
product-invariant handles.

## Verification

- `cargo fmt --all --check`: passed.
- workspace `cargo check`: passed.
- workspace Clippy with `-D warnings`: passed.
- workspace tests: passed; the temporal crate has 57 passing tests.
- focused M11-001c tests: 11 passed, 0 failed.
- `make ci`: passed. The default shell did not export `TLA2TOOLS_JAR`, so the
  comparison gate's TLC branch reported its documented skip.
- Explicit pinned comparison replay with the local TLC jar: passed TLC 2.19
  and P 3.1 with no comparison skips.
- Lean build: 20 jobs passed.
- New resource projections/partition/transfer declarations use no axioms;
  reference-checker soundness reports only standard `propext` and `Quot.sound`.
- Independent NanoDA replay: 95,941 declarations checked with no errors.
- Claim-specific evidence and shared-vector readback: passed.
- `git diff --check`: passed.

Evidence identity:
`nmlt-open-congruence-evidence-v1:sha256:01e487c470f0c1d2537a19e3227fe1c23ef50c1da6d41b65e6d563b5738550b4`.

## Assurance boundary and remaining work

The Lean checker validates natural/Boolean tables against typed state/action
maps, exact nominal payload identity, resource variance, and whole wiring.
Rust emits this restricted profile from canonical strings and nominal payload
identities and revalidates the isolated certificate with the normalized
predicate. Lean's `accepted_implementation_contract` is general over every
accepted certificate, rather than one fixture. The Rust encoder and validator
are still not verified extraction, so the evidence continues to record
`verified_implementation_theorem = false`.

The proved grade projection retains the numeric uncertainty upper bound, not
the Rust uncertainty certificate family/profile identity. The stronger family
checks remain enforced by `nmlt-grades` on the Rust acceptance path.

The bounded normalized Rust validator now has an equivalent execution-level
proof through pinned Charon/Aeneas translation. Its numeric envelope now carries
a canonical atom dictionary and independently reads all active fields back;
Lean proves unique decoding and referenced-ID coverage. M11-001c remains open
for verified extraction of the rich system-to-certificate encoder and the Rust
readback implementation. The structural resource bundling and general
accepted-certificate semantic theorem are complete.
Fairness and hidden divergence remain M11-005 obligations. M11-002 through
M11-009 have not been started by this change.

## Follow-on mapped, resource, and encoding results

`OpenMappedCongruence.lean` proves the
two-sided product result with typed bijective boundary maps, mapped complete
wiring, direction preservation, composite assumption/guarantee variance, and
invariant transport. Its positive control has different concrete and abstract
port types on both sides and a real synchronization.

`OpenResourceCongruence.lean` now combines capability, grade, rely, and
guarantee rules with the mapped behavioral theorem for every structural product
action. `OpenEncodingCorrespondence.lean` checks the restricted canonical Rust
profile as finite tables against supplied typed `Fin` maps and derives the
general accepted-certificate contract. The remaining boundary is the stronger
execution claim described above: verified extraction or an equivalent proof
covering arbitrary executions of the Rust encoder and validator.

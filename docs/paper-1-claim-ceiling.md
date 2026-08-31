# Paper 1 claim ceiling

**Working theme:** Resource-aware weak refinement for a finite programming
language core

**Status:** active post-pivot ceiling; the manuscript is not submission-ready

This file is the single claim ceiling for the Paper 1 directory.

## Claims currently allowed

1. NMLT defines a finite resource-bearing `Behavior` in Lean with state,
   initialization, transition, observation, action visibility, port
   annotations, declared capability ownership, grades, and rely/guarantee
   profiles.
2. `ResourceBehavior.parallel` has actual left, right, and synchronized
   step constructors.
3. `Composable` is the current language product-formation judgment. It
   checks wiring, visibility, direction/payload agreement, declared ownership
   partition, matched transfer/receive, and rely discharge.
4. `ResourceWeakRefinement` attaches state/observation maps,
   hidden/visible step conditions, resource refinement, and concrete
   hidden-profile compatibility to stutter.
5. `liftParallel` is a checked conditional theorem over the actual
   static product-step relation.
6. `SemanticClosure` constructs finite behaviors from a decoded
   `behavior-core-v1` artifact, decides the repository's formation and
   refinement bundle, and constructs `Certificate.lifted`.
7. The Rust gate regenerates the committed primary artifact byte-for-byte, and
   the Rust explorer displays a two-state synchronization with total grade
   `work=3` and a one-time transfer as non-authoritative inspection.
8. The separate hand-written Lean `ResourceWorldTransfer` example
   constructs an actual dynamic synchronization and proves its exact ownership
   change.
9. `ResourceWorld.liftProductSteps` gives conditional one-step dynamic
   lifting for local, peer, synchronized, and resource-compatible hidden steps.

## Required qualifications

- The static product and dynamic authority-world product are distinct semantic
  layers. No correspondence theorem currently unifies them.
- The static product is a closed binary construction for the current fixture,
  not a general open-interface-preserving operator. It does not preserve
  peer-side hiding, direction, or payload on product actions.
- The dynamic layer has no behavior-level initializer or observation, and the
  static and dynamic layers have no correspondence theorem.
- `ResourceWeakRefinement` has no separate path/trace adequacy theorem. The
  checked result is closure of the defined one-step relation under product.
- The theorem does not use every field of the bundled product-formation
  judgments. The formation controls demonstrate rejection of malformed
  products; they do not prove logical necessity or minimality of every premise.
- The artifact source digest identifies the separately supplied source bytes.
  Lean does not re-run Rust elaboration. Byte-for-byte compiler reproduction is
  established only by the repository fixture gate.
- `Certificate.liftedStep` maps any supplied dynamic step; it does not
  prove that the decoded initial state admits such a step or that the step is
  reachable.
- Received authority is not yet available to later receiver actions.
- Consumption can vacate authority. Claims should say ownership uniqueness,
  no fabrication under the encoded rules, and explained changes—not authority
  conservation.

## What the controls establish

`ResourceBehaviorControls` and the negative source fixtures separately
exercise:

- hidden connected boundaries;
- incomplete wiring preservation;
- shared declared capability ownership;
- unmatched transfer and receive;
- nonmonotone grades;
- undischarged reliance;
- hidden resource effects; and
- incomplete state maps and incompatible ports.

They establish that the current formation/refinement definitions and compiler
reject those cases. Do not describe all of them as countermodels to weakened
congruence theorems.

## Required non-claims

Paper 1 must not claim:

- verified Rust-to-Lean compilation;
- source/artifact correspondence for arbitrary accepted artifacts;
- one unified static/dynamic behavior semantics;
- general open-system interface preservation;
- dynamic initialization or observation;
- path/trace adequacy of the refinement relation;
- theorem-premise minimality;
- artifact-derived dynamic step existence or reachability;
- fairness, divergence, infinite traces, or liveness transport;
- reusable post-receive authority;
- arbitrary composition, grade algebras, or infinite state;
- proof or model-check authority for `nmlt-eval`; or
- production readiness.

## Required artifacts

- `mechanization/lean/NMLT/Behavior/ResourceBehavior.lean`
- `mechanization/lean/NMLT/Behavior/ResourceWorld.lean`
- `mechanization/lean/NMLT/Artifact/BehaviorCore.lean`
- `mechanization/lean/NMLT/Artifact/SemanticClosure.lean`
- `examples/pivot/visible_resource_sync.nmlt`
- `examples/pivot/visible_resource_sync.behavior-core-v1.json`
- `mechanization/lean/AXIOMS.md`

The paper gate must run the complete Rust/Lean reproduction, pass the focused
axiom audit, contain no generated PDF in Git, and receive independent semantic
and cross-family critical review.

# RFC 0038: Candidate-selected Lean tactic imports

- Status: Under review
- Created: 2026-10-03
- Scope: Correction to RFC 0036 native proof generation

## Problem and non-goals

Every structured tactic candidate currently imports all of `Lean.Elab.Tactic`.
An explicit-module task using only intro, constructor and exact failed while
reading an unrelated `SimpTrace.olean.private` under the ordinary process limits.
The file was present and readable. A diagnostic replacement with the smaller
ElabTerm module compiled the same proof under the same limits. This is an import
or host-resource failure, not a mathematical refutation. It does not justify
raising file, data, Lean-memory, time, output or descendant limits.

## Behavior and trust

Derive a sorted, duplicate-free list of fixed Lean modules from the validated
candidate step variants. Basic intro, assumption, rfl, exact, apply, constructor
and decide use the pinned frontend's built-in elaborators without importing
their implementation modules into the proof. SimpOnly, SimpaOnly, Omega and Grind
select their corresponding tactic modules. Closed-term candidates also add
no automation import. These are adapter choices, never candidate-supplied
module names. This selects direct modules, not a claim of a globally minimal
transitive dependency closure.

Compilation alone is insufficient: Windows comparisons that explicitly imported
BuiltinTactic/ElabTerm, or ElabTerm alone, compiled the basic proof but failed
with allocation errors under the ordinary export job's unchanged 1 GiB memory
bound. These logs do not independently measure that the bound was reached. Keep those failed
observations distinct from acceptance. The no-additional-import basic path must
pass export and fresh independent checking as part of validation below.

Keep the existing bounded grammar, target comparison, universe checks,
transitive axiom policy, complete export/dependency checks, NanoDA verification,
fresh reconstruction and tool pins. No automatic fallback to a broader import
or larger process policy is allowed. More expensive selected tactics can still
exceed ordinary limits; a host failure remains a failed attempt.

The existing fixed `grind` and `decide` variants remain available. Additional
arguments, native-decide options and arbitrary tactics remain rejected.

## Editor wrappers and compatibility

The context-only Lake editor retains its existing broad tactic import and
project process policy for diagnostics. Candidate extraction accepts only the
exact target wrapper with either no automation imports, that fixed editor
header, or the candidate's fixed generated header. All other wrapper bytes
remain exact, and arbitrary inserted imports or target changes are rejected.
The accepted proof is regenerated from the validated candidate; an editor file
is not an acceptance artifact.

Generated proof bytes change. Exact CLI implementation pins already invalidate
old task bindings under a new executable. Old accepted records require their
retained executable/tools; do not rewrite, migrate or reinterpret them. Result
and candidate schemas remain unchanged. This RFC records the generated-source
and wrapper compatibility change; it grants no new trust to automation.

## Validation

Run a real explicit-module conjunction proof using intro, constructor, rfl and
exact, then fresh Lean/NanoDA rechecking with the unchanged ordinary policy.
Reject an invalid candidate, unsupported tactic arguments, extra imports and a
changed target wrapper. Exercise the existing Lake simp/simpa, omega, grind and
editor workflows, plus decide and the full Rust/Lean/NanoDA gates. Keep export
hashes and actual transitive axioms. Linux must reproduce the formerly failing
basic tactic case on the final source; passing Windows alone does not settle
the Linux import failure. Review and publication approval remain separate.

Pinned upstream references: [tactic umbrella](https://github.com/leanprover/lean4/blob/819816b2e0a3bf405af45ae5c7af2491d8f5bee6/src/Lean/Elab/Tactic.lean),
[term tactics](https://github.com/leanprover/lean4/blob/819816b2e0a3bf405af45ae5c7af2491d8f5bee6/src/Lean/Elab/Tactic/ElabTerm.lean),
[builtin tactics](https://github.com/leanprover/lean4/blob/819816b2e0a3bf405af45ae5c7af2491d8f5bee6/src/Lean/Elab/Tactic/BuiltinTactic.lean).

# M9 Integration and Correspondence Research — 2026-07-19

The archive was searched for “typed intermediate representation interpreter
semantic preservation,” “definitional interpreter compiler correctness
transition system,” and “proof carrying code checked intermediate language
execution.” No directly reusable archived record matched these queries. This
absence is recorded so current-web material is not misrepresented as prior
NMLT research.

The M9 design therefore continues the already accepted RFC 0013 evidence:
make the executable consumer accept only an opaque independently checked core,
retain a declarative reference semantics, and treat shared vectors as drift
controls rather than correspondence proofs. The focused search did not justify
expanding the supported language fragment or weakening the TCB.

Lean 4.30.0 is the pinned proof-assistant release used for the new extrinsic
checker and correspondence statements. Its official release record is
<https://github.com/leanprover/lean4/releases/tag/v4.30.0>.

The resulting decision is conservative: the Rust kernel remains normative for
runtime acceptance, the Lean file states the mathematical obligations, and
hashing, canonical decoding, and Rust/Lean implementation correspondence stay
named residual-trust entries until a verified extraction or program-level
refinement proof replaces them.

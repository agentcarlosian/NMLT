# RFC 0039: Guarded Lean 4 tactic scripts (`nmlt-lean-proof-candidate-v3`)

- Status: Under review
- Created: 2026-11-26
- Scope: Extension to RFC 0036 and RFC 0038 bound Lean candidate proofs

## Problem and motivation

RFC 0036 and RFC 0038 intentionally limited structured tactic candidates (`v2`) to
11 single-line tactics (`intro`, `exact`, `apply`, `simp only`, `simpa only`,
`assumption`, `rfl`, `constructor`, `omega`, `grind`, `decide`). While safe,
real Lean 4 and Mathlib proofs routinely require `rw`, `cases`, `induction`,
`have`, `rcases`, `obtain`, `calc`, `ring`, `linarith`, `simp`, `refine`,
`ext`, `funext`, `specialize`, and multi-line structured tactic blocks.

Requiring users or agents to hand-compile those proofs into closed raw proof
terms (`v1`) prevents `nmlt lean-task` from being usable on ordinary Lean 4 /
Mathlib projects even though the backend already enforces formal target-type
equality in Lean, transitive axiom checking, ProcessKit containment, and
independent NanoDA kernel verification.

## Proposed behavior

Add a third candidate schema variant, `nmlt-lean-proof-candidate-v3`
(`Candidate::Script { task_sha256: String, script: String }`), alongside
unchanged `v1` (`Term`) and `v2` (`Automation`):

1. **Backward compatibility:** `Candidate::from_proof` continues to parse raw
   closed terms as `v1` and 11-tactic scripts as `v2` first, preserving exact
   hashes and behavior for all existing tasks and tests.
2. **Guarded tactic scripts (`v3`):** Any `by ...` proof body outside `v2` is
   validated by a lexer-aware Lean 4 static guard before acceptance as `v3`:
   - Size bound: `1..=32768` UTF-8 bytes, at most 512 lines, no NUL or
     non-whitespace ASCII control characters.
   - **Layout-escape prevention:** The first non-empty line must begin with
     `by` (followed by newline or space), and every subsequent non-empty,
     non-comment line must be indented (leading space or tab) so Lean 4's
     layout parser cannot terminate `theorem NMLTChecked.result` and begin a
     new top-level command.
   - **Lexer-aware scanning:** Properly skips `--` line comments, nested
     `/- ... -/` block comments, `"..."` strings, `r#"..."#` raw strings, and
     character literals so comments/strings cannot hide banned tokens.
   - **Balanced delimiters:** `()`, `[]`, `{}`, and `⟨⟩` must balance across
     non-comment/non-string tokens.
   - **Banned tokens:** Rejects soundness bypasses (`sorry`, `admit`, `sorryAx`,
     `native_decide`, `+native`, `unsafe`, `implemented_by`, `extern`,
     `partial`), top-level declarations/commands (`import`, `axiom`, `constant`,
     `opaque`, `theorem`, `lemma`, `def`, `abbrev`, `structure`, `class`,
     `inductive`, `instance`, `example`, `namespace`, `section`, `end`, `open`,
     `variable`, `universe`, `attribute`, `set_option`, `export`, `mutual`,
     `noncomputable`, `private`, `protected`), metaprogramming/IO hooks
     (`macro`, `macro_rules`, `syntax`, `declare_syntax_cat`, `elab`,
     `elab_rules`, `notation`, `infix*`, `prefix`, `postfix`, `initialize`,
     `builtin_initialize`, `run_cmd`, `run_meta`, `run_elab`, `run_tac`,
     `dbg_trace`, `IO`, `System`, `Lean.Elab`, `Lean.Meta`, `Lean.Core`,
     `Lean.Environment`, `Lean.addDecl`, `Lean.trustCompiler`), `#` command
     prefixes, and backtick syntax quotations.
3. **Minimal fixed imports:** `Candidate::Script::imports()` scans non-comment
   identifier tokens and adds only the fixed RFC 0038 modules required by
   `simp`/`dsimp`, `simpa`, `omega`, or `grind`. Lake/Mathlib tasks continue to
   inherit tactics imported by `NMLTTask`.

## Trust boundary

The static guard reduces elaboration-time metaprogramming/command injection risk
inside the contained Lean process, but **proof validity still rests strictly
on Lean target equality (`ci.type == mkConst NMLTTask.target`), transitive
axiom enforcement, `lean4export`, and independent NanoDA verification**.
ProcessKit containment limits remain unchanged.

## Validation and negative controls

- Positive unit tests for multi-line `rw`, `cases`, `induction`, `have`, `calc`,
  `ring`, `linarith`, Unicode notation (`⟨⟩`, `·`, `←`, `→`), comments, and
  editor wrapper round-trips (LF and CRLF).
- Negative controls rejecting column-0 command injection, `sorry`, `admit`,
  `native_decide`, `decide +native`, `run_tac`, `#eval`, `set_option`,
  `initialize`, `Lean.addDecl`, backtick quotations, unbalanced delimiters,
  unterminated block comments/strings, and oversized scripts.

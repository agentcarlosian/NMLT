# Paper 1: Hidden Actions Are Not Contextual Silence

Technical paper on weak refinement vs synchronized composition.

- **TeX / PDF:** `main.tex`, `main.pdf`
- **Lean:** `Core/Transition.lean`, `Counterexamples/CompositionCongruence.lean`, `Behavior/WeakConditionalCongruence.lean`, `Behavior/OpenComposition.lean`
- **Internal claim ceiling (authors):** `../../docs/paper-1-claim-ceiling.md`
- **Plain companion (authors):** `../../docs/paper-1-in-plain-english.md`

## Build

```bash
cd papers/hidden-silence
pdflatex main.tex
pdflatex main.tex
```

Do not strengthen claims beyond the checked Lean theorems and the claim ceiling.

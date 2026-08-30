# Paper 1: resource-aware open refinement

This directory now follows the language-and-mathematics pivot. The earlier
hidden-silence draft referred to transition modules removed from the active
project; that historical version remains recoverable from the immutable
build-week-judge-demo-2026 tag.

- Manuscript: [main.tex](main.tex)
- Active claim ceiling: [claim-ceiling.md](claim-ceiling.md)
- Current axiom audit: [axiom-audit-2026-08-30.md](axiom-audit-2026-08-30.md)
- Normative semantics:
  [ResourceBehavior.lean](../../mechanization/lean/NMLT/Behavior/ResourceBehavior.lean)
- Checked source fixture:
  [visible_resource_sync.nmlt](../../examples/pivot/visible_resource_sync.nmlt)
- Canonical artifact:
  [visible_resource_sync.behavior-core-v1.json](../../examples/pivot/visible_resource_sync.behavior-core-v1.json)

Build the manuscript with pdflatex in a disposable output directory. Do not
commit generated PDFs or auxiliary files.

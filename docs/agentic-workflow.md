# Agentic workflow

NMLT uses an agent to search for candidate formalizations and localized edits;
it does not use an agent to decide what is true. The normative boundary is
[RFC 0011](../rfcs/0011-authority-bounded-agentic-repair.md).

The workflow is an artifact graph:

```text
intent + property + controls (trusted identities)
                  |
candidate -> checker feedback -> untrusted repair proposal
                  |                    |
                  +---- authority gate-+
                               |
                         isolated recheck
                               |
                result + witness + residual gaps
```

The authority gate compares exact digests before applying edits. The isolated
recheck runs every stage instead of accepting an agent's statement that a fix
works. A successful parse is only syntax completion, a successful type check is
only static completion, and a bounded semantic pass is only `model_checked`.

The checked-in deterministic Phase 6
[evaluation](../benchmarks/agentic/evaluation.json) moves the three
hand-authored held-out fixtures from 0/3 baseline completion to 3/3 assisted
completion. It also records 21/21 protected-artifact modification rejections,
3/3 retained-and-killed negative controls, and zero `unknown` or conflict
promotions. This is protocol-conformance evidence for a deterministic repair
baseline, not evidence that an LLM or general agent improves NMLT development.

Human intent agreement was not measured and has no field or review artifact in
the v1 evaluation. It therefore remains an explicit future evaluation
requirement and must not be inferred from checker acceptance, frozen intent
digests, or the absence of an evidence conflict.

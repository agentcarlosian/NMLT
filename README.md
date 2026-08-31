# NMLT

**NMLT — New Mathematics, Languages, and Techniques — is a research repository
for trustworthy computation.** It develops a new programming language and the
mathematics needed to describe behavior, composition, resources, and
refinement.

> To truly progress, humanity needs new mathematics, new languages, and new
> techniques.

NMLT is pre-alpha research software. It must not authorize safety-critical,
financial, security-critical, or irreversible actions.

## Artifact-derived semantic closure

This checkpoint replaces the former contest verifier suite with one finite,
resource-aware language path:

```text
.nmlt source
  → lossless syntax, resolution, and typed elaboration       Rust
  → deterministic behavior-core-v1 artifact                 Rust
  → artifact-derived behavior and conditional refinement   Lean
  → bounded operational inspection, assurance: none         Rust
```

The supported slice includes typed input/output ports, affine capabilities,
named natural-number grades, nominal rely/guarantee facts, binary composition,
observations, hiding, and explicit finite-state refinement maps.

Lean defines the static behavioral semantics and proves the current conditional
composition theorem. The Lean decoder validates canonical structure and source
identification, reconstructs the finite behavior objects and refinement
witnesses carried by the primary artifact, and instantiates that theorem. No
verified Rust-to-Lean compiler claim is made.

Dynamic authority worlds are not part of this checkpoint. The theorem is a
conditional safety result; it does not establish reachability, fairness, or
liveness.

Rust exploration is a language-design aid and always reports `assurance: none`.

## Reproduce

```bash
make ci
make metatheory
```

The accepted pivot and historical boundary are recorded in
[`docs/decisions/0004-language-mathematics-pivot.md`](docs/decisions/0004-language-mathematics-pivot.md)
and [`docs/history.md`](docs/history.md).

NMLT is licensed under Apache-2.0. See [`LICENSE`](LICENSE).

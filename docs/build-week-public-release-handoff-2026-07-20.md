# Build Week and public-release handoff — 2026-07-20

## Recommendation

NMLT is technically substantial enough to publish and submit as a pre-alpha
research project. Do not delay the submission to finish M11-001c.

Before announcing the repository or submitting it to OpenAI Build Week, finish
the release-presentation tasks below. They will improve judge confidence and
usability more than another deep proof layer before the deadline.

Recommended decision:

> Fix hosted CI, add a five-minute judge path and Build Week explanation,
> record the demo, then make the repository public.

The repository may alternatively remain private if both judging addresses are
given access, as permitted by the supplied challenge requirements.

## Current readiness

### In good shape

- Apache-2.0 licensing is present and detected by GitHub.
- `main` is clean and synchronized with `origin/main`.
- No obvious tracked API keys, private keys, credential files, or secret
  filenames were found in the targeted release-readiness check.
- Commit metadata exposes only the project contact address and GitHub noreply
  addresses.
- The repository is compact and contains real implementation, tests, formal
  mechanization, evidence, negative controls, and documentation.
- `SECURITY.md`, `CONTRIBUTING.md`, governance, threat-model, and trusted-
  component documentation are present.
- The project is a strong fit for the **Developer tools** track.
- The README now leads with user-facing capabilities while retaining honest
  assurance boundaries.

The credential check was targeted, not an exhaustive repository security
audit. Do not describe it as exhaustive coverage.

### Blocking a polished launch

1. Hosted CI is not visibly green.
   - The latest Rust workspace job succeeded.
   - The latest Lean job was cancelled at the workflow's 15-minute timeout.
   - Earlier recent `main` runs also appear failed or cancelled.
2. There is no five-minute judge workflow with expected output.
3. There is no prebuilt release, container, Codespace, hosted sandbox, or
   equivalent no-rebuild testing path.
4. The README does not yet explain the Build Week/Codex/GPT-5.6 development
   story directly.
5. The required public demo video and `/feedback` session ID still need to be
   prepared for the submission.

## Priority execution plan

### P0 — Make hosted CI trustworthy

The Lean workflow currently has:

```yaml
timeout-minutes: 15
use-mathlib-cache: false
use-github-cache: false
```

Investigate the latest cancelled Lean run before changing semantics. Likely
release-oriented options include:

- raise the Lean job timeout;
- enable a safe cache keyed by the pinned Lean toolchain and lake manifest;
- split compilation, NanoDA checking, and source-policy checks into separately
  visible jobs;
- keep the expensive complete proof gate scheduled or manually runnable while
  retaining a reliable required PR check.

Do not weaken proof policy or silently skip NanoDA/metatheory checks merely to
obtain a green badge. Document any distinction between fast PR CI and the full
reproduction gate.

Exit condition: the default-branch GitHub Actions run is green, or every
non-green optional job is clearly named and documented rather than appearing
as a broken required check.

### P0 — Add a five-minute judge path

Add a prominent README section with exact commands and abbreviated expected
results. The shortest compelling workflow should demonstrate both success and
failure:

1. Run the accepted provider reference model.
2. Run an intentionally broken seeded-defect model.
3. Show its structured counterexample.
4. Show source-bound evidence or stale/forged evidence rejection.

Candidate commands should use existing CLI and fixtures, for example:

```bash
cargo run -p nmlt-cli -- model-check --json \
  benchmarks/seeded-defects/provider-attempt/reference.nmlt

cargo run -p nmlt-cli -- model-check --json \
  benchmarks/seeded-defects/provider-attempt/dispatch-before-authorize.nmlt
```

Verify exact fixture names and expected output before publishing the commands.
Avoid requiring Lean, TLC, P, Quint, and NanoDA for the first judge experience.
Those belong in the full verification/reproduction path.

### P0 — Provide a no-rebuild testing option

Because this is a developer tool, judges should not need to rebuild the full
research environment. Choose the smallest deliverable that fits the deadline:

1. preferred: a GitHub Release containing a pinned Linux x86-64 CLI binary and
   checksums;
2. good alternative: a small container image or reproducible Docker command;
3. good alternative: a Codespace/devcontainer with the quick demo ready;
4. minimum fallback: a single script that builds only the Rust CLI and runs
   the curated demo without the full Lean/comparison stack.

Clearly distinguish the quick executable demo from `make ci`, which exercises
the larger evidence and comparison environment.

### P0 — Add the Build Week story

Add a short README section or linked submission note answering:

- What problem does NMLT address?
- What can a judge run today?
- Where did GPT-5.6 and Codex materially accelerate development?
- Which architectural and assurance decisions remained human-directed?
- How does the project prevent AI-generated output from becoming trusted
  evidence without independent checking?

Suggested core message:

> Codex accelerated construction and recovery of a multi-language verification
> chain spanning Rust, Lean, pinned Charon/Aeneas translation, evidence
> manifests, mutation controls, TLA+, and P. NMLT treats generated work as a
> proposal: acceptance remains bound to independent kernels, exact identities,
> explicit trust boundaries, and adversarial controls.

Do not claim that GPT-5.6 or Codex proves arbitrary NMLT programs, or that
M11-001c is complete.

## Demo video plan — under three minutes

- **0:00–0:25 — Problem:** software and AI-generated claims can outrun their
  evidence.
- **0:25–0:55 — Model:** show a readable `.nmlt` provider-attempt model.
- **0:55–1:25 — Positive path:** run the accepted reference model and explain
  the bounded result.
- **1:25–1:55 — Negative path:** run one seeded semantic defect and show the
  structured counterexample.
- **1:55–2:20 — Evidence:** show source-bound evidence and stale/forged-result
  rejection.
- **2:20–2:45 — Verified boundary:** briefly show the dependency-free Rust
  kernel, generated Lean, and explicit limitations.
- **2:45–2:58 — Codex/GPT-5.6:** state what the agent accelerated and what
  remained independently checked.

The video must be public on YouTube, shorter than three minutes, and include
audio explaining both Codex and GPT-5.6 usage, according to the supplied
challenge requirements.

## Devpost submission checklist

- [ ] Review the official rules and confirm eligibility.
- [ ] Select **Developer tools**.
- [ ] Prepare a concise project description for judges rather than pasting the
      technical README.
- [ ] Record and publish the sub-three-minute YouTube demo.
- [ ] Obtain the `/feedback` Codex Session ID for the session containing most
      of the core implementation.
- [ ] Provide the repository URL.
- [ ] If private, share access with `testing@devpost.com` and
      `build-week-event@openai.com`, then verify access.
- [ ] If public, verify no private issues, actions logs, artifacts, branch
      names, or repository metadata expose unintended information.
- [ ] Include setup instructions, supported platforms, sample inputs, and the
      five-minute judge workflow.
- [ ] Provide a no-rebuild or minimal-build testing method.
- [ ] Submit before Tuesday, July 21 at 5:00 PM PT, based on the supplied
      challenge details.

## GitHub presentation checklist

- [ ] Change the About text to hyphenate `behavior-first`.
- [ ] Add topics such as `formal-methods`, `verification`,
      `programming-languages`, `rust`, `lean`, `model-checking`, and
      `developer-tools`.
- [ ] Add a screenshot or short terminal animation near the top of the README.
- [ ] Ensure the default branch has a trustworthy CI result.
- [ ] Confirm GitHub identifies the Apache-2.0 license.
- [ ] Consider disabling unused repository features or populating them before
      launch; the wiki is currently enabled but empty/unused.
- [ ] Create a tagged release if distributing a judge binary.

Suggested About text:

> NMLT — New Mathematics, Languages, and Techniques: a research repository for
> behavior-first, evidence-carrying programming, and trustworthy computation.

## Submission positioning

Lead with the working developer experience rather than the unfinished research
program:

1. readable behavior-first models;
2. deterministic bounded checking;
3. structured counterexamples;
4. evidence bound to exact sources, tools, and limits;
5. independent rejection of stale or forged claims;
6. a translated Rust/Lean validation boundary with explicit residual trust.

Present unfinished M11 work as deliberate claim discipline, not as a missing
demo feature. The strongest differentiator is not “everything is verified.” It
is that NMLT records exactly what was checked, by which mechanism, under which
bounds, and refuses to promote unsupported results.

## After submission

Resume the active technical handoff in
[`reboot-handoff-2026-07-20.md`](reboot-handoff-2026-07-20.md): complete the
bottom-up generated-equality soundness stack, strengthen the final native Lean
equality theorem, update evidence, and then continue closing the rich encoder
and readback boundary for M11-001c.

# Reboot handoff — 2026-07-19

Captured at 2026-07-19T18:04:59-05:00 for Carlosian
<carlosian@agentmail.to>. This is a non-normative continuation record. The
execution gates in [`Plan.md`](../Plan.md), accepted RFCs, schemas, and checked
evidence remain authoritative.

## Durable repository state

- Local repository: `~/NMLT`
- Private remote: `agentcarlosian/NMLT`
- Branch: `main`
- Durable implementation baseline: commit
  `a5ab96724266148540f589f515e708ed921403f2`
  (`Start M11 open-system refinement`)
- Author and committer identity: Carlosian <carlosian@agentmail.to>
- Baseline state when this record was prepared: clean working tree, with
  `HEAD` equal to `origin/main`
- Baseline GitHub Actions run:
  [29707063688](https://github.com/agentcarlosian/NMLT/actions/runs/29707063688),
  successful for both the Rust workspace and Lean metatheory jobs
- Pinned tools: Rust 1.94.0 and Lean 4.30.0

The commit containing this handoff is documentation-only and follows the
implementation baseline. After a restart, use `git log -1 --oneline` rather
than assuming that `a5ab967` is still the branch tip.

## Completed continuation point

M9's bounded source-to-typed-core route and M10's bounded behavior/refinement
and certificate seed are complete at the scopes recorded in `Plan.md`. M11 is
active. **M11-001a is complete; its parent M11-001 remains open.**

M11-001a added:

- a finite Rust open-system model with input/output/internal polarities,
  global input receptiveness, one-output/one-input synchronous connections,
  conservative noncircular symbolic discharge, lifted refinement checks, and
  checked state, transition, and conservative work-item limits;
- an axiom-free Lean structural one-sided exact-action product-congruence
  theorem, plus separate composability-preservation and product-receptiveness
  results;
- adversarial controls for broken wiring, hidden synchronization,
  nonreceptiveness, aliasing, channel mismatch, and resource exhaustion; and
- claim-specific schema, evidence, TCB inventory, source bindings, metatheory
  policy, and independent NanoDA checking.

The exact evidence identity is
`nmlt-open-composition-evidence-v1:sha256:2709d1686729570b37f856a3343330ef0969ce3c103c44b5d6d61579e4caa0f3`.
Its validation gate is `./tools/check_metatheory.sh`.

## Assurance boundary that must survive continuation

M11-001a establishes only its finite exact-action safety profile. Do not infer
any of the following from it:

- semantic or circular assume/guarantee satisfaction;
- payload subtyping or typed payload compatibility;
- capability, grade, rely-condition, fairness, liveness, progress, divergence,
  or resource preservation;
- weak-hiding or label-map congruence;
- a Rust/Lean compiler-correspondence theorem; or
- a two-sided open-refinement theorem.

Rust implements one-to-one boundary connections. Lean permits arbitrary
bidirectional wiring relations and requires the complete relation to agree
across refinement. Its positive structural result is one-sided,
state-surjective, and exact-action. The detailed frozen profile and promotion
boundary are in the
[M11 research note](research-notes/m11-open-system-refinement-2026-07-19.md).
RFC 0008 remains under review and therefore supplies design guidance, not an
accepted normative language contract.

## Exact next objective: M11-001b

Resume with **contract-sound, label-aware open refinement**. Its required
scope is:

1. replace symbolic claim-name equality with canonical finite assumption and
   guarantee predicates;
2. define canonical payload-type identity and reject payload substitution;
3. define contravariant preservation of assumptions and covariant
   preservation of guarantees;
4. state and implement the resulting finite open-refinement relation;
5. prove identity and composition for that relation in Lean without new
   axioms; and
6. bind the exact executable and theorem claims, negative controls, source
   identities, TCB components, and limitations into reproducible evidence.

Use `search-the-archives` before freezing the rules. Search the local archive
and current primary sources separately for interface refinement,
assume/guarantee contract refinement, alternating simulation, variance,
payload/data refinement, and circular contract discharge. Record archive gaps
as gaps rather than negative evidence.

A safe implementation order is research and counterexamples; a frozen finite
semantic profile; Rust data/checker rules; Lean definitions and
identity/composition laws; adversarial controls; evidence/TCB/docs; then the
complete local and remote gates. Do not begin M11-001c by silently broadening
M11-001b.

After M11-001b, the planned order is M11-001c full supported congruence, then
M11-002 through M11-009 as listed in `Plan.md`. M11-001c owns two-sided
lifting, composite contract soundness, invariant transport, and any claimed
correspondence between the executable checker and Lean statement.

## Important files

- [`Plan.md`](../Plan.md) — authoritative milestone gates and open items
- [`crates/nmlt-temporal/src/open.rs`](../crates/nmlt-temporal/src/open.rs) —
  executable M11-001a profile
- [`OpenComposition.lean`](../mechanization/lean/NMLT/Behavior/OpenComposition.lean)
  — checked structural results and controls
- [M11 research note](research-notes/m11-open-system-refinement-2026-07-19.md)
  — sources, frozen semantics, and nonclaims
- [M11 evidence manifest](../benchmarks/results/open-composition/m11-001a-evidence.json)
  — exact machine-readable claim and identity
- [`docs/threat-model.md`](threat-model.md) and
  [`security/trusted-components.toml`](../security/trusted-components.toml) —
  TCB boundary
- [`rfcs/0008-mechanization-and-compositional-refinement.md`](../rfcs/0008-mechanization-and-compositional-refinement.md)
  — under-review compositional-refinement design

## Local mathematics archive state

The archive is at `~/research-archive`. It is not a Git repository and has no
remote backup, so its hashes establish readback only—not remote durability or
authorship. Its authoritative editable inputs are the generator, normalized
offline index, curated-source manifest, and eight-loop research manifest. The
search index is derived.

- Schema: `math-frontier-search-v2`
- Records: 2,347 = 2,334 original + 13 curated additions
- Curation overlays: 8
- Research loops: 8
- Build identity:
  `a380ca306603c27052d25780037821a4b5db8b2ba7716da03b49d44c5e912a7d`
- Generated index SHA-256:
  `ec47047fa958a39cfb466989e9d2f866c0e288b02aa6845f7b4ac7dcd459eb4a`

The full local checksum manifest is
`~/research-archive/math_frontier_checkpoint_2026-07-19.sha256`. Its critical
input identities are:

- original web bundle:
  `fb853c559a1f91fe7e9192972d69dfcd58a51565f3bb0bfdd399d027220c061b`;
- normalized offline index:
  `0461f81e36dbf1f973fac4dbfab7abedfdda3e563453ff01548f73a3219935f6`;
- generator:
  `2c2025b99196234343b7fda26fb0b777ce450e8045de166473976298efe95f2e`;
- curated-source manifest:
  `fee19f2338ad671b926ede791e3b9f6ec40f6e8bfc0a0bfc72edbcd869e7dbc6`;
  and
- research-loop manifest:
  `d5bfdf32066b085cfeae1c7378b7421c01e86bcdf764e0f5356054babcc4a7a0`.

The 2026-07-19 M11 repair added arXiv `1201.4449`, `1210.2450`, and
`1306.3050`. The current check passes with 13 additions, 8 overlays, and 8
research loops. Live collector attempts were rate-limited or timed out; that
is missing coverage, not evidence against the missing work. The first archive
maintenance decision after reboot is whether to place `~/research-archive` in
a separate private version-controlled repository or another durable backup.

## Resume and validation commands

```sh
cd ~/NMLT
git status --short --branch
git log -3 --format='%h %ad %an <%ae> %s' --date=iso-strict
git fetch --prune
git rev-parse HEAD
git rev-parse origin/main
gh auth status
gh repo view --json nameWithOwner,visibility,isPrivate
make ci
env PATH="~/.elan/toolchains/leanprover--lean4---v4.30.0/bin:$PATH" \
  ./tools/check_metatheory.sh

cd ~/research-archive
python3 build_math_frontier_search_index.py --check
sha256sum -c math_frontier_checkpoint_2026-07-19.sha256
```

`make ci` may skip TLC when `TLA2TOOLS_JAR` is unset and P when its toolchain is
absent; retain those as explicit unvalidated comparison scopes. GitHub emitted
a Node 20 deprecation warning for the pinned `actions/checkout` revision. The
pinned Lean toolchain exists under `~/.elan/toolchains`, but this
shell did not have that directory on `PATH`; the explicit command above avoids
depending on an Elan shim. A Dependabot update exists, but changing that
pinned action also changes TCB/evidence identities and should be handled as a
separate audited maintenance change, not folded into M11-001b.

No credential or token is stored in this record.

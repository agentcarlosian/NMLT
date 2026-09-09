# Complete private-history port

Destination: `agentcarlosian/NMLT`, based on public `main` at
`07b7a8d6ac31f3d02a639dfbd2f64758633bd368`.

This port preserves the complete reachable NMLT-Private history, not only R2.
All **101 private commits**, the tips of its **13 branches**, and the commits
named by its **3 annotated tags** are ancestors of preservation commit
`824cf30eeae0827103fee6c8130585f50edefd8c`. The previously uncommitted R2 work is
preserved in `3b8d247cfba8b7bc0e87493375a625ddf1bc6f4a`. Original commit IDs,
parents and authorship are retained. The [ref inventory](../history/private-refs-2026-09-09.json)
records every original branch/tag target and the zero-missing-commit check.

Public main had already merged the five pivot PRs using a parallel commit
series. The integration merge joins that public ancestry with the original
private development line. Its active Rust/Lean/schema/example implementation
matches the validated R2 snapshot. Public historical notices, the disposition
of RFC 0004, and the newer pinned checkout/Lean-action versions are retained.

Two side branches contain additional historical research: nine commits on
`local-box-main` and one quarantined Grok resource-congruence experiment.
A history-preservation merge retains their exact trees and ancestry for later
inspection or focused PRs. They do not replace the current validated language
implementation. Historical claims retain their original scope and quarantine
status.

The original repository is preserved locally, with a pre-port Git bundle in
the workspace's `work/` directory. Ongoing development uses the separate public
repository checkout at `NMLT/`, whose `origin` is the public repository. This
port is submitted on `codex/port-private-history` against public `main` so it
can be reviewed before merging. R3 follows on a separate branch.

## Validation and review

- Every private branch/tag commit is reachable from the preservation merge;
  no private commit is missing.
- The active implementation was compared against the completed R2 snapshot;
  differences are public documentation disposition and pinned CI action updates.
- Full reproduction is rerun from the public checkout to validate relocation
  and the merged tree. The R2 source-checkout evidence remains historical and
  is not relabeled as a build of this checkout.
- The publication preflight inspected reachable private-history text blobs for
  high-confidence credential patterns without printing matched values; it found
  no candidates. This is a limited publication check, not a security audit.
- Independent cross-family/human review is an open gate. This is explicitly
  recorded as required by the contribution policy; the migration PR is a draft.

The R2 claim ceilings and exact-executable replay requirements continue to
apply. Preserving a historical commit does not certify its earlier claims
under the current toolchain.

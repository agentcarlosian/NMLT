# RFC 0037: Console-free contained Windows processes

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-10-03

## Problem and evidence

The Windows reproduction gate intermittently rejects successful fixed square
workers with `Io` and confirmed cleanup. A same-handle observation after the
existing 100 ms post-exit settle found a nonsignaled `conhost.exe` in that exact
Job Object. No filename exemption is justified. A root that leaves descendants
must still fail. Separate Lean version-probe `Io` failures have not been shown
to have this same cause.

A bounded diagnostic confirmed all 14 launches received `0x08000004`
(`CREATE_NO_WINDOW | CREATE_SUSPENDED`). Twelve workers had private windowless
console attachments and valid explicit pipe handles. Both Lean probes passed.
This is not a missing flag-forwarding defect or demonstrated handle leak.
An isolated comparison used `0x0000000c` (`DETACHED_PROCESS | CREATE_SUSPENDED`):
all twelve workers had no console attachment, their explicit pipes worked and
both Lean probes passed. Instrumentation can perturb timing; these samples do
not establish a statistical failure rate or complete compatibility.

## Proposed behavior and non-goals

Launch managed Windows child tools with `DETACHED_PROCESS` and explicit standard
pipes. They start without a console. This changes console API availability;
tools that require a console are outside the supported pipe-based adapter
contract. It is not Job Object detachment. Preserve suspended launch, assignment
before user code, kill-on-close, process/memory/CPU limits, deadlines, cancellation,
raw output bounds and fail-closed teardown. A child can explicitly allocate a
console; ordinary downstream process creation can also allocate a fresh console
when its parent has none. Any remaining descendants still cause rejection.

Do not use ProcessKit's unrelated `spawn_detached`, add breakaway flags, ignore
`conhost.exe`, increase settle/cleanup deadlines, or treat host failures as
mathematical negative controls. There is no language/Lean semantics change and
no added filesystem or network sandbox.

## Implementation and trust

ProcessKit 3.3.4 has no public managed console-free builder. Vendor its pinned
source with license/provenance and one narrow opt-in builder. The upstream
`create_no_window` behavior stays intact. NMLT selects the new builder only on
Windows; the existing managed spawn path supplies `CREATE_SUSPENDED` and the
Job Object assignment. Do not carry diagnostic logging into production.
Record the vendored patch in the trusted-component inventory and retain a
machine-checkable upstream/modified file inventory.

## Compatibility and evidence interpretation

Advance all five supervisor contract identifiers to v2 and identify the patched
ProcessKit implementation and Windows console mode. Keep resource values and
result schemas unchanged. Policy validation must reject v1 contracts; do not
silently reinterpret old records under new launch behavior. Existing exact
executable identities remain enforced. Old records require their retained old
executable and policy; new runs require new records/locks where tool identity is
pinned. Linux execution semantics are unchanged, but newly recorded contract
identities name this implementation revision too.

Replay validates captured identities and observations, not physical OS events.
This change cannot repair previously failed runs or strengthen proof assurance.

## Validation and negative controls

- Demonstrate console absence with an actual child opening `CONOUT$`, while
  stdin/stdout/stderr round-trip through pipes; test single and concurrent roots.
- Keep roots with live children rejected and teardown confirmed. Retain timeout,
  cancellation/drop, abrupt-parent-death, memory/process ceilings and nested
  project tests with their existing limits.
- Run the actual asynchronous worker regressions, independent Lean version
  probes and complete native Lean/Lake proof/replay/recheck workflows.
- Reject old supervisor contract strings and altered current policies.
- Run complete Windows reproduction and Linux regression on the final tree;
  add a bounded Windows CI job. A failed check remains a blocker.

## Alternatives and unresolved questions

The existing windowless-console mode is compatible with more console APIs but
retains the observed helper lifecycle. Longer settling would only alter the
acceptance deadline without explaining ownership. Filename exemptions would
weaken containment. Replacing the full supervisor is much broader than exposing
one creation flag. A future upstream managed builder could remove the vendor
patch after equivalent review and regression checks.

Native child tools may allocate their own consoles. The full Lake/Lean path
must be checked rather than inferred from a version probe. No claim is made
that every observed Lean `Io` has been diagnosed or that all external tools are
compatible. Publication and maintainer RFC acceptance remain separate actions.

Microsoft references: [process creation flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags),
[console handles](https://learn.microsoft.com/en-us/windows/console/console-handles).

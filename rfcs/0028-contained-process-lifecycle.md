# RFC 0028: Contained local process lifecycle

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-08

## Contract

R2 external jobs need deadlines and cleanup that cover descendants, not only the
direct child. The common supervisor uses pinned ProcessKit 3.3.4 to create a
private process container, preserves bounded raw stdout/stderr, forwards only
explicit environment variables, and retains independent timeout/cancellation.
The synchronous worker route uses this same supervisor.

On Windows, the kernel Job Object is assigned before child code executes. Its
limits are 1 GiB of committed memory across the job, 16 processes, and one core's
CPU rate; the kernel kills the whole job when the supervising process dies.
Requested setup failures are errors, never an unbounded fallback. On Unix,
ProcessKit selects a cgroup/reaper when available or a process group; inherited
per-process limits bound data-segment allocation to 2 GiB, CPU time to 32 seconds, file
size to 64 MiB, descriptors to 256, and core dumps to zero. Linux also requests
direct-child parent-death cleanup. The observed mechanism and applicable limits
are recorded explicitly. [ProcessKit containment and limits](https://docs.rs/crate/processkit/3.3.4/source/docs/process-groups.md).

The process-group fallback cannot contain a deliberately detached session, and
Unix parent-death cleanup does not promise to kill the whole tree. These are
explicit platform limits, not equivalent guarantees. Neither profile is a
filesystem or network sandbox: processes retain the user's OS permissions.
Source/tool trust and a stronger security sandbox remain separate from this
process/resource contract. [Platform boundaries](https://docs.rs/crate/processkit/3.3.4/source/docs/untrusted-children.md).

## Outcomes and identity

Every exit path requests container teardown. Unconfirmed teardown remains an
uncertain host outcome. A root leaving live descendants is not reported as a
successful completed tool. A definite missing executable reports no child;
ambiguous startup failures remain conservative. Pipe overflow is an explicit
failure, including stderr bytes before text decoding; no truncated response
can settle an attempt as success.

The session configuration advances to version 2 and binds the supervisor
contract. Captured observations include the actual containment policy for
started processes. Replay validates it as captured configuration, not as proof
that an OS event physically occurred. Old records require their original
executable; local ownership still does not imply exactly-once effects.

## Validation

Retain worker/Lean, raw-byte, timeout, cancellation, and output-bound regressions.
Add descendants holding inherited pipes, explicit cancellation/drop cleanup,
Windows abrupt-parent-death cleanup, and Windows memory/process ceilings. All
test children and markers remain within bounded local fixtures. Record platform
coverage without attributing the Windows guarantees to an untested Unix host.

Linux integration follow-up (2026-09-09): the supervisor now requests whole-group
termination when its deadline/cancellation fires, before waiting for inherited
pipe EOF. A shared-group ProcessKit run controls its direct child; waiting for
its output before killing the group let a descendant retain the pipes. Linux
and macOS fallback monitoring uses enriched membership to distinguish a vanished
leader from a still-live process group. The existing descendant fixtures cover
timeout, cancel, drop and root exit; process limits and the documented `setsid`
limitation of the POSIX fallback are unchanged.

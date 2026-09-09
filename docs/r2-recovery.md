# Resume saved local work

Source job runs retain their source files and executable in the jobs directory.
Each operation's intent and reply is flushed before execution continues. The
[recovery contract](../rfcs/0030-durable-source-resumption.md) specifies exactly
what can be restored and what remains uncertain.

```bash
nmlt jobs-recover target/jobs
nmlt jobs-resume target/jobs --emit-run target/resumed.json
```

For Lean jobs, add `--lean-bin` with the original pinned installation. If the
compiler/runtime has changed, invoke the retained `target/jobs/nmlt` executable
(`nmlt.exe` on Windows). Resumption takes the saved inputs and limits; it does
not accept replacements. Replay a completed record with its retained source:

```bash
nmlt replay target/resumed.json --source target/jobs/sources/main.nmlt
```

If a dispatched action has no saved completed observation, the command reports
its run, task, slot and generation. After inspecting/reconciling that external
action, explicitly acknowledge the remaining uncertainty:

```bash
nmlt jobs-resume target/jobs --emit-run target/reconciled.json --acknowledge-uncertain-effects "Checked the local effect; retain the original spend and continue from failure"
```

This records a failure, retains the charged attempt, and lets source code choose
its fallback. It does not rerun the old action or assert that it never completed.
A saved successful result is reused; a lost collect reply does not collect twice.
Earlier poll replies still select the same source branches.

For a project, use the saved run directory containing `invocation.json`:

```bash
nmlt resume project/.nmlt/run-PID-STAMP --project project
```

The same uncertainty option is available. The command uses the saved manifest,
lock, source closure, input values and budgets, and prints the new `project.json`
path. Working edits do not alter the resumed invocation. The original job
journal remains the single source of authority across subsequent resumptions.
All project jobs use this host, including `job_square`. For a standalone
`job_square` run, supply `--job-slots` to select it.

An incomplete final journal append stops recovery. Preserve and remove only
that uncommitted suffix with the explicit repair command:

```bash
nmlt jobs-repair target/jobs --acknowledge-incomplete-tail "Inspected the interrupted final append"
```

The command saves the suffix and hashes before trimming it. Complete corrupt
rows, identity mismatches and missing headers remain errors. It starts no work
and does not acknowledge unresolved effects; run `jobs-resume` afterward. A
source/runtime I/O failure can leave a partial outer result file, so use a new
`--emit-run` path. No JSON editing is required for the supported recovery paths.

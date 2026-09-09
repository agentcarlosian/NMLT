# Local projects

The project loop wraps the existing executable-only workflow interpreter and
job adapters. [RFC 0025](../rfcs/0025-local-projects-and-dependency-locks.md)
is under review. The [completion tracker](r2-completion-tracker.md) keeps the
other R2 requirements visible.

```bash
nmlt init demo
nmlt fmt demo
nmlt check-project demo
nmlt run demo
nmlt run demo --arg input=4
nmlt test demo
```

The initialized example starts two workers, handles a failed negative input,
and reuses a collected value. Default inputs return `Ok(50)`; changing `input`
to 4 returns `Ok(32)`. The generated tests assert those distinct results.
A failed assertion or bounded execution stop gives a nonzero exit code.
Malformed test inputs and expectations fail before dispatch. `check-project`
performs Rust checks and lock validation; it does not prove a program correct.

`nmlt.toml` selects `source`, `entry`, `max_steps`, typed `[inputs]`, optional
`[jobs]` with `slots`, `max_attempts`, and `timeout_ms`, and `[[tests]]` with
`name`, `entry`, `inputs`, and `expect`. Expectations match the function's return
type; for example `expect = { Ok = 50 }`. No shell command is taken from a
manifest. An optional `[tools]` table selects `lean` by path. Local imports
continue to use the canonical sibling-module rules.

`nmlt.lock` records the exact executable, imported source dependencies, and the
configured Lean installation's `bin`/`lib` file identities. An edited main file
or changed input does not require relocking. Changed imports or installed tools
require an explicit `nmlt lock demo` update. The lock is generated JSON; users
configure projects in TOML and do not edit lock/record JSON by hand. Hashing a
Lean installation takes longer than a worker-only check. The OS, external system
libraries, and concurrent filesystem changes remain host trust assumptions.

Each run writes a fresh directory under `.nmlt/` with a source snapshot,
underlying source execution, project context, stderr, and job evidence. The exact
executable is retained once per identity in `.nmlt/`. The result prints the
`project.json` path. Replay uses saved sources even after working-source edits:

```bash
nmlt replay demo/.nmlt/run-.../project.json --project demo
```

Replay validates recorded inputs, bounds, identities, dependency bytes, and the
underlying execution. It launches no jobs and needs no original live journal.
Use the retained executable after rebuilding/upgrading NMLT. Captured Lean output
is evidence of the original run; replay is not a fresh Lean check. Neither
replay nor locks authenticate a deliberately forged store.

`fmt` formats the workflow import closure, preserving strings, comments, and
non-whitespace tokens. Compilation of every planned output precedes writes.
`fmt --check` reports differences and leaves files intact. An imported library
changed by formatting requires relocking. This formatter serves the workflow
profile; finite behavior source is not silently reformatted under workflow rules.

Prefix commands with `--json` to receive `nmlt-diagnostic-v1` on stderr. Workflow
compiler failures preserve file, half-open byte span, line/column, and related
declaration locations. Type mismatches include expected/actual types. CLI,
manifest, file, and execution errors use the same envelope; fields without an
applicable source location are null. Successful result formats retain their
meaning and `assurance: none`.

All project job calls now use the resumable source host, including the
`job_square` convenience operation. Continue a saved invocation with
`nmlt resume <run-directory> --project <project-directory>`; unresolved effects
require explicit operator acknowledgement. The command retains the original
job budget, sources and manifest and writes a new project result. See the
[recovery guide](r2-recovery.md), including incomplete-tail quarantine.

# Reusable source packages

R2 workflows can share functions and nominal records across local source files.
Use the existing `import Name` syntax to load sibling `Name.nmlt`. The workflow
route typechecks every file in the import graph and records all their identities.
This is executable-only: package loading does not start host jobs or check Lean.

The [package example](../examples/pivot/package_batch/main.nmlt) separates its
entry, input handling, arithmetic, and reports into four files:

| File | Responsibility |
|---|---|
| `main.nmlt` | Import Work and Reports, fold a batch into a summary |
| `Work.nmlt` | Define Item and try its input, then its fallback |
| `Arithmetic.nmlt` | Return the square of a nonnegative Int as an Outcome |
| `Reports.nmlt` | Define Summary and update accepted/rejected counts and total |

From the repository root, using Bash quoting for the JSON argument:

```bash
cargo build -p nmlt-cli
target/debug/nmlt typecheck examples/pivot/package_batch/main.nmlt --profile workflow
target/debug/nmlt run examples/pivot/package_batch/main.nmlt --entry main --arg 'items=[{"input":-3,"fallback":5},{"input":4,"fallback":9},{"input":-1,"fallback":-2}]' --max-steps 1000 --emit-run target/package-run.json
target/debug/nmlt replay target/package-run.json --source examples/pivot/package_batch/main.nmlt
```

Choose a new record filename for each run. This batch returns a
`Reports.Summary` with accepted 2, rejected 1, and total 41. An empty list
returns a summary of zeros. The existing [pure workflow guide](r2-pure-workflows.md)
covers records, lists, folds, outcomes, and execution limits.

## Import rules

```nmlt
import Arithmetic

fn twice(input: Int) -> Outcome<Int> {
  match Arithmetic.square(input) {
    Ok(value) => Ok(value + value),
    Err(message) => Err(message)
  }
}
```

Put imports at the file root. Every import names a file in the entry directory,
with exact spelling and case. Basenames are ASCII identifiers of at most 64
characters, followed by `.nmlt`. Subdirectories, path imports, aliases, reserved
import names, and case-conflicting filenames are unsupported. Source files must
be regular files; symbolic links are rejected.

Library declarations receive their filename namespace: the record `Item` in
`Work.nmlt` is `Work.Item`. Inside Work, unqualified names search the current local
module and then Work's file root. Local `module` wrappers still group declarations.
Record-only dependencies are allowed.

Import a library in every file that refers to its name. Work importing Arithmetic
does not let main call `Arithmetic.square` unless main also imports Arithmetic.
No imported namespace can collide with that file's top-level declarations, and
entry declarations cannot collide with any loaded library namespace. Unused
library functions are checked; an unrelated sibling file is outside the package.

Packages are capped at 32 sources, 128 KiB per file, 1 MiB combined source,
eight import edges on the longest path, and 64 functions and 64 records total.
Cycles, invalid declarations, missing files, and exceeded limits are errors.

## Replay and diagnostics

Pure run formats and the pure workflow profile are version 3. The `sources`
manifest records every loaded file's logical name, byte length, and SHA-256.
Runtime stop locations contain a `source` index into this manifest and local
`start`/`end` byte offsets. Compiler diagnostics identify the original file.

Changing even an unused dependency comment invalidates old replay. Restore the
exact source bytes or create a new run. Moving the whole package directory is
allowed. The entry file may also be renamed when passed to replay: its recorded
logical name is retained, while dependencies still use their imported names.
Older version 1/2 records require their original executable or a fresh run;
the current executable does not migrate them.

The manifest describes the per-file bytes actually compiled. It does not
authenticate a producer or claim an atomic snapshot of a concurrently changing
directory. Replay preserves incomplete outcomes and never turns them into a
successful host job or Lean proof. See [RFC 0021](../rfcs/0021-workflow-source-packages.md)
for the reader API, scope rules, limits, and remaining design questions.

Packages can also contain the opt-in effects described in the
[source job guide](r2-source-jobs.md). Effect summaries propagate across imports;
pure entries remain on the version 3 route. Workflow typecheck output is now
version 4 and lists `requires_jobs` for each entry.

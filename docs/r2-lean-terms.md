# R2 Lean proof terms

The executable-only workflow profile accepts statement and candidate proof
terms as ordinary `Text` values:

```nmlt
fn check(statement: Text, proof: Text) -> Outcome<Text> {
  let handle = job_start_lean_check(statement, proof);
  job_collect(handle)
}
```

For example, pass `statement="forall n : Nat, n + 0 = n"` and
`proof="fun n => Nat.add_zero n"` through the CLI's JSON-valued `--arg` options.
[The full example](../examples/pivot/lean_terms.nmlt) first tries an incorrect
candidate, then uses the supplied proof after the failed job. Configure the
pinned direct executable with `--lean-bin` or the project's `[tools]` setting.

The [RFC](../rfcs/0029-pinned-init-proof-terms.md) defines the closed term grammar
and limits. Statements are checked as propositions in `Init`; candidates must
type-check with an empty transitive axiom set. No arbitrary tactic or command
channel is provided. A successful collection contains the generated source
hash. A rejection or axiom-policy failure is `Err(Text)` and can select source
fallback. Unsupported syntax or oversized requests fail before dispatch.

Every request binds the direct executable, pinned version, policy and complete
`bin`/`lib` installation identity. The process/resource policy is retained with
the observation. Replay validates the captured operation and receipt; it does
not launch Lean or claim fresh proof checking. Imported Lean projects and
reviewed target environments remain part of R3.

Run `make r2-lean-terms` for real arithmetic, logic, axiom rejection, fallback
and unchanged-journal replay checks.

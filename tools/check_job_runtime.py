#!/usr/bin/env python3
"""Exercise the local subprocess prototype; this is not a Lean check."""

from __future__ import annotations

import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    parent = ROOT / "target" / "r2-jobs"
    parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    cases = [("retry", ["-3", "5"], 25, 2),
             ("direct", ["6"], 36, 1),
             ("failed", ["-3"], None, 1)]
    for name, inputs, expected, count in cases:
        journal = output / f"{name}.jsonl"
        result = subprocess.run(
            ["cargo", "run", "--quiet", "-p", "nmlt-runtime", "--example",
             "local_worker", "--", str(journal), *inputs],
            cwd=ROOT, capture_output=True, text=True, timeout=120, check=False)
        if (result.returncode == 0) != (expected is not None):
            raise SystemExit(f"{name}: wrong process outcome: {result.stderr}")
        record = json.loads(result.stdout)
        (output / f"{name}.json").write_text(result.stdout, encoding="utf-8")
        if record["assurance"] != "none" or len(record["outcomes"]) != count:
            raise SystemExit(f"{name}: incorrect scope or attempt count")
        if record["accounting"]["charged_work"] != count:
            raise SystemExit(f"{name}: dispatched work was not retained")
        expected_value = None if expected is None else {"type": "int", "value": expected}
        if record["reused_values"] != [expected_value, expected_value]:
            raise SystemExit(f"{name}: wrong immutable result reuse")
        if expected is None and record["outcomes"][-1]["kind"] != "failed":
            raise SystemExit("failed work was promoted to success")
        events = [json.loads(line) for line in journal.read_text().splitlines()]
        # Five committed transitions per attempt: allocate, transfer, dispatch,
        # validated delivery, collect. Each child starts after dispatch commits.
        if len(events) != 1 + 5 * count:
            raise SystemExit(f"{name}: wrong journal length")
        if [entry["command"]["kind"] for entry in events[1:]] != [
                "reserve", "transfer", "dispatch", "deliver", "collect"] * count:
            raise SystemExit(f"{name}: wrong lifecycle order")
    print(f"ok: 3 local subprocess cases; failure/retry, direct result, and all-failed; {output.relative_to(ROOT)}")
    print("scope: executable-only worker prototype, assurance none")


if __name__ == "__main__":
    main()

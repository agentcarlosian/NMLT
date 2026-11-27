#!/usr/bin/env python3
"""Reference NMLT custom adapter speaking `nmlt-job-response-v1` (RFC 0042).

Deterministically proposes guarded Lean 4 tactic scripts (`v3`) for named
strategy/goal queries. Always echoes `binding` verbatim and emits compact
canonical JSON with empty stderr.
"""

from __future__ import annotations

import argparse
import json
import sys


FAST_STRATEGIES = {
    "true_intro": "by\n  exact True.intro\n",
    "add_zero": "by\n  intro n\n  simpa only [Nat.add_zero]\n",
    "zero_add": "by\n  intro n\n  simpa only [Nat.zero_add]\n",
    "refl": "by\n  rfl\n",
}

DEEP_STRATEGIES = {
    "add_comm": (
        "by\n"
        "  intro n m\n"
        "  induction n with\n"
        "  | zero => simpa only [Nat.zero_add, Nat.add_zero]\n"
        "  | succ k ih => simp [Nat.succ_add, Nat.add_succ, ih]\n"
    ),
    "omega_linear": "by\n  intro n m h\n  omega\n",
}


def propose(profile: str, query: str) -> tuple[bool, str]:
    table = FAST_STRATEGIES if profile == "fast" else {**FAST_STRATEGIES, **DEEP_STRATEGIES}
    if query in table:
        return True, table[query]
    return False, f"no {profile} strategy for goal '{query}'"


def main() -> int:
    parser = argparse.ArgumentParser(description="NMLT reference tactic proposer adapter")
    parser.add_argument("--profile", choices=("fast", "deep"), default="fast")
    args = parser.parse_args()

    raw = sys.stdin.read()
    dispatch = json.loads(raw)
    binding = dispatch["binding"]
    inner = json.loads(dispatch["input"]["value"])
    query = inner["input"]

    ok, payload = propose(args.profile, query)
    if ok:
        outcome = {"kind": "completed", "value": {"type": "text", "value": payload}}
    else:
        outcome = {"kind": "failed", "message": payload}

    response = {
        "schema": "nmlt-job-response-v1",
        "binding": binding,
        "outcome": outcome,
        "observed_work": 1,
    }
    sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

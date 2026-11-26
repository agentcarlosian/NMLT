#!/usr/bin/env python3
"""Summarize paired manual observations. This neither runs nor authenticates a pilot."""
import argparse
import csv
import json
import math
from pathlib import Path
import statistics

AUDIENCES = {"mathematician", "Lean user", "agent-system engineer"}
NUMERIC = ("setup_seconds", "hands_on_seconds", "diagnosis_seconds", "maintenance_seconds",
           "cpu_seconds", "peak_memory_mb", "model_tokens")
BOOLEAN = ("accepted", "rechecked", "reuse_explained")


def summarize(rows: list[dict]) -> dict:
    pairs, participants = {}, {}
    quality = {arm: {"accepted": 0, "rejected": 0} for arm in ("library", "nmlt")}
    required_workflows_complete = bool(rows)
    for source in rows:
        row = dict(source)
        if row["arm"] not in quality or row["audience"] not in AUDIENCES or not row["participant"] or not row["task"]:
            raise ValueError("missing participant/task or unknown audience/arm")
        if row.get("arm_position") not in ("1", "2"):
            raise ValueError("arm_position must record first (1) or second (2)")
        row["arm_position"] = int(row["arm_position"])
        repetition = int(row["repetition"])
        if repetition < 0:
            raise ValueError("repetition must be nonnegative")
        previous = participants.setdefault(row["participant"], row["audience"])
        if previous != row["audience"]:
            raise ValueError("one participant cannot fill multiple independent audience slots")
        for name in NUMERIC:
            row[name] = float(row[name])
            if not math.isfinite(row[name]) or row[name] < 0:
                raise ValueError(f"invalid nonnegative observation: {name}")
        for name in BOOLEAN:
            if row[name] not in ("true", "false"):
                raise ValueError(f"expected explicit true/false for {name}")
            row[name] = row[name] == "true"
        expected, rejected = int(row["invalid_controls_expected"]), int(row["invalid_controls_rejected"])
        if expected < 1 or rejected < 0 or rejected > expected:
            raise ValueError("invalid negative-control denominator")
        if row["accepted"] and not row["rechecked"]:
            raise ValueError("an unrechecked success cannot enter accepted pilot evidence")
        row["controls_complete"] = expected == rejected
        required_workflows_complete &= row["accepted"] and row["reuse_explained"] and row["controls_complete"]
        row["invalid_controls_expected"] = expected
        quality[row["arm"]]["accepted"] += int(row["accepted"])
        quality[row["arm"]]["rejected"] += int(not row["accepted"])
        key = (row["participant"], row["task"], repetition)
        pair = pairs.setdefault(key, {})
        if row["arm"] in pair:
            raise ValueError("duplicate participant/task/repetition/arm")
        pair[row["arm"]] = row
    reductions, unresolved, grouped_orders = {}, [], {}
    quality_regression = False
    for key, pair in pairs.items():
        if set(pair) != {"library", "nmlt"}:
            raise ValueError("incomplete paired observation; retain it as an unfinished trial")
        if {row["arm_position"] for row in pair.values()} != {1, 2}:
            raise ValueError("each pair must record complementary arm positions")
        order = tuple(sorted(pair, key=lambda arm: pair[arm]["arm_position"]))
        grouped_orders.setdefault(key[:2], []).append(order)
        base, trial = pair["library"], pair["nmlt"]
        if base["invalid_controls_expected"] != trial["invalid_controls_expected"]:
            raise ValueError("arms used different negative-control denominators")
        quality_regression |= (base["accepted"] and not trial["accepted"]) or not trial["controls_complete"]
        if not base["controls_complete"]:
            unresolved.append({"pair": key, "reason": "baseline controls incomplete"})
        if base["hands_on_seconds"] == 0:
            unresolved.append({"pair": key, "reason": "zero baseline time; ratio undefined"})
        else:
            reductions.setdefault(key[0], []).append(1 - trial["hands_on_seconds"] / base["hands_on_seconds"])
    paired_protocol_complete = bool(grouped_orders)
    repetition_groups = []
    for (participant, task), orders in sorted(grouped_orders.items()):
        complete = len(orders) >= 2 and set(orders) == {("library", "nmlt"), ("nmlt", "library")}
        paired_protocol_complete &= complete
        repetition_groups.append({"participant": participant, "task": task,
                                  "paired_repetitions": len(orders),
                                  "observed_orders": [list(order) for order in sorted(set(orders))],
                                  "complete": complete})
        if not complete:
            unresolved.append({"participant": participant, "task": task,
                               "reason": "at least two paired repetitions covering both arm orders are required"})
    # Aggregate within participant first; repeated runs are not independent users.
    participant_reductions = {key: statistics.median(value) for key, value in reductions.items()}
    median = statistics.median(participant_reductions.values()) if participant_reductions else None
    roles = {role: sum(value == role for value in participants.values()) for role in sorted(AUDIENCES)}
    return {"schema": "nmlt-qualitative-pilot-summary-v1", "assurance": "none",
            "status": "no_observations" if not rows else "manual_observations_require_review",
            "participants": len(participants), "audience_counts": roles, "paired_observations": len(pairs),
            "paired_protocol_complete": paired_protocol_complete, "repetition_groups": repetition_groups,
            "participant_median_reductions": participant_reductions, "median_reduction": median,
            "observed_participant_range": [min(participant_reductions.values()), max(participant_reductions.values())]
                if participant_reductions else None,
            "uncertainty": "descriptive range only; no population confidence or general usefulness claim",
            "quality_counts": quality, "quality_regression": quality_regression, "unresolved": unresolved,
            "required_workflows_complete": required_workflows_complete,
            "descriptive_target_met": bool(median is not None and median >= 0.20 and all(roles.values())
                                           and paired_protocol_complete and required_workflows_complete
                                           and not quality_regression and not unresolved),
            "independence_and_evidence_authentication": "must be assessed by the pilot coordinator"}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    with args.csv.open(encoding="utf-8", newline="") as handle:
        result = summarize(list(csv.DictReader(handle)))
    with args.output.open("x", encoding="utf-8", newline="\n") as handle:
        json.dump(result, handle, indent=2)
        handle.write("\n")
    print(f"{result['participants']} observed participants; {result['status']}; {args.output}")


if __name__ == "__main__":
    main()

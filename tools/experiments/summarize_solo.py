#!/usr/bin/env python3
"""Validate a separate solo evidence index; never authenticate human or proof claims."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import tomllib

from laboratory import ROOT, digest, read
from replay_evaluation import expected_type_rejection, native, source_response

PACKET = ROOT / "examples/evaluation/solo-packet.json"


def inspect_result(path: Path, task: dict, pin: str, candidate: dict, tools: dict) -> dict:
    """Check retained result identity/bytes, without replacing native verification."""
    record = read(native(path))
    if (record.get("schema") != "nmlt-lean-result-v5"
            or record.get("status") != "independently_checked"
            or record.get("task_sha256") != pin or record.get("task") != task
            or record.get("candidate") != candidate
            or candidate.get("task_sha256") != pin
            or candidate.get("schema") not in ("nmlt-lean-proof-candidate-v1", "nmlt-lean-proof-candidate-v2", "nmlt-lean-proof-candidate-v3")
            or not re.fullmatch(r"[0-9a-f]{64}", pin)
            or record.get("checked_declarations", 0) <= 0
            or record.get("checked_declarations") != len(record.get("exported_declarations", []))
            or "NMLTChecked.result" not in record.get("exported_declarations", [])):
        raise ValueError("wrong native proof result identity or declaration count")
    if (task.get("implementation_sha256") != tools.get("nmlt")
            or task.get("lean", {}).get("executable_sha256") != tools.get("lean")
            or record.get("tools", {}).get("exporter_sha256") != tools.get("exporter")
            or record.get("tools", {}).get("nanoda_sha256") != tools.get("nanoda")
            or any(not re.fullmatch(r"[0-9a-f]{64}", tools.get(name, ""))
                   for name in ("nmlt", "lean", "exporter", "nanoda"))):
        raise ValueError("native result tools differ from the recorded run")
    export = native(path.parent / "build/environment.ndjson").read_bytes()
    if (not export or len(export) != record.get("export_bytes")
            or hashlib.sha256(export).hexdigest() != record.get("export_sha256")):
        raise ValueError("changed proof export bytes")
    return record


def summarize(report: dict, directory: Path) -> dict:
    if report.get("schema") != "nmlt-solo-validation-v1":
        raise ValueError("unknown solo evidence schema")
    packet = read(PACKET)
    if report.get("packet_sha256") != digest(packet):
        raise ValueError("solo packet changed")
    if (report.get("independent_human_participants") != 0
            or report.get("live_model_calls") != 0):
        raise ValueError("this solo track is not an independent human/model study")
    receipts = report.get("receipts", [])
    if not receipts or len({r["id"] for r in receipts}) != len(receipts):
        raise ValueError("missing or duplicate execution receipts")
    by_id = {}
    for receipt in receipts:
        path = (directory / receipt["path"]).resolve()
        if not path.is_relative_to(directory.resolve()) or not path.is_file():
            raise ValueError("missing or escaping evidence path")
        if hashlib.sha256(path.read_bytes()).hexdigest() != receipt["sha256"]:
            raise ValueError("changed execution receipt")
        by_id[receipt["id"]] = (receipt, path)

    def evidence_path(value):
        path = (directory / value).resolve()
        if not path.is_relative_to(directory.resolve()):
            raise ValueError("escaping execution artifact")
        return path

    def command_record(identifier, expected):
        record = read(by_id[identifier][1])
        if record.get("exit_code") != expected or record.get("timed_out") is not False:
            raise ValueError("command did not reach its exact expected exit")
        for stream in ("stdout", "stderr"):
            if hashlib.sha256(evidence_path(record[stream]).read_bytes()).hexdigest() != record[stream + "_sha256"]:
                raise ValueError("changed command output")
        return record
    roles = report.get("roles", {})
    if set(roles) != set(packet["roles"]):
        raise ValueError("missing role walkthrough")
    for role, operations in packet["roles"].items():
        if set(roles[role]) != set(operations):
            raise ValueError(f"missing operation for {role}")
        for operation, ids in roles[role].items():
            if not ids or any(identifier not in by_id for identifier in ids):
                raise ValueError(f"missing receipt for {role}/{operation}")
    pairs = report.get("proof_rechecks", [])
    tool_paths = report.get("tool_paths", {})
    if set(tool_paths) != {"nmlt", "lean", "lake", "exporter", "nanoda"}:
        raise ValueError("missing actual command tool paths")
    if not re.fullmatch(r"[0-9a-f]{64}", report.get("tool_sha256", {}).get("lake", "")):
        raise ValueError("missing Lake executable identity")
    tool_args = [value for name, option in (("lean", "--lean-bin"), ("exporter", "--exporter"), ("nanoda", "--nanoda"))
                 for value in (option, tool_paths[name])]
    if (len(pairs) != len(packet["proof_rechecks"])
            or {pair.get("label") for pair in pairs} != set(packet["proof_rechecks"])):
        raise ValueError("missing specified accepted/fresh native proof pair")
    for pair in pairs:
        paths = [by_id[pair[key]][1] for key in ("accepted", "fresh")]
        if paths[0].samefile(paths[1]):
            raise ValueError("fresh checking needs a distinct result artifact")
        task = read(by_id[pair["task"]][1])
        pin = by_id[pair["task_pin"]][1].read_text(encoding="utf-8").strip()
        candidate = read(by_id[pair["candidate"]][1])
        records = [inspect_result(path, task, pin, candidate, report.get("tool_sha256", {})) for path in paths]
        accepted_command = command_record(pair["accepted_command"], 0)["command"]
        if any(path.name != "result.json" for path in paths):
            raise ValueError("native result artifact has the wrong output filename")
        if accepted_command[1:3] == ["lean-task", "prove"]:
            expected = [tool_paths["nmlt"], "lean-task", "prove", "--task", str(by_id[pair["task"]][1]),
                        "--task-sha256", pin, "--candidate", str(by_id[pair["candidate"]][1]),
                        *tool_args, "--output", str(paths[0].parent)]
        elif len(accepted_command) > 1 and accepted_command[1] == "run":
            run_path, source_path, registry_path = (by_id[pair[key]][1] for key in ("source_run", "source", "registry"))
            run = read(run_path)
            if (run["context"]["assurance"] != "none" or run["execution"]["stop"]["kind"] != "returned"
                    or run["execution"]["stop"]["value"]["kind"] != "ok" or len(run["snapshot"]["observations"]) != 1):
                raise ValueError("source command did not retain exactly one successful observation")
            response = source_response(run["snapshot"]["observations"][0], {"schema": "nmlt-project-proof-request-v1",
                "alias": pair["alias"], "task_sha256": pin, "proof": candidate["proof"]}, False)
            jobs = paths[0].parents[3]
            if (paths[0] != jobs / "projects/results" / response["dispatch_sha256"] / "result.json"
                    or response["result_sha256"] != hashlib.sha256(paths[0].read_bytes()).hexdigest()
                    or response["export_sha256"] != records[0]["export_sha256"]):
                raise ValueError("source observation identifies a different accepted result")
            registry = tomllib.loads(registry_path.read_text(encoding="utf-8"))
            if (registry.get("schema") != "nmlt-lean-project-jobs-v1"
                    or registry.get("timeout_ms") != packet["native_timeout_seconds"] * 1000
                    or registry.get("tasks") != [{"alias": pair["alias"], "task": str(by_id[pair["task"]][1].relative_to(registry_path.parent)).replace("\\", "/"), "task_sha256": pin}]
                    or any(Path(registry[key]).resolve() != Path(tool_paths[name]).resolve()
                           for key, name in (("lean_bin", "lean"), ("exporter", "exporter"), ("nanoda", "nanoda")))):
                raise ValueError("source registry differs from the selected task/tools/limits")
            expected = [tool_paths["nmlt"], "run", str(source_path), "--entry", "check", "--arg", "alias=" + json.dumps(pair["alias"]),
                        "--arg", "proof=" + json.dumps(candidate["proof"]), "--max-steps", "200", "--emit-run", str(run_path),
                        "--jobs-dir", str(jobs), "--job-slots", "1", "--max-jobs", "2", "--job-timeout-ms", "30000",
                        "--lean-projects", str(registry_path)]
        else:
            raise ValueError("accepted command is not the selected prove/source operation")
        if accepted_command != expected:
            raise ValueError("accepted command does not identify the selected task/candidate/tools/result")
        fresh_command = command_record(pair["fresh_command"], 0)["command"]
        if fresh_command != [tool_paths["nmlt"], "lean-task", "recheck", "--record", str(paths[0]),
                             "--task-sha256", pin, *tool_args, "--output", str(paths[1].parent)]:
            raise ValueError("fresh command does not identify the distinct accepted/result artifacts")
        if any(records[0][key] != records[1][key]
               for key in ("task_sha256", "task", "candidate", "tools", "export_sha256", "export_bytes", "exported_declarations")):
            raise ValueError("fresh check does not match its accepted proof")
    controls = report.get("negative_controls", [])
    if (len(controls) != len(packet["negative_controls"])
            or {c["id"]: c["expected"] for c in controls} != packet["negative_controls"]):
        raise ValueError("missing, duplicate or substituted specified negative controls")
    for control in controls:
        if control["receipt"] not in by_id or control["observed"] != control["expected"]:
            raise ValueError("host failure or wrong rejection cannot count as a semantic control")
        if control["expected"] not in ("type_rejection", "stale_identity_rejection", "checked_refutation"):
            raise ValueError("unknown semantic control")
        receipt, path = by_id[control["receipt"]]
        command = command_record(control["command"], 0 if control["expected"] == "checked_refutation" else 1)
        if control["expected"] != "checked_refutation":
            argv = command["command"]
            if (argv[1:3] != ["lean-task", "prove"]
                    or evidence_path(command["stderr"]) != path
                    or (evidence_path(argv[argv.index("--output") + 1]) / "result.json").exists()):
                raise ValueError("negative control retained a success or has the wrong command/diagnostic")
        if control["expected"] == "type_rejection" and not expected_type_rejection(path.read_text(encoding="utf-8")):
            raise ValueError("type rejection receipt contains no native type diagnostic")
        if control["expected"] == "stale_identity_rejection" and "selected task hash" not in path.read_text(encoding="utf-8"):
            raise ValueError("stale identity receipt contains no selected-task rejection")
        if control["expected"] == "checked_refutation":
            if (command.get("label") != "math-reuse"
                    or command["command"][0] != tool_paths["lake"]
                    or command["command"][1:] != ["env", "lean", "--run", "tests/SoloReuse.lean"]
                    or "nested reuse:" not in evidence_path(command["stdout"]).read_text(encoding="utf-8")):
                raise ValueError("mathematical refutation typecheck did not succeed")
    uses = report.get("second_uses", [])
    if {use["role"] for use in uses} != set(packet["roles"]):
        raise ValueError("each role needs a second use")
    for use in uses:
        if (use["first"] == use["second"] or not use.get("explanation", "").strip()
                or use["receipt"] not in by_id):
            raise ValueError("second use lacks a distinct question/workflow and evidence")
    comparison = report.get("comparison")
    if comparison is not None:
        if (comparison.get("orders") != [["library", "nmlt"], ["nmlt", "library"]]
                or comparison.get("matched_contract") is not True
                or comparison.get("receipt") not in by_id):
            raise ValueError("comparison must retain matched reversed-order pairs")
    if not report.get("local_value") or not report.get("limitations"):
        raise ValueError("local value and limitations must be recorded")
    return {"schema": "nmlt-solo-summary-v1", "assurance": "none",
            "structural_index_complete": True, "operations_reviewed": False, "roles": list(roles),
            "proof_pairs": len(pairs), "negative_controls": len(controls),
            "independent_human_participants": 0, "live_model_calls": 0,
            "milestone_complete": False,
            "remaining_gate": "review actual operation results, complete current Rust/Lean/NanoDA/platform gates and record maintainer assessment",
            "limitations": report["limitations"],
            "note": "Checks indexed local artifacts, not operator identity or proof validity; actual checker runs and full source/tool provenance remain required."}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = summarize(read(args.report), args.report.resolve().parent)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Execute the frozen solo role packet with real CLI operations and fresh proofs."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

from laboratory import ROOT, cli_binary, digest, read
from replay_evaluation import expected_type_rejection, native, source_response
from summarize_solo import PACKET, inspect_result, summarize


def write(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8", newline="\n")


def sha(path: Path) -> str:
    return hashlib.sha256(native(path).read_bytes()).hexdigest()


def run_command(evidence: Path, commands: list, label: str, argv: list, cwd: Path,
                timeout: float, expected: int = 0, contains: str | None = None):
    """Retain failures/timeouts before checking the exact expected outcome."""
    started = time.monotonic()
    expired = None
    try:
        result = subprocess.run(list(map(str, argv)), cwd=cwd, capture_output=True,
                                text=True, encoding="utf-8", timeout=timeout)
        output = (result.stdout, result.stderr)
        exit_code = result.returncode
    except subprocess.TimeoutExpired as error:
        expired = error
        output = (error.stdout or b"", error.stderr or b"")
        exit_code = None
    stdout, stderr = evidence / f"{label}.stdout.txt", evidence / f"{label}.stderr.txt"
    for path, content in zip((stdout, stderr), output, strict=True):
        path.write_bytes(content if isinstance(content, bytes) else content.encode("utf-8"))
    entry = {"label": label, "command": list(map(str, argv)), "exit_code": exit_code,
             "timed_out": expired is not None, "timeout_seconds": timeout,
             "expected_exit_code": expected, "wall_seconds": time.monotonic() - started,
             "stdout": stdout.name, "stderr": stderr.name,
             "stdout_sha256": sha(stdout), "stderr_sha256": sha(stderr)}
    commands.append(entry)
    write(evidence / "progress.json", commands)
    write(evidence / f"{label}.command.json", entry)
    if expired is not None:
        raise expired
    if result.returncode != expected or (contains and contains not in result.stdout + result.stderr):
        raise RuntimeError(f"{label}: exit {result.returncode}; {result.stderr[-2000:]}; evidence: {evidence}")
    return result


def dispatches(directory: Path) -> dict:
    return {path.name: sha(path) for path in directory.glob("dispatch-*.json")}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("lean-bin", "exporter", "nanoda"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--output-dir", type=Path)
    args = parser.parse_args()
    packet = read(PACKET)
    evidence = args.output_dir or Path(tempfile.mkdtemp(prefix="solo-", dir=ROOT / "target"))
    if args.output_dir:
        evidence.mkdir(parents=True, exist_ok=False)
    evidence = evidence.resolve()
    source_binary = cli_binary()
    binary = evidence / source_binary.name
    shutil.copy2(source_binary, binary)
    lake = args.lean_bin.resolve().with_name("lake.exe" if os.name == "nt" else "lake")
    tool_paths = {"nmlt": binary, "lean": args.lean_bin.resolve(), "lake": lake,
                  "exporter": args.exporter.resolve(), "nanoda": args.nanoda.resolve()}
    tools = ["--lean-bin", args.lean_bin.resolve(), "--exporter", args.exporter.resolve(), "--nanoda", args.nanoda.resolve()]
    report = {"schema": "nmlt-solo-validation-v1", "packet_sha256": digest(packet),
              "independent_human_participants": 0, "live_model_calls": 0,
              "roles": {role: {op: [] for op in ops} for role, ops in packet["roles"].items()},
              "receipts": [], "proof_rechecks": [], "negative_controls": [], "second_uses": [],
              "comparison": None, "local_value": "Pending completed operations.", "limitations": packet["limits"],
              "measures": {"source_edits": 2, "manual_seconds": None, "cpu_seconds": None,
                           "peak_memory_bytes": None, "tokens": None, "explicit_acknowledgements": 0},
              "tool_paths": {name: str(path) for name, path in tool_paths.items()},
              "tool_sha256": {name: sha(path) for name, path in tool_paths.items()}}
    write(evidence / "packet.json", packet)
    inputs = [Path(__file__), PACKET, ROOT / "tools/experiments/summarize_solo.py",
              ROOT / "tools/experiments/replay_evaluation.py", ROOT / "tools/experiments/laboratory.py",
              ROOT / "tools/experiments/check_evaluation.py",
              ROOT / "mechanization/lean/tests/SoloReuse.lean",
              ROOT / "mechanization/lean/NMLT/Discovery/NestedReuse.lean",
              *sorted((ROOT / "examples/lean-project").rglob("*.lean")),
              ROOT / "examples/pivot/async_fallback.nmlt", ROOT / "examples/pivot/lean_project.nmlt"]
    report["source_sha256"] = {path.relative_to(ROOT).as_posix(): sha(path) for path in inputs}
    commands = []

    def receipt(identifier, path, kind="command"):
        report["receipts"].append({"id": identifier, "path": path.relative_to(evidence).as_posix(),
                                   "kind": kind, "sha256": sha(path)})
        return identifier

    def command(label, params, expected=0, contains=None, executable=binary, cwd=ROOT):
        print(f"start: {label}", flush=True)
        result = run_command(evidence, commands, label, [executable, *params], cwd,
                             packet["command_timeout_seconds"], expected, contains)
        receipt(label, evidence / f"{label}.command.json")
        print(f"ok: {label} ({commands[-1]['wall_seconds']:.1f}s)", flush=True)
        return result

    def cover(role, operations, *ids):
        for operation in operations:
            report["roles"][role][operation].extend(ids)

    command("math-reuse", ["env", "lean", "--run", "tests/SoloReuse.lean"],
            executable=lake,
            cwd=ROOT / "mechanization/lean", contains="nested reuse:")
    receipt("math-source", evidence / "packet.json", "frozen_packet")
    cover("mathematician", packet["roles"]["mathematician"], "math-reuse")
    report["negative_controls"].append({"id": "math-bound-and-formation", "expected": "checked_refutation",
        "observed": "checked_refutation", "receipt": "math-reuse", "command": "math-reuse"})
    report["second_uses"].append({"role": "mathematician", "first": "four-premise subset classification",
        "second": "nested monitored synchronization and resource bound", "receipt": "math-reuse",
        "explanation": "The visibility construction is applied at both nested boundaries; initialized behavior and rejected formation/bound claims prevent vacuous success."})

    project = evidence / "project"
    shutil.copytree(ROOT / "examples/lean-project", project)
    bound = evidence / "bound"
    command("bind", ["lean-task", "bind", "--project", project, "--lean-bin", args.lean_bin.resolve(), "--output", bound])
    task = bound / "task.json"; pin = (bound / "task.sha256").read_text().strip()
    candidate = evidence / "wrong.json"
    write(candidate, {"schema": "nmlt-lean-proof-candidate-v1", "task_sha256": pin, "proof": "fun n => n"})
    wrong = command("wrong", ["lean-task", "prove", "--task", task, "--task-sha256", pin,
                    "--candidate", candidate, *tools, "--output", evidence / "wrong"], expected=1)
    if not expected_type_rejection(wrong.stderr) or (evidence / "wrong/result.json").exists():
        raise RuntimeError("wrong candidate had a host failure, not a type rejection")
    receipt("wrong-diagnostic", evidence / "wrong.stderr.txt", "type_rejection")
    report["negative_controls"].append({"id": "wrong-term", "expected": "type_rejection",
        "observed": "type_rejection", "receipt": "wrong-diagnostic", "command": "wrong"})

    def prove_and_recheck(label, selected_task, selected_pin, proof):
        proposal = evidence / f"{label}.json"
        write(proposal, {"schema": "nmlt-lean-proof-candidate-v1", "task_sha256": selected_pin, "proof": proof})
        command(label, ["lean-task", "prove", "--task", selected_task, "--task-sha256", selected_pin,
                        "--candidate", proposal, *tools, "--output", evidence / label])
        return fresh(label, evidence / label / "result.json", selected_task, selected_pin, proposal)

    def fresh(label, record, selected_task, selected_pin, proposal):
        task_value, candidate_value = read(selected_task), read(proposal)
        result = inspect_result(record, task_value, selected_pin, candidate_value, report["tool_sha256"])
        command(label + "-fresh", ["lean-task", "recheck", "--record", record, "--task-sha256", selected_pin,
                                  *tools, "--output", evidence / (label + "-fresh")])
        fresh_record = evidence / (label + "-fresh/result.json")
        checked = inspect_result(fresh_record, task_value, selected_pin, candidate_value, report["tool_sha256"])
        if any(result[key] != checked[key] for key in ("tools", "export_sha256", "export_bytes", "exported_declarations")):
            raise RuntimeError("fresh proof changed tools or exported content")
        accepted_id = receipt(label + "-proof", record, "native_proof")
        fresh_id = receipt(label + "-fresh-proof", fresh_record, "fresh_recheck")
        report["proof_rechecks"].append({"label": label, "accepted": accepted_id, "fresh": fresh_id,
            "task": receipt(label + "-task", selected_task, "bound_task"),
            "task_pin": receipt(label + "-pin", selected_task.with_suffix(".sha256"), "task_pin"),
            "candidate": receipt(label + "-candidate", proposal, "candidate"),
            "accepted_command": label, "fresh_command": label + "-fresh"})
        return accepted_id, fresh_id

    first_ids = prove_and_recheck("repair", task, pin, "fun n => Example.offset_eq n")
    cover("Lean developer", ["proof_repair", "fresh_recheck"], "wrong-diagnostic", *first_ids)
    definitions = project / "Example/Definitions.lean"
    original_definitions = definitions.read_text(encoding="utf-8")
    updated_definitions = original_definitions.replace("0 + n", "n + 0").replace("Nat.zero_add n", "Nat.add_zero n")
    if "0 + n" not in original_definitions or "Nat.zero_add n" not in original_definitions or updated_definitions == original_definitions:
        raise RuntimeError("expected Example/Definitions.lean template terms were not found for revision")
    definitions.write_text(updated_definitions, encoding="utf-8", newline="\n")
    goals = project / "Example/Goals.lean"
    goals.write_text("import Example.Definitions\nnamespace Example\n"
                     "theorem zero_offset (n : Nat) : offset n + 1 = n + 1 := by sorry\n"
                     "end Example\n", encoding="utf-8", newline="\n")
    reason = evidence / "revision-reason.txt"
    reason.write_text("Second question: right-zero offset and its successor, with explicit context revision.\n", encoding="utf-8")
    revised = evidence / "revised"
    command("revise", ["lean-task", "revise", "--project", project, "--task", task, "--task-sha256", pin,
                       "--reason", reason, "--lean-bin", args.lean_bin.resolve(), "--output", revised])
    new_task = revised / "task.json"; new_pin = (revised / "task.sha256").read_text().strip()
    if new_pin == pin or read(new_task)["revision"]["parent_task_sha256"] != pin:
        raise RuntimeError("context revision did not invalidate task identity")
    command("stale", ["lean-task", "prove", "--task", new_task, "--task-sha256", new_pin,
                      "--candidate", evidence / "repair.json", *tools, "--output", evidence / "stale"], expected=1, contains="selected task hash")
    if (evidence / "stale/result.json").exists():
        raise RuntimeError("stale candidate retained an accepted result")
    receipt("stale-diagnostic", evidence / "stale.stderr.txt", "stale_identity_rejection")
    report["negative_controls"].append({"id": "stale-candidate", "expected": "stale_identity_rejection",
        "observed": "stale_identity_rejection", "receipt": "stale-diagnostic", "command": "stale"})
    proof = "fun n => congrArg Nat.succ (Example.offset_eq n)"
    second_ids = prove_and_recheck("revised-proof", new_task, new_pin, proof)
    cover("Lean developer", ["context_revision", "second_use", "fresh_recheck"], "revise", "stale-diagnostic", *second_ids)
    report["second_uses"].append({"role": "Lean developer", "first": pin, "second": new_pin,
        "receipt": second_ids[1], "explanation": "Reuse the dependency lemma and proof workflow after changing both the offset definition and successor question; reject the old candidate identity."})

    worker = evidence / "worker.nmlt"; shutil.copyfile(ROOT / "examples/pivot/async_fallback.nmlt", worker)
    def worker_args(label, entry, values):
        return ["run", worker, "--entry", entry, "--max-steps", "100", "--emit-run", evidence / (label + ".json"),
                "--jobs-dir", evidence / label, "--job-slots", "2", "--max-jobs", "2", "--job-timeout-ms", packet["worker_timeout_seconds"] * 1000,
                *[item for key, value in values.items() for item in ["--arg", f"{key}={value}"]]]
    for index, ((value, fallback), expected) in enumerate(zip(packet["worker_inputs"], packet["worker_expected_outputs"], strict=True)):
        label = f"worker-{index}"
        command(label, worker_args(label, "main", {"input": value, "fallback": fallback}))
        record = read(evidence / (label + ".json"))
        if record["execution"]["stop"] != {"kind": "returned", "value": {"kind": "ok", "value": {"kind": "int", "value": expected}}}:
            raise RuntimeError("worker fallback/reuse outcome changed")
        journal = evidence / label / "journal.jsonl"; before = journal.read_bytes()
        before_dispatches = dispatches(evidence / label)
        command(label + "-replay", ["replay", evidence / (label + ".json"), "--source", worker])
        command(label + "-recover", ["jobs-recover", evidence / label])
        if journal.read_bytes() != before or dispatches(evidence / label) != before_dispatches:
            raise RuntimeError("completed recovery changed retained journal or dispatch artifacts")
        cover("software engineer", ["modified_input", "failure", "replay", "recovery", "second_use"], label, label + "-replay", label + "-recover")
    command("worker-cancel", worker_args("worker-cancel", "cancel", {"input": 5}))
    cancelled = read(evidence / "worker-cancel.json")
    journal_rows = [json.loads(line) for line in cancelled["snapshot"]["journal"].splitlines()]
    if not any(row.get("command", {}).get("kind") == "cancel" for row in journal_rows):
        raise RuntimeError("cancellation request was not retained")
    command("worker-cancel-replay", ["replay", evidence / "worker-cancel.json", "--source", worker])
    cover("software engineer", ["cancellation"], "worker-cancel", "worker-cancel-replay")

    source = evidence / "proof.nmlt"; shutil.copyfile(ROOT / "examples/pivot/lean_project.nmlt", source)
    registry = evidence / "projects.toml"
    registry.write_text("\n".join(['schema = "nmlt-lean-project-jobs-v1"',
        f'lean_bin = {json.dumps(args.lean_bin.resolve().as_posix())}', f'exporter = {json.dumps(args.exporter.resolve().as_posix())}',
        f'nanoda = {json.dumps(args.nanoda.resolve().as_posix())}', f'timeout_ms = {packet["native_timeout_seconds"] * 1000}', '[[tasks]]',
        'alias = "revised"', 'task = "revised/task.json"', f'task_sha256 = "{new_pin}"', '']), encoding="utf-8", newline="\n")
    def native_args(label):
        return ["run", source, "--entry", "check", "--arg", 'alias="revised"', "--arg", "proof=" + json.dumps(proof),
                "--max-steps", "200", "--emit-run", evidence / (label + ".json"), "--jobs-dir", evidence / label,
                "--job-slots", "1", "--max-jobs", "2", "--job-timeout-ms", "30000", "--lean-projects", registry]
    command("native-source", native_args("native-source"))
    source_record = read(evidence / "native-source.json")
    if (source_record["context"]["assurance"] != "none"
            or source_record["execution"]["stop"]["kind"] != "returned"
            or source_record["execution"]["stop"]["value"]["kind"] != "ok"):
        raise RuntimeError("source native proof did not complete")
    observations = source_record["snapshot"]["observations"]
    if len(observations) != 1:
        raise RuntimeError("native proof had a host failure")
    proof_receipt = source_response(observations[0], {"schema": "nmlt-project-proof-request-v1",
        "alias": "revised", "task_sha256": new_pin, "proof": proof}, False)
    result_path = evidence / "native-source/projects/results" / proof_receipt["dispatch_sha256"] / "result.json"
    if sha(result_path) != proof_receipt["result_sha256"]:
        raise RuntimeError("native receipt does not identify retained result")
    native_ids = fresh("native-source", result_path, new_task, new_pin, evidence / "revised-proof.json")
    report["proof_rechecks"][-1].update({"source_run": receipt("native-run", evidence / "native-source.json", "source_run"),
        "source": receipt("native-program", source, "source_program"),
        "registry": receipt("native-registry", registry, "operator_registry"), "alias": "revised"})
    command("native-replay", ["replay", evidence / "native-source.json", "--source", source])
    cover("software engineer", ["replay", "second_use"], "native-source", "native-replay", *native_ids)
    report["second_uses"].append({"role": "software engineer", "first": "bounded square fallback workflow",
        "second": "bound native proof source workflow", "receipt": "native-replay",
        "explanation": "Reuse source job collection and replay in a different adapter workflow; separately change square inputs and recover both completed journals without new dispatch."})

    interrupted = evidence / "interrupted"
    with (evidence / "interrupted.stdout.txt").open("wb") as stdout, (evidence / "interrupted.stderr.txt").open("wb") as stderr:
        process = subprocess.Popen([str(binary), *map(str, native_args("interrupted"))], cwd=ROOT, stdout=stdout, stderr=stderr)
        started = time.monotonic(); reached = False; kill_requested = False
        try:
            while process.poll() is None and time.monotonic() - started < packet["interruption_wait_seconds"]:
                for path in interrupted.glob("projects/results/*.work.json"):
                    try:
                        work = Path(read(path)["work_directory"])
                    except (json.JSONDecodeError, OSError):
                        continue
                    if native(work / "build/NMLTTask.lean").is_file():
                        reached = True; break
                if reached: break
                time.sleep(0.05)
        finally:
            if process.poll() is None:
                process.kill()
                kill_requested = True
            process.wait(timeout=30)
    write(evidence / "interruption.json", {"coordinator_pid": process.pid, "exit_code": process.returncode,
        "after_native_reconstruction": reached, "kill_requested": kill_requested,
        "wall_seconds": time.monotonic() - started,
        "scope": "A kill was requested only for the source coordinator; unresolved external effects remain explicit."})
    if not reached or not kill_requested or process.returncode == 0:
        raise RuntimeError(f"native proof was not interrupted after reconstruction: {evidence}")
    receipt("interruption", evidence / "interruption.json", "interruption")
    before_dispatches = dispatches(interrupted)
    if not before_dispatches:
        raise RuntimeError("interruption retained no dispatch")
    command("interrupted-recover", ["jobs-recover", interrupted])
    command("interrupted-needs-ack", ["jobs-resume", interrupted, "--emit-run", evidence / "unacknowledged.json"], expected=1, contains="unresolved external effects")
    command("interrupted-settle", ["jobs-resume", interrupted, "--emit-run", evidence / "settled.json",
        "--acknowledge-uncertain-effects", "Solo fixture killed the coordinator during native reconstruction; retain failed attempt and acknowledge unresolved effects."])
    settled = read(evidence / "settled.json")
    settled_stop = settled["execution"]["stop"]
    if (dispatches(interrupted) != before_dispatches
            or len(settled["snapshot"]["acknowledgements"]) != 1
            or settled_stop.get("kind") != "returned"
            or settled_stop.get("value", {}).get("kind") != "err"):
        raise RuntimeError("interrupted recovery changed dispatch artifacts or silently accepted the failed attempt")
    command("interrupted-replay", ["replay", evidence / "settled.json", "--source", source])
    report["measures"]["explicit_acknowledgements"] = 1
    cover("Lean developer", ["interruption", "recovery"], "interruption", "interrupted-needs-ack", "interrupted-settle", "interrupted-replay")
    cover("software engineer", ["recovery"], "interrupted-settle", "interrupted-replay")
    report["local_value"] = "One visibility proof supports a nested static question; exact task identities make stale revisions explicit; retained source sessions replay and recover with no new dispatch observed in the recorded artifacts. This does not prove exactly-once physical effects. The native protocol adds registries, pins, records and an explicit uncertainty acknowledgement. No human effort advantage was measured."
    write(evidence / "commands.json", commands)
    receipt("commands", evidence / "commands.json")
    if any(sha(ROOT / path) != expected for path, expected in report["source_sha256"].items()):
        raise RuntimeError("solo source inputs changed during execution")
    if {name: sha(path) for name, path in tool_paths.items()} != report["tool_sha256"]:
        raise RuntimeError("solo tools changed during execution")
    write(evidence / "report.json", report)
    write(evidence / "summary.json", summarize(report, evidence))
    print(f"ok: solo operational walkthroughs; evidence: {evidence}", flush=True)


if __name__ == "__main__":
    main()

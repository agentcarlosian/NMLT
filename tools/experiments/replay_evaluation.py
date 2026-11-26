#!/usr/bin/env python3
"""Counterbalanced recorded replay using the SAME native proof adapter in both arms.

One unit is the batch theorem over 30 reference solutions, not 30 operational
trials. Four runs (two pairs) test orchestration, not human usefulness or AI skill.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

from check_evaluation import CORPUS, MANIFEST, sha, validate_manifest, write
from laboratory import ROOT, cli_binary, read, source_identity


def native(path: Path) -> Path:
    return Path("\\\\?\\" + str(path.absolute())) if os.name == "nt" and not str(path).startswith("\\\\?\\") else path


def load(path: Path) -> dict:
    return json.loads(native(path).read_text(encoding="utf-8"))


def expected_type_rejection(message: str) -> bool:
    return ("NMLTProof rejected the task/proof" in message
            and re.search(r"NMLTProof\.lean:\d+:\d+: (?:error: )?Type mismatch", message) is not None)


def source_response(observation: dict, expected_request: dict, invalid: bool) -> dict | None:
    request_value = observation["dispatch"]["input"]
    if request_value["type"] != "text" or json.loads(request_value["value"]) != expected_request:
        raise ValueError("source arm changed candidate order, text, alias, or task")
    completion = observation["completion"]
    if "Ok" not in completion or completion["Ok"]["exit_code"] != 0:
        raise ValueError("source arm had a tool failure, not a proof rejection")
    response = json.loads(bytes(completion["Ok"]["stdout"]).decode())
    if invalid:
        if (response["outcome"]["kind"] != "failed"
                or not expected_type_rejection(response["outcome"]["message"])):
            raise ValueError("first source candidate was not rejected by the proof checker")
        return None
    if response["outcome"]["kind"] != "completed":
        raise ValueError("second source candidate failed to produce a checked receipt")
    receipt = json.loads(response["outcome"]["value"]["value"])
    if (receipt["alias"] != expected_request["alias"]
            or receipt["task_sha256"] != expected_request["task_sha256"]):
        raise ValueError("source receipt names a different target binding")
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lean-bin", type=Path, required=True)
    parser.add_argument("--exporter", type=Path, required=True)
    parser.add_argument("--nanoda", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path)
    args = parser.parse_args()
    manifest = read(MANIFEST)
    validate_manifest(manifest)
    directory = args.output_dir or Path(tempfile.mkdtemp(prefix="evaluation-replay-", dir=ROOT / "target"))
    if args.output_dir:
        directory.mkdir(parents=True, exist_ok=False)
    directory = directory.resolve()
    print(f"evidence: {directory}", flush=True)
    binary = cli_binary()
    lean, exporter, nanoda = args.lean_bin.resolve(), args.exporter.resolve(), args.nanoda.resolve()
    tools = ["--lean-bin", lean, "--exporter", exporter, "--nanoda", nanoda]
    commands, accepted, rejected = [], [], []
    identity = {"manifest_sha256": sha(MANIFEST), "execution_identity": source_identity(),
                "runner_sha256": sha(Path(__file__)), "tool_sha256": {
                    "nmlt": sha(binary), "lean": sha(lean), "exporter": sha(exporter), "nanoda": sha(nanoda)}}
    write(directory / "identity.json", identity)

    def command(label: str, params: list, success: bool = True, contains: str | None = None) -> str:
        print(f"start: {label}", flush=True)
        started = time.monotonic()
        result = subprocess.run([str(binary), *map(str, params)], cwd=ROOT, capture_output=True,
                                text=True, encoding="utf-8", timeout=2400)
        (directory / f"{label}.stdout.txt").write_text(result.stdout, encoding="utf-8")
        (directory / f"{label}.stderr.txt").write_text(result.stderr, encoding="utf-8")
        commands.append({"label": label, "exit_code": result.returncode,
                         "wall_seconds": time.monotonic() - started})
        write(directory / "progress.json", commands)
        if ((result.returncode == 0) != success
                or (contains and contains not in result.stdout + result.stderr)):
            raise SystemExit(f"{label} failed: {result.stdout[-2000:]} {result.stderr[-2000:]}; {directory}")
        print(f"ok: {label} ({commands[-1]['wall_seconds']:.1f}s)", flush=True)
        return result.stdout

    project = directory / "project"
    project.mkdir()
    shutil.copyfile(CORPUS, project / "Corpus.lean")
    (project / "Goal.lean").write_text("import Corpus\ntheorem EvaluationGoal : "
                                      "NMLT.Evaluation.Corpus.Bundle := by sorry\n", encoding="utf-8", newline="\n")
    write(project / "nmlt-lean.json", {"schema": "nmlt-lean-project-v1", "modules": ["Corpus", "Goal"],
          "target_module": "Goal", "target": "EvaluationGoal",
          "permitted_axioms": manifest["comparison"]["permitted_axioms"]})
    bound = directory / "bound"
    command("bind", ["lean-task", "bind", "--project", project, "--lean-bin", lean, "--output", bound])
    task, pin = bound / "task.json", (bound / "task.sha256").read_text().strip()
    proofs = manifest["comparison"]["candidates"]
    for index, proof in enumerate(proofs):
        write(directory / f"candidate-{index}.json", {"schema": "nmlt-lean-proof-candidate-v1",
              "task_sha256": pin, "proof": proof})
    registry = directory / "projects.toml"
    registry.write_text("\n".join([
        'schema = "nmlt-lean-project-jobs-v1"',
        f"lean_bin = {json.dumps(lean.as_posix())}", f"exporter = {json.dumps(exporter.as_posix())}",
        f"nanoda = {json.dumps(nanoda.as_posix())}", "timeout_ms = 1800000", "[[tasks]]",
        'alias = "corpus"', 'task = "bound/task.json"', f'task_sha256 = "{pin}"', "",
    ]), encoding="utf-8", newline="\n")
    source = ROOT / "examples/evaluation/replay.nmlt"

    def inspect_and_recheck(label: str, result_path: Path) -> None:
        record = load(result_path)
        if (record["schema"] != "nmlt-lean-result-v5" or record["status"] != "independently_checked"
                or record["task_sha256"] != pin or record["checked_declarations"] <= 0
                or record["task"] != load(task)
                or record["candidate"] != load(directory / "candidate-1.json")):
            raise SystemExit(f"wrong native proof meaning: {result_path}")
        if not {row["declaration"] for row in manifest["proof_tasks"]}.issubset(record["exported_declarations"]):
            raise SystemExit("accepted batch omitted a reference solution from its checked dependency closure")
        export = native(result_path.parent / "build/environment.ndjson").read_bytes()
        if len(export) != record["export_bytes"] or hashlib.sha256(export).hexdigest() != record["export_sha256"]:
            raise SystemExit(f"changed proof export: {result_path}")
        # Every success crosses the real adapter again from retained artifacts.
        output = directory / (label + "-fresh")
        command(label + "-fresh", ["lean-task", "recheck", "--record", result_path,
                "--task-sha256", pin, *tools, "--output", output])
        fresh = load(output / "result.json")
        if (fresh["status"] != "independently_checked" or fresh["task_sha256"] != pin
                or fresh["task"] != record["task"] or fresh["candidate"] != record["candidate"]):
            raise SystemExit("fresh recheck changed the accepted task")
        accepted.append({"label": label, "record": str(result_path.relative_to(directory)),
                         "export_sha256": record["export_sha256"], "fresh_record": str(output.relative_to(directory))})

    for pair_index, order in enumerate(manifest["comparison"]["orders"]):
        for arm in order:
            label = f"pair-{pair_index}-{arm}"
            if arm == "library":
                command(label + "-invalid", ["lean-task", "prove", "--task", task,
                        "--task-sha256", pin, "--candidate", directory / "candidate-0.json",
                        *tools, "--output", directory / (label + "-invalid")], False, "rejected the task/proof")
                if not expected_type_rejection((directory / (label + "-invalid.stderr.txt")).read_text(encoding="utf-8")):
                    raise SystemExit("library arm failed outside the intended candidate type check")
                rejected.append(label + "-invalid")
                output = directory / label
                command(label, ["lean-task", "prove", "--task", task, "--task-sha256", pin,
                        "--candidate", directory / "candidate-1.json", *tools, "--output", output])
                inspect_and_recheck(label, output / "result.json")
            else:
                record_path, jobs = directory / (label + ".json"), directory / label
                command(label, ["run", source, "--entry", "replay", "--arg", 'alias="corpus"',
                        "--arg", "proof=" + json.dumps(proofs[1]), "--max-steps", "200",
                        "--emit-run", record_path, "--jobs-dir", jobs, "--job-slots", "1",
                        "--max-jobs", "2", "--job-timeout-ms", "30000", "--lean-projects", registry])
                run_record = load(record_path)
                if run_record["context"]["assurance"] != "none":
                    raise SystemExit("source orchestration cannot itself be proof authority")
                stop = run_record["execution"]["stop"]
                if stop["kind"] != "returned" or stop["value"]["kind"] != "ok":
                    raise SystemExit(f"source workflow did not complete: {stop}")
                results = []
                observations = run_record["snapshot"]["observations"]
                if len(observations) != 2:
                    raise SystemExit(f"source arm did not execute the same two candidates: {len(observations)}")
                for index, observation in enumerate(observations):
                    receipt = source_response(observation, {
                        "schema": "nmlt-project-proof-request-v1", "alias": "corpus",
                        "task_sha256": pin, "proof": proofs[index]}, index == 0)
                    if receipt is None:
                        rejected.append(label + "-invalid")
                        continue
                    path = jobs / "projects/results" / receipt["dispatch_sha256"] / "result.json"
                    if hashlib.sha256(native(path).read_bytes()).hexdigest() != receipt["result_sha256"]:
                        raise SystemExit("source receipt differs from retained proof bytes")
                    results.append(path)
                if len(results) != 1:
                    raise SystemExit(f"source arm accepted {len(results)} candidates; expected exactly one")
                inspect_and_recheck(label, results[0])
                command(label + "-replay", ["replay", record_path, "--source", source])
                command(label + "-recover", ["jobs-recover", jobs])

    # Identity/corpus changes invalidate the whole comparison rather than silently
    # mixing revisions in an aggregate.
    validate_manifest(read(MANIFEST))
    final_tools = {"nmlt": sha(binary), "lean": sha(lean), "exporter": sha(exporter), "nanoda": sha(nanoda)}
    if (sha(MANIFEST) != identity["manifest_sha256"] or source_identity() != identity["execution_identity"]
            or sha(Path(__file__)) != identity["runner_sha256"] or final_tools != identity["tool_sha256"]):
        raise SystemExit("comparison inputs changed during execution")
    if len(accepted) != 4 or len(rejected) != 4:
        raise SystemExit("comparison did not complete every frozen candidate in both pairs")
    write(directory / "summary.json", {
        "schema": "nmlt-evaluation-replay-v1", "assurance": "none", "identity": identity,
        "paired_repeats": 2, "arms": 4, "candidate_attempts": len(accepted) + len(rejected),
        "accepted_batch_proofs": len(accepted), "fresh_rechecks": len(accepted),
        "invalid_candidates_rejected": len(rejected), "accepted": accepted, "rejected": rejected, "commands": commands,
        "mathematical_reference_solutions_per_batch": 30, "operational_task_trials": 0,
        "live_model_runs": 0, "independent_participants": 0,
        "claim": "matched adapter orchestration regression only; no usefulness or speedup claim",
    })
    print(f"ok: four batch proofs and four fresh native rechecks; evidence: {directory}")


if __name__ == "__main__":
    main()

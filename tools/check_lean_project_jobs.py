#!/usr/bin/env python3
"""Real pinned project proofs through source jobs, replay and restart recovery."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def native(path):
    return Path("\\\\?\\" + str(path.absolute())) if os.name == "nt" and not str(path).startswith("\\\\?\\") else path


def read(path):
    return json.loads(native(path).read_text(encoding="utf-8"))


def write(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lean-bin", type=Path, required=True)
    parser.add_argument("--exporter", type=Path, required=True)
    parser.add_argument("--nanoda", type=Path, required=True)
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--quiet", "-p", "nmlt-cli"], cwd=ROOT, check=True)
    evidence = Path(tempfile.mkdtemp(prefix="r3-project-jobs-", dir=ROOT / "target"))
    print(f"evidence: {evidence}", flush=True)
    binary = evidence / ("nmlt.exe" if os.name == "nt" else "nmlt")
    shutil.copy2(ROOT / "target/debug" / binary.name, binary)
    source = evidence / "main.nmlt"
    shutil.copy2(ROOT / "examples/pivot/lean_project.nmlt", source)
    project = evidence / "project"
    shutil.copytree(ROOT / "examples/lean-lake-project", project, ignore=shutil.ignore_patterns(".lake"))
    tool_args = ["--lean-bin", args.lean_bin.resolve(), "--exporter", args.exporter.resolve(), "--nanoda", args.nanoda.resolve()]
    accepted = []
    cases = []

    def command(label, params, success=True, contains=None):
        completed = subprocess.run([str(binary), *map(str, params)], cwd=ROOT, capture_output=True,
                                   text=True, encoding="utf-8", timeout=2400)
        (evidence / f"{label}.stdout.txt").write_text(completed.stdout, encoding="utf-8")
        (evidence / f"{label}.stderr.txt").write_text(completed.stderr, encoding="utf-8")
        assert (completed.returncode == 0) == success, (label, completed.stdout[-3000:], completed.stderr, evidence)
        if contains:
            assert contains in completed.stderr, (label, completed.stderr, evidence)
        cases.append({"case": label, "exit_code": completed.returncode})
        print(f"ok: {label}", flush=True)
        return completed.stdout

    bound = evidence / "bound"
    command("bind", ["lean-task", "bind", "--project", project, "--lean-bin", args.lean_bin.resolve(), "--output", bound])
    pin = (bound / "task.sha256").read_text().strip()

    def registry(name, timeout=1800000, selected=pin):
        path = evidence / f"{name}.toml"
        path.write_text("\n".join([
            'schema = "nmlt-lean-project-jobs-v1"',
            f"lean_bin = {json.dumps(args.lean_bin.resolve().as_posix())}",
            f"exporter = {json.dumps(args.exporter.resolve().as_posix())}",
            f"nanoda = {json.dumps(args.nanoda.resolve().as_posix())}",
            f"timeout_ms = {timeout}", "[[tasks]]", 'alias = "offset"',
            'task = "bound/task.json"', f'task_sha256 = "{selected}"', ""
        ]), encoding="utf-8", newline="\n")
        return path

    registered = registry("projects")

    def invocation(label, entry="check", proof="fun n => Support.shift_eq n", alias="offset", selected=registered):
        return ["run", source, "--entry", entry, "--arg", f"alias={json.dumps(alias)}", "--arg", f"proof={json.dumps(proof)}",
                "--max-steps", "200", "--emit-run", evidence / f"{label}.json", "--jobs-dir", evidence / label,
                "--job-slots", "2" if entry == "parallel" else "1", "--max-jobs", "3", "--job-timeout-ms", "30000", "--lean-projects", selected]

    def collect(label, record_path=None):
        record_path = record_path or evidence / f"{label}.json"
        record = read(record_path)
        jobs = record_path.parent / record["project_session"]
        assert record["context"]["assurance"] == "none"
        identity = record["snapshot"]["manifest"]["configuration"]["projects"]
        assert identity == record["context"]["projects"] and identity["bindings"] == [{"alias": "offset", "task_sha256": pin}]
        assert identity["process_contract"].startswith("nmlt-contained-project-worker-v1;")
        assert record["snapshot"]["manifest"]["configuration"]["bounds"]["timeout_ms"] == 30000
        for observation in record["snapshot"]["observations"]:
            completion = observation["completion"]
            if "Ok" not in completion:
                continue
            output = completion["Ok"]
            if output["exit_code"] != 0:
                continue
            response = json.loads(bytes(output["stdout"]).decode())
            if response["outcome"]["kind"] != "completed":
                continue
            receipt = json.loads(response["outcome"]["value"]["value"])
            result = jobs / "projects/results" / receipt["dispatch_sha256"] / "result.json"
            assert hashlib.sha256(native(result).read_bytes()).hexdigest() == receipt["result_sha256"]
            proof_record = read(result)
            assert proof_record["status"] == "independently_checked" and proof_record["task_sha256"] == pin
            accepted.append({"case": label, "result": str(result.relative_to(evidence)), "receipt": receipt})
        return record

    for label, entry, proof in [("direct", "check", "fun n => Support.shift_eq n"),
                                ("repair", "repair", "by\n  intro n\n  exact Support.shift_eq n\n"),
                                ("parallel", "parallel", "fun n => Support.shift_eq n"),
                                ("cancel", "cancel", "fun n => Support.shift_eq n")]:
        command(label, invocation(label, entry, proof))
        record = collect(label)
        returned = record["execution"]["stop"]
        assert returned["kind"] == "returned", returned
        assert returned["value"]["kind"] == ("err" if label == "cancel" else "ok"), returned
        journal = (evidence / label / "journal.jsonl").read_bytes()
        command(label + "-replay", ["replay", evidence / f"{label}.json", "--source", source])
        command(label + "-recover", ["jobs-recover", evidence / label])
        assert (evidence / label / "journal.jsonl").read_bytes() == journal
    assert len(accepted) == 3, accepted

    # The normal project interface locks the registry, tests native proofs and
    # resumes its saved source record from a different run directory.
    application = evidence / "application"
    application.mkdir()
    shutil.copy2(source, application / "main.nmlt")
    (application / "nmlt.toml").write_text('''schema = "nmlt-project-v1"
source = "main.nmlt"
entry = "verified"
max_steps = 200
[inputs]
alias = "offset"
proof = "fun n => Support.shift_eq n"
[jobs]
slots = 1
max_attempts = 3
timeout_ms = 30000
[tools]
lean_projects = "../projects.toml"
[[tests]]
name = "bound native proof"
entry = "verified"
inputs = { alias = "offset", proof = "fun n => Support.shift_eq n" }
expect = true
''', encoding="utf-8", newline="\n")
    command("project-lock", ["lock", application])
    assert read(application / "nmlt.lock")["projects"]["bindings"] == [{"alias": "offset", "task_sha256": pin}]
    command("project-check", ["check-project", application])
    tested = json.loads(command("project-test", ["test", application]))
    assert tested["passed"] and len(tested["tests"]) == 1
    project_record = Path(tested["tests"][0]["record"].removeprefix("\\\\?\\"))
    collect("project-test", project_record.parent / "source.json")
    command("project-replay", ["replay", project_record, "--project", application])
    before = (project_record.parent / "jobs/journal.jsonl").read_bytes()
    resumed = json.loads(command("project-resume", ["resume", project_record.parent, "--project", application]))
    assert resumed["value"] == {"kind": "bool", "value": True}
    resumed_record = Path(resumed["record"].removeprefix("\\\\?\\"))
    command("project-resumed-replay", ["replay", resumed_record, "--project", application])
    assert (project_record.parent / "jobs/journal.jsonl").read_bytes() == before
    changed_registry = registered.read_text(encoding="utf-8")
    registered.write_text(changed_registry.replace("timeout_ms = 1800000", "timeout_ms = 1700000"), encoding="utf-8", newline="\n")
    command("changed-registry-lock", ["check-project", application], False, "differs from nmlt.lock")
    registered.write_text(changed_registry, encoding="utf-8", newline="\n")
    command("unknown-alias", invocation("unknown-alias", alias="unregistered"), False)
    command("changed-task-pin", invocation("changed-task-pin", selected=registry("wrong-pin", selected="0" * 64)), False, "selected pin")
    command("timeout", invocation("timeout", selected=registry("short-deadline", timeout=1)), False)
    timed = read(evidence / "timeout.json")
    assert timed["execution"]["stop"]["kind"] == "job_stopped"
    command("timeout-needs-ack", ["jobs-resume", evidence / "timeout", "--emit-run", evidence / "timeout-resume.json"], False, "unresolved external effects")
    command("timeout-ack", ["jobs-resume", evidence / "timeout", "--emit-run", evidence / "timeout-settled.json", "--acknowledge-uncertain-effects", "Frozen test observed deadline termination; retain failed attempt."])
    assert read(evidence / "timeout-settled.json")["execution"]["stop"]["value"]["kind"] == "err"

    # Interrupt the actual source driver only after the proof worker has begun
    # reconstructing its native project. The old dispatch must never relaunch.
    interrupted = evidence / "interrupted"
    with (evidence / "interrupted.stdout.txt").open("wb") as stdout, (evidence / "interrupted.stderr.txt").open("wb") as stderr:
        process = subprocess.Popen([str(binary), *map(str, invocation("interrupted"))], cwd=ROOT, stdout=stdout, stderr=stderr)
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline and process.poll() is None:
            work_files = list(interrupted.glob("projects/results/*.work.json"))
            if any((Path(read(path)["work_directory"]) / "lake-project/root").exists() for path in work_files):
                break
            time.sleep(0.05)
        else:
            process.kill()
            process.wait(timeout=30)
            raise AssertionError("worker did not reach project reconstruction")
        process.kill()
        process.wait(timeout=30)
    original_dispatch = (interrupted / "dispatch-0.json").read_bytes()
    command("interrupted-recover", ["jobs-recover", interrupted])
    command("interrupted-needs-ack", ["jobs-resume", interrupted, "--emit-run", evidence / "interrupted-resume.json"], False, "unresolved external effects")
    command("interrupted-ack", ["jobs-resume", interrupted, "--emit-run", evidence / "interrupted-settled.json", "--acknowledge-uncertain-effects", "Frozen test killed the source driver during native rebuild; retain failed attempt."])
    assert (interrupted / "dispatch-0.json").read_bytes() == original_dispatch
    settled = read(evidence / "interrupted-settled.json")
    assert settled["execution"]["stop"]["value"]["kind"] == "err"
    assert len(settled["snapshot"]["acknowledgements"]) == 1
    command("interrupted-replay", ["replay", evidence / "interrupted-settled.json", "--source", source])

    # Every proof bundle must survive loss of the authoring project and binding.
    for path in [project, bound]:
        target = path.with_name(path.name + "-unavailable")
        assert path.resolve().is_relative_to(evidence.resolve()) and target.resolve().is_relative_to(evidence.resolve())
        path.rename(target)
    for index, item in enumerate(accepted):
        result = evidence / item["result"]
        fresh = evidence / f"fresh-{index}"
        command(f"fresh-{index}", ["lean-task", "recheck", "--record", result, "--task-sha256", pin, *tool_args, "--output", fresh])
        assert read(fresh / "result.json")["export_sha256"] == item["receipt"]["export_sha256"]
        item["fresh_record"] = str((fresh / "result.json").relative_to(evidence))
    original_result = evidence / accepted[0]["result"]
    export = original_result.parent / "build/environment.ndjson"
    original = native(export).read_bytes()
    native(export).write_bytes(original + b"\n")
    try:
        command("changed-export", ["replay", evidence / "direct.json", "--source", source], False, "project export identity changed")
    finally:
        native(export).write_bytes(original)
    retained_source = evidence / "direct/projects/tasks/offset/lake-sources/packages/p1/lib/Support/Core.lean"
    original = native(retained_source).read_bytes()
    native(retained_source).write_bytes(original + b"\n")
    try:
        command("changed-source", ["replay", evidence / "direct.json", "--source", source], False, "differs from its pin")
    finally:
        native(retained_source).write_bytes(original)
    command("restored-replay", ["replay", evidence / "direct.json", "--source", source])
    write(evidence / "summary.json", {"schema": "nmlt-r3-project-jobs-validation-v1", "accepted": accepted, "cases": cases})
    print(f"evidence: {evidence}", flush=True)


if __name__ == "__main__":
    main()

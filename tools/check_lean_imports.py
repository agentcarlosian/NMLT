#!/usr/bin/env python3
"""Real Lean/Lake import discovery, source closures and independent rechecking."""
import argparse
import copy
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from check_lean_tasks import ROOT, digest, read, write


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lean-bin", type=Path, required=True)
    parser.add_argument("--exporter", type=Path, required=True)
    parser.add_argument("--nanoda", type=Path, required=True)
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--quiet", "-p", "nmlt-cli"], cwd=ROOT, check=True)
    binary = ROOT / "target/debug" / ("nmlt.exe" if os.name == "nt" else "nmlt")
    parent = ROOT / "target/r3-lean-imports"
    parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    tool_args = ["--lean-bin", str(args.lean_bin.resolve()), "--exporter", str(args.exporter.resolve()),
                 "--nanoda", str(args.nanoda.resolve())]
    rejected = []

    def project(label):
        path = evidence / f"project-{label}"
        shutil.copytree(ROOT / "examples/lean-project-discovery", path)
        return path

    def command(label, parameters, expected=True, contains=None, executable=None):
        result = subprocess.run([str(executable or binary), "lean-task", *map(str, parameters)],
                                cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=600)
        (evidence / f"{label}.stdout.txt").write_text(result.stdout, encoding="utf-8")
        (evidence / f"{label}.stderr.txt").write_text(result.stderr, encoding="utf-8")
        assert (result.returncode == 0) == expected, (label, result.stdout, result.stderr, evidence)
        if contains:
            assert contains in result.stderr, (label, result.stderr)
        if not expected:
            assert not (evidence / label / "result.json").exists(), label
            assert not (evidence / label / "task.json").exists(), label
            rejected.append(label)
        print(f"ok: {label}", flush=True)

    def bind(label, selected_project, expected=True, contains=None):
        output = evidence / label
        command(label, ["bind", "--project", selected_project, "--lean-bin", args.lean_bin.resolve(),
                        "--output", output], expected, contains)
        if expected:
            task = read(output / "task.json")
            pin = (output / "task.sha256").read_text(encoding="utf-8").strip()
            assert digest(task) == pin
            return task, pin

    def prove(label, task, selected_pin, expected=True, contains=None):
        task_path = evidence / f"{label}-task.json"
        candidate = evidence / f"{label}-candidate.json"
        write(task_path, task)
        write(candidate, {"schema": "nmlt-lean-proof-candidate-v1", "task_sha256": selected_pin,
                          "proof": "fun n => App.value_eq n"})
        command(label, ["prove", "--task", task_path, "--task-sha256", selected_pin,
                        "--candidate", candidate, *tool_args, "--output", evidence / label], expected, contains)
        if expected:
            return read(evidence / label / "result.json")

    original_project = project("original")
    # Even a malformed unrelated source does not belong to the selected closure.
    (original_project / "src/App/Unused.lean").write_text("import Missing.Unreachable\n", encoding="utf-8")
    task, pin = bind("bound", original_project)
    assert task["schema"] == "nmlt-lean-task-v3"
    assert task["manifest"]["modules"] == ["Support.Core", "App.Definitions", "App.Goals"]
    closure = task["discovery"]
    assert closure == read(evidence / "bound/source-imports.json")
    assert [m["source_root"] for m in closure["modules"]] == ["vendor/support", "src", "src"]
    assert closure["toolchain_modules"] == ["Init"]
    assert all(m["header"]["isModule"] for m in closure["modules"])
    assert any(i["isMeta"] and i["isExported"] for i in closure["modules"][-1]["header"]["imports"])
    assert "App.Unused" not in task["manifest"]["modules"]
    accepted = prove("proof", task, pin)
    assert accepted["schema"] == "nmlt-lean-result-v5"
    assert accepted["status"] == "independently_checked"
    assert accepted["task"]["discovery"] == closure
    assert accepted["checked_declarations"] == len(accepted["exported_declarations"]) > 0
    assert accepted["proof"]["axioms"] == []

    moved = evidence / "unavailable-original"
    assert original_project.resolve().is_relative_to(evidence.resolve())
    assert moved.parent.resolve() == evidence.resolve()
    original_project.rename(moved)
    command("fresh", ["recheck", "--record", evidence / "proof/result.json", "--task-sha256", pin,
                      *tool_args, "--output", evidence / "fresh"], executable=evidence / "proof" / binary.name)
    fresh = read(evidence / "fresh/result.json")
    assert fresh["export_sha256"] == accepted["export_sha256"]
    assert fresh["task"]["discovery"] == closure
    assert moved.resolve().is_relative_to(evidence.resolve()) and original_project.parent.resolve() == evidence.resolve()
    moved.rename(original_project)

    changed = copy.deepcopy(task)
    changed["sources"][0]["text"] = changed["sources"][0]["text"].replace("0 + n", "1 + n")
    prove("changed-dependency", changed, pin, False, "selected task hash")

    changed = copy.deepcopy(task)
    changed["discovery"]["modules"][-1]["header"]["imports"][-1]["isMeta"] = False
    prove("rehashed-header", changed, digest(changed), False, "import header differs")

    changed = copy.deepcopy(task)
    changed["manifest"]["modules"].insert(1, "App.Unused")
    changed["sources"].insert(1, {"module": "App.Unused", "text": "def unused : Nat := 0\n"})
    unused = copy.deepcopy(changed["discovery"]["modules"][0])
    unused["module"] = "App.Unused"
    unused["source_root"] = "src"
    changed["discovery"]["modules"].insert(1, unused)
    prove("unreachable-source", changed, digest(changed), False, "unreachable module")

    changed = copy.deepcopy(task)
    order = changed["manifest"]["modules"]
    order[0], order[1] = order[1], order[0]
    for values in [changed["sources"], changed["discovery"]["modules"]]:
        values[0], values[1] = values[1], values[0]
    prove("reordered-imports", changed, digest(changed), False, "dependency order")

    missing = project("missing")
    core = missing / "vendor/support/Support/Core.lean"
    core.write_text("import Missing.Dependency\n", encoding="utf-8")
    bind("missing-import", missing, False, "unresolved source import Missing.Dependency")

    cached = project("cached")
    (cached / "vendor/support/Support/Core.lean").unlink()
    shutil.copyfile(evidence / "proof/build/Support/Core.olean", cached / "vendor/support/Support/Core.olean")
    bind("cached-olean-only", cached, False, "unresolved source import Support.Core")

    cyclic = project("cycle")
    (cyclic / "vendor/support/Support/Core.lean").write_text("import App.Goals\n", encoding="utf-8")
    bind("import-cycle", cyclic, False, "cyclic source import")

    ambiguous = project("ambiguous")
    (ambiguous / "src/Support").mkdir()
    shutil.copyfile(ambiguous / "vendor/support/Support/Core.lean", ambiguous / "src/Support/Core.lean")
    bind("ambiguous-roots", ambiguous, False, "ambiguous source module Support.Core")

    alias = project("alias")
    config = read(alias / "nmlt-lean.json")
    config["source_roots"] = [".", "src", "vendor/support"]
    write(alias / "nmlt-lean.json", config)
    core = alias / "vendor/support/Support/Core.lean"
    core.write_text("import src.App.Definitions\n", encoding="utf-8")
    bind("source-alias", alias, False, "multiple module identities")

    shadow = project("shadow")
    (shadow / "src/Init.lean").write_text("prelude\n", encoding="utf-8")
    bind("toolchain-shadow", shadow, False, "shadows pinned toolchain module Init")

    malformed = project("malformed")
    (malformed / "src/App/Goals.lean").write_text("import ../Invalid\n", encoding="utf-8")
    bind("header-error", malformed, False, "import parser rejected the header")

    escape = project("escape")
    config = read(escape / "nmlt-lean.json")
    config["source_roots"] = ["../project-original/src"]
    write(escape / "nmlt-lean.json", config)
    bind("root-escape", escape, False, "project-relative paths")

    casing = project("casing")
    (casing / "src/App/Goals.lean").write_text("import app.Definitions\n", encoding="utf-8")
    bind("module-casing", casing, False, "source path casing")

    linked = project("link")
    link_path = linked / "vendor-link"
    destination = linked / "vendor/support"
    assert link_path.parent.resolve().is_relative_to(evidence.resolve())
    assert destination.resolve().is_relative_to(evidence.resolve())
    if os.name == "nt":
        subprocess.run(["powershell.exe", "-NoProfile", "-NonInteractive", "-Command",
                        "New-Item -ItemType Junction -Path $env:NMLT_TEST_JUNCTION_PATH "
                        "-Target $env:NMLT_TEST_JUNCTION_TARGET | Out-Null"],
                       env={**os.environ, "NMLT_TEST_JUNCTION_PATH": str(link_path),
                            "NMLT_TEST_JUNCTION_TARGET": str(destination)},
                       creationflags=subprocess.CREATE_NO_WINDOW, check=True, timeout=30)
    else:
        link_path.symlink_to(destination, target_is_directory=True)
    config = read(linked / "nmlt-lean.json")
    config["source_roots"] = ["src", "vendor-link"]
    write(linked / "nmlt-lean.json", config)
    bind("source-link", linked, False, "must not contain links")

    write(evidence / "summary.json", {"accepted": ["proof"], "fresh_rechecks": ["fresh"],
          "rejected": rejected, "source_modules": task["manifest"]["modules"],
          "checked_declarations": accepted["checked_declarations"], "export_sha256": accepted["export_sha256"]})
    print(f"evidence: {evidence.relative_to(ROOT)}", flush=True)
    print("scope: native Lean import discovery in explicitly selected local source roots", flush=True)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Actual proof dependency graphs, Lean reference parity and retained rechecking."""
import argparse
import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from check_lean_tasks import ROOT, digest, read, write


def lean_reference_parity(args, build, graph, evidence):
    """Compare every node with Lean's own expression constant collector."""
    names = ", ".join(json.dumps(node["name"], ensure_ascii=False) for node in graph["nodes"])
    helper = '''import NMLTProof
import Lean.Util.ForEachExpr
set_option maxHeartbeats 2000000
open Lean Meta
private def references (expression : Expr) : List String :=
  (expression.getUsedConstants.toList.map toString).mergeSort (· ≤ ·)
private def projectionReferences (expression : Expr) : IO (List Name) := do
  let names ← IO.mkRef ({} : NameSet)
  expression.forEach fun child => do
    if let .proj name _ _ := child then
      names.modify fun found => found.insert name
  return (← names.get).toList
run_meta do
  let selected : List String := [__NAMES__]
  let env := (← getEnv).setExporting false
  for (name, ci) in env.constants.toList do
    if selected.contains name.toString then
      let mut reduction : NameSet := {}
      let mut projections : NameSet := {}
      for name in (← projectionReferences ci.type) do
        projections := projections.insert name
      if let some value := ci.value? (allowOpaque := true) then
        for name in (← projectionReferences value) do
          projections := projections.insert name
      if let .recInfo val := ci then
        for rule in val.rules do
          reduction := reduction.insert rule.ctor
          for reference in rule.rhs.getUsedConstants do
            reduction := reduction.insert reference
          for name in (← projectionReferences rule.rhs) do
            projections := projections.insert name
      let data := Json.mkObj [
        ("name", toJson name.toString),
        ("type_references", toJson (references ci.type)),
        ("value_references", toJson ((ci.value? (allowOpaque := true)).map references |>.getD [])),
        ("projection_references", toJson ((projections.toList.map toString).mergeSort (· ≤ ·))),
        ("reduction_references", toJson ((reduction.toList.map toString).mergeSort (· ≤ ·)))]
      IO.println ("NMLT_REFERENCE=" ++ data.compress)
'''.replace("__NAMES__", names)
    path = build / "CheckDependencyReferences.lean"
    path.write_text(helper, encoding="utf-8", newline="\n")
    environment = os.environ.copy()
    environment["LEAN_PATH"] = str(build.resolve())
    result = subprocess.run([str(args.lean_bin.resolve()), "-M", "768", "-s", "65536", "-j", "1", str(path.resolve())],
                            cwd=build, env=environment, capture_output=True, text=True,
                            encoding="utf-8", timeout=30)
    (evidence / "lean-reference-parity.stdout.txt").write_text(result.stdout, encoding="utf-8")
    (evidence / "lean-reference-parity.stderr.txt").write_text(result.stderr, encoding="utf-8")
    assert result.returncode == 0 and not result.stderr, (result.stdout, result.stderr)
    rows = [json.loads(line.removeprefix("NMLT_REFERENCE=")) for line in result.stdout.splitlines()
            if line.startswith("NMLT_REFERENCE=")]
    expected = [{key: node[key] for key in ("name", "type_references", "value_references", "reduction_references", "projection_references")}
                for node in graph["nodes"]]
    assert sorted(rows, key=lambda row: row["name"]) == expected, evidence
    print(f"ok: Lean reference parity for all {len(rows)} declarations", flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lean-bin", type=Path, required=True)
    parser.add_argument("--exporter", type=Path, required=True)
    parser.add_argument("--nanoda", type=Path, required=True)
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--quiet", "-p", "nmlt-cli"], cwd=ROOT, check=True)
    binary = ROOT / "target/debug" / ("nmlt.exe" if os.name == "nt" else "nmlt")
    parent = ROOT / "target/r3-lean-dependencies"
    parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    project = evidence / "project"
    shutil.copytree(ROOT / "examples/lean-project-dependencies", project)
    tool_args = ["--lean-bin", str(args.lean_bin.resolve()), "--exporter", str(args.exporter.resolve()),
                 "--nanoda", str(args.nanoda.resolve())]
    rejected = []

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
            assert not (evidence / label / "proof-dependencies.json").exists(), label
            rejected.append(label)
        print(f"ok: {label}", flush=True)

    command("bound", ["bind", "--project", project, "--lean-bin", args.lean_bin.resolve(),
                      "--output", evidence / "bound"])
    task = read(evidence / "bound/task.json")
    pin = (evidence / "bound/task.sha256").read_text(encoding="utf-8").strip()
    assert digest(task) == pin and task["schema"] == "nmlt-lean-task-v3"
    candidate = evidence / "candidate.json"
    write(candidate, {"schema": "nmlt-lean-proof-candidate-v1", "task_sha256": pin,
                      "proof": "fun box => Graph.checked box"})
    command("proof", ["prove", "--task", evidence / "bound/task.json", "--task-sha256", pin,
                      "--candidate", candidate, *tool_args, "--output", evidence / "proof"])
    record = read(evidence / "proof/result.json")
    graph = record["proof_dependencies"]
    assert record["schema"] == "nmlt-lean-result-v5" and record["status"] == "independently_checked"
    assert graph == read(evidence / "proof/proof-dependencies.json")
    assert graph["schema"] == "nmlt-lean-proof-dependencies-v1"
    assert graph["export_sha256"] == record["export_sha256"]
    assert [node["name"] for node in graph["nodes"]] == record["exported_declarations"]
    nodes = {node["name"]: node for node in graph["nodes"]}
    assert len(nodes) == record["checked_declarations"]
    assert nodes["NMLTChecked.result"]["value_references"] == ["Graph.Box", "Graph.checked"]
    assert nodes["Graph.Box.value"]["value_references"] == ["Graph.Box"]
    assert nodes["Graph.Box.value"]["projection_references"] == ["Graph.Box"]
    assert nodes["Graph.fixed"]["kind"] == "opaque"
    assert nodes["Graph.word"]["value_references"] == []
    assert nodes["Graph.word"]["literal_support"] == ["Char.ofNat", "String.ofList"]
    assert "Graph.Tree.rec" in nodes["Graph.Forest.rec"]["reduction_references"]
    assert "Graph.Forest.rec" in nodes["Graph.Forest.rec"]["reduction_references"]
    assert "Graph.goal" not in nodes and "Graph.unused_draft" not in nodes and "sorryAx" not in nodes
    assert record["proof"]["axioms"] == []
    assert any(group["kind"] == "quotient" and group["members"] == ["Quot", "Quot.ind", "Quot.lift", "Quot.mk"]
               for group in graph["groups"])
    assert any("Graph.Tree" in group["members"] and "Graph.Forest" in group["members"]
               for group in graph["groups"])
    lean_reference_parity(args, evidence / "proof/build", graph, evidence)

    moved = evidence / "unavailable-project"
    assert project.resolve().is_relative_to(evidence.resolve()) and moved.parent.resolve() == evidence.resolve()
    project.rename(moved)
    command("fresh", ["recheck", "--record", evidence / "proof/result.json", "--task-sha256", pin,
                      *tool_args, "--output", evidence / "fresh"], executable=evidence / "proof" / binary.name)
    assert read(evidence / "fresh/result.json")["proof_dependencies"] == graph
    assert (evidence / "fresh/proof-dependencies.json").read_bytes() == (evidence / "proof/proof-dependencies.json").read_bytes()
    assert (evidence / "fresh/proof-dependencies.md").read_bytes() == (evidence / "proof/proof-dependencies.md").read_bytes()
    assert moved.resolve().is_relative_to(evidence.resolve()) and project.parent.resolve() == evidence.resolve()
    moved.rename(project)

    def node(changed, name):
        return next(node for node in changed["proof_dependencies"]["nodes"] if node["name"] == name)

    def recheck(label, changed, contains):
        path = evidence / f"{label}-record.json"
        write(path, changed)
        command(label, ["recheck", "--record", path, "--task-sha256", pin,
                        *tool_args, "--output", evidence / label], False, contains)

    changed = copy.deepcopy(record)
    changed["proof_dependencies"]["export_sha256"] = "0" * 64
    recheck("changed-graph-export", changed, "graph differs")

    changed = copy.deepcopy(record)
    node(changed, "NMLTChecked.result")["value_references"] = []
    recheck("changed-root-references", changed, "graph differs")

    changed = copy.deepcopy(record)
    node(changed, "Graph.project")["value_references"] = ["Missing.declaration"]
    recheck("missing-reference", changed, "missing proof dependency")

    for label, name, field, value in [
        ("changed-interior-reference", "Graph.project", "value_references", []),
        ("changed-reduction-rules", "Graph.Forest.rec", "reduction_references", []),
        ("changed-literal-support", "Graph.word", "literal_support", []),
        ("changed-projection-reference", "Graph.Box.value", "projection_references", []),
        ("changed-declaration-kind", "Graph.fixed", "kind", "definition"),
    ]:
        changed = copy.deepcopy(record)
        node(changed, name)[field] = value
        recheck(label, changed, "fresh proof artifacts differ")

    changed = copy.deepcopy(record)
    changed["proof_dependencies"]["groups"] = [group for group in changed["proof_dependencies"]["groups"]
                                               if group["kind"] != "quotient"]
    recheck("changed-export-group", changed, "fresh proof artifacts differ")

    changed = copy.deepcopy(record)
    node(changed, "Graph.checked")["planning_status"] = "complete"
    recheck("draft-status-in-graph", changed, "unknown field")

    changed = copy.deepcopy(record)
    del changed["proof_dependencies"]
    recheck("missing-graph", changed, "missing field")

    changed = copy.deepcopy(record)
    changed["schema"] = "nmlt-lean-result-v3"
    recheck("old-result-format", changed, "retained original CLI executable")

    write(evidence / "draft-candidate.json", {"schema": "nmlt-lean-proof-candidate-v1",
                                              "task_sha256": pin, "proof": "fun box => Graph.goal box"})
    command("draft-proof", ["prove", "--task", evidence / "bound/task.json", "--task-sha256", pin,
                             "--candidate", evidence / "draft-candidate.json", *tool_args,
                             "--output", evidence / "draft-proof"], False, "axiom")
    write(evidence / "summary.json", {"accepted": ["proof"], "fresh_rechecks": ["fresh"],
          "rejected": rejected, "checked_declarations": len(nodes), "lean_reference_parity": len(nodes),
          "export_bytes": record["export_bytes"], "export_sha256": record["export_sha256"]})
    print(f"evidence: {evidence.relative_to(ROOT)}", flush=True)
    print("scope: actual exported declaration references, independently of draft plans", flush=True)


if __name__ == "__main__":
    main()

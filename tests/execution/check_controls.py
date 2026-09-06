#!/usr/bin/env python3
"""Exercise the actual Lean executable on finite execution controls."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[2]
checker = Path(sys.argv[1]).resolve()
artifact = root / "examples/pivot/affine_continuation.behavior-core-v2.json"
source = root / "examples/pivot/affine_continuation.nmlt"
original = json.loads(artifact.read_text())
consume = json.loads((root / "examples/pivot/receive_consume.behavior-execution-v1.json").read_text())
cases = []


def witness_case(name, change, expected="execution:"):
    witness = copy.deepcopy(consume)
    change(witness)
    cases.append((name, None, witness, expected))


def set_owner(witness, index, owner):
    witness["states"][index]["authority"]["permit"] = owner


witness_case("wrong initial owner", lambda w: set_owner(w, 0, "Receiver"), "initializer")
witness_case("vacant initial permit", lambda w: set_owner(w, 0, None), "initializer")
witness_case("wrong initial control", lambda w: w["states"][0].update(left=1), "initializer")
witness_case("missing capability", lambda w: w["states"][0].update(authority={}), "object fields")
witness_case("extra capability", lambda w: w["states"][0]["authority"].update(extra=None), "object fields")
witness_case("unknown owner", lambda w: set_owner(w, 1, "Other"), "unknown owner")
witness_case("two claimed owners", lambda w: set_owner(w, 1, ["Sender", "Receiver"]), "owner must be")
witness_case("left sentinel", lambda w: w["states"][1].update(left=4), "finite domain")
witness_case("right sentinel", lambda w: w["states"][1].update(right=2), "finite domain")
witness_case("unknown action", lambda w: w["actions"][0].update(left="missing"), "unknown action")
witness_case("no action endpoint", lambda w: w["actions"][0].update(left=None, right=None), "endpoints")
witness_case("retained sender ownership", lambda w: set_owner(w, 1, "Sender"), "actual unified step")
witness_case("lost transfer", lambda w: set_owner(w, 1, None), "actual unified step")
witness_case("fabricated post-consumption authority", lambda w: set_owner(w, 3, "Receiver"), "actual unified step")
witness_case("hidden step changes owner", lambda w: set_owner(w, 2, "Sender"), "actual unified step")
witness_case("local step changes peer control", lambda w: w["states"][2].update(right=0), "actual unified step")
witness_case("unmatched receive", lambda w: w["actions"][0].update(right=None), "actual unified step")
witness_case("stale artifact identity", lambda w: w.update(artifact_sha256="0" * 64), "stale execution artifact digest")
witness_case("unsupported witness version", lambda w: w.update(schema="behavior-execution-v9"), "unsupported witness schema")
witness_case("empty path", lambda w: w.update(states=[], actions=[]), "no initial state")
witness_case("wrong path length", lambda w: w["states"].pop(), "one more state")


def early_use(w):
    after = copy.deepcopy(w["states"][0])
    after["authority"]["permit"] = None
    w.update(states=[w["states"][0], after], actions=[{"left": "use", "right": None}])


witness_case("use before acquisition with true control guard", early_use, "actual unified step")


def repeat_use(w):
    w["states"].append(copy.deepcopy(w["states"][-1]))
    w["actions"].append({"left": "use", "right": None})


witness_case("consume twice", repeat_use, "actual unified step")


def sender_after_transfer(w):
    after = copy.deepcopy(w["states"][1])
    after["authority"]["permit"] = None
    w.update(states=w["states"][:2] + [after], actions=w["actions"][:1] + [{"left": None, "right": "finish"}])


witness_case("sender use after transfer", sender_after_transfer, "actual unified step")

for name, mutate, expected in [
    ("unsupported core version", lambda p: p.update(schema="behavior-core-v9"), "unsupported schema"),
    ("claimed initial world", lambda p: p["initial_authority"]["Network"].update(permit="Receiver"), "initial_authority disagrees"),
    ("missing known input", lambda p: p["known_capabilities"].update(Receiver={}), "known_capabilities disagrees"),
    ("wrong known type", lambda p: p["known_capabilities"]["Receiver"].update(permit="Once<Bool>"), "known_capabilities disagrees"),
]:
    altered = copy.deepcopy(original)
    mutate(altered)
    cases.append((name, altered, copy.deepcopy(consume), expected))

with tempfile.TemporaryDirectory(prefix="nmlt-execution-controls-") as directory:
    directory = Path(directory)
    for index, (name, altered, witness, expected) in enumerate(cases):
        core_path = artifact
        if altered is not None:
            core_path = directory / f"core-{index}.json"
            core_path.write_text(json.dumps(altered), encoding="utf-8")
            witness["artifact_sha256"] = hashlib.sha256(core_path.read_bytes()).hexdigest()
        witness_path = directory / f"path-{index}.json"
        witness_path.write_text(json.dumps(witness), encoding="utf-8")
        result = subprocess.run([str(checker), str(core_path), str(source), str(witness_path)], capture_output=True, text=True, timeout=30)
        if result.returncode == 0 or expected not in result.stderr:
            raise SystemExit(f"control {name!r} failed: {result.returncode}\n{result.stdout}\n{result.stderr}")
    # Source mismatch must be exercised with an otherwise accepted path.
    stale_source = directory / "stale.nmlt"
    stale_source.write_bytes(source.read_bytes() + b"\n")
    valid = root / "examples/pivot/receive_consume.behavior-execution-v1.json"
    result = subprocess.run([str(checker), str(artifact), str(stale_source), str(valid)], capture_output=True, text=True, timeout=30)
    if result.returncode == 0 or "stale source digest" not in result.stderr:
        raise SystemExit("source identity control failed")

    primary = root / "examples/pivot/visible_resource_sync.behavior-core-v2.json"
    primary_core = json.loads(primary.read_text())
    primary_path = json.loads((root / "examples/pivot/visible_resource_sync.behavior-execution-v1.json").read_text())
    action = primary_core["systems"]["AbstractSender"]["actions"]["send"]
    action.update(guards=["false"], guard_ast=[{"kind": "bool", "type": "Bool", "value": False}])
    failed_refinement = directory / "failed-refinement.json"
    failed_refinement.write_text(json.dumps(primary_core), encoding="utf-8")
    primary_path["artifact_sha256"] = hashlib.sha256(failed_refinement.read_bytes()).hexdigest()
    primary_witness = directory / "primary-path.json"
    primary_witness.write_text(json.dumps(primary_path), encoding="utf-8")
    result = subprocess.run([str(checker), str(failed_refinement), str(root / "examples/pivot/visible_resource_sync.nmlt"), str(primary_witness)], capture_output=True, text=True, timeout=30)
    if result.returncode == 0 or "artifact theorem application failed" not in result.stderr:
        raise SystemExit(f"v2 refinement control failed: {result.stderr}")

print(f"ok: {len(cases) + 2} execution rejection controls; real Lean executable")

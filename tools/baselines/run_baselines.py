#!/usr/bin/env python3
"""Frozen R0 comparison scripts, not an NMLT runtime or a proof kernel."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import time
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IMPLEMENTATION_SHA256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
MANIFEST = ROOT / "examples/baselines/manifest.json"
FROZEN_MANIFEST_SHA256 = "d9b83650e404b2e0f45f496374425d727ca112caa158f12bf4991558683fa992"
PROOF_TARGET = "forall n : Nat, 0 + n = n"
DISCOVERY_TARGETS = [
    "forall a b : Nat, a + b = a",
    "forall a b : Nat, a + b = b + a",
]
PROOFS = {
    "wrong-term": "intro n\nexact n",
    "existing-lemma": "intro n\nexact Nat.zero_add n",
    "induction": "intro n\ninduction n with\n| zero => rfl\n| succ n ih => exact congrArg Nat.succ ih",
}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def encoded(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def load_manifest(path: Path = MANIFEST) -> dict:
    manifest = json.loads(path.read_text(encoding="utf-8"))
    # This is a frozen comparison corpus, not configurable workflow software.
    # Its complete inputs, expected outputs and budgets must change together
    # under a deliberately revised corpus identity.
    if digest(encoded(manifest)) != FROZEN_MANIFEST_SHA256:
        raise ValueError("manifest differs from the complete frozen corpus, including budgets and outcomes")
    pin = (ROOT / "mechanization/lean/lean-toolchain").read_text(encoding="utf-8").strip()
    if pin.rsplit(":", 1)[-1].removeprefix("v") != manifest["lean_version"]:
        raise ValueError("baseline Lean version differs from repository toolchain pin")
    # Python finite predicates and Lean goals must stay the same frozen tasks.
    if (manifest["schema"] != "nmlt-r0-baselines-v1"
            or manifest["proof"]["target"] != PROOF_TARGET
            or manifest["proof"]["strategies"] != list(PROOFS)
            or manifest["discovery"]["targets"] != DISCOVERY_TARGETS
            or manifest["discovery"]["candidates"] != ["left_projection", "commutative"]
            or manifest["discovery"]["domain"] != [0, 1, 2, 3, 4]
            or manifest["axiom_policy"] != "empty"):
        raise ValueError("manifest does not describe the frozen baseline tasks")
    return manifest


def lean_source(target: str, proof: str, heartbeats: int) -> str:
    body = "\n".join("  " + line for line in proof.splitlines())
    return ("import Init\nset_option autoImplicit false\n"
            f"set_option maxHeartbeats {heartbeats}\n"
            f"theorem R0.frozen_target : ({target}) := by\n{body}\n"
            "#print axioms R0.frozen_target\n")


class LeanChecker:
    def __init__(self, command: list[str], manifest: dict):
        if not isinstance(command, list) or not command or not all(isinstance(part, str) and part for part in command):
            raise ValueError("Lean command must be a nonempty JSON array of strings")
        self.command = command
        self.timeout = manifest["check_timeout_seconds"]
        try:
            version = subprocess.run(command + ["--version"], capture_output=True,
                                     text=True, timeout=self.timeout, check=False)
        except (OSError, subprocess.TimeoutExpired) as error:
            raise ValueError(f"Lean unavailable: {error}") from error
        self.version = (version.stdout + version.stderr).strip()
        match = re.search(r"Lean \(version (\d+\.\d+\.\d+)", self.version)
        if version.returncode != 0 or not match or match.group(1) != manifest["lean_version"]:
            raise ValueError(f"Lean version mismatch; required {manifest['lean_version']}; got {self.version}")

    def identity(self) -> dict:
        return {"command": self.command, "version": self.version, "axiom_policy": "empty"}

    def check(self, source: str) -> dict:
        started = time.monotonic()
        try:
            result = subprocess.run(self.command + ["--stdin"], input=source,
                                    capture_output=True, text=True, timeout=self.timeout,
                                    check=False)
            output = result.stdout + result.stderr
            if result.returncode == 0 and "'R0.frozen_target' does not depend on any axioms" in output:
                outcome = "accepted"
            elif result.returncode == 1 and "error:" in output:
                outcome = "rejected"
            elif result.returncode == 0:
                outcome = "axiom_policy_failure"
            else:
                outcome = "tool_failure"
            answer = {"outcome": outcome, "exit_code": result.returncode, "diagnostics": output}
        except subprocess.TimeoutExpired:
            answer = {"outcome": "timeout", "exit_code": None, "diagnostics": "Lean check exceeded wall timeout"}
        except OSError as error:
            answer = {"outcome": "tool_failure", "exit_code": None, "diagnostics": str(error)}
        answer.update({"elapsed_seconds": time.monotonic() - started,
                       "source_sha256": digest(source.encode()),
                       "evidence": "Lean process checking and empty transitive-axiom report; no independent checker"})
        return answer


def identity(manifest: dict, checker: LeanChecker | None) -> dict:
    return {"manifest_sha256": digest(encoded(manifest)),
            "implementation_sha256": IMPLEMENTATION_SHA256,
            "checker": checker.identity() if checker else None}


def record_for(path: Path, scenario: str, run_identity: dict, resume: bool) -> dict:
    if path.exists():
        if not resume:
            raise ValueError(f"{path} already exists; use --resume or a fresh output directory")
        record = json.loads(path.read_text(encoding="utf-8"))
        if record.get("identity") != run_identity or record.get("scenario") != scenario:
            raise ValueError("resume identity mismatch: inputs, implementation, or checker changed")
        return record
    if resume:
        raise ValueError(f"cannot resume missing record: {path}")
    return {"schema": "nmlt-r0-record-v1", "scenario": scenario, "identity": run_identity,
            "run_id": str(uuid.uuid4()), "context_sha256": digest(encoded([scenario, run_identity])),
            "status": "incomplete",
            "attempts": [], "model_calls": 0, "model_tokens": 0,
            "model_charges_usd": 0, "assurance": "explicit per-result evidence only"}


def check_attempt(directory: Path, checker: LeanChecker, task: str, source: str) -> dict:
    source_path = directory / f"{task}.lean"
    source_path.parent.mkdir(parents=True, exist_ok=True)
    source_path.write_bytes(source.encode("utf-8"))
    result = checker.check(source)
    result["source_file"] = source_path.name
    return result


def write_report(directory: Path, record: dict) -> None:
    lines = [f"# R0 {record['scenario']} baseline", "", f"Run status: **{record['status']}**.", "",
             "These are frozen local comparison tasks, not NMLT runtime results or an independent proof-checker report.", "",
             "| Task | Observed outcome |", "|---|---|"]
    for attempt in record["attempts"]:
        outcome = attempt.get("current_outcome", attempt.get("outcome"))
        if outcome is None:
            outcome = json.dumps(attempt["observed"], sort_keys=True)
        lines.append(f"| {attempt['task_id']} | {outcome} |")
    lines.extend(["", "`record.json` retains exact identities, check diagnostics, budgets and recorded measurements.",
                  "Lean files beside this report are the exact submitted sources. No model was called.", ""])
    (directory / "report.md").write_text("\n".join(lines), encoding="utf-8")


def run_proof(directory: Path, manifest: dict, checker: LeanChecker, resume: bool,
              stop_after: int | None) -> dict:
    path = directory / "record.json"
    record = record_for(path, "proof", identity(manifest, checker), resume)
    tasks = manifest["proof"]["strategies"]
    expected = manifest["proof"]["expected"]
    if len(record["attempts"]) > len(tasks):
        raise ValueError("resume record exceeds frozen proof budget")
    # Cached status alone cannot establish acceptance on resume. Recheck every
    # completed candidate, including rejections, against the exact frozen goal.
    for index, attempt in enumerate(record["attempts"]):
        if attempt.get("task_id") != tasks[index]:
            raise ValueError("resume proof task order or identity changed")
        source = lean_source(PROOF_TARGET, PROOFS[tasks[index]], manifest["max_heartbeats"])
        checked = check_attempt(directory, checker, tasks[index], source)
        attempt.setdefault("rechecks", []).append(checked)
        attempt["current_outcome"] = checked["outcome"]
    new_attempts = 0
    while len(record["attempts"]) < len(tasks):
        if stop_after is not None and new_attempts >= stop_after:
            break
        index = len(record["attempts"])
        task = tasks[index]
        source = lean_source(PROOF_TARGET, PROOFS[task], manifest["max_heartbeats"])
        checked = check_attempt(directory, checker, task, source)
        record["attempts"].append({"task_id": task, "attempt": index + 1,
                                   "target": PROOF_TARGET, "check": checked,
                                   "current_outcome": checked["outcome"], "rechecks": []})
        new_attempts += 1
        write_json(path, record)
    record["strategy_attempts_charged"] = len(record["attempts"])
    record["checker_invocations_recorded"] = sum(1 + len(a["rechecks"]) for a in record["attempts"])
    record["wall_seconds_recorded"] = sum(a["check"]["elapsed_seconds"] + sum(
        c["elapsed_seconds"] for c in a["rechecks"]) for a in record["attempts"])
    observed = [a["current_outcome"] for a in record["attempts"]]
    record["status"] = ("complete" if observed == expected else
                        "incomplete" if observed == expected[:len(observed)] else "failed")
    record["expected_outcomes"] = expected
    record["declared_budget"] = manifest["proof"]["max_strategy_attempts"]
    write_json(path, record)
    write_report(directory, record)
    return record


def run_discovery(directory: Path, manifest: dict, checker: LeanChecker, resume: bool) -> dict:
    path = directory / "record.json"
    record = record_for(path, "discovery", identity(manifest, checker), resume)
    # Recompute finite witnesses and recheck Lean output on every invocation.
    # This is artifact replay, not a new discovery or a persistent search engine.
    if record["attempts"]:
        record.setdefault("replay_history", []).append(record["attempts"])
    attempts = []
    for name, target in zip(manifest["discovery"]["candidates"], DISCOVERY_TARGETS):
        pairs = [(a, b) for a in manifest["discovery"]["domain"]
                 for b in manifest["discovery"]["domain"]]
        failures = [(a, b) for a, b in pairs
                    if not (a + b == (a if name == "left_projection" else b + a))]
        if failures:
            witness = failures[0]
            if witness != (0, 1):
                raise ValueError("frozen counterexample changed")
            proof = "intro h\nhave impossible : (1 : Nat) = 0 := h 0 1\ncases impossible"
            checked_target = f"Not ({target})"
            output = "refuted"
        else:
            witness = None
            proof = "intro a b\nexact Nat.add_comm a b"
            checked_target = target
            output = "known_theorem"
        source = lean_source(checked_target, proof, manifest["max_heartbeats"])
        checked = check_attempt(directory, checker, name, source)
        attempts.append({"task_id": name, "candidate": target, "finite_pairs_checked": len(pairs),
                         "first_counterexample": witness, "checked_target": checked_target,
                         "outcome": output if checked["outcome"] == "accepted" else "unknown",
                         "check": checked, "novelty": "known calibration task; no novelty claim"})
    record.update({"attempts": attempts, "candidate_attempts_charged": len(attempts),
                   "checker_invocations_recorded": sum(len(items) for items in record.get("replay_history", [])) + len(attempts),
                   "declared_budget": manifest["discovery"]["max_candidates"],
                   "status": "complete" if [a["outcome"] for a in attempts] == manifest["discovery"]["expected"] else "failed",
                   "evidence": "25 enumerated input pairs per candidate; universal result/refutation only from Lean checking"})
    record["wall_seconds_recorded"] = sum(a["check"]["elapsed_seconds"] for items in
        record.get("replay_history", []) + [attempts] for a in items)
    write_json(path, record)
    write_report(directory, record)
    return record


class WorkerCase:
    """One fixed local worker slot; only the four manifest scenarios use it."""
    def __init__(self, run_id: str, task: str):
        self.run_id, self.task = run_id, task
        self.generation = 0
        self.owner = None
        self.status = "idle"
        self.result = None
        self.spent = 0
        self.ignored = 0
        self.events = []

    def event(self, kind: str, **detail) -> None:
        self.events.append({"kind": kind, "generation": self.generation,
                            "permit_owner": self.owner, "status": self.status,
                            "spent_attempts": self.spent, **detail})

    def reserve(self, value: int) -> None:
        if self.generation >= 2 or self.status not in ("idle", "cancelled", "failed", "succeeded"):
            raise ValueError("frozen slot unavailable or generation budget exhausted")
        self.generation += 1
        self.value = value
        self.owner, self.status, self.result = "coordinator", "reserved", None
        self.event("reserve", input=value)

    def receive(self) -> None:
        if self.status != "reserved" or self.owner != "coordinator":
            raise ValueError("receive requires the coordinator's reserved permit")
        self.owner, self.status = "worker", "ready"
        self.event("receive")

    def dispatch(self) -> dict:
        if self.status != "ready" or self.owner != "worker":
            raise ValueError("dispatch requires received unused authority")
        self.owner, self.status = None, "running"
        self.spent += 1
        self.event("dispatch")
        # Actual pure local work; delayed delivery is simulated separately.
        response = {"run_id": self.run_id, "task_id": self.task, "slot": 0,
                    "attempt": self.generation, "generation": self.generation,
                    "owner": "worker", "input": self.value, "output_type": "square-result"}
        response.update({"outcome": "failed", "result": None} if self.value < 0
                        else {"outcome": "succeeded", "result": self.value * self.value})
        return response

    def cancel(self) -> None:
        if self.status not in ("reserved", "ready", "running"):
            raise ValueError("cannot cancel a terminal attempt")
        self.owner, self.status = None, "cancelled"
        self.event("cancel", completion="may already have completed" if self.spent else "not dispatched")

    def deliver(self, response: dict) -> None:
        binding = {"run_id": self.run_id, "task_id": self.task, "slot": 0,
                   "attempt": self.generation, "generation": self.generation,
                   "owner": "worker", "input": self.value, "output_type": "square-result"}
        if self.status != "running" or any(response.get(k) != v or type(response.get(k)) is not type(v)
                                           for k, v in binding.items()):
            self.ignored += 1
            self.event("ignore_response", response_generation=response.get("generation"))
            return
        expected = {"outcome": "failed", "result": None} if self.value < 0 else {
            "outcome": "succeeded", "result": self.value * self.value}
        if any(response.get(k) != v or type(response.get(k)) is not type(v) for k, v in expected.items()):
            raise ValueError("local worker response failed its exact output contract")
        self.status, self.result = response["outcome"], response["result"]
        self.event("collect", result=self.result)


def worker_scenario(spec: dict, run_id: str) -> dict:
    worker = WorkerCase(run_id, spec["id"])
    worker.reserve(spec["inputs"][0])
    worker.receive()
    if spec["id"] == "cancel_before_dispatch":
        worker.cancel()
    else:
        first = worker.dispatch()
        if spec["id"] == "cancel_late_slot_reuse":
            worker.cancel()
            worker.reserve(spec["inputs"][1])
            worker.receive()
            second = worker.dispatch()
            worker.deliver(first)
            worker.deliver(second)
            worker.deliver(second)
        else:
            worker.deliver(first)
    observed = {"status": worker.status, "result": worker.result,
                "spent": worker.spent, "ignored": worker.ignored}
    expected = {key: spec["expected_" + key] for key in observed}
    return {"task_id": spec["id"], "observed": observed, "expected": expected,
            "matches_contract": observed == expected, "events": worker.events,
            "generation": worker.generation, "permit_owner": worker.owner}


def run_worker(directory: Path, manifest: dict, resume: bool) -> dict:
    path = directory / "record.json"
    record = record_for(path, "worker", identity(manifest, None), resume)
    started = time.monotonic()
    record["attempts"] = [worker_scenario(spec, record["run_id"]) for spec in manifest["worker"]["scenarios"]]
    record["status"] = "complete" if all(a["matches_contract"] for a in record["attempts"]) else "failed"
    record["elapsed_seconds"] = time.monotonic() - started
    record["evidence"] = "four deterministic local Python scenarios, not NMLT semantics, exhaustive interleavings, or an external exactly-once guarantee"
    record["replay_kind"] = "recompute pure local work and scenario checks; no external effects"
    record["declared_budget"] = {"slots": manifest["worker"]["max_slots"],
                                 "generations_per_scenario": manifest["worker"]["max_generations_per_scenario"]}
    write_json(path, record)
    write_report(directory, record)
    return record


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--scenario", choices=["all", "proof", "discovery", "worker"], default="all")
    parser.add_argument("--lean-command-json", default='["lean"]')
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--stop-after", type=int, help="stop after this many new proof strategies; then use --resume")
    args = parser.parse_args(argv)
    try:
        if args.stop_after is not None and (args.stop_after < 1 or args.scenario != "proof"):
            raise ValueError("--stop-after requires --scenario proof and a positive count")
        manifest = load_manifest()
        checker = None if args.scenario == "worker" else LeanChecker(json.loads(args.lean_command_json), manifest)
        results = []
        if args.scenario in ("all", "proof"):
            results.append(run_proof(args.output_dir / "proof", manifest, checker, args.resume, args.stop_after))
        if args.scenario in ("all", "discovery"):
            results.append(run_discovery(args.output_dir / "discovery", manifest, checker, args.resume))
        if args.scenario in ("all", "worker"):
            results.append(run_worker(args.output_dir / "worker", manifest, args.resume))
        for result in results:
            print(f"{result['scenario']}: {result['status']} ({len(result['attempts'])} frozen tasks); {args.output_dir / result['scenario'] / 'record.json'}")
        return 1 if any(result["status"] == "failed" for result in results) else 0
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(json.dumps({"status": "error", "message": str(error), "mathematical_outcome": "unknown"}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Human-readable, one-command NMLT judge demonstration."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
from typing import Any


class DemoFailure(Exception):
    def __init__(self, message: str, exit_code: int = 1) -> None:
        super().__init__(message)
        self.exit_code = exit_code


class Presenter:
    def __init__(self, *, color: bool, pace: float) -> None:
        self.color = color
        self.pace = pace

    def paint(self, code: str, value: str) -> str:
        if not self.color:
            return value
        return f"\033[{code}m{value}\033[0m"

    def heading(self, value: str) -> None:
        print()
        print(self.paint("1;36", value))
        print(self.paint("36", "=" * len(value)))
        self.wait()

    def status(self, label: str, value: str, *, good: bool = True) -> None:
        color = "1;32" if good else "1;31"
        print(f"{self.paint(color, label):<18} {value}")

    def note(self, value: str) -> None:
        print(self.paint("33", value))

    def wait(self, multiplier: float = 1.0) -> None:
        if self.pace > 0:
            time.sleep(self.pace * multiplier)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the accepted, counterexample, and stale-evidence judge path."
    )
    parser.add_argument("--nmlt", help="Path to a prebuilt nmlt executable.")
    parser.add_argument(
        "--paced",
        action="store_const",
        const=1.2,
        dest="pace",
        default=0.0,
        help="Pause between presentation beats for screen recording.",
    )
    parser.add_argument(
        "--pace",
        type=float,
        default=0.0,
        metavar="SECONDS",
        help="Use a custom pause between presentation beats.",
    )
    parser.add_argument("--no-color", action="store_true", help="Disable ANSI color.")
    return parser.parse_args()


def project_root() -> Path:
    return Path(__file__).resolve().parents[2]


def resolve_binary(root: Path, explicit: str | None) -> Path:
    candidates: list[Path] = []
    if explicit:
        candidates.append(Path(explicit).expanduser())
    if os.environ.get("NMLT_BIN"):
        candidates.append(Path(os.environ["NMLT_BIN"]).expanduser())
    candidates.extend(
        [
            root / "nmlt",
            root / "target" / "release" / "nmlt",
            root / "target" / "evidence" / "release" / "nmlt",
        ]
    )
    for candidate in candidates:
        resolved = candidate.resolve()
        if resolved.is_file() and os.access(resolved, os.X_OK):
            return resolved
    raise DemoFailure(
        "No prebuilt nmlt executable found. Pass --nmlt PATH or build "
        "target/release/nmlt.",
        2,
    )


def run_report(binary: Path, model: Path) -> tuple[dict[str, Any], bytes]:
    completed = subprocess.run(
        [str(binary), "model-check", "--json", str(model)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", errors="replace").strip()
        raise DemoFailure(f"model checker failed for {model.name}: {detail}", 2)
    try:
        value = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise DemoFailure(f"invalid JSON from model checker: {error}", 2) from error
    if not isinstance(value, dict):
        raise DemoFailure("model checker returned a non-object JSON report", 2)
    return value, completed.stdout


def require(condition: bool, message: str) -> None:
    if not condition:
        raise DemoFailure(message)


def short_identity(value: str) -> str:
    digest = value.rsplit(":", 1)[-1]
    return f"sha256:{digest[:16]}..."


def show_manual_boundary(presenter: Presenter, scenario: Path) -> None:
    c_text = (scenario / "provider_dispatch.c").read_text(encoding="utf-8")
    safe_rust = (scenario / "provider_dispatch_guarded.rs").read_text(encoding="utf-8")
    bad_rust = (scenario / "provider_dispatch_dropped_guard.rs").read_text(
        encoding="utf-8"
    )
    require("if (!attempt->armed)" in c_text, "C reference guard is missing")
    require("if !self.armed" in safe_rust, "guarded Rust fixture is missing its guard")
    require(
        "armed guard was dropped" in bad_rust,
        "dropped-guard Rust fixture is not labeled",
    )

    presenter.heading("[1/3] Review the migration boundary")
    print("C reference contract:")
    print("    if (!attempt->armed) return false;")
    print("Guard-preserving Rust candidate:")
    print("    if !self.armed { return false; }")
    print("Dropped-guard Rust candidate:")
    print("    // BUG: the C contract's armed guard was dropped during the port.")
    print("    self.dispatched = true;")
    print()
    presenter.note(
        "MANUAL ABSTRACTION: NMLT checks the authored finite behavior model. "
        "It does not parse C or Rust, prove source equivalence, or prove memory safety."
    )
    print("Behavioral mapping: C/Rust armed guard -> NMLT require armed")
    presenter.wait(1.5)


def check_accepted(
    presenter: Presenter, binary: Path, model: Path
) -> tuple[dict[str, Any], bytes]:
    presenter.heading("[2/3] Check the preserved workflow and the dropped guard")
    print("$ nmlt model-check guard-preserved.nmlt")
    report, raw = run_report(binary, model)
    properties = report.get("properties")
    require(report.get("result") == "model_checked", "accepted model did not pass")
    require(report.get("complete") is True, "accepted exploration was incomplete")
    require(isinstance(properties, list), "accepted report has no property list")
    require(properties, "accepted model checked no properties")
    require(
        all(item.get("result") == "model_checked" for item in properties),
        "at least one accepted-model property did not pass",
    )

    presenter.status("MODEL CHECKED", "complete finite exploration")
    print(
        f"  explored: {report.get('explored_states')} states, "
        f"{report.get('explored_transitions')} transitions"
    )
    bounds = report.get("bounds", {})
    print(
        f"  bounds: max_states={bounds.get('max_states')}, "
        f"max_depth={bounds.get('max_depth')}"
    )
    for item in properties:
        print(f"  PASS {item.get('property')}")
    presenter.note(
        "Meaning: every reachable state in this finite model was explored. "
        "This is not an unbounded theorem about native C or Rust."
    )
    presenter.wait(1.5)
    return report, raw


def check_mutant(presenter: Presenter, binary: Path, model: Path) -> None:
    print()
    print("Behavioral diff in the manually authored model:")
    print("    - require armed")
    print("    + [guard omitted]")
    print("$ nmlt model-check guard-dropped.nmlt")
    report, _ = run_report(binary, model)
    require(report.get("result") == "refuted", "dropped-guard model was not refuted")
    require(report.get("complete") is True, "dropped-guard report was incomplete")
    properties = report.get("properties")
    require(isinstance(properties, list), "dropped-guard report has no properties")
    violation = next(
        (
            item
            for item in properties
            if item.get("property") == "DispatchRequiresArm"
            and item.get("result") == "refuted"
        ),
        None,
    )
    require(violation is not None, "DispatchRequiresArm was not refuted")
    steps = violation.get("witness", {}).get("steps")
    require(isinstance(steps, list) and steps, "counterexample witness is missing")
    bad_step = next(
        (
            step
            for step in steps
            if step.get("action") == "dispatch"
            and step.get("state", {}).get("armed") is False
            and step.get("state", {}).get("dispatched") is True
        ),
        None,
    )
    require(bad_step is not None, "expected dropped-guard witness state was not found")

    presenter.status("COUNTEREXAMPLE", "DispatchRequiresArm", good=False)
    for step in steps:
        state = step.get("state", {})
        action = step.get("action") or "initial"
        print(
            f"  [{step.get('index')}] {action:<10} "
            f"phase={state.get('phase')}, armed={str(state.get('armed')).lower()}, "
            f"dispatched={str(state.get('dispatched')).lower()}"
        )
    presenter.note(
        "The trace is decisive for the authored model: dispatch is reachable "
        "while armed=false."
    )
    presenter.wait(1.5)


def check_stale_evidence(
    presenter: Presenter, binary: Path, accepted_model: Path
) -> None:
    presenter.heading("[3/3] Refuse stale model-check evidence")
    with tempfile.TemporaryDirectory(prefix="nmlt-judge-demo-") as temporary:
        live_model = Path(temporary) / "migration.nmlt"
        shutil.copyfile(accepted_model, live_model)

        saved_report, saved_raw = run_report(binary, live_model)
        replay_report, replay_raw = run_report(binary, live_model)
        require(saved_raw == replay_raw, "unchanged deterministic replay did not match")
        saved_id = saved_report.get("semantic_binding", {}).get("source_set_id")
        replay_id = replay_report.get("semantic_binding", {}).get("source_set_id")
        require(
            isinstance(saved_id, str) and saved_id == replay_id,
            "unchanged source binding did not match",
        )
        presenter.status("READBACK PASS", "saved report matches exact source replay")
        print(f"  bound model: {short_identity(saved_id)}")

        with live_model.open("a", encoding="utf-8") as output:
            output.write(
                "\n// Exact source bytes changed after the prior report was saved.\n"
            )

        current_report, current_raw = run_report(binary, live_model)
        current_id = current_report.get("semantic_binding", {}).get("source_set_id")
        require(isinstance(current_id, str), "current source binding is missing")
        require(current_id != saved_id, "source change did not change its binding")
        require(current_raw != saved_raw, "source change did not change the report")

        print("  changed: one comment in the exact NMLT model source")
        presenter.status("STALE EVIDENCE", "REJECTED", good=False)
        print(f"  saved:   {short_identity(saved_id)}")
        print(f"  current: {short_identity(current_id)}")
        print("  prior model_checked result was not applied")
        presenter.note(
            "Digest freshness binds the result to exact model bytes. It does not "
            "prove that the manual model faithfully represents the C or Rust source."
        )
        presenter.wait(1.5)


def main() -> int:
    args = parse_args()
    if args.pace < 0:
        raise DemoFailure("--pace must be zero or positive", 2)
    root = project_root()
    scenario = root / "demos" / "judge" / "c-to-rust"
    accepted_model = scenario / "guard-preserved.nmlt"
    dropped_model = scenario / "guard-dropped.nmlt"
    for required in [scenario, accepted_model, dropped_model]:
        if not required.exists():
            raise DemoFailure(f"required demo path is missing: {required}", 2)

    binary = resolve_binary(root, args.nmlt)
    color = sys.stdout.isatty() and not args.no_color and "NO_COLOR" not in os.environ
    presenter = Presenter(color=color, pace=args.pace)
    binary_digest = hashlib.sha256(binary.read_bytes()).hexdigest()

    print("NMLT JUDGE DEMO")
    print("Catch a dropped workflow guard before generated code is trusted.")
    print(f"checker: {binary}")
    print(f"binary:  sha256:{binary_digest[:16]}...")
    print("runtime: prebuilt local CLI; no build or network used")
    show_manual_boundary(presenter, scenario)
    check_accepted(presenter, binary, accepted_model)
    check_mutant(presenter, binary, dropped_model)
    check_stale_evidence(presenter, binary, accepted_model)

    presenter.heading("JUDGE RESULT")
    presenter.status("PASS", "all expected controls behaved correctly")
    print("  accepted model: complete bounded exploration")
    print("  seeded defect: exact counterexample returned")
    print("  changed model: prior evidence rejected as stale")
    print()
    print(
        "NMLT is pre-alpha research for finite workflow review. "
        "See demos/judge/README.md for the assurance boundary."
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except DemoFailure as error:
        print(f"JUDGE DEMO FAILED: {error}", file=sys.stderr)
        raise SystemExit(error.exit_code) from error

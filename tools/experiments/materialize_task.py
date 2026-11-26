#!/usr/bin/env python3
"""Create an ordinary local Lean project for one frozen evaluation task card."""
import argparse
from pathlib import Path

from check_evaluation import CORPUS, MANIFEST, sha, validate_manifest, write
from laboratory import read


def materialize(task_id: str, output: Path) -> dict:
    manifest = read(MANIFEST)
    validate_manifest(manifest)
    selected = next((row for row in manifest["proof_tasks"] if row["id"] == task_id), None)
    if selected is None:
        raise ValueError(f"unknown frozen task: {task_id}")
    output.mkdir(parents=True, exist_ok=False)
    definitions = CORPUS.read_text(encoding="utf-8").split("-- Ordinary statements.", 1)[0]
    (output / "Definitions.lean").write_text(definitions + "\nend NMLT.Evaluation.Corpus\n", encoding="utf-8", newline="\n")
    (output / "Goal.lean").write_text("import Definitions\nopen NMLT.Evaluation.Corpus\n"
                                     f"theorem EvaluationGoal {selected['header']} := by sorry\n",
                                     encoding="utf-8", newline="\n")
    write(output / "nmlt-lean.json", {"schema": "nmlt-lean-project-v1", "modules": ["Definitions", "Goal"],
          "target_module": "Goal", "target": "EvaluationGoal", "permitted_axioms": ["propext"]})
    card = {"schema": "nmlt-evaluation-task-card-v1", "assurance": "none",
            "corpus_sha256": sha(MANIFEST), "task": selected,
            "status": "unproved project; bind and check through the existing adapter",
            "reference_solution": selected["declaration"],
            "exposure": "public reference corpus; not a blind task"}
    write(output / "task-card.json", card)
    return card


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    materialize(args.task, args.output)
    print(f"created unproved task project: {args.output}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Fail closed on stale public documentation and trust-inventory drift."""

from __future__ import annotations

import re
import os
import subprocess
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
IGNORED_PARTS = {".git", ".lake", ".cache", "target"}
LINK = re.compile(r"\[[^\]]*\]\(([^)]+)\)")

ACTIVE_SURFACES = (
    "README.md",
    "Plan.md",
    "SECURITY.md",
    "CONTRIBUTING.md",
    "GOVERNANCE.md",
    "CHANGELOG.md",
    "docs/README.md",
    "docs/architecture.md",
    "docs/roadmap.md",
    "docs/language-sketch.md",
    "docs/core-calculus.md",
    "docs/threat-model.md",
    "mechanization/lean/README.md",
    "examples/README.md",
    "rfcs/README.md",
    ".github/PULL_REQUEST_TEMPLATE.md",
    ".github/ISSUE_TEMPLATE/research.yml",
    ".github/ISSUE_TEMPLATE/bug.yml",
)

REMOVED_COMPONENTS = (
    "nmlt-agent",
    "nmlt-engine",
    "nmlt-grades",
    "nmlt-open-kernel",
    "nmlt-temporal",
    "nmlt-verify",
)

REQUIRED_TRUSTED_PATHS = {
    "crates/nmlt-cli/src/main.rs",
    "tools/check_public_surface.py",
    "mechanization/lean/NMLT.lean",
    "mechanization/lean/NMLT/Core/Transition.lean",
    "mechanization/lean/NMLT/Core/TypedCore.lean",
    "mechanization/lean/NMLT/Typing/Judgments.lean",
    "mechanization/lean/NMLT/Metatheory/Soundness.lean",
    "mechanization/lean/NMLT/Correspondence/M9Kernel.lean",
}


def markdown_files() -> list[Path]:
    files: list[Path] = []
    for directory, names, filenames in os.walk(ROOT):
        names[:] = [name for name in names if name not in IGNORED_PARTS]
        base = Path(directory)
        files.extend(base / name for name in filenames if name.endswith(".md"))
    return files


def check_links() -> list[str]:
    failures: list[str] = []
    for document in markdown_files():
        text = document.read_text(encoding="utf-8")
        for match in LINK.finditer(text):
            target = match.group(1).strip("<>")
            if target.startswith(("http:", "https:", "mailto:", "#")):
                continue
            path_text = target.split("#", 1)[0]
            if not path_text:
                continue
            candidate = document.parent / path_text
            if not candidate.exists():
                line = text.count("\n", 0, match.start()) + 1
                failures.append(
                    f"{document.relative_to(ROOT)}:{line}: missing link target {target}"
                )
    return failures


def check_trusted_paths() -> list[str]:
    manifest_path = ROOT / "security/trusted-components.toml"
    with manifest_path.open("rb") as handle:
        manifest = tomllib.load(handle)

    failures: list[str] = []
    seen: set[str] = set()
    listed_paths: set[str] = set()
    for component in manifest.get("components", []):
        identifier = component.get("id", "<unnamed>")
        if identifier in seen:
            failures.append(f"duplicate trusted component id: {identifier}")
        seen.add(identifier)
        for path_text in component.get("paths", []):
            listed_paths.add(path_text)
            if not (ROOT / path_text).exists():
                failures.append(
                    f"trusted component {identifier} names missing path {path_text}"
                )
    for profile in manifest.get("claim_profiles", []):
        for identifier in profile.get("trusted_components", []):
            if identifier not in seen:
                failures.append(
                    f"claim profile {profile.get('name', '<unnamed>')} references "
                    f"unknown trusted component {identifier}"
                )
    for path_text in sorted(REQUIRED_TRUSTED_PATHS - listed_paths):
        failures.append(f"required active trusted path is not inventoried: {path_text}")
    return failures


def check_generated_pdfs() -> list[str]:
    result = subprocess.run(
        ["git", "ls-files", "*.pdf"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return [
        f"generated PDF is tracked: {line}"
        for line in result.stdout.splitlines()
        if line
    ]


def check_active_component_names() -> list[str]:
    failures: list[str] = []
    for relative in ACTIVE_SURFACES:
        path = ROOT / relative
        text = path.read_text(encoding="utf-8").lower()
        for component in REMOVED_COMPONENTS:
            if component in text:
                failures.append(
                    f"{relative}: active surface names removed component {component}"
                )
    return failures


def main() -> int:
    failures = (
        check_links()
        + check_trusted_paths()
        + check_generated_pdfs()
        + check_active_component_names()
    )
    if failures:
        for failure in failures:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(
        "ok: public links, trusted-component paths, removed-component names, "
        "and generated-PDF policy"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

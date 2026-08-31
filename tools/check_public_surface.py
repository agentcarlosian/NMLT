#!/usr/bin/env python3
"""Fail closed on stale public documentation and trust-inventory drift."""

from __future__ import annotations

import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
IGNORED_PARTS = {".git", ".lake", "target"}
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
    "docs/artifact-identity.md",
    "docs/getting-started.md",
    "docs/roadmap.md",
    "docs/language-sketch.md",
    "docs/core-calculus.md",
    "docs/threat-model.md",
    "docs/decisions/0004-language-mathematics-pivot.md",
    "mechanization/lean/README.md",
    "examples/README.md",
    "crates/nmlt-certificate/README.md",
    "rfcs/README.md",
    "rfcs/0004-artifact-identity.md",
    ".gitignore",
    ".github/PULL_REQUEST_TEMPLATE.md",
    ".github/ISSUE_TEMPLATE/research.yml",
    ".github/ISSUE_TEMPLATE/bug.yml",
)

HISTORICAL_DOCUMENTS = (
    "docs/phase-0-closeout.md",
    "docs/reproduction-2026-07-18.md",
    "docs/m9-completion-audit-2026-07-19.md",
    "docs/test-report-m11-001b-2026-07-19.md",
    "docs/test-report-m11-001c-core-2026-07-19.md",
    "docs/reboot-handoff-2026-07-19.md",
    "docs/reboot-handoff-2026-07-20.md",
    "docs/metatheory/phase-1-mathematical-core.md",
    "docs/metatheory/research-synthesis-2026-07-18.md",
    "docs/research-notes/m10-behavior-refinement-and-certificates-2026-07-19.md",
    "docs/research-notes/m11-contract-refinement-2026-07-19.md",
    "docs/research-notes/m11-open-system-refinement-2026-07-19.md",
    "docs/research-notes/m11-two-sided-congruence-2026-07-19.md",
    "docs/research-notes/m9-bidirectional-elaboration-2026-07-19.md",
    "docs/research-notes/m9-contract-resolution-2026-07-19.md",
    "docs/research-notes/m9-independent-kernel-2026-07-19.md",
    "docs/research-notes/m9-integration-and-correspondence-2026-07-19.md",
    "docs/research-notes/m9-resolution-and-explicit-core-2026-07-19.md",
    "docs/research-notes/phase-0-foundations-2026-07-18.md",
    "docs/research-notes/source-to-typed-core-and-project-identity-2026-07-19.md",
)

REQUIRED_PUBLIC_TEXT = {
    "docs/language-sketch.md": (
        "rely ContractFact.Authorized",
        "guarantee ContractFact.Ready",
    ),
    "docs/artifact-identity.md": (
        "Partially active identity specification",
        "Historical pre-pivot identity design",
    ),
    "rfcs/0004-artifact-identity.md": (
        "Partially superseded by the language-and-mathematics pivot",
        "Pivot disposition",
    ),
    "examples/README.md": ("durable fixture path",),
}

FORBIDDEN_PUBLIC_TEXT = {
    "examples/README.md": ("before the public stack is finalized",),
    ".gitignore": (
        "PGenerated",
        "PCheckerOutput",
        "tools/quint",
        "papers/",
    ),
}

HISTORICAL_SECTION_MARKERS = {
    "docs/artifact-identity.md": "## Historical pre-pivot identity design",
}

ACTIVE_COMPONENT_EXCEPTIONS = {
    ("docs/artifact-identity.md", "nmlt-temporal"),
}

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
        if marker := HISTORICAL_SECTION_MARKERS.get(relative):
            text = text.split(marker.lower(), 1)[0]
        for component in REMOVED_COMPONENTS:
            if (relative, component) in ACTIVE_COMPONENT_EXCEPTIONS:
                continue
            if component in text:
                failures.append(
                    f"{relative}: active surface names removed component {component}"
                )
    return failures


def check_historical_banners() -> list[str]:
    failures: list[str] = []
    snapshot = "0417f6e16ad64f92f79002293f54fd705c1dbc80"
    for relative in HISTORICAL_DOCUMENTS:
        text = (ROOT / relative).read_text(encoding="utf-8")
        opening = "\n".join(text.splitlines()[:8])
        if "> **Historical record.**" not in opening:
            failures.append(f"{relative}: missing top-of-file historical banner")
        if "history.md" not in opening or snapshot not in opening:
            failures.append(
                f"{relative}: historical banner does not link history and snapshot"
            )
    return failures


def check_public_contracts() -> list[str]:
    failures: list[str] = []
    for relative, required in REQUIRED_PUBLIC_TEXT.items():
        text = (ROOT / relative).read_text(encoding="utf-8")
        for phrase in required:
            if phrase not in text:
                failures.append(f"{relative}: missing required public text {phrase!r}")
    for relative, forbidden in FORBIDDEN_PUBLIC_TEXT.items():
        text = (ROOT / relative).read_text(encoding="utf-8")
        for phrase in forbidden:
            if phrase in text:
                failures.append(f"{relative}: stale public text {phrase!r}")
    return failures


def main() -> int:
    failures = (
        check_links()
        + check_trusted_paths()
        + check_generated_pdfs()
        + check_active_component_names()
        + check_historical_banners()
        + check_public_contracts()
    )
    if failures:
        for failure in failures:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(
        "ok: public links, trusted-component paths, removed-component names, "
        "historical banners, public contracts, and generated-PDF policy"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Optional local CLI/REPL/LeanInteract comparison; produces no NMLT proof acceptance."""
import argparse
import hashlib
import importlib.metadata
import json
import os
import platform
from pathlib import Path
import shutil
import subprocess
import time

import lean_interact
from lean_interact import LeanREPLConfig, LeanServer, LocalProject

ROOT = Path(__file__).resolve().parents[1]
REPL_REVISION = "bbeedf38e0898869fc3b7c009e1ea877b46204e4"
LEAN_VERSION = "v4.33.1"


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")


def classify(messages):
    if any(message.get("severity") == "error" for message in messages):
        return "rejected"
    if any("sorryAx" in message.get("data", "") or "uses 'sorry'" in message.get("data", "") for message in messages):
        return "admitted"
    if any("does not depend on any axioms" in message.get("data", "") for message in messages):
        return "elaborated_empty_axioms"
    if any("Example.offset_eq" in message.get("data", "") for message in messages):
        return "context"
    return "unclassified"


def decode_sequence(text):
    decoder = json.JSONDecoder()
    result = []
    while text.strip():
        value, end = decoder.raw_decode(text.lstrip())
        result.append(value)
        text = text.lstrip()[end:]
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lean-bin", type=Path, required=True)
    parser.add_argument("--repl-project", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    lean = args.lean_bin.resolve()
    lake = lean.with_name("lake.exe" if os.name == "nt" else "lake")
    repl_project = args.repl_project.resolve()
    repl = repl_project / ".lake/build/bin" / ("repl.exe" if os.name == "nt" else "repl")
    output = args.output.resolve()
    output.mkdir()
    version = subprocess.check_output([str(lean), "--version"], text=True, encoding="utf-8").strip()
    assert version.startswith("Lean (version 4.33.1,"), version
    assert importlib.metadata.version("lean-interact") == "0.11.5"
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repl_project, text=True).strip()
    assert revision == REPL_REVISION
    changes = subprocess.check_output(["git", "diff", "--name-only"], cwd=repl_project, text=True).splitlines()
    assert changes == ["lean-toolchain"]
    assert (repl_project / "lean-toolchain").read_text().strip() == f"leanprover/lean4:{LEAN_VERSION}"
    project = output / "project"
    shutil.copytree(ROOT / "examples/lean-project", project)
    (project / "lean-toolchain").write_text(f"leanprover/lean4:{LEAN_VERSION}\n", encoding="utf-8")
    (project / "lakefile.toml").write_text('name = "R3InterfaceComparison"\ndefaultTargets = ["Example"]\n\n'
                                         '[[lean_lib]]\nname = "Example"\n', encoding="utf-8")
    (project / "Example.lean").write_text("import Example.Goals\n", encoding="utf-8")
    built = subprocess.run([str(lake), "build"], cwd=project, text=True, encoding="utf-8", capture_output=True, timeout=180)
    (output / "project-build.stdout.txt").write_text(built.stdout, encoding="utf-8")
    (output / "project-build.stderr.txt").write_text(built.stderr, encoding="utf-8")
    assert built.returncode == 0, (built.stdout, built.stderr)
    header = "import Example.Goals\n\n"
    cases = [
        ("lookup", "#check Example.offset_eq\n", "context"),
        ("wrong", "theorem Probe.candidate (n : Nat) : Example.offset n = n := n\n", "rejected"),
        ("valid", "theorem Probe.candidate (n : Nat) : Example.offset n = n := Example.offset_eq n\n"
                  "#print axioms Probe.candidate\n", "elaborated_empty_axioms"),
        ("admitted", "theorem Probe.candidate (n : Nat) : Example.offset n = n := by sorry\n"
                     "#print axioms Probe.candidate\n", "admitted"),
    ]
    environment = os.environ.copy()
    environment.update(LEAN_SYSROOT=str(lean.parent.parent), LEAN_STACK_SIZE_KB="65536", MIMALLOC_ARENA_RESERVE="65536",
                       LEAN_PATH=os.pathsep.join([str(project / ".lake/build/lib/lean"), str(repl_project / ".lake/build/lib/lean")]))
    environment["PATH"] = str(lean.parent) + os.pathsep + environment.get("PATH", "")
    observations = []
    for label, body, expected in cases:
        path = project / f"Probe_{label}.lean"
        path.write_text(header + body, encoding="utf-8", newline="\n")
        start = time.perf_counter()
        ran = subprocess.run([str(lean), "--json", "--threads=1", "--memory=768", str(path)],
                             cwd=project, env=environment, text=True, encoding="utf-8", capture_output=True, timeout=30)
        messages = decode_sequence(ran.stdout)
        actual = classify(messages)
        assert not ran.stderr and actual == expected and ((ran.returncode == 0) == (expected != "rejected")), (label, ran.stdout, ran.stderr)
        write(output / f"cli-{label}.json", {"exit_code":ran.returncode,"messages":messages})
        observations.append({"backend":"cli","case":label,"classification":actual,"seconds":time.perf_counter()-start})
    requests = "".join(json.dumps({"cmd":header+body})+"\n\n" for _, body, _ in cases)
    start = time.perf_counter()
    ran = subprocess.run([str(repl)], cwd=project, env=environment, input=requests,
                         text=True, encoding="utf-8", capture_output=True, timeout=60)
    replies = decode_sequence(ran.stdout)
    write(output / "direct-repl-process.json", {"exit_code":ran.returncode,"stdout":ran.stdout,"stderr":ran.stderr})
    assert ran.returncode == 0 and not ran.stderr and len(replies) == len(cases), (ran.returncode,ran.stdout,ran.stderr)
    direct_seconds = time.perf_counter()-start
    for (label, _, expected), reply in zip(cases, replies):
        actual = classify(reply.get("messages", []))
        assert actual == expected, (label, reply)
        write(output / f"repl-{label}.json", reply)
        observations.append({"backend":"direct_repl","case":label,"classification":actual})
    # Use the supplied project and already built local REPL. No default-version
    # selection or download is involved in this comparison.
    local_project = LocalProject(directory=project, lake_path=lake, auto_build=False)
    config = LeanREPLConfig(project=local_project, local_repl_path=repl_project, build_repl=False,
                            lake_path=lake, enable_incremental_optimization=False, enable_parallel_elaboration=False)
    original_environment = {key:os.environ.get(key) for key in ("LEAN_STACK_SIZE_KB", "MIMALLOC_ARENA_RESERVE", "LEAN_SYSROOT", "PATH")}
    os.environ["LEAN_STACK_SIZE_KB"] = "65536"
    os.environ["MIMALLOC_ARENA_RESERVE"] = "65536"
    os.environ["LEAN_SYSROOT"] = str(lean.parent.parent)
    os.environ["PATH"] = environment["PATH"]
    try:
        with LeanServer(config) as server:
            for label, body, expected in cases:
                start = time.perf_counter()
                reply = server.run_dict({"cmd":header+body}, timeout=30)
                actual = classify(reply.get("messages", []))
                assert actual == expected, (label,reply)
                write(output / f"leaninteract-{label}.json", reply)
                observations.append({"backend":"leaninteract","case":label,"classification":actual,"seconds":time.perf_counter()-start})
            imported = server.run_dict({"cmd":"import Example.Goals"}, timeout=30)
            for label, body, expected in cases:
                start = time.perf_counter()
                reply = server.run_dict({"cmd":body,"env":imported["env"]}, timeout=30)
                actual = classify(reply.get("messages", []))
                assert actual == expected, (label,reply)
                write(output / f"leaninteract-retained-{label}.json", reply)
                observations.append({"backend":"leaninteract_retained_environment","case":label,"classification":actual,"seconds":time.perf_counter()-start})
    finally:
        for key,value in original_environment.items():
            if value is None:
                os.environ.pop(key,None)
            else:
                os.environ[key]=value
    package = Path(lean_interact.__file__).parent
    package_sources = {path.relative_to(package).as_posix():hashlib.sha256(path.read_bytes()).hexdigest()
                       for path in sorted(package.rglob("*.py"))}
    write(output / "lean-interact-sources.json", package_sources)
    write(output / "summary.json", {"schema":"nmlt-lean-interface-comparison-v1","assurance":"none","platform":platform.system(),
          "lean_version":LEAN_VERSION,"lean_version_report":version,"lean_interact_version":"0.11.5","repl_revision":revision,
          "repl_overlay":{"lean-toolchain":f"leanprover/lean4:{LEAN_VERSION}"},
          "lean_sha256":hashlib.sha256(lean.read_bytes()).hexdigest(),
          "repl_sha256":hashlib.sha256(repl.read_bytes()).hexdigest(),
          "lean_interact_sources_sha256":hashlib.sha256(json.dumps(package_sources,sort_keys=True,separators=(",",":")).encode()).hexdigest(),
          "script_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
          "direct_repl_batch_seconds":direct_seconds,"observations":observations,
          "scope":"Four fixed cases on one host; no independent proof checking, sandbox-equivalence or performance claim"})
    freeze = subprocess.check_output([os.sys.executable,"-m","pip","freeze"], text=True)
    (output / "python-environment.txt").write_text(freeze, encoding="utf-8")
    print(f"ok: all {len(observations)} interface observations agree; evidence: {output}",flush=True)


if __name__ == "__main__":
    main()

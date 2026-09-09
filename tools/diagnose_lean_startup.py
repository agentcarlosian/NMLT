#!/usr/bin/env python3
"""Read-only Linux Lean startup calibration under the existing adapter limits."""
import json
from pathlib import Path
import resource
import subprocess
import sys

lean = Path(sys.argv[1]).resolve()

def limits():
    for kind, value in [(resource.RLIMIT_DATA, 2 * 1024**3), (resource.RLIMIT_CPU, 5),
                        (resource.RLIMIT_CORE, 0), (resource.RLIMIT_FSIZE, 64 * 1024**2),
                        (resource.RLIMIT_NOFILE, 256)]:
        resource.setrlimit(kind, (value, value))

for name, options, bounded in [
    ("clean_environment", {}, False),
    ("current_stack", {}, True),
    ("small_arena", {"MIMALLOC_ARENA_RESERVE": "65536"}, True),
    ("lazy_arena", {"MIMALLOC_ARENA_EAGER_COMMIT": "0"}, True),
    ("small_lazy_arena", {"MIMALLOC_ARENA_RESERVE": "65536", "MIMALLOC_ARENA_EAGER_COMMIT": "0"}, True),
]:
    env = {"LEAN_STACK_SIZE_KB": "65536", **options}
    result = subprocess.run([str(lean), "--threads=1", "--memory=512", "--tstack=65536", "--version"],
                            env=env, preexec_fn=limits if bounded else None,
                            capture_output=True, text=True, timeout=10)
    print(json.dumps({"case": name, "exit": result.returncode, "stdout": result.stdout[:2048],
                      "stderr": result.stderr[:2048]}), flush=True)

"""Temporary Linux calibration of the bounded target helper."""
import json
from pathlib import Path
import resource
import subprocess
import sys

lean = Path(sys.argv[1]).resolve()
source = next(Path("target/r3-lean-tasks").glob("run-*/bound/build/NMLTTask.lean")).resolve()
environment = {"LEAN_SYSROOT": str(lean.parent.parent), "LEAN_PATH": str(source.parent),
               "PATH": str(lean.parent), "LEAN_STACK_SIZE_KB": "65536", "MIMALLOC_ARENA_RESERVE": "65536"}

def limits():
    for key, value in [(resource.RLIMIT_DATA, 2 * 1024**3), (resource.RLIMIT_CPU, 32),
                       (resource.RLIMIT_CORE, 0), (resource.RLIMIT_FSIZE, 64 * 1024**2),
                       (resource.RLIMIT_NOFILE, 256)]:
        resource.setrlimit(key, (value, value))

for memory in [512, 768, 1024, 1536]:
    result = subprocess.run([str(lean), "--threads=1", f"--memory={memory}", "-DmaxHeartbeats=200000",
                             "-o", f"calibration-{memory}.olean", source.name], cwd=source.parent,
                            env=environment, preexec_fn=limits, capture_output=True, text=True, timeout=30)
    print(json.dumps({"memory_mib": memory, "exit": result.returncode,
                      "cumulative_max_rss_kib": resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss,
                      "stdout": result.stdout[-4096:], "stderr": result.stderr[-4096:]}), flush=True)

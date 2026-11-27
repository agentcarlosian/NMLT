#!/usr/bin/env python3
"""Minimal reference custom subprocess adapter for NMLT (RFC 0042).

Reads a single JSON `Dispatch` from stdin, inspects the inner
`nmlt-custom-adapter-request-v1` payload, and prints a canonical
`nmlt-job-response-v1` JSON object to stdout.
"""
import json
import sys


def main() -> int:
    dispatch = json.loads(sys.stdin.read())
    inner = json.loads(dispatch["input"]["value"])
    text = inner["input"]
    if text.startswith("fail:"):
        outcome = {"kind": "failed", "message": text[5:] or "rejected"}
    else:
        outcome = {
            "kind": "completed",
            "value": {"type": "text", "value": f"checked:{text.upper()}"},
        }
    response = {
        "schema": "nmlt-job-response-v1",
        "binding": dispatch["binding"],
        "outcome": outcome,
        "observed_work": 1,
    }
    sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

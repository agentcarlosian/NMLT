#!/usr/bin/env python3
"""Exercise pinned native Lean LSP against an exported, ordinary Lake workspace."""
import argparse
import json
import os
from pathlib import Path
import queue
import subprocess
import threading
import time


def check(workspace, lean, output):
    workspace, lean, output = map(lambda p: Path(p).resolve(), (workspace, lean, output))
    source = workspace / "NMLTProof.lean"
    text = source.read_text(encoding="utf-8")
    needle = "Support.shift_eq n"
    assert needle in text, "LSP fixture expects the vendor support proof"
    env = os.environ.copy()
    env.update(LEAN_NUM_THREADS="1", LEAN_STACK_SIZE_KB="65536", MIMALLOC_ARENA_RESERVE="65536")
    env["PATH"] = str(lean.parent) + os.pathsep + env.get("PATH", "")
    lake = lean.with_name("lake.exe" if os.name == "nt" else "lake")
    messages, incoming = [], queue.Queue()
    output.mkdir()
    with (output / "stderr.txt").open("wb") as stderr:
        server = subprocess.Popen([str(lake), "--no-cache", "serve"], cwd=workspace,
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr,
                                  env=env, creationflags=0x08000000 if os.name == "nt" else 0)

        def read():
            try:
                total = 0
                while True:
                    headers = {}
                    while True:
                        line = server.stdout.readline()
                        if not line:
                            return incoming.put(RuntimeError("LSP output closed"))
                        if line in (b"\r\n", b"\n"):
                            break
                        key, value = line.decode("ascii").split(":", 1)
                        headers[key.lower()] = value.strip()
                    length = int(headers["content-length"])
                    total += length
                    assert 0 < length <= 1024 * 1024 and total <= 16 * 1024 * 1024
                    incoming.put(json.loads(server.stdout.read(length)))
            except BaseException as error:
                incoming.put(error)

        threading.Thread(target=read, daemon=True).start()

        def transmit(value):
            messages.append({"sent": value})
            payload = json.dumps(value, ensure_ascii=False).encode()
            server.stdin.write(f"Content-Length: {len(payload)}\r\n\r\n".encode() + payload)
            server.stdin.flush()

        def send(method, params, request_id=None):
            value = {"jsonrpc": "2.0", "method": method, "params": params}
            if request_id is not None:
                value["id"] = request_id
            transmit(value)

        def wait(predicate, timeout=45):
            deadline = time.monotonic() + timeout
            while True:
                message = incoming.get(timeout=max(0.01, deadline - time.monotonic()))
                if isinstance(message, BaseException):
                    raise message
                messages.append({"received": message})
                if "method" in message and "id" in message:
                    transmit({"jsonrpc": "2.0", "id": message["id"], "result": None})
                    continue
                if predicate(message):
                    return message
                if time.monotonic() > deadline:
                    raise TimeoutError("LSP response deadline exceeded")

        def response(request_id):
            message = wait(lambda m: m.get("id") == request_id and "method" not in m)
            assert "error" not in message, message
            return message["result"]

        def diagnostics(version, has_errors):
            return wait(lambda m: m.get("method") == "textDocument/publishDiagnostics"
                        and m["params"].get("version") == version
                        and any(d["severity"] == 1 for d in m["params"]["diagnostics"]) == has_errors)

        try:
            send("initialize", {"processId": os.getpid(), "rootUri": workspace.as_uri(),
                               "capabilities": {}, "initializationOptions": {"hasWidgets": False}}, 1)
            initialized = response(1)
            send("initialized", {})
            send("textDocument/didOpen", {"textDocument": {"uri": source.as_uri(), "languageId": "lean4",
                                                           "version": 1, "text": text}})
            line = next(i for i, value in enumerate(text.splitlines()) if needle in value)
            column = text.splitlines()[line].index(needle)
            position = {"textDocument": {"uri": source.as_uri()}, "position": {"line": line, "character": column + 9}}
            send("textDocument/hover", position, 2)
            hover = response(2)
            assert "Support.shift_eq" in json.dumps(hover), hover
            position["position"] = {"line": line - 1, "character": len(text.splitlines()[line - 1])}
            send("$/lean/plainGoal", position, 3)
            goal = response(3)
            assert goal and goal.get("goals"), goal
            send("textDocument/didChange", {"textDocument": {"uri": source.as_uri(), "version": 2},
                                           "contentChanges": [{"text": text.replace(needle, "True.intro")}]})
            rejected = diagnostics(2, True)
            assert any("Type mismatch" in d["message"] or "type mismatch" in d["message"] for d in rejected["params"]["diagnostics"]), rejected
            send("textDocument/didChange", {"textDocument": {"uri": source.as_uri(), "version": 3},
                                           "contentChanges": [{"text": text}]})
            wait(lambda m: m.get("method") == "$/lean/fileProgress"
                 and m["params"]["textDocument"].get("version") == 3
                 and m["params"]["processing"] == [])
            final_diagnostics = [m["received"]["params"]["diagnostics"] for m in messages
                                 if m.get("received", {}).get("method") == "textDocument/publishDiagnostics"
                                 and m["received"]["params"].get("version") == 3]
            if not final_diagnostics:
                final_diagnostics.append(diagnostics(3, False)["params"]["diagnostics"])
            assert final_diagnostics and not any(d["severity"] == 1 for d in final_diagnostics[-1])
            send("shutdown", {}, 4)
            response(4)
            transmit({"jsonrpc": "2.0", "method": "exit"})
            server.wait(timeout=15)
            assert server.returncode == 0
            (output / "summary.json").write_text(json.dumps({"schema": "nmlt-lean-lsp-validation-v1",
                "status": "context_only", "assurance": "none", "server": initialized,
                "hover": hover, "goal": goal, "wrong_proof_diagnostic": rejected,
                "repaired_document_version": 3}, indent=2) + "\n", encoding="utf-8")
        finally:
            if server.poll() is None:
                if os.name == "nt":
                    subprocess.run(["taskkill", "/PID", str(server.pid), "/T", "/F"], capture_output=True)
                else:
                    server.terminate()
                server.wait(timeout=15)
            (output / "messages.json").write_text(json.dumps(messages, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"ok: native Lake LSP hover, goal, error and repair ({output})", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--lean-bin", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    check(args.workspace, args.lean_bin, args.output)

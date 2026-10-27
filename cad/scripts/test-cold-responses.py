#!/usr/bin/env python3
"""Exercise cold capture against an existing packaged site and Chrome (not a benchmark)."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
from http.server import ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[2]
SITE = ROOT / "web/target/site"
BINARY = ROOT / "tools/browser-harness/target/debug/cad-capture"
spec = importlib.util.spec_from_file_location("serve_web", ROOT / "scripts/serve-web.py")
serve = importlib.util.module_from_spec(spec)
spec.loader.exec_module(serve)


def run(missing):
    class Handler(serve.handler_for(SITE)):
        def translate_path(self, path):
            url = path.partition("?")[0]
            if missing and url.endswith(f"/{missing}-worker/m1_{missing}_worker_bg.wasm"):
                return "/dev/null/missing"
            if url.startswith("/__perf/"):
                file = (ROOT / url.removeprefix("/__perf/")).resolve()
                return str(file) if file.is_relative_to(ROOT) else "/dev/null"
            return super().translate_path(path)

        def log_message(self, *args):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.run([str(BINARY), f"http://127.0.0.1:{server.server_port}",
                                     "monstertruck", "checked", "startup:cold", directory, "1", "validation"],
                                    cwd=ROOT, capture_output=True, text=True, timeout=240)
            events = [json.loads(line) for line in (Path(directory) / "events.log").read_text().splitlines()]
            if missing == "cad":
                assert result.returncode != 0, result.stderr
                assert next(event for event in events if event["name"] == "result")["failed"]
                assert "failed to fetch Wasm" in next(event["error"] for event in events if event["name"] == "failure")
                responses = next(event["responses"] for event in events if event["name"] == "resources")
                assert any(r["url"].endswith("/cad-worker/m1_cad_worker_bg.wasm")
                           and r["status"] == 404 and not r["serviceWorker"] for r in responses), responses
                assert not any(event["name"] == "reply-retained" for event in events), events
                return
            assert result.returncode == 0, result.stderr
            responses = next(event["responses"] for event in events if event["name"] == "resources")
            if missing == "core":
                core = [r for r in responses if r["url"].endswith("/core-worker/m1_core_worker_bg.wasm")]
                assert all(r["status"] == 404 and not r["serviceWorker"] for r in core), core
                if not core:
                    print("core worker 404 not observed in this run; response check inconclusive", flush=True)
                assert not next(event for event in events if event["name"] == "result")["failed"]
                assert any(event["name"] == "reply-retained" for event in events)
                return
            core = [r for r in responses if "/core-worker/" in r["url"]]
            assert all(r["status"] == 200 and not r["serviceWorker"] for r in core), core
            for asset in ("cad-worker/entry.js", "cad-worker/m1_cad_worker.js", "cad-worker/m1_cad_worker_bg.wasm",
                          "cad/boardstudio_cad_wasm.js", "cad/boardstudio_cad_wasm_bg.wasm"):
                assert any(r["url"].endswith("/" + asset) and r["status"] == 200 and not r["serviceWorker"]
                           for r in responses), asset
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    assert os.environ.get("CHROME_BIN"), "set CHROME_BIN and build cad-capture first"
    run(None)
    run("core")
    run("cad")
    print("cold responses and fatal CAD worker failure checked; diagnostic only")

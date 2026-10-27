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
            if missing and url.endswith("/core-worker/m1_core_worker_bg.wasm"):
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
            if missing:
                assert result.returncode != 0 and "landing core worker response failed" in result.stderr, result.stderr
                assert next(event for event in events if event["name"] == "result")["failed"]
                return
            assert result.returncode == 0, result.stderr
            responses = next(event["responses"] for event in events if event["name"] == "resources")
            for asset in ("core-worker/entry.js", "core-worker/m1_core_worker.js", "core-worker/m1_core_worker_bg.wasm",
                          "cad-worker/entry.js", "cad-worker/m1_cad_worker.js", "cad-worker/m1_cad_worker_bg.wasm",
                          "cad/boardstudio_cad_wasm.js", "cad/boardstudio_cad_wasm_bg.wasm"):
                assert any(r["url"].endswith("/" + asset) and r["status"] == 200 and not r["serviceWorker"]
                           for r in responses), asset
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    assert os.environ.get("CHROME_BIN"), "set CHROME_BIN and build cad-capture first"
    run(False)
    run(True)
    print("cold response capture and missing core worker failure control passed; diagnostic only")

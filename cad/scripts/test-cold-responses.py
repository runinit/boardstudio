#!/usr/bin/env python3
"""Exercise cold capture against an existing packaged site and Chrome (not a benchmark)."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import sys
from http.server import ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[2]
SITE = ROOT / "web/target/site"
sys.path.insert(0, str(ROOT / "scripts"))
import browser_tools  # noqa: E402
spec = importlib.util.spec_from_file_location("serve_web", ROOT / "scripts/serve-web.py")
serve = importlib.util.module_from_spec(spec)
spec.loader.exec_module(serve)


def run(missing, binary, output, chrome):
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
        directory = output / (missing or "normal")
        directory.mkdir()
        result = subprocess.run([str(binary), f"http://127.0.0.1:{server.server_port}",
                                 "monstertruck", "checked", "startup:cold", str(directory), "1", "validation"],
                                cwd=ROOT, env=dict(os.environ, CHROME_BIN=chrome), capture_output=True, text=True, timeout=240)
        (directory / "process.log").write_text(f"exit={result.returncode}\n{result.stdout}\n{result.stderr}")
        events = [json.loads(line) for line in (directory / "events.log").read_text().splitlines()]
        if missing == "cad":
            assert result.returncode != 0, result.stderr
            assert next(event for event in events if event["name"] == "result")["failed"]
            assert "failed to fetch Wasm" in next(event["error"] for event in events if event["name"] == "failure")
            responses = next(event["responses"] for event in events if event["name"] == "resources")
            assert any(r["url"].endswith("/cad-worker/m1_cad_worker_bg.wasm")
                       and r["status"] == 404 and not r["serviceWorker"] for r in responses), responses
            assert not any(event["name"] == "reply-retained" for event in events), events
            return
        responses = next(event["responses"] for event in events if event["name"] == "resources")
        if missing == "core":
            core = [r for r in responses if r["url"].endswith("/core-worker/m1_core_worker_bg.wasm")]
            assert all(r["status"] == 404 and not r["serviceWorker"] for r in core), core
            if core:
                assert result.returncode != 0, result.stderr
                assert next(event for event in events if event["name"] == "result")["failed"]
                assert any(event["name"] == "failure" and "observed core worker response failed" in event["error"]
                           for event in events), events
            else:
                print("core worker 404 not observed in this run; response check inconclusive", flush=True)
                assert result.returncode == 0, result.stderr
                assert not next(event for event in events if event["name"] == "result")["failed"]
            return
        assert result.returncode == 0, result.stderr
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
    chrome = browser_tools.locate_chrome()
    binary = browser_tools._build_harness("cad-capture")
    parent = ROOT / "web/target/test-results/cad-responses"
    parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    print(f"CAD response control output: {output}", flush=True)
    run(None, binary, output, chrome)
    run("core", binary, output, chrome)
    run("cad", binary, output, chrome)
    print("cold responses and fatal CAD worker failure checked; diagnostic only")

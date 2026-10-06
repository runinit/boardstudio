#!/usr/bin/env python3
"""Run the real Cadrum WASM bindings in headless Chromium (needs `cad/wasm/pkg`, built by build-cadrum-wasm.py)."""
import functools
import http.server
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading

CAD_ROOT = Path(__file__).resolve().parents[1]
PAGE = "scripts/browser-smoke/smoke.html"


def chromium():
    for name in (os.environ.get("CHROME"), "chromium", "chromium-browser", "google-chrome", "chrome"):
        if name and shutil.which(name):
            return shutil.which(name)
    raise RuntimeError("Chrome or Chromium is required (set CHROME to override)")


def serve():
    class Quiet(http.server.SimpleHTTPRequestHandler):
        extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map,
                          ".wasm": "application/wasm", ".js": "text/javascript"}

        def log_message(self, *args):
            pass

    server = http.server.ThreadingHTTPServer(
        ("127.0.0.1", 0), functools.partial(Quiet, directory=str(CAD_ROOT)))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def run_page(url, browser=None, timeout=180):
    with tempfile.TemporaryDirectory() as profile:
        command = [browser or chromium(), "--headless=new", "--no-sandbox", "--disable-gpu",
                   f"--user-data-dir={profile}", "--virtual-time-budget=120000", "--dump-dom", url]
        return subprocess.run(command, capture_output=True, text=True, timeout=timeout, check=False)


def parse_result(dom):
    marker = '<pre id="result">'
    start = dom.find(marker)
    if start < 0:
        raise RuntimeError("smoke page did not render a result")
    text = dom[start + len(marker):dom.index("</pre>", start)]
    text = text.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
    if text == "pending":
        raise RuntimeError("smoke page never finished")
    return json.loads(text)


def main():
    if not (CAD_ROOT / "wasm/pkg/boardstudio_cadrum_wasm_bg.wasm").exists():
        sys.exit("cad/wasm/pkg is missing; run python3 cad/scripts/build-cadrum-wasm.py first")
    server = serve()
    try:
        done = run_page(f"http://127.0.0.1:{server.server_address[1]}/{PAGE}")
    finally:
        server.shutdown()
    result = parse_result(done.stdout)
    for item in result["checks"]:
        print(("ok   " if item["ok"] else "FAIL ") + item["name"] + (f" ({item['detail']})" if not item["ok"] and item["detail"] else ""))
    if result["error"]:
        print("page error:", result["error"])
    if not result["ok"]:
        sys.exit(1)
    print(f"{len(result['checks'])} browser checks passed")


if __name__ == "__main__":
    main()

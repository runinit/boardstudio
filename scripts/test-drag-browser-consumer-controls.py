#!/usr/bin/env python3
"""Verify real drag-consumer failures cannot produce a success receipt.

Build the drag-smoke binary first and supply the pinned browser via CHROME_BIN.
"""

import os
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
import signal
import subprocess
import tempfile
import threading

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "tools/browser-harness/target/debug/drag-smoke"
BUTTONS = "".join(
    f"<button>{name}</button>"
    for name in ("Create new keyboard", "Split keyboard", "Continue to layout")
)
FIXTURE = '''<div role="slider" tabindex="0" aria-label="Rows" aria-valuenow="10"></div>
<script>
const slider = document.querySelector('[role=slider]');
const get = slider.getAttribute.bind(slider); let reads = 0;
slider.getAttribute = name => name === 'aria-valuenow' ? (++reads === 1 ? '10' : '9') : get(name);
</script>'''

with tempfile.TemporaryDirectory(prefix="ticket005-controls-", dir="/tmp") as directory:
    base = Path(directory)
    controls = (
        ("assertion", "", "slider Rows did not reach 10"),
        ("exception", "<script>setTimeout(() => { throw new Error('consumer exception control'); }, 0)</script>",
         "consumer exception control"),
        ("rejection", "<script>Promise.reject(new Error('consumer rejection control'))</script>",
         "consumer rejection control"),
        ("close-exception", "<script>addEventListener('beforeunload', () => { throw new Error('consumer close exception control'); })</script>",
         "consumer close exception control"),
    )
    for name, prefix, expected in controls:
        page = base / f"{name}.html"
        page.write_text("<!doctype html>" + prefix + BUTTONS + FIXTURE)
        receipt = base / name / "receipt.json"
        env = os.environ.copy()
        env.update(
            BOARDSTUDIO_PREVIEW_URL=page.as_uri(),
            BOARDSTUDIO_DRAG_RECEIPT_PATH=str(receipt),
            BOARDSTUDIO_DRAG_SOURCE_HEAD="control",
            BOARDSTUDIO_DRAG_SOURCE_SHA256="control",
            BOARDSTUDIO_DRAG_BUILD_SHA256="control",
            BOARDSTUDIO_DRAG_REPO_ROOT=str(ROOT),
        )
        result = subprocess.run([str(BINARY)], env=env, text=True, capture_output=True, timeout=90)
        output = result.stdout + result.stderr
        print(f"{name}: exit {result.returncode}, receipt exists {receipt.exists()}")
        print(output, flush=True)
        if result.returncode == 0 or receipt.exists() or expected not in output:
            raise SystemExit(f"{name} control failed")

    # Kill only this consumer's Chrome child while its observed target is open.
    killed = threading.Event()
    class LoseTransport(BaseHTTPRequestHandler):
        def do_GET(self):
            children = Path(f"/proc/{process.pid}/task/{process.pid}/children").read_text().split()
            chrome = Path(os.environ["CHROME_BIN"]).resolve()
            matches = [int(pid) for pid in children if Path(f"/proc/{pid}/exe").resolve() == chrome]
            if len(matches) != 1:
                raise RuntimeError(f"expected one owned Chrome child, got {matches}")
            os.kill(matches[0], signal.SIGKILL)
            killed.set()
            self.send_response(200)
            self.end_headers()

    server = HTTPServer(("127.0.0.1", 0), LoseTransport)
    receipt = base / "transport-loss" / "receipt.json"
    env.update(BOARDSTUDIO_PREVIEW_URL=f"http://127.0.0.1:{server.server_port}/",
               BOARDSTUDIO_DRAG_RECEIPT_PATH=str(receipt))
    process = subprocess.Popen([str(BINARY)], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    thread = threading.Thread(target=server.handle_request)
    thread.start()
    output, _ = process.communicate(timeout=90)
    thread.join(timeout=5)
    server.server_close()
    print(f"transport-loss: exit {process.returncode}, receipt exists {receipt.exists()}\n{output}", flush=True)
    if not killed.is_set() or process.returncode == 0 or receipt.exists() or "browser shutdown failed" not in output:
        raise SystemExit("transport-loss control failed")

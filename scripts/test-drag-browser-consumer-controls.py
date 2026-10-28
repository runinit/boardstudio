#!/usr/bin/env python3
"""Verify real drag-consumer failures cannot produce a success receipt.

Build the drag-smoke binary and supply the pinned browser via CHROME_BIN.
"""

import os
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
import signal
import subprocess
import tempfile
import threading

import browser_tools

ROOT = Path(__file__).resolve().parents[1]
BINARY = browser_tools.build_drag_harness()
BROWSER_TMP = Path.home() / ".cache/tmp"
# Leave room for the nested browser profiles and their Unix socket paths.
if len(os.fsencode(BROWSER_TMP)) > 40:
    BROWSER_TMP = Path("/tmp")
BROWSER_TMP.mkdir(parents=True, exist_ok=True)
BUTTONS = '<section id="m1-library-landing-content">' + "".join(
    f"<button>{name}</button>"
    for name in ("Create new keyboard", "Split keyboard", "Continue to layout")
) + '<label class="m1-library-import-action" aria-disabled="false">Import project<input type="file"></label></section>'
FIXTURE = '''<div role="slider" tabindex="0" aria-label="Rows" aria-valuenow="10"></div>
<script>
const slider = document.querySelector('[role=slider]');
const get = slider.getAttribute.bind(slider); let reads = 0;
slider.getAttribute = name => name === 'aria-valuenow' ? (++reads === 1 ? '10' : '9') : get(name);
</script>'''

with tempfile.TemporaryDirectory(prefix="bs-", dir=BROWSER_TMP) as directory:
    base = Path(directory)
    controls = (
        ("storage-ready", '''<script>addEventListener('DOMContentLoaded', () => {
          const importAction = document.querySelector('.m1-library-import-action');
          importAction.setAttribute('aria-disabled', 'true');
          const read = importAction.getAttribute.bind(importAction);
          // Release only after a second disabled observation in the driver's polling loop.
          let observed = false, nextPoll = false;
          importAction.getAttribute = name => {
            const value = read(name);
            if (name === 'aria-disabled' && value === 'true') {
              if (!observed) { observed = true; queueMicrotask(() => { nextPoll = true; }); }
              else if (nextPoll) queueMicrotask(() => importAction.setAttribute('aria-disabled', 'false'));
            }
            return value;
          };
          const split = [...document.querySelectorAll('button')].find(b => b.textContent === 'Split keyboard');
          const next = [...document.querySelectorAll('button')].find(b => b.textContent === 'Continue to layout');
          split.remove(); next.remove();
          document.querySelector('button').addEventListener('click', e => {
            if (!e.isTrusted || read('aria-disabled') !== 'false')
              throw new Error('create clicked before storage ready');
            importAction.parentNode.append(split, next);
          });
        });</script>''', "slider Rows did not reach 10"),
        ("storage-failure", '''<script>addEventListener('DOMContentLoaded', () => {
          document.querySelector('.m1-library-import-action').setAttribute('aria-disabled', 'true');
          const alert = document.createElement('p'); alert.className = 'm1-status';
          alert.setAttribute('role', 'alert');
          alert.textContent = 'Storage could not be initialized safely. Reload this page to retry. lock rejected';
          document.querySelector('#m1-library-landing-content').prepend(alert);
          document.querySelector('button').addEventListener('click', () => { throw new Error('create clicked after storage failure'); });
        });</script>''', "Storage could not be initialized safely"),
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
            TMPDIR=str(BROWSER_TMP),
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
        if (result.returncode == 0 or receipt.exists() or expected not in output
                or "create clicked before storage ready" in output
                or "create clicked after storage failure" in output):
            raise SystemExit(f"{name} control failed")

    # Break removal of this consumer's run directory after Chrome has opened a page.
    class BreakCleanup(BaseHTTPRequestHandler):
        def do_GET(self):
            runs = list(profile_root.glob("bs-drag-*"))
            if len(runs) != 1:
                raise RuntimeError(f"expected one owned run directory, got {runs}")
            runs[0].rename(profile_root / "moved")
            runs[0].write_text("not a directory")
            self.send_response(200)
            self.end_headers()
            self.wfile.write(("<!doctype html>" + BUTTONS + FIXTURE).encode())

    profile_root = base / "profiles"
    profile_root.mkdir()
    server = HTTPServer(("127.0.0.1", 0), BreakCleanup)
    receipt = base / "cleanup" / "receipt.json"
    env.update(TMPDIR=str(profile_root),
               BOARDSTUDIO_PREVIEW_URL=f"http://127.0.0.1:{server.server_port}/",
               BOARDSTUDIO_DRAG_RECEIPT_PATH=str(receipt))
    thread = threading.Thread(target=server.handle_request, daemon=True)
    thread.start()
    try:
        result = subprocess.run([str(BINARY)], env=env, text=True, capture_output=True, timeout=90)
    finally:
        thread.join(timeout=5)
        server.server_close()
    output = result.stdout + result.stderr
    print(f"cleanup: exit {result.returncode}, receipt exists {receipt.exists()}\n{output}", flush=True)
    if result.returncode == 0 or receipt.exists() or "cannot remove browser profile" not in output:
        raise SystemExit("cleanup control failed")
    env.pop("TMPDIR")

    # Kill only this consumer's Chrome child while its observed target is open.
    transport_profiles = base / "tp"
    transport_profiles.mkdir()
    killed = threading.Event()
    class LoseTransport(BaseHTTPRequestHandler):
        def do_GET(self):
            runs = list(transport_profiles.glob("bs-drag-*"))
            if len(runs) != 1:
                raise RuntimeError(f"expected one owned run directory, got {runs}")
            profile_arg = f"--user-data-dir={runs[0] / 'profile'}".encode()
            tasks = Path(f"/proc/{process.pid}/task")
            children = {pid for task in tasks.iterdir()
                        for pid in (task / "children").read_text().split()}
            matches = [int(pid) for pid in children
                       if profile_arg in Path(f"/proc/{pid}/cmdline").read_bytes().replace(b"\0", b" ").split()]
            if len(matches) != 1:
                raise RuntimeError(f"expected one owned Chrome child, got {matches}")
            fd = os.pidfd_open(matches[0])
            try:
                if profile_arg not in Path(f"/proc/{matches[0]}/cmdline").read_bytes().replace(b"\0", b" ").split():
                    raise RuntimeError("Chrome child changed before signalling")
                signal.pidfd_send_signal(fd, signal.SIGKILL)
            finally:
                os.close(fd)
            killed.set()
            self.send_response(200)
            self.end_headers()

    server = HTTPServer(("127.0.0.1", 0), LoseTransport)
    receipt = base / "transport-loss" / "receipt.json"
    env.update(TMPDIR=str(transport_profiles),
               BOARDSTUDIO_PREVIEW_URL=f"http://127.0.0.1:{server.server_port}/",
               BOARDSTUDIO_DRAG_RECEIPT_PATH=str(receipt))
    process = subprocess.Popen([str(BINARY)], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    thread = threading.Thread(target=server.handle_request, daemon=True)
    thread.start()
    try:
        output, _ = process.communicate(timeout=90)
    except subprocess.TimeoutExpired:
        process.kill()
        process.communicate()
        raise
    finally:
        thread.join(timeout=5)
        server.server_close()
    print(f"transport-loss: exit {process.returncode}, receipt exists {receipt.exists()}\n{output}", flush=True)
    if not killed.is_set() or process.returncode == 0 or receipt.exists() or "browser shutdown failed" not in output:
        raise SystemExit("transport-loss control failed")

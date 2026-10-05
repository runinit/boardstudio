#!/usr/bin/env python3
"""One bounded same-origin React→M1 service-worker/copy rehearsal.

Routes bytes unchanged from the pinned 5175 and immutable f956 root build. The
only server mutation is changing the chosen upstream. Browser profile is a fresh
uniquely named agent-browser session. No storage/cache clearing or SW unregister.
"""
from __future__ import annotations
import datetime as dt
import hashlib
import http.client
import http.server
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import time
import urllib.parse
import urllib.request
import zipfile

REPO = Path(__file__).resolve().parents[5]
EVIDENCE = Path(__file__).resolve().parent
BUILD = REPO / "web/target/builds/frontend-layout-pointer-cache-20261005"
INPUT = EVIDENCE / "reference-joined-input.boardstudio"

class State:
    mode = "reference"
    lock = threading.Lock()
    requests: list[dict] = []

STATE = State()

def record(item: dict) -> None:
    with STATE.lock:
        STATE.requests.append(item)
        (OUT / "proxy-requests.jsonl").write_text(
            "".join(json.dumps(v, sort_keys=True) + "\n" for v in STATE.requests), encoding="utf-8")

class Proxy(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.0"
    def log_message(self, *_args):
        pass
    def do_POST(self):
        path = urllib.parse.urlsplit(self.path).path
        mode = path.rsplit("/", 1)[-1]
        if path != "/__rehearsal__/switch/" + mode or mode not in {"reference", "candidate"}:
            self.send_error(404); return
        with STATE.lock:
            STATE.mode = mode
        payload = json.dumps({"mode": mode}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers(); self.wfile.write(payload)
    def do_GET(self): self.forward()
    def do_HEAD(self): self.forward()
    def forward(self):
        with STATE.lock:
            mode = STATE.mode
        upstream_port = 5175 if mode == "reference" else 34830
        parsed = urllib.parse.urlsplit(self.path)
        path = urllib.parse.urlunsplit(("", "", parsed.path, parsed.query, ""))
        conn = http.client.HTTPConnection("127.0.0.1", upstream_port, timeout=40)
        headers = {k: v for k, v in self.headers.items() if k.lower() not in {
            "host", "connection", "proxy-connection", "keep-alive", "transfer-encoding", "upgrade"}}
        headers["Host"] = f"127.0.0.1:{upstream_port}"
        headers["Accept-Encoding"] = "identity"
        started = time.monotonic()
        try:
            conn.request(self.command, path, headers=headers)
            response = conn.getresponse()
            self.send_response(response.status, response.reason)
            hop = {"connection", "keep-alive", "proxy-authenticate", "proxy-authorization",
                   "te", "trailers", "transfer-encoding", "upgrade", "proxy-connection"}
            for key, value in response.getheaders():
                if key.lower() not in hop:
                    self.send_header(key, value)
            self.send_header("Connection", "close")
            self.end_headers()
            self.close_connection = True
            byte_count = 0
            if self.command != "HEAD":
                while True:
                    chunk = response.read(256 * 1024)
                    if not chunk: break
                    self.wfile.write(chunk); byte_count += len(chunk)
            record({"time_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
                    "mode": mode, "upstream_port": upstream_port, "method": self.command,
                    "path": path, "status": response.status, "bytes": byte_count,
                    "response_headers": {k.lower(): v for k, v in response.getheaders() if k.lower() in {"content-type", "content-length", "content-encoding", "transfer-encoding"}},
                    "elapsed_ms": round((time.monotonic() - started) * 1000, 2)})
        except (BrokenPipeError, ConnectionResetError):
            record({"mode": mode, "upstream_port": upstream_port, "method": self.command,
                    "path": path, "status": "client-disconnected"})
        except Exception as exc:
            record({"mode": mode, "upstream_port": upstream_port, "method": self.command,
                    "path": path, "status": "proxy-error", "error": repr(exc)})
            if not self.wfile.closed:
                try: self.send_error(502, str(exc))
                except OSError: pass
        finally:
            conn.close()


def browser(session: str, steps: list, *args: str):
    argv = ["agent-browser", "--session", session, *args, "--json"]
    result = subprocess.run(argv, cwd=REPO, text=True, capture_output=True)
    item = {"argv": argv, "exit": result.returncode, "stdout": result.stdout,
            "stderr": result.stderr, "time_utc": dt.datetime.now(dt.timezone.utc).isoformat()}
    steps.append(item); save_json(OUT / "browser-steps.json", steps)
    value = json.loads(result.stdout) if result.stdout.strip() else {}
    # Navigation can exceed the CLI command timeout while service-worker cache
    # installation continues. Accept only this reported timeout; the following
    # explicit DOM/worker checks determine whether the page actually arrived.
    if result.returncode and not (args and args[0] == "open" and "timed out" in str(value).lower()):
        raise RuntimeError(f"agent-browser command failed: {args}: {result.stderr[-1200:]} {value}")
    if not value.get("success", True) and not (args and args[0] == "open" and "timed out" in str(value).lower()):
        raise RuntimeError(f"browser command failed: {args}: {value}")
    return value.get("data", value)

def evaluate(session: str, steps: list, expression: str):
    wrapped = f"(async()=>JSON.stringify(await ({expression})))()"
    result = browser(session, steps, "eval", wrapped)
    v = result.get("result", result)
    return json.loads(v) if isinstance(v, str) else v

def wait_fn(session: str, steps: list, expr: str, timeout_ms: int = 20000):
    browser(session, steps, "wait", "--fn", expr, "--timeout", str(timeout_ms))

def save_json(path: Path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")

def main():
    global OUT
    if len(sys.argv) != 3:
        print("usage: rehearsal.py PORT OUTPUT_DIR", file=sys.stderr); return 2
    port, OUT = int(sys.argv[1]), Path(sys.argv[2]).resolve()
    if OUT.exists() and any(OUT.iterdir()): raise SystemExit(f"output directory is not empty: {OUT}")
    OUT.mkdir(parents=True, exist_ok=True)
    session = "f96-cutover-" + dt.datetime.now(dt.timezone.utc).strftime("%H%M%S") + "-" + str(os.getpid())
    steps: list[dict] = []
    report = {"status": "running", "session": session, "origin": f"http://127.0.0.1:{port}",
              "reference": "http://127.0.0.1:5175/", "candidate": "http://127.0.0.1:34830/",
              "candidate_build": str(BUILD), "candidate_source_commit": "f956c0dfe9ec942cb79b9905565e9c404fb96732",
              "input_archive": str(INPUT), "input_sha256": hashlib.sha256(INPUT.read_bytes()).hexdigest(),
              "storage_policy": "fresh isolated browser session; no localStorage/IndexedDB/CacheStorage clearing; no service-worker unregister",
              "direct_indexeddb_migration": "not supplied; archive import is the compatibility boundary"}
    server = http.server.ThreadingHTTPServer(("127.0.0.1", port), Proxy)
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
    origin = report["origin"]
    failure = None
    try:
        browser(session, steps, "open", origin + "/")
        wait_fn(session, steps, "Boolean(document.querySelector('.wb-project-trigger') || document.querySelector('.wb-open-project'))")
        wait_fn(session, steps, "Boolean(navigator.serviceWorker.controller)", 120000)
        worker_initial = evaluate(session, steps, "({url:location.href, shell:document.querySelector('main')?.className, controller:navigator.serviceWorker.controller?.scriptURL ?? null, registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>r.active?.scriptURL ?? null), databases:await indexedDB.databases(), localStorageKeys:Object.keys(localStorage)})")
        report["reference_initial"] = worker_initial
        if not any((u or "").endswith("/sw.js") for u in worker_initial["registrations"]):
            raise RuntimeError(f"reference worker was not registered: {worker_initial}")
        if not worker_initial["controller"]:
            browser(session, steps, "reload")
            wait_fn(session, steps, "Boolean(document.querySelector('.wb-project-trigger') || document.querySelector('.wb-open-project'))")
            worker_initial = evaluate(session, steps, "({url:location.href,shell:document.querySelector('main')?.className,controller:navigator.serviceWorker.controller?.scriptURL ?? null,registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>r.active?.scriptURL ?? null),databases:await indexedDB.databases(),localStorageKeys:Object.keys(localStorage)})")
            report["reference_initial"] = worker_initial
        if not worker_initial["controller"]:
            raise RuntimeError(f"reference page never became worker-controlled: {worker_initial}")
        # Import a copied public archive using React's actual project file input.
        with zipfile.ZipFile(INPUT) as archive:
            input_document = json.loads(archive.read("project.json"))
        report["input_document"] = {k: input_document.get(k) for k in ("id", "name", "revision", "format")}
        browser(session, steps, "upload", "input.wb-project-file-input", str(INPUT))
        imported_id = json.dumps(input_document["id"])
        wait_fn(session, steps, f"Boolean(document.querySelector('.wb-project-trigger')) && localStorage.getItem('boardstudio-v2-active-project') === {imported_id}")
        imported = evaluate(session, steps, "({shell:document.querySelector('main')?.className,title:document.querySelector('.wb-project-name')?.textContent ?? document.querySelector('h1')?.textContent,projectMenu:!!document.querySelector('.wb-project-trigger'),databases:await indexedDB.databases(),localStorageKeys:Object.keys(localStorage),active:localStorage.getItem('boardstudio-v2-active-project')})")
        report["reference_import"] = imported
        if not imported["projectMenu"]: raise RuntimeError(f"reference archive import did not reach workbench: {imported}")
        # Publicly save a second copy, then reload and read the stored project via the visible UI.
        ref_copy = OUT / "reference-resaved.boardstudio"
        browser(session, steps, "click", "button.wb-project-trigger")
        browser(session, steps, "download", "button[title='Save project copy…']", str(ref_copy))
        if not ref_copy.is_file() or not ref_copy.stat().st_size:
            raise RuntimeError("React's public Save project copy action produced no archive")
        with zipfile.ZipFile(ref_copy) as archive:
            saved_document = json.loads(archive.read("project.json"))
            saved_entries = sorted(archive.namelist())
        report["reference_saved_copy"] = {"path": str(ref_copy), "bytes": ref_copy.stat().st_size,
                                          "sha256": hashlib.sha256(ref_copy.read_bytes()).hexdigest(),
                                          "document": {k: saved_document.get(k) for k in ("id", "name", "revision", "format")},
                                          "entries": saved_entries,
                                          "same_project_identity": saved_document.get("id") == input_document.get("id") and saved_document.get("name") == input_document.get("name")}
        if not report["reference_saved_copy"]["same_project_identity"]:
            raise RuntimeError("React archive round-trip changed project identity")
        browser(session, steps, "reload")
        wait_fn(session, steps, f"Boolean(document.querySelector('.wb-project-trigger')) && localStorage.getItem('boardstudio-v2-active-project') === {imported_id}")
        report["reference_reload_readback"] = evaluate(session, steps, "({shell:document.querySelector('main')?.className,title:document.querySelector('.wb-project-name')?.textContent,active:localStorage.getItem('boardstudio-v2-active-project'),databases:await indexedDB.databases(),controller:navigator.serviceWorker.controller?.scriptURL ?? null})")
        # Change only proxy target; do not unregister, clear caches, or change origin/profile.
        req = urllib.request.Request(origin + "/__rehearsal__/switch/candidate", method="POST", data=b"")
        with urllib.request.urlopen(req, timeout=5) as response: report["switch_candidate"] = json.loads(response.read())
        browser(session, steps, "reload")
        time.sleep(5)
        transition = evaluate(session, steps, "({url:location.href,shell:document.querySelector('main')?.className,reactShell:!!document.querySelector('.wb-root'),candidateShell:!!document.querySelector('.m1-canvas,.m1-library-landing'),controller:navigator.serviceWorker.controller?.scriptURL ?? null,registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL ?? null,waiting:r.waiting?.scriptURL ?? null,installing:r.installing?.scriptURL ?? null})),cacheNames:await caches.keys(),databases:await indexedDB.databases(),localStorageKeys:Object.keys(localStorage)})")
        report["candidate_transition"] = transition
        if not transition["candidateShell"] or not (transition["controller"] or "").endswith("/service-worker.js"):
            report["status"] = "blocked-by-existing-service-worker-transition"
            report["finding"] = "After proxy switched from exact reference bytes to immutable candidate bytes on the same origin/path, reload remained on the old shell or old controller; no cache/storage/registration was cleared. This is one observed transition result, not a repeated attempt."
        else:
            # If transition naturally works, import the React-produced copy into M1,
            # save/reload it, then switch back without resetting the browser profile.
            browser(session, steps, "upload", "input.m1-project-file-input", str(ref_copy))
            wait_fn(session, steps, "!!document.querySelector('.m1-canvas')")
            report["candidate_import"] = evaluate(session, steps, "({shell:document.querySelector('main')?.className,databases:await indexedDB.databases(),localStorageKeys:Object.keys(localStorage)})")
            browser(session, steps, "reload")
            wait_fn(session, steps, "!!document.querySelector('.m1-canvas')")
            report["candidate_reload_readback"] = evaluate(session, steps, "({shell:document.querySelector('main')?.className,databases:await indexedDB.databases(),localStorageKeys:Object.keys(localStorage),controller:navigator.serviceWorker.controller?.scriptURL ?? null})")
            req = urllib.request.Request(origin + "/__rehearsal__/switch/reference", method="POST", data=b"")
            with urllib.request.urlopen(req, timeout=5) as response: report["switch_reference"] = json.loads(response.read())
            browser(session, steps, "reload"); time.sleep(5)
            report["rollback_transition"] = evaluate(session, steps, "({shell:document.querySelector('main')?.className,reactShell:!!document.querySelector('.wb-root'),candidateShell:!!document.querySelector('.m1-canvas,.m1-library-landing'),controller:navigator.serviceWorker.controller?.scriptURL ?? null,registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>r.active?.scriptURL ?? null),cacheNames:await caches.keys(),databases:await indexedDB.databases(),localStorageKeys:Object.keys(localStorage)})")
            if report["rollback_transition"]["reactShell"]:
                report["status"] = "candidate-import-reload-rollback-react-shell-observed"
            else:
                report["status"] = "candidate-import-reload-rollback-transition-blocked"
    except Exception as exc:
        failure = repr(exc); report["status"] = "setup-or-public-journey-failed"; report["error"] = failure
        try:
            report["failure_dom"] = evaluate(session, steps, "({url:location.href,title:document.title,readyState:document.readyState,mainClasses:[...document.querySelectorAll('main')].map(e=>e.className),knownControls:[...document.querySelectorAll('.wb-project-trigger,.wb-open-project')].map(e=>({tag:e.tagName,className:e.className})),loadedScripts:[...document.scripts].map(e=>e.src).filter(Boolean),resources:performance.getEntriesByType('resource').map(e=>({name:e.name,type:e.initiatorType,duration:e.duration,size:e.transferSize})),registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>r.active?.scriptURL??null),controller:navigator.serviceWorker.controller?.scriptURL??null})")
            report["failure_browser_errors"] = browser(session, steps, "errors")
            report["failure_browser_network"] = browser(session, steps, "network", "requests")
        except Exception as diagnostic_error:
            report["failure_dom_error"] = repr(diagnostic_error)
    finally:
        report["proxy_requests"] = len(STATE.requests)
        report["proxy_modes"] = sorted({x.get("mode") for x in STATE.requests})
        hold = report["status"] in {"setup-or-public-journey-failed", "blocked-by-existing-service-worker-transition", "candidate-import-reload-rollback-transition-blocked"}
        report["session_retained_for_review"] = hold
        report["release_file"] = str(OUT / "release-session") if hold else None
        save_json(OUT / "report.json", report)
        if hold:
            # Keep the exact loaded app, profile, worker/cache state, and proxy
            # available for a selector correction or independent diagnosis.
            # Bounded to five minutes; create the release_file to clean up early.
            deadline = time.monotonic() + 300
            while time.monotonic() < deadline and not (OUT / "release-session").exists():
                time.sleep(1)
        try:
            browser(session, steps, "close")
        except Exception as exc:
            report["session_close_error"] = repr(exc)
            save_json(OUT / "report.json", report)
        server.shutdown(); server.server_close()
    print(json.dumps(report, indent=2, sort_keys=True))
    return 1 if failure else 0

if __name__ == "__main__":
    raise SystemExit(main())

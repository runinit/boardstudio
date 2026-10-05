#!/usr/bin/env python3
"""Create a fresh React-controlled profile for the final handoff rehearsal.

This reference-only bootstrap imports and re-saves the retained archive through
React's public UI, verifies its normal `/sw.js` controller, then leaves the
browser open while an explicit release token stops only the proxy owner.
"""
from __future__ import annotations

import datetime as dt
import hashlib
import http.client
import http.server
import json
import os
from pathlib import Path
import sys
import threading
import time
import urllib.parse
import zipfile

HERE = Path(__file__).resolve().parent
DEFAULT_INPUT = HERE / "run-f956-20261005-08" / "reference-resaved.boardstudio"
DEFAULT_PORT = 34836
sys.path.insert(0, str(HERE))
import continue_rehearsal as helpers  # noqa: E402


class State:
    def __init__(self, upstream: str, output: Path):
        self.upstream = upstream.rstrip("/") + "/"
        self.output = output
        self.lock = threading.Lock()
        self.requests: list[dict] = []

    def record(self, item: dict) -> None:
        with self.lock:
            self.requests.append(item)
            (self.output / "proxy-requests.jsonl").write_text(
                "".join(json.dumps(value, sort_keys=True) + "\n" for value in self.requests),
                encoding="utf-8")


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference-url", default="http://127.0.0.1:5175/",
                        help="Pinned ordinary React reference package")
    parser.add_argument("--origin-port", type=int, default=DEFAULT_PORT)
    parser.add_argument("--input-archive", type=Path, default=DEFAULT_INPUT)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--session", help="Optional unique agent-browser session name")
    parser.add_argument("--release-file", type=Path,
                        help="Create this file to stop the proxy only; the browser remains open")
    args = parser.parse_args()
    output = args.output_dir.resolve()
    input_archive = args.input_archive.resolve()
    if output.exists() and any(output.iterdir()):
        raise SystemExit(f"output directory is not empty: {output}")
    if not input_archive.is_file():
        raise SystemExit(f"input archive does not exist: {input_archive}")
    output.mkdir(parents=True, exist_ok=True)
    session = args.session or (
        "f96-cutover-final-" + dt.datetime.now(dt.timezone.utc).strftime("%H%M%S")
        + "-" + str(os.getpid()))
    origin = f"http://127.0.0.1:{args.origin_port}/"
    upstream = args.reference_url.rstrip("/") + "/"
    state = State(upstream, output)
    steps: list[dict] = []
    input_info = helpers.archive_info(input_archive)
    with zipfile.ZipFile(input_archive) as archive:
        input_document = json.loads(archive.read("project.json"))
    project_id = input_document["id"]

    class Handler(http.server.BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.0"

        def log_message(self, *_args):
            pass

        def do_GET(self):
            self.forward()

        def do_HEAD(self):
            self.forward()

        def forward(self):
            parsed = urllib.parse.urlsplit(state.upstream)
            request_url = urllib.parse.urlsplit(self.path)
            path = (parsed.path.rstrip("/") + (request_url.path or "/"))
            if not path:
                path = "/"
            conn = http.client.HTTPConnection(parsed.hostname, parsed.port or 80, timeout=60)
            headers = {key: value for key, value in self.headers.items() if key.lower() not in {
                "host", "connection", "proxy-connection", "keep-alive", "transfer-encoding", "upgrade"}}
            headers["Host"] = parsed.netloc
            headers["Accept-Encoding"] = "identity"
            start = time.monotonic()
            try:
                conn.request(self.command, urllib.parse.urlunsplit(("", "", path, request_url.query, "")),
                             headers=headers)
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
                        if not chunk:
                            break
                        self.wfile.write(chunk)
                        byte_count += len(chunk)
                state.record({"time_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
                              "upstream": upstream, "method": self.command, "path": path,
                              "status": response.status, "bytes": byte_count,
                              "elapsed_ms": round((time.monotonic() - start) * 1000, 2)})
            except (BrokenPipeError, ConnectionResetError):
                state.record({"upstream": upstream, "method": self.command, "path": path,
                              "status": "client-disconnected"})
            except Exception as exc:
                state.record({"upstream": upstream, "method": self.command, "path": path,
                              "status": "proxy-error", "error": repr(exc)})
                try:
                    self.send_error(502, str(exc))
                except OSError:
                    pass
            finally:
                conn.close()

    server = http.server.ThreadingHTTPServer(("127.0.0.1", args.origin_port), Handler)
    server.daemon_threads = True
    threading.Thread(target=server.serve_forever, daemon=True).start()
    release = (args.release_file or output / "release-session").resolve()
    helpers.save_json(output / "owner.json", {"pid": os.getpid(), "session": session,
                                               "origin": origin, "reference_upstream": upstream,
                                               "release_file": str(release),
                                               "browser_close_on_exit": False})
    report = {"status": "running", "session": session, "origin": origin,
              "reference_upstream": upstream, "input_archive": input_info,
              "storage_policy": "fresh isolated agent-browser session; no clearing/unregister",
              "browser_closed": False}
    failure = None
    try:
        helpers.browser(session, steps, output, "open", origin)
        helpers.wait_for(session, steps, output,
                         "document.querySelector('.wb-project-trigger') || document.querySelector('.wb-open-project')")
        reference_state = helpers.evaluate(session, steps, output,
            "({url:location.href,controller:navigator.serviceWorker.controller?.scriptURL??null,"
            "registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null})),"
            "active:localStorage.getItem('boardstudio-v2-active-project'),"
            "databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys()})")
        report["fresh_profile_before_import"] = reference_state
        if reference_state["active"]:
            raise RuntimeError(f"generated browser session already has an active project; refusing to reuse it: {reference_state}")
        if not reference_state["controller"]:
            helpers.browser(session, steps, output, "reload")
            helpers.wait_for(session, steps, output,
                             "document.querySelector('.wb-project-trigger') || document.querySelector('.wb-open-project')")
            helpers.wait_for(session, steps, output, "navigator.serviceWorker.controller")
            reference_state = helpers.evaluate(session, steps, output,
                "({url:location.href,controller:navigator.serviceWorker.controller?.scriptURL??null,"
                "registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null})),"
                "active:localStorage.getItem('boardstudio-v2-active-project'),"
                "databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys()})")
        report["reference_before_import"] = reference_state
        if not (reference_state["controller"] or "").endswith("/sw.js"):
            raise RuntimeError(f"bootstrap is not controlled by the ordinary React /sw.js: {reference_state}")
        root_registration = next((item for item in reference_state["registrations"] if item["scope"] == origin), None)
        if (not root_registration or not (root_registration["active"] or "").endswith("/sw.js")
                or root_registration["waiting"] or root_registration["installing"]):
            raise RuntimeError(f"React root worker registration is not settled: {reference_state}")

        expected = json.dumps(project_id)
        helpers.browser(session, steps, output, "upload", "input.wb-project-file-input", str(input_archive))
        helpers.wait_for(session, steps, output,
                         f"document.querySelector('.wb-project-trigger') && localStorage.getItem('{helpers.REFERENCE_ACTIVE_KEY}') === {expected}")
        helpers.wait_for(session, steps, output, "document.querySelector('.wb-save-state.is-saved')")
        copy_archive = output / "reference-resaved.boardstudio"
        helpers.browser(session, steps, output, "click", ".wb-project-trigger")
        helpers.browser(session, steps, output, "download", "button[title='Save project copy…']", str(copy_archive))
        archive_info = helpers.archive_info(copy_archive)
        comparison = helpers.compare_project_archives(input_archive, copy_archive)
        report["reference_saved_copy"] = archive_info
        report["reference_archive_comparison"] = comparison
        if (not comparison["project_document_within_accepted_numeric_tolerance"]
                or not comparison["asset_paths_equal"] or not comparison["asset_hashes_equal"]):
            raise RuntimeError(f"React public copy differs from imported project/assets: {comparison}")
        helpers.wait_for(session, steps, output, "document.querySelector('.wb-save-state.is-saved')")
        helpers.browser(session, steps, output, "reload")
        helpers.wait_for(session, steps, output,
                         f"document.querySelector('.wb-project-trigger') && document.querySelector('.wb-save-state.is-saved') && localStorage.getItem('{helpers.REFERENCE_ACTIVE_KEY}') === {expected}")
        report["reference_reload_readback"] = helpers.evaluate(session, steps, output,
            "({url:location.href,title:document.title,projectName:document.querySelector('.wb-project-name')?.textContent??null,"
            "active:localStorage.getItem('boardstudio-v2-active-project'),"
            "controller:navigator.serviceWorker.controller?.scriptURL??null,"
            "registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null})),"
            "databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys()})")
        if report["reference_reload_readback"]["projectName"] != input_document.get("name"):
            raise RuntimeError(f"reference reload did not display the imported project name: {report['reference_reload_readback']}")
        final_state = report["reference_reload_readback"]
        final_registration = next((item for item in final_state["registrations"] if item["scope"] == origin), None)
        if (not (final_state["controller"] or "").endswith("/sw.js")
                or not final_registration or not (final_registration["active"] or "").endswith("/sw.js")
                or final_registration["waiting"] or final_registration["installing"]):
            raise RuntimeError(f"reference reload is not controlled by a settled ordinary /sw.js worker: {final_state}")
        report["status"] = "reference-bootstrap-ready"
    except Exception as exc:
        failure = repr(exc)
        report["status"] = "bootstrap-stopped-at-first-failed-boundary"
        report["error"] = failure
        try:
            report["failure_state"] = helpers.evaluate(session, steps, output,
                "({url:location.href,title:document.title,controller:navigator.serviceWorker.controller?.scriptURL??null,"
                "active:localStorage.getItem('boardstudio-v2-active-project'),"
                "registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null})),"
                "databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys()})")
        except Exception as diag:
            report["failure_state_error"] = repr(diag)
    finally:
        report["proxy_requests"] = len(state.requests)
        helpers.save_json(output / "report.json", report)
        while not release.exists():
            time.sleep(1)
        server.shutdown()
        server.server_close()
    print(json.dumps(report, indent=2, sort_keys=True))
    return 1 if failure else 0


if __name__ == "__main__":
    raise SystemExit(main())

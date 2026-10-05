#!/usr/bin/env python3
"""Continue the same-origin archive cutover using the preserved browser profile.

This script intentionally never creates/closes an agent-browser session and
never clears storage, CacheStorage, or service-worker registrations. It is
prepared for the repaired candidate; do not run until that package is published.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import http.client
import http.server
import json
import math
from pathlib import Path
import subprocess
import threading
import time
import urllib.parse
import urllib.request
import zipfile

HERE = Path(__file__).resolve().parent
DEFAULT_SESSION = "f96-cutover-164703-3448630"
DEFAULT_ORIGIN_PORT = 34831
DEFAULT_INPUT = HERE / "run-f956-20261005-08" / "reference-resaved.boardstudio"
M1_ACTIVE_KEY = "boardstudio-m1-active-project:boardstudio-m1-root"
REFERENCE_ACTIVE_KEY = "boardstudio-v2-active-project"


class ProxyState:
    def __init__(self, reference_url: str, candidate_url: str, output: Path):
        self.targets = {"reference": reference_url, "candidate": candidate_url}
        self.mode = "candidate"
        self.lock = threading.Lock()
        self.requests: list[dict] = []
        self.output = output

    def record(self, event: dict) -> None:
        with self.lock:
            self.requests.append(event)
            (self.output / "proxy-requests.jsonl").write_text(
                "".join(json.dumps(item, sort_keys=True) + "\n" for item in self.requests),
                encoding="utf-8")


def save_json(path: Path, value) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def browser(session: str, steps: list[dict], output: Path, *args: str):
    argv = ["agent-browser", "--session", session, *args, "--json"]
    result = subprocess.run(argv, text=True, capture_output=True)
    item = {"argv": argv, "exit": result.returncode, "stdout": result.stdout,
            "stderr": result.stderr, "time_utc": dt.datetime.now(dt.timezone.utc).isoformat()}
    steps.append(item)
    save_json(output / "browser-steps.json", steps)
    try:
        value = json.loads(result.stdout) if result.stdout.strip() else {}
    except json.JSONDecodeError:
        value = {"unparsed_stdout": result.stdout}
    if result.returncode or not value.get("success", True):
        raise RuntimeError(f"agent-browser {args[0]} failed: {value} {result.stderr[-1000:]}")
    return value.get("data", value)


def evaluate(session: str, steps: list[dict], output: Path, expression: str):
    wrapped = f"(async()=>JSON.stringify(await ({expression})))()"
    result = browser(session, steps, output, "eval", wrapped)
    value = result.get("result", result)
    return json.loads(value) if isinstance(value, str) else value


def wait_for(session: str, steps: list[dict], output: Path, predicate: str,
             timeout_ms: int = 120000) -> None:
    # --fn expressions deliberately return Boolean, avoiding object serialization
    # failures in the browser command protocol.
    browser(session, steps, output, "wait", "--fn", f"Boolean({predicate})",
            "--timeout", str(timeout_ms))


def archive_info(path: Path) -> dict:
    with zipfile.ZipFile(path) as archive:
        document = json.loads(archive.read("project.json"))
        asset_hashes = {name: hashlib.sha256(archive.read(name)).hexdigest()
                        for name in sorted(archive.namelist()) if name.startswith("assets/")}
        return {"path": str(path), "bytes": path.stat().st_size,
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                "document": {key: document.get(key) for key in ("id", "name", "revision", "format")},
                "asset_hashes": asset_hashes,
                "asset_count": len(asset_hashes),
                "entries": sorted(archive.namelist())}


NUMERIC_ABS_TOLERANCE = 1e-8
NUMERIC_REL_TOLERANCE = 1e-12


def compare_document_values(expected, actual, path="$", differences=None):
    """Compare ProjectDoc fields using the accepted F94 abs/relative tolerances."""
    if differences is None:
        differences = []
    if isinstance(expected, dict) and isinstance(actual, dict):
        for key in sorted(set(expected) | set(actual)):
            child = f"{path}/{key}"
            if key not in expected or key not in actual:
                differences.append({"path": child, "expected": expected.get(key, "<missing>"),
                                   "actual": actual.get(key, "<missing>"), "within_tolerance": False})
            else:
                compare_document_values(expected[key], actual[key], child, differences)
    elif isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            differences.append({"path": path, "expected_length": len(expected),
                                "actual_length": len(actual), "within_tolerance": False})
        else:
            for index, (left, right) in enumerate(zip(expected, actual)):
                compare_document_values(left, right, f"{path}/{index}", differences)
    elif (isinstance(expected, (int, float)) and not isinstance(expected, bool)
          and isinstance(actual, (int, float)) and not isinstance(actual, bool)):
        if expected != actual:
            within = math.isclose(float(expected), float(actual),
                                  abs_tol=NUMERIC_ABS_TOLERANCE,
                                  rel_tol=NUMERIC_REL_TOLERANCE)
            differences.append({"path": path, "expected": expected, "actual": actual,
                                "delta": actual - expected, "within_tolerance": within})
    elif type(expected) is not type(actual) or expected != actual:
        differences.append({"path": path, "expected": expected, "actual": actual,
                            "within_tolerance": False})
    return differences


def compare_project_archives(expected_path: Path, actual_path: Path) -> dict:
    with zipfile.ZipFile(expected_path) as expected_zip, zipfile.ZipFile(actual_path) as actual_zip:
        expected_doc = json.loads(expected_zip.read("project.json"))
        actual_doc = json.loads(actual_zip.read("project.json"))
    differences = compare_document_values(expected_doc, actual_doc)
    expected = archive_info(expected_path)
    actual = archive_info(actual_path)
    asset_paths_equal = set(expected["asset_hashes"]) == set(actual["asset_hashes"])
    asset_hashes_equal = asset_paths_equal and expected["asset_hashes"] == actual["asset_hashes"]
    return {"project_document_differences": differences,
            "project_document_within_accepted_numeric_tolerance": all(
                item["within_tolerance"] for item in differences),
            "asset_paths_equal": asset_paths_equal,
            "asset_hashes_equal": asset_hashes_equal,
            "expected_asset_hashes": expected["asset_hashes"],
            "actual_asset_hashes": actual["asset_hashes"],
            "numeric_tolerance": {"absolute": NUMERIC_ABS_TOLERANCE,
                                  "relative": NUMERIC_REL_TOLERANCE}}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate-url", required=True,
                        help="Published repaired candidate base URL, including any path prefix")
    parser.add_argument("--reference-url", required=True,
                        help="Copied pinned React rollback package, including the declared worker handoff overlay")
    parser.add_argument("--origin-port", type=int, default=DEFAULT_ORIGIN_PORT)
    parser.add_argument("--session", default=DEFAULT_SESSION,
                        help="Preserved agent-browser session; this script never closes it")
    parser.add_argument("--input-archive", "--rollback-archive", dest="input_archive",
                        type=Path, default=DEFAULT_INPUT,
                        help="Copied React archive from run08 or equivalent retained copy")
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--release-file", type=Path,
                        help="Create this file to stop the proxy owner; browser remains open")
    args = parser.parse_args()
    output = args.output_dir.resolve()
    candidate_url = args.candidate_url.rstrip("/") + "/"
    reference_url = args.reference_url.rstrip("/") + "/"
    input_archive = args.input_archive.resolve()
    if output.exists() and any(output.iterdir()):
        raise SystemExit(f"output directory is not empty: {output}")
    if not input_archive.is_file():
        raise SystemExit(f"input archive does not exist: {input_archive}")
    output.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(input_archive) as archive:
        input_document = json.loads(archive.read("project.json"))
    project_id = input_document["id"]
    state = ProxyState(reference_url, candidate_url, output)
    steps: list[dict] = []
    origin = f"http://127.0.0.1:{args.origin_port}/"
    report = {"status": "running", "session": args.session, "origin": origin,
              "reference_upstream": reference_url, "candidate_upstream": candidate_url,
              "input_archive": archive_info(input_archive),
              "same_origin_policy": "fixed origin and root scope; no cache/storage clearing or worker unregister",
              "direct_indexeddb_migration": "not supplied; public .boardstudio archive import is the compatibility boundary",
              "phases": []}

    class Handler(http.server.BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.0"

        def log_message(self, *_args):
            pass

        def do_POST(self):
            path = urllib.parse.urlsplit(self.path).path
            mode = path.rsplit("/", 1)[-1]
            if path != "/__rehearsal__/switch/" + mode or mode not in state.targets:
                self.send_error(404)
                return
            with state.lock:
                state.mode = mode
            body = json.dumps({"mode": mode}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            self.forward()

        def do_HEAD(self):
            self.forward()

        def forward(self):
            with state.lock:
                mode = state.mode
                upstream = state.targets[mode]
            parsed_upstream = urllib.parse.urlsplit(upstream)
            parsed_path = urllib.parse.urlsplit(self.path)
            prefix = parsed_upstream.path.rstrip("/")
            upstream_path = prefix + (parsed_path.path or "/")
            path = urllib.parse.urlunsplit(("", "", upstream_path, parsed_path.query, ""))
            conn = http.client.HTTPConnection(parsed_upstream.hostname, parsed_upstream.port or 80, timeout=60)
            headers = {key: value for key, value in self.headers.items() if key.lower() not in {
                "host", "connection", "proxy-connection", "keep-alive", "transfer-encoding", "upgrade"}}
            headers["Host"] = parsed_upstream.netloc
            headers["Accept-Encoding"] = "identity"
            start = time.monotonic()
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
                        if not chunk:
                            break
                        self.wfile.write(chunk)
                        byte_count += len(chunk)
                state.record({"time_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
                              "mode": mode, "upstream": upstream, "method": self.command,
                              "path": path, "status": response.status, "bytes": byte_count,
                              "elapsed_ms": round((time.monotonic() - start) * 1000, 2)})
            except (BrokenPipeError, ConnectionResetError):
                state.record({"mode": mode, "upstream": upstream, "method": self.command,
                              "path": path, "status": "client-disconnected"})
            except Exception as exc:
                state.record({"mode": mode, "upstream": upstream, "method": self.command,
                              "path": path, "status": "proxy-error", "error": repr(exc)})
                try:
                    self.send_error(502, str(exc))
                except OSError:
                    pass
            finally:
                conn.close()

    server = http.server.ThreadingHTTPServer(("127.0.0.1", args.origin_port), Handler)
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    save_json(output / "owner.json", {"pid": __import__("os").getpid(), "origin": origin,
                                      "candidate_upstream": candidate_url, "session": args.session,
                                      "release_file": str(args.release_file or output / "release-session")})
    failure = None
    try:
        # Read-only identity check before first navigation/reload. This proves we
        # resumed the prior browser context and will not silently use a new one.
        before = evaluate(args.session, steps, output,
            "({url:location.href,controller:navigator.serviceWorker.controller?.scriptURL??null,"
            "registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null})),"
            "active:localStorage.getItem('boardstudio-v2-active-project'),"
            "databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys()})")
        report["preserved_profile_before"] = before
        if before["url"].split("/", 3)[:3] != origin.rstrip("/").split("/", 3)[:3]:
            raise RuntimeError(f"preserved browser is not on expected origin: {before}")
        if before["active"] != project_id or "boardstudio-v2" not in before["databases"]:
            raise RuntimeError(f"preserved copied project/storage identity differs: {before}")
        if not (before["controller"] or "").endswith("/sw.js"):
            raise RuntimeError(f"preserved React service worker identity differs: {before}")
        root_registration = next((item for item in before["registrations"] if item["scope"] == origin), None)
        if (not root_registration or not (root_registration["active"] or "").endswith("/sw.js")
                or root_registration["waiting"] or root_registration["installing"]):
            raise RuntimeError(f"preserved React worker registration is not settled: {before}")

        def switch(mode: str):
            request = urllib.request.Request(origin + f"__rehearsal__/switch/{mode}", method="POST", data=b"")
            with urllib.request.urlopen(request, timeout=5) as response:
                value = json.loads(response.read())
            report["phases"].append({"switch": value})

        def phase_reload(label: str, family: str):
            browser(args.session, steps, output, "reload")
            if family == "m1":
                wait_for(args.session, steps, output,
                         "document.querySelector('.m1-canvas') || document.querySelector('.m1-library-landing')")
                expected_shell = "Boolean(document.querySelector('.m1-canvas') || document.querySelector('.m1-library-landing'))"
            else:
                wait_for(args.session, steps, output,
                         "document.querySelector('.wb-project-trigger') || document.querySelector('.wb-open-project')")
                expected_shell = "Boolean(document.querySelector('.wb-project-trigger') || document.querySelector('.wb-open-project'))"
            expected_worker_path = "/service-worker.js" if family == "m1" else "/sw.js"
            worker_path_js = json.dumps(expected_worker_path)
            wait_for(args.session, steps, output,
                     f"navigator.serviceWorker.controller && new URL(navigator.serviceWorker.controller.scriptURL).pathname.replace(/\\/$/, '') === {worker_path_js}")
            # Worker activation is asynchronous. Poll the actual root registration
            # until this family is active and has no installing/waiting successor.
            settle_deadline = time.monotonic() + 120
            settled_worker = None
            while time.monotonic() < settle_deadline:
                settled_worker = evaluate(args.session, steps, output,
                    "(async()=>{const regs=await navigator.serviceWorker.getRegistrations();const root=regs.find(r=>r.scope==="
                    + json.dumps(origin) + ");return root?{scope:root.scope,active:root.active?.scriptURL??null,"
                    "waiting:root.waiting?.scriptURL??null,installing:root.installing?.scriptURL??null}:null})()")
                active_path = urllib.parse.urlsplit(settled_worker["active"]).path.rstrip("/") if settled_worker and settled_worker["active"] else None
                if (settled_worker and active_path == expected_worker_path
                        and not settled_worker["waiting"] and not settled_worker["installing"]):
                    break
                time.sleep(1)
            else:
                raise RuntimeError(f"{label}: expected active worker never settled without waiting/installing: {settled_worker}")
            current = evaluate(args.session, steps, output,
                "({url:location.href,m1:!!document.querySelector('.m1-canvas,.m1-library-landing'),"
                "react:!!document.querySelector('.wb-project-trigger,.wb-open-project'),"
                "controller:navigator.serviceWorker.controller?.scriptURL??null,"
                "registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null})),"
                "databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys(),"
                "reactActive:localStorage.getItem('boardstudio-v2-active-project'),m1Active:localStorage.getItem('" + M1_ACTIVE_KEY + "')})")
            expected_registration = next((item for item in current["registrations"]
                                          if item["scope"] == origin), None)
            worker_ready = bool(
                current["controller"]
                and urllib.parse.urlsplit(current["controller"]).path.rstrip("/") == expected_worker_path
                and expected_registration
                and expected_registration["active"]
                and urllib.parse.urlsplit(expected_registration["active"]).path.rstrip("/") == expected_worker_path
                and not expected_registration["waiting"]
                and not expected_registration["installing"])
            report["phases"].append({"reload": label, "expected_shell_predicate": expected_shell,
                                    "expected_worker_path": expected_worker_path,
                                    "worker_active_no_waiting": worker_ready,
                                    "observed": current})
            if family == "m1" and not current["m1"]:
                raise RuntimeError(f"{label}: M1 shell did not become visible: {current}")
            if family == "react" and not current["react"]:
                raise RuntimeError(f"{label}: React shell did not become visible: {current}")
            if not worker_ready:
                raise RuntimeError(f"{label}: expected final worker is not active/controling without a waiting update: {current}")
            return current

        def import_and_save(family: str, archive_path: Path, label: str) -> Path:
            expected = json.dumps(project_id)
            if family == "m1":
                landing_has_input = evaluate(args.session, steps, output,
                    "Boolean(document.querySelector('.m1-library-landing input[type=file][accept=\".boardstudio\"]'))")
                if not landing_has_input:
                    browser(args.session, steps, output, "click", "details.m1-project-menu > summary")
                browser(args.session, steps, output, "upload", "input[type='file'][accept='.boardstudio']", str(archive_path))
                wait_for(args.session, steps, output,
                         f"document.querySelector('.m1-project-menu') && localStorage.getItem('{M1_ACTIVE_KEY}') === {expected}")
                wait_for(args.session, steps, output,
                         "document.querySelector('.m1-save-state[data-state=\"saved\"]')")
                browser(args.session, steps, output, "click", "details.m1-project-menu > summary")
                copy_selector = "button.m1-project-copy-action"
            else:
                browser(args.session, steps, output, "click", ".wb-project-trigger")
                browser(args.session, steps, output, "upload", "input.wb-project-file-input", str(archive_path))
                wait_for(args.session, steps, output,
                         f"document.querySelector('.wb-project-trigger') && localStorage.getItem('{REFERENCE_ACTIVE_KEY}') === {expected}")
                wait_for(args.session, steps, output, "document.querySelector('.wb-save-state.is-saved')")
                browser(args.session, steps, output, "click", ".wb-project-trigger")
                copy_selector = "button[title='Save project copy…']"
            target = output / f"{label}.boardstudio"
            browser(args.session, steps, output, "download", copy_selector, str(target))
            info = archive_info(target)
            if info["document"]["id"] != project_id or info["document"]["name"] != input_document.get("name"):
                raise RuntimeError(f"{label}: public archive save changed project identity: {info}")
            archive_comparison = compare_project_archives(input_archive, target)
            if (not archive_comparison["project_document_within_accepted_numeric_tolerance"]
                    or not archive_comparison["asset_paths_equal"]
                    or not archive_comparison["asset_hashes_equal"]):
                raise RuntimeError(f"{label}: project/asset payload changed outside accepted round-trip tolerance: {archive_comparison}")
            report["phases"].append({"archive_comparison_to_input": label,
                                     "comparison": archive_comparison})
            report["phases"].append({"public_import_and_save": label, "archive": info})
            if family == "m1":
                wait_for(args.session, steps, output,
                         "document.querySelector('.m1-save-state[data-state=\"saved\"]')")
            else:
                wait_for(args.session, steps, output, "document.querySelector('.wb-save-state.is-saved')")
            browser(args.session, steps, output, "reload")
            if family == "m1":
                predicate = f"document.querySelector('.m1-project-menu') && document.querySelector('.m1-save-state[data-state=\"saved\"]') && localStorage.getItem('{M1_ACTIVE_KEY}') === {expected}"
            else:
                predicate = f"document.querySelector('.wb-project-trigger') && document.querySelector('.wb-save-state.is-saved') && localStorage.getItem('{REFERENCE_ACTIVE_KEY}') === {expected}"
            wait_for(args.session, steps, output, predicate)
            readback = evaluate(args.session, steps, output,
                "({url:location.href,react:!!document.querySelector('.wb-project-trigger'),"
                "m1:!!document.querySelector('.m1-project-menu'),"
                "reactName:document.querySelector('.wb-project-name')?.textContent??null,"
                "m1Name:document.querySelector('.m1-project-name')?.textContent??null,"
                "reactActive:localStorage.getItem('boardstudio-v2-active-project'),"
                "m1Active:localStorage.getItem('" + M1_ACTIVE_KEY + "'),"
                "databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys()})")
            report["phases"].append({"reload_readback": label, "observed": readback})
            visible_name = readback["m1Name"] if family == "m1" else readback["reactName"]
            if visible_name != input_document.get("name"):
                raise RuntimeError(f"{label}: reload did not display the saved project name: {readback}")
            return target

        # The proxy changes only upstream bytes; origin, profile and registrations
        # remain untouched. Each direction stops on its first failed boundary.
        phase_reload("reference-to-candidate", "m1")
        candidate_copy = import_and_save("m1", input_archive, "candidate-copy-from-reference")

        switch("reference")
        phase_reload("candidate-to-reference-rollback", "react")
        rollback_copy = import_and_save("react", candidate_copy, "reference-copy-from-candidate")

        switch("candidate")
        phase_reload("reference-to-candidate-readoption", "m1")
        final_copy = import_and_save("m1", rollback_copy, "candidate-readopted-copy")
        report["final_candidate_archive"] = archive_info(final_copy)
        report["status"] = "completed-same-origin-cutover-rollback-readoption"
    except Exception as exc:
        failure = repr(exc)
        report["status"] = "stopped-at-first-failed-boundary"
        report["error"] = failure
        try:
            report["failure_state"] = evaluate(args.session, steps, output,
                "({url:location.href,title:document.title,readyState:document.readyState,"
                "mainClasses:[...document.querySelectorAll('main')].map(e=>e.className),"
                "knownControls:[...document.querySelectorAll('.wb-project-trigger,.wb-open-project,.m1-project-menu,.m1-canvas,.m1-library-landing')].map(e=>({tag:e.tagName,className:e.className})),"
                "controller:navigator.serviceWorker.controller?.scriptURL??null,"
                "registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null})),"
                "databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys(),"
                "reactActive:localStorage.getItem('boardstudio-v2-active-project'),m1Active:localStorage.getItem('" + M1_ACTIVE_KEY + "')})")
        except Exception as diag:
            report["failure_state_error"] = repr(diag)
    finally:
        report["proxy_requests"] = len(state.requests)
        report["proxy_upstreams_seen"] = sorted({item.get("upstream") for item in state.requests})
        report["browser_session_closed"] = False
        report["proxy_owner_waits_for_release_file"] = True
        save_json(output / "report.json", report)
        release_file = (args.release_file or output / "release-session").resolve()
        while not release_file.exists():
            time.sleep(1)
        server.shutdown()
        server.server_close()
    print(json.dumps(report, indent=2, sort_keys=True))
    return 1 if failure else 0


if __name__ == "__main__":
    raise SystemExit(main())

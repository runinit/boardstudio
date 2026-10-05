#!/usr/bin/env python3
"""Resume the preserved M1-controlled cutover at rollback, without resetting state."""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import http.client
import http.server
import importlib
import json
from pathlib import Path
import threading
import time
import urllib.parse
import urllib.request
import zipfile

base = importlib.import_module("continue_rehearsal")
M1_ACTIVE_KEY = base.M1_ACTIVE_KEY
REFERENCE_ACTIVE_KEY = base.REFERENCE_ACTIVE_KEY


def save_json(path: Path, value) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate-url", required=True)
    parser.add_argument("--reference-url", required=True)
    parser.add_argument("--origin-port", type=int, required=True)
    parser.add_argument("--session", required=True)
    parser.add_argument("--candidate-archive", type=Path, required=True,
                        help="Archive already produced and verified in the forward phase")
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--previous-report", type=Path, required=True)
    args = parser.parse_args()
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        raise SystemExit(f"output directory is not empty: {output}")
    candidate_url = args.candidate_url.rstrip("/") + "/"
    reference_url = args.reference_url.rstrip("/") + "/"
    archive_path = args.candidate_archive.resolve()
    previous_report = args.previous_report.resolve()
    input_info = base.archive_info(archive_path)
    with zipfile.ZipFile(archive_path) as archive:
        input_document = json.loads(archive.read("project.json"))
    project_id = input_document["id"]
    origin = f"http://127.0.0.1:{args.origin_port}/"
    requests: list[dict] = []
    steps: list[dict] = []
    lock = threading.Lock()
    mode = {"value": "reference"}
    report = {
        "status": "running-from-rollback-boundary",
        "session": args.session,
        "origin": origin,
        "candidate_upstream": candidate_url,
        "reference_upstream": reference_url,
        "resumed_from": "live M1-controlled profile after successful reference-to-candidate public import/save/reload",
        "previous_forward_report": str(previous_report),
        "previous_forward_status": json.loads(previous_report.read_text()).get("status"),
        "candidate_archive": input_info,
        "same_origin_policy": "same profile, origin, worker registrations, databases and caches; no clearing/unregister",
        "direct_indexeddb_migration": "not supplied; public .boardstudio import is the compatibility boundary",
        "phases": [],
    }
    worker_paths = {"/service-worker.js", "/sw.js"}
    mutable_shell_paths = {"/", "/index.html"}

    class Handler(http.server.BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.0"

        def log_message(self, *_args):
            pass

        def record(self, item):
            with lock:
                requests.append(item)
                (output / "proxy-requests.jsonl").write_text(
                    "".join(json.dumps(row, sort_keys=True) + "\n" for row in requests), encoding="utf-8")

        def do_POST(self):
            path = urllib.parse.urlsplit(self.path).path
            selected = path.rsplit("/", 1)[-1]
            if path != "/__rehearsal__/switch/" + selected or selected not in {"candidate", "reference"}:
                self.send_error(404)
                return
            mode["value"] = selected
            body = json.dumps({"mode": selected}).encode()
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
            selected = mode["value"]
            upstream = reference_url if selected == "reference" else candidate_url
            parsed_upstream = urllib.parse.urlsplit(upstream)
            parsed_request = urllib.parse.urlsplit(self.path)
            request_path = parsed_request.path or "/"
            upstream_path = parsed_upstream.path.rstrip("/") + request_path
            upstream_target = urllib.parse.urlunsplit(("", "", upstream_path, parsed_request.query, ""))
            worker_script = request_path.rstrip("/") in worker_paths
            mutable_shell = request_path in mutable_shell_paths or request_path.rstrip("/") in mutable_shell_paths
            refreshable = worker_script or mutable_shell
            original_conditional = {
                name: self.headers.get(name)
                for name in ("If-Modified-Since", "If-None-Match")
                if self.headers.get(name) is not None
            }
            forwarded = {key: value for key, value in self.headers.items() if key.lower() not in {
                "host", "connection", "proxy-connection", "keep-alive", "transfer-encoding", "upgrade"}}
            forwarded["Host"] = parsed_upstream.netloc
            forwarded["Accept-Encoding"] = "identity"
            policy = "pass-through"
            if refreshable:
                for key in list(forwarded):
                    if key.lower() in {"if-modified-since", "if-none-match"}:
                        del forwarded[key]
                policy = ("worker" if worker_script else "mutable-shell") + ": strip conditional validators upstream; suppress ETag/Last-Modified and send Cache-Control:no-store downstream"
            conn = http.client.HTTPConnection(parsed_upstream.hostname, parsed_upstream.port or 80, timeout=60)
            start = time.monotonic()
            try:
                conn.request(self.command, upstream_target, headers=forwarded)
                response = conn.getresponse()
                response_headers = response.getheaders()
                body = b"" if self.command == "HEAD" else response.read()
                self.send_response(response.status, response.reason)
                hop = {"connection", "keep-alive", "proxy-authenticate", "proxy-authorization",
                       "te", "trailers", "transfer-encoding", "upgrade", "proxy-connection"}
                for key, value in response_headers:
                    low = key.lower()
                    if low in hop or (refreshable and low in {"etag", "last-modified", "cache-control"}):
                        continue
                    self.send_header(key, value)
                if refreshable:
                    self.send_header("Cache-Control", "no-store")
                    if self.command != "HEAD":
                        self.send_header("Content-Length", str(len(body)))
                self.send_header("Connection", "close")
                self.end_headers()
                self.close_connection = True
                if body:
                    self.wfile.write(body)
                self.record({
                    "time_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
                    "mode": selected,
                    "upstream": upstream,
                    "method": self.command,
                    "path": upstream_target,
                    "status": response.status,
                    "bytes": len(body),
                    "original_conditional_request_headers": original_conditional,
                    "forwarded_conditionals_stripped": refreshable,
                    "worker_validator_policy": policy if refreshable else None,
                    "upstream_last_modified": next((v for k, v in response_headers if k.lower() == "last-modified"), None),
                    "upstream_etag": next((v for k, v in response_headers if k.lower() == "etag"), None),
                    "body_sha256": hashlib.sha256(body).hexdigest() if refreshable and body else None,
                    "elapsed_ms": round((time.monotonic() - start) * 1000, 2),
                })
            except Exception as exc:
                self.record({"mode": selected, "upstream": upstream, "method": self.command,
                             "path": upstream_target, "status": "proxy-error", "error": repr(exc),
                             "original_conditional_request_headers": original_conditional})
                try:
                    self.send_error(502, str(exc))
                except OSError:
                    pass
            finally:
                conn.close()

    server = http.server.ThreadingHTTPServer(("127.0.0.1", args.origin_port), Handler)
    server.daemon_threads = True
    threading.Thread(target=server.serve_forever, daemon=True).start()
    save_json(output / "owner.json", {"pid": __import__("os").getpid(), "origin": origin,
                                      "session": args.session, "initial_mode": "reference",
                                      "release_file": str(output / "release-session")})
    failure = None
    try:
        # Prove the exact prior candidate date validator cannot suppress the
        # different rollback worker bytes, without touching the browser state.
        def conditional_probe(path, validator, expected_sha):
            request = urllib.request.Request(origin + path.lstrip("/"), headers={
                "If-Modified-Since": validator,
                "Accept-Encoding": "identity",
            })
            with urllib.request.urlopen(request, timeout=15) as response:
                body = response.read()
                probe = {"path": path, "status": response.status, "bytes": len(body),
                         "sha256": hashlib.sha256(body).hexdigest(),
                         "cache_control": response.headers.get("Cache-Control"),
                         "last_modified": response.headers.get("Last-Modified"),
                         "etag": response.headers.get("ETag")}
            if probe["status"] != 200 or probe["sha256"] != expected_sha or probe["cache_control"] != "no-store":
                raise RuntimeError(f"conditional validator probe did not return current rollback bytes: {probe}")
            return probe

        report["conditional_worker_probe"] = conditional_probe(
            "/service-worker.js", "Mon, 05 Oct 2026 17:18:27 GMT",
            "a8607a7281c3ef0a6d15f27582395e75668e94f331c0402ffec82ccf17cd55d3")
        report["conditional_shell_probe"] = conditional_probe(
            "/", "Mon, 05 Oct 2026 17:18:22 GMT",
            "10ebd0aae42700240237c25c23778d3788058e5640b0a09814e3be5ace79a114")

        def evaluate(expression):
            return base.evaluate(args.session, steps, output, expression)

        def wait_for(predicate, timeout=120000):
            return base.wait_for(args.session, steps, output, predicate, timeout)

        def click(selector):
            return base.browser(args.session, steps, output, "click", selector)

        def upload(selector, path):
            return base.browser(args.session, steps, output, "upload", selector, str(path))

        def reload_as(label, family):
            base.browser(args.session, steps, output, "reload")
            if family == "react":
                wait_for("document.querySelector('.wb-project-trigger') || document.querySelector('.wb-open-project')")
                expected_path = "/sw.js"
            else:
                wait_for("document.querySelector('.m1-canvas') || document.querySelector('.m1-library-landing')")
                expected_path = "/service-worker.js"
            path_js = json.dumps(expected_path)
            wait_for(f"navigator.serviceWorker.controller && new URL(navigator.serviceWorker.controller.scriptURL).pathname.replace(/\\/$/, '') === {path_js}")
            deadline = time.monotonic() + 120
            registration = None
            while time.monotonic() < deadline:
                registration = evaluate("(async()=>{const rs=await navigator.serviceWorker.getRegistrations();const r=rs.find(x=>x.scope===" + json.dumps(origin) + ");return r?{scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null}:null})()")
                active_path = urllib.parse.urlsplit(registration["active"]).path.rstrip("/") if registration and registration["active"] else None
                if active_path == expected_path and not registration["waiting"] and not registration["installing"]:
                    break
                time.sleep(1)
            else:
                raise RuntimeError(f"{label}: final worker did not settle: {registration}")
            state = evaluate("({url:location.href,react:!!document.querySelector('.wb-project-trigger,.wb-open-project'),m1:!!document.querySelector('.m1-canvas,.m1-library-landing'),controller:navigator.serviceWorker.controller?.scriptURL??null,registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null})),databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys(),reactActive:localStorage.getItem('boardstudio-v2-active-project'),m1Active:localStorage.getItem('" + M1_ACTIVE_KEY + "')})")
            phase = {"reload": label, "family": family, "expected_worker_path": expected_path,
                     "worker_active_no_waiting": True, "observed": state}
            report["phases"].append(phase)
            if (family == "react" and not state["react"]) or (family == "m1" and not state["m1"]):
                raise RuntimeError(f"{label}: expected {family} shell missing: {state}")
            return state

        def import_save(family, source_archive, label):
            expected = json.dumps(project_id)
            if family == "react":
                click(".wb-project-trigger")
                upload("input.wb-project-file-input", source_archive)
                wait_for(f"document.querySelector('.wb-project-trigger') && localStorage.getItem('{REFERENCE_ACTIVE_KEY}') === {expected}")
                wait_for("document.querySelector('.wb-save-state.is-saved')")
                click(".wb-project-trigger")
                save_selector = "button[title='Save project copy…']"
            else:
                has_input = evaluate("Boolean(document.querySelector('.m1-library-landing input[type=file][accept=\".boardstudio\"]'))")
                if not has_input:
                    click("details.m1-project-menu > summary")
                upload("input[type='file'][accept='.boardstudio']", source_archive)
                wait_for(f"document.querySelector('.m1-project-menu') && localStorage.getItem('{M1_ACTIVE_KEY}') === {expected}")
                wait_for("document.querySelector('.m1-save-state[data-state=\"saved\"]')")
                click("details.m1-project-menu > summary")
                save_selector = "button.m1-project-copy-action"
            target = output / f"{label}.boardstudio"
            base.browser(args.session, steps, output, "download", save_selector, str(target))
            comparison = base.compare_project_archives(archive_path, target)
            info = base.archive_info(target)
            if (info["document"]["id"] != project_id
                    or not comparison["project_document_within_accepted_numeric_tolerance"]
                    or not comparison["asset_paths_equal"] or not comparison["asset_hashes_equal"]):
                raise RuntimeError(f"{label}: project/asset copy differs: {comparison}")
            report["phases"].append({"public_import_save_export": label, "archive": info, "comparison": comparison})
            wait_for("document.querySelector('.wb-save-state.is-saved')" if family == "react" else "document.querySelector('.m1-save-state[data-state=\"saved\"]')")
            base.browser(args.session, steps, output, "reload")
            selector = ".wb-project-trigger" if family == "react" else ".m1-project-menu"
            key = REFERENCE_ACTIVE_KEY if family == "react" else M1_ACTIVE_KEY
            saved = ".wb-save-state.is-saved" if family == "react" else '.m1-save-state[data-state="saved"]'
            wait_for(f"document.querySelector('{selector}') && document.querySelector('{saved}') && localStorage.getItem('{key}') === {expected}")
            state = evaluate("({url:location.href,react:!!document.querySelector('.wb-project-trigger'),m1:!!document.querySelector('.m1-project-menu'),reactName:document.querySelector('.wb-project-name')?.textContent??null,m1Name:document.querySelector('.m1-project-name')?.textContent??null,reactActive:localStorage.getItem('boardstudio-v2-active-project'),m1Active:localStorage.getItem('" + M1_ACTIVE_KEY + "'),databases:(await indexedDB.databases()).map(x=>x.name)})")
            visible = state["reactName"] if family == "react" else state["m1Name"]
            if visible != input_document.get("name"):
                raise RuntimeError(f"{label}: reload did not read back copied project: {state}")
            report["phases"].append({"reload_readback": label, "observed": state})
            return target

        before = evaluate("({url:location.href,controller:navigator.serviceWorker.controller?.scriptURL??null,registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null})),reactActive:localStorage.getItem('boardstudio-v2-active-project'),m1Active:localStorage.getItem('" + M1_ACTIVE_KEY + "'),databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys()})")
        report["live_resume_state"] = before
        if (before["url"] != origin or not (before["controller"] or "").endswith("/service-worker.js")
                or before["m1Active"] != project_id or "boardstudio-m1-root" not in before["databases"]):
            raise RuntimeError(f"live browser is not at the expected M1-controlled rollback boundary: {before}")

        rollback = reload_as("candidate-to-reference-rollback", "react")
        rollback_archive = import_save("react", archive_path, "reference-copy-from-candidate")
        switch_request = urllib.request.Request(origin + "__rehearsal__/switch/candidate", method="POST", data=b"")
        with urllib.request.urlopen(switch_request, timeout=5) as response:
            report["phases"].append({"switch": json.loads(response.read())})
        final = reload_as("reference-to-candidate-readoption", "m1")
        final_archive = import_save("m1", rollback_archive, "candidate-readopted-copy")
        report["final_candidate_archive"] = base.archive_info(final_archive)
        report["status"] = "completed-same-origin-cutover-rollback-readoption"
    except Exception as exc:
        failure = repr(exc)
        report["status"] = "stopped-at-first-failed-boundary"
        report["error"] = failure
        try:
            report["failure_state"] = base.evaluate(args.session, steps, output,
                "({url:location.href,title:document.title,readyState:document.readyState,controller:navigator.serviceWorker.controller?.scriptURL??null,registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null})),databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys(),reactActive:localStorage.getItem('boardstudio-v2-active-project'),m1Active:localStorage.getItem('" + M1_ACTIVE_KEY + "')})")
        except Exception as diag:
            report["failure_state_error"] = repr(diag)
    finally:
        report["proxy_requests"] = len(requests)
        report["browser_session_closed"] = False
        save_json(output / "report.json", report)
        release = output / "release-session"
        while not release.exists():
            time.sleep(1)
        server.shutdown()
        server.server_close()
    print(json.dumps(report, indent=2, sort_keys=True))
    return 1 if failure else 0


if __name__ == "__main__":
    raise SystemExit(main())

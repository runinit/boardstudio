#!/usr/bin/env python3
"""Isolated public offline/update/lazy-module QA for one immutable M1 build.

Usage: offline-qualification.py <final-build-dir> <port> <output-dir>
The script may build a test-only root update worker, serves the build read-only,
and drives only a uniquely named agent-browser session. It never connects to the
Codex in-app browser or edits the build/reference/user browser profile.
"""
from __future__ import annotations
import argparse
import datetime as dt
import hashlib
import http.server
import json
import mimetypes
import os
from pathlib import Path
import secrets
import shutil
import socketserver
import subprocess
import sys
import threading
import time
import urllib.parse

REPO = Path(__file__).resolve().parents[4]
LAZY_ASSET = "assets/layout-generators/src/index.js"
MIME = {
    ".mjs": "text/javascript", ".js": "text/javascript", ".wasm": "application/wasm",
    ".boardstudio": "application/zip", ".json": "application/json", ".svg": "image/svg+xml",
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def tree_digest(site: Path, expected: dict[str, str]) -> str:
    """Verify the build's retained asset manifest and return its stable digest."""
    rows = []
    for relative, wanted in sorted(expected.items()):
        path = (site / relative).resolve()
        if site.resolve() not in path.parents:
            raise RuntimeError(f"asset escaped immutable site: {relative}")
        if not path.is_file():
            raise RuntimeError(f"candidate asset is missing: {relative}")
        actual = sha256(path)
        if actual != wanted:
            raise RuntimeError(f"candidate asset hash mismatch: {relative}: {actual} != {wanted}")
        rows.append((relative, actual))
    return hashlib.sha256(json.dumps(rows, separators=(",", ":")).encode()).hexdigest()


class QAState:
    def __init__(self, root: Path, subpath: Path, overlay: Path, output: Path):
        self.root, self.subpath, self.overlay, self.output = root, subpath, overlay, output
        self.root_update = False
        self.root_lazy_fault = False
        self.lock = threading.Lock()
        self.requests: list[dict] = []
        self.update_module = overlay / "boardstudio_offline_worker.js"
        self.update_bootstrap = overlay / "service-worker.js"

    def record(self, item: dict) -> None:
        with self.lock:
            self.requests.append(item)
            with (self.output / "server-requests.jsonl").open("a", encoding="utf-8") as stream:
                stream.write(json.dumps(item, sort_keys=True) + "\n")


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):
        return

    def end_headers(self):
        # Match the published candidate server: the app relies on cross-origin
        # isolation for its shared-memory/WebAssembly paths.
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()

    def _send(self, status: int, body: bytes, content_type: str):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def do_HEAD(self):
        self._serve()

    def do_GET(self):
        self._serve()

    def _serve(self):
        parsed = urllib.parse.urlsplit(self.path)
        request_path = urllib.parse.unquote(parsed.path)
        if request_path == "/boardstudio":
            self.send_response(308)
            self.send_header("Location", "/boardstudio/")
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        in_subpath = request_path.startswith("/boardstudio/")
        base = self.server.qa.subpath if in_subpath else self.server.qa.root
        relative = request_path[len("/boardstudio/"):] if in_subpath else request_path.lstrip("/")
        if not relative or relative.endswith("/"):
            relative += "index.html"
        relative_path = Path(relative)
        if relative_path.is_absolute() or ".." in relative_path.parts:
            self._send(400, b"invalid path", "text/plain")
            return

        state = self.server.qa
        if (state.root_update and not in_subpath and relative == "service-worker.js"
                and state.update_bootstrap.is_file()):
            file = state.update_bootstrap
        elif (state.root_update and not in_subpath and relative == "boardstudio_offline_worker.js"
                and state.update_module.is_file()):
            file = state.update_module
        elif state.root_lazy_fault and not in_subpath and relative == LAZY_ASSET:
            state.record({"method": self.command, "path": request_path, "status": 503,
                          "bytes": 0, "fault": "one-shot public lazy-module failure"})
            self._send(503, b"QA fault: temporarily unavailable", "text/plain")
            return
        else:
            file = (base / relative_path).resolve()
            if base.resolve() not in file.parents:
                self._send(400, b"invalid path", "text/plain")
                return
        try:
            body = file.read_bytes()
        except OSError:
            state.record({"method": self.command, "path": request_path, "status": 404,
                          "bytes": 0, "fault": None})
            self._send(404, b"Not found", "text/plain")
            return
        state.record({"method": self.command, "path": request_path, "status": 200,
                      "bytes": len(body), "update_overlay": str(file).startswith(str(state.overlay))})
        content_type = MIME.get(file.suffix.lower()) or mimetypes.guess_type(file.name)[0] or "application/octet-stream"
        self._send(200, body, content_type)


class Server(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True
    allow_reuse_address = True


def save_json(path: Path, value) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def browser_call(session: str, *args: str, log_dir: Path, steps: list[dict]):
    command = ["agent-browser", "--session", session, *args, "--json"]
    result = subprocess.run(command, cwd=REPO, text=True, capture_output=True)
    item = {"argv": command, "exit": result.returncode, "stdout": result.stdout,
            "stderr": result.stderr, "time_utc": dt.datetime.now(dt.timezone.utc).isoformat()}
    steps.append(item)
    save_json(log_dir / "browser-steps.json", steps)
    if result.returncode:
        raise RuntimeError(f"agent-browser failed ({result.returncode}): {args}: {result.stderr[-1500:]}")
    try:
        response = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"agent-browser did not return JSON for {args}: {result.stdout[-1000:]}") from error
    if not response.get("success", True):
        raise RuntimeError(f"agent-browser command failed: {args}: {response}")
    return response.get("data", response)


def eval_json(session: str, expression: str, *, log_dir: Path, steps: list[dict]):
    # agent-browser eval awaits a returned Promise. Keep JSON serialization
    # inside that promise so CacheStorage and service-worker operations produce
    # their resolved values instead of JSON.stringify(Promise) returning "{}".
    script = f"(async()=>JSON.stringify(await ({expression})))()"
    data = browser_call(session, "eval", script, log_dir=log_dir, steps=steps)
    value = data.get("result", data)
    if isinstance(value, str):
        return json.loads(value)
    return value


def wait_selector(session: str, selector: str, *, log_dir: Path, steps: list[dict]):
    browser_call(session, "wait", selector, log_dir=log_dir, steps=steps)


def wait_expr(session: str, expression: str, *, log_dir: Path, steps: list[dict]):
    browser_call(session, "wait", "--fn", expression, log_dir=log_dir, steps=steps)


def open_workspace(session: str, url: str, scope: str, *, log_dir: Path, steps: list[dict]):
    browser_call(session, "open", url, log_dir=log_dir, steps=steps)
    wait_expr(session, "document.querySelector('.m1-library-landing') || document.querySelector('.m1-canvas')",
              log_dir=log_dir, steps=steps)
    state = eval_json(session, "({canvas:!!document.querySelector('.m1-canvas'),landing:!!document.querySelector('.m1-library-landing')})",
                      log_dir=log_dir, steps=steps)
    if not state["canvas"]:
        wait_selector(session, ".m1-library-landing .m1-library button", log_dir=log_dir, steps=steps)
        browser_call(session, "click", ".m1-library-landing .m1-library button:first-child", log_dir=log_dir, steps=steps)
    wait_selector(session, ".m1-canvas", log_dir=log_dir, steps=steps)
    wait_expr(session, "navigator.serviceWorker.controller && document.querySelector('.m1-canvas')",
              log_dir=log_dir, steps=steps)
    wait_expr(session, f"navigator.serviceWorker.controller.scriptURL.startsWith(location.origin + {json.dumps(scope)} + 'service-worker.js')",
              log_dir=log_dir, steps=steps)
    return eval_json(session, "({url:location.href,online:navigator.onLine,worker:navigator.serviceWorker.controller.scriptURL,"
                              "workspace:document.querySelector('[role=tab][aria-selected=true]')?.textContent.trim(),"
                              "title:document.title,canvas:!!document.querySelector('.m1-canvas')})",
                     log_dir=log_dir, steps=steps)


def cache_names(session: str, *, log_dir: Path, steps: list[dict]):
    return eval_json(session, "caches.keys()", log_dir=log_dir, steps=steps)


def prepare_update_overlay(build: Path, out: Path, root_manifest: dict, version: str, commands: list[dict]):
    overlay = out / "update-overlay"
    overlay.mkdir()
    manifest = dict(root_manifest)
    manifest["version"] = version
    manifest_path = overlay / "offline-manifest.json"
    save_json(manifest_path, manifest)
    root_site = Path(json.loads((build / "provenance.json").read_text())["root"]["site"]).resolve()
    for relative in manifest["assets"]:
        target = root_site / relative
        if not target.is_file():
            raise RuntimeError(f"update manifest references missing candidate asset: {relative}")
    package = overlay / "pkg"
    env = dict(os.environ, BOARDSTUDIO_OFFLINE_MANIFEST=str(manifest_path))
    wasm_cmd = ["wasm-pack", "build", str(REPO / "web"), "--target", "web", "--out-name",
                "boardstudio_offline_worker", "--out-dir", str(package), "--release", "--locked",
                "--no-default-features", "--features", "service-worker"]
    started = dt.datetime.now(dt.timezone.utc).isoformat()
    run = subprocess.run(wasm_cmd, cwd=REPO, env=env, text=True, capture_output=True)
    (overlay / "update-worker-build.log").write_text(run.stdout + "\n" + run.stderr, encoding="utf-8")
    commands.append({"name": "compile isolated QA root-update worker", "argv": wasm_cmd,
                     "cwd": str(REPO), "exit": run.returncode, "started": started,
                     "finished": dt.datetime.now(dt.timezone.utc).isoformat(),
                     "log": str(overlay / "update-worker-build.log")})
    save_json(out / "commands.json", commands)
    if run.returncode:
        raise RuntimeError(f"isolated update-worker build failed; see {overlay / 'update-worker-build.log'}")
    embed_cmd = ["node", str(REPO / "scripts/web/embed-worker-wasm.mjs"), str(package),
                 str(manifest_path), str(overlay / "service-worker.js")]
    embed = subprocess.run(embed_cmd, cwd=REPO, text=True, capture_output=True)
    (overlay / "embed-worker.log").write_text(embed.stdout + "\n" + embed.stderr, encoding="utf-8")
    commands.append({"name": "embed isolated QA root-update worker", "argv": embed_cmd,
                     "cwd": str(REPO), "exit": embed.returncode,
                     "started": dt.datetime.now(dt.timezone.utc).isoformat(),
                     "finished": dt.datetime.now(dt.timezone.utc).isoformat(),
                     "log": str(overlay / "embed-worker.log")})
    save_json(out / "commands.json", commands)
    if embed.returncode or not (overlay / "boardstudio_offline_worker.js").is_file():
        raise RuntimeError(f"could not prepare isolated update-worker overlay; see {overlay / 'embed-worker.log'}")
    return overlay


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("final_build", type=Path, help="immutable complete build directory with provenance.json")
    parser.add_argument("port", type=int, help="isolated QA server port (0 chooses an available local port)")
    parser.add_argument("output_dir", type=Path, help="new, external directory for overlay and raw QA receipts")
    args = parser.parse_args()
    build = args.final_build.resolve()
    out = args.output_dir.resolve()
    if not (0 <= args.port <= 65535):
        parser.error("port must be 0..65535")
    if not build.is_dir() or not (build / "provenance.json").is_file():
        parser.error("final_build must be a complete build directory containing provenance.json")
    if out == build or build in out.parents:
        parser.error("output_dir must be outside the immutable build directory")
    if out.exists() and any(out.iterdir()):
        parser.error("output_dir must be new or empty")
    out.mkdir(parents=True, exist_ok=True)
    provenance_path = build / "provenance.json"
    provenance_bytes_before = provenance_path.read_bytes()
    provenance = json.loads(provenance_bytes_before)
    if provenance.get("status") not in (None, "complete"):
        parser.error(f"candidate build is not complete: {provenance.get('status')}")
    root_site = Path(provenance["root"]["site"]).resolve()
    sub_site = Path(provenance["subpath"]["site"]).resolve()
    if out == root_site or out in root_site.parents or root_site in out.parents:
        parser.error("output_dir must not overlap the immutable root site")
    if out == sub_site or out in sub_site.parents or sub_site in out.parents:
        parser.error("output_dir must not overlap the immutable subpath site")
    root_manifest_path = build / "offline-manifest-root.json"
    sub_manifest_path = build / "offline-manifest-subpath.json"
    for required in (root_manifest_path, sub_manifest_path):
        if not required.is_file():
            parser.error(f"missing required build manifest: {required}")
    root_manifest = json.loads(root_manifest_path.read_text(encoding="utf-8"))
    sub_manifest = json.loads(sub_manifest_path.read_text(encoding="utf-8"))
    root_tree_before = tree_digest(root_site, provenance["root"]["assets"])
    sub_tree_before = tree_digest(sub_site, provenance["subpath"]["assets"])
    run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + secrets.token_hex(3)
    update_version = "qa-f94-" + run_id.replace("T", "-").replace("Z", "")
    commands: list[dict] = []
    steps: list[dict] = []
    session = "f94-offline-" + secrets.token_hex(6)
    lazy_fault_asset = LAZY_ASSET
    overlay = None
    server = None
    thread = None
    report = {
        "run_id": run_id, "status": "running", "build_dir": str(build),
        "build_id": provenance.get("build_id"), "source_commit": provenance.get("source_commit"),
        "provenance_sha256": sha256(provenance_path), "root_site": str(root_site),
        "subpath_site": str(sub_site), "root_manifest_version": root_manifest["version"],
        "subpath_manifest_version": sub_manifest["version"], "root_manifest_assets": len(root_manifest["assets"]),
        "subpath_manifest_assets": len(sub_manifest["assets"]), "root_tree_sha256": root_tree_before,
        "subpath_tree_sha256": sub_tree_before, "qa_update_version": update_version,
        "browser_session": session, "scope_cache_prefixes": {
            "/": "boardstudio-m1-offline-path-2f-", "/boardstudio/":
            "boardstudio-m1-offline-path-" + "/boardstudio/".encode().hex() + "-"},
        "lazy_asset_fault": {"path": "/" + lazy_fault_asset,
                             "mode": "one scoped HTTP 503 for the real Parts-workspace dynamic import after deleting that URL from the isolated root CacheStorage; restore the serving response, reload and re-enter Parts"},
        "claims": [], "limitations": [
            "Only the named isolated agent-browser session is driven; the selected in-app browser and user profile are never used.",
            "The update worker is freshly compiled into output_dir/update-overlay with the candidate root manifest and a unique version; the immutable build remains the serving source.",
            "Lazy-module failure is a test-only server 503 during a real public Parts-workspace import, not a claim that Chromium offline emulation caused the error.",
            "This checks UI recovery after the real module becomes reachable on reload/re-entry; it does not add a production Retry control or prove recovery from every lazy asset failure.",
            "Browser/test evidence is not run by preparing this script; a later execution must retain a complete report and both build-tree hashes unchanged."
        ]}
    save_json(out / "report.json", report)

    try:
        overlay = prepare_update_overlay(build, out, root_manifest, update_version, commands)
        qa = QAState(root_site, sub_site, overlay, out)
        server = Server(("127.0.0.1", args.port), Handler)
        server.qa = qa
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        port = server.server_address[1]
        origin = f"http://127.0.0.1:{port}"
        report["origin"] = origin
        report["server_pid"] = os.getpid()
        report["update_overlay"] = str(overlay)
        save_json(out / "report.json", report)

        # Use one unique profile for both scopes so CacheStorage genuinely contains
        # the root and /boardstudio/ caches side by side and can prove isolation.
        root_initial = open_workspace(session, origin + "/", "/", log_dir=out, steps=steps)
        report["root_initial"] = root_initial
        if origin not in root_initial["worker"]:
            raise RuntimeError("root page did not get its own same-origin worker")
        root_names = cache_names(session, log_dir=out, steps=steps)
        root_name = next((name for name in root_names if name.startswith(report["scope_cache_prefixes"]["/"])), None)
        if root_name != report["scope_cache_prefixes"]["/"] + root_manifest["version"]:
            raise RuntimeError(f"root release cache is missing/unexpected: {root_names}")
        report["root_release_cache"] = root_name

        sub_initial = open_workspace(session, origin + "/boardstudio/", "/boardstudio/", log_dir=out, steps=steps)
        report["subpath_initial"] = sub_initial
        all_names = cache_names(session, log_dir=out, steps=steps)
        sub_prefix = report["scope_cache_prefixes"]["/boardstudio/"]
        sub_name = next((name for name in all_names if name.startswith(sub_prefix)), None)
        if sub_name != sub_prefix + sub_manifest["version"]:
            raise RuntimeError(f"subpath release cache is missing/unexpected: {all_names}")
        if root_name not in all_names:
            raise RuntimeError(f"subpath install erased root cache: {all_names}")
        report["subpath_release_cache"] = sub_name
        report["initial_cache_names"] = all_names

        # Mark an unrelated same-origin cache so the real worker's scoped cleanup
        # must preserve it during the root version transition.
        sentinel_cache = "unrelated-m1-qa-preserve-" + run_id
        sentinel = eval_json(session,
            f"(async()=>{{const c=await caches.open({json.dumps(sentinel_cache)});"
            f"await c.put(new Request(location.origin+'/__qa/sentinel'),new Response('preserve-me'));"
            "return await (await c.match(location.origin+'/__qa/sentinel')).text()})()",
            log_dir=out, steps=steps)
        if sentinel != "preserve-me":
            raise RuntimeError("could not establish isolated unrelated-cache sentinel")
        report["unrelated_cache_sentinel"] = {"name": sentinel_cache, "value": sentinel}

        # Flip only the root worker URL to the overlay. The same installed page
        # asks its production Registration to update; Rust policy performs install,
        # skipWaiting, scoped deletion and clients.claim.
        qa.root_update = True
        update = eval_json(session,
            "(async()=>{const r=await navigator.serviceWorker.ready;await r.update();"
            "return {scope:r.scope,updateViaCache:r.updateViaCache,worker:r.active?.scriptURL}})()",
            log_dir=out, steps=steps)
        report["root_update_request"] = update
        deadline = time.monotonic() + 45
        updated_names = []
        while time.monotonic() < deadline:
            updated_names = cache_names(session, log_dir=out, steps=steps)
            if (report["scope_cache_prefixes"]["/"] + update_version) in updated_names \
                    and root_name not in updated_names:
                break
            time.sleep(0.2)
        new_root_name = report["scope_cache_prefixes"]["/"] + update_version
        if new_root_name not in updated_names or root_name in updated_names:
            raise RuntimeError(f"root scoped update did not replace its release cache: {updated_names}")
        if sub_name not in updated_names:
            raise RuntimeError(f"root update deleted the subpath release cache: {updated_names}")
        preserved = eval_json(session,
            f"(async()=>{{const c=await caches.open({json.dumps(sentinel_cache)});"
            "return {sentinel:await (await c.match(location.origin+'/__qa/sentinel'))?.text(),keys:await caches.keys()}})()",
            log_dir=out, steps=steps)
        if preserved["sentinel"] != "preserve-me":
            raise RuntimeError("root update removed or damaged the unrelated cache")
        report["root_update_result"] = {"new_root_cache": new_root_name,
            "old_root_cache_removed": root_name not in updated_names,
            "subpath_cache_preserved": sub_name in updated_names,
            "unrelated_cache_preserved": preserved["sentinel"] == "preserve-me",
            "cache_names": updated_names}

        # Publicly reopen each path offline after the root update; both are in the
        # same profile and have independently scoped controlling registrations.
        browser_call(session, "set", "offline", "on", log_dir=out, steps=steps)
        report["subpath_offline"] = open_workspace(session, origin + "/boardstudio/", "/boardstudio/",
                                                   log_dir=out, steps=steps)
        if report["subpath_offline"]["online"] is not False:
            raise RuntimeError("subpath reopened without Chromium's offline state")
        report["root_offline"] = open_workspace(session, origin + "/", "/", log_dir=out, steps=steps)
        if report["root_offline"]["online"] is not False:
            raise RuntimeError("root reopened without Chromium's offline state")
        offline_names = cache_names(session, log_dir=out, steps=steps)
        if new_root_name not in offline_names or sub_name not in offline_names:
            raise RuntimeError(f"offline reopens lost their scoped caches: {offline_names}")
        report["offline_cache_names"] = offline_names
        browser_call(session, "set", "offline", "off", log_dir=out, steps=steps)

        # Trigger one actual lazy module request through the public Parts tab.
        # Remove only that one URL from this isolated root cache and have this
        # test server return one 503. Restore the real build bytes and use the
        # actual UI/workspace reload path to prove the import recovers.
        browser_call(session, "open", origin + "/", log_dir=out, steps=steps)
        wait_selector(session, ".m1-canvas", log_dir=out, steps=steps)
        evicted = eval_json(session,
            f"(async()=>{{const names=await caches.keys();const n=names.find(x=>x==={json.dumps(new_root_name)});"
            "if(!n) throw new Error('updated root cache is absent');const c=await caches.open(n);"
            f"const url=new URL({json.dumps(LAZY_ASSET)},location.origin).href;"
            "return {cache:n,url,deleted:await c.delete(url)}})()",
            log_dir=out, steps=steps)
        if not evicted["deleted"]:
            raise RuntimeError("real lazy-module entry was not present in the updated root cache")
        qa.root_lazy_fault = True
        browser_call(session, "click", "#m1-tab-Parts", log_dir=out, steps=steps)
        wait_selector(session, ".m1-parts-load-error[role=alert]", log_dir=out, steps=steps)
        fault_alert = eval_json(session,
            "({text:document.querySelector('.m1-parts-load-error[role=alert]')?.textContent.trim(),"
            "workspace:document.querySelector('[role=tab][aria-selected=true]')?.textContent.trim()})",
            log_dir=out, steps=steps)
        if fault_alert["workspace"] != "Parts" or "could not be loaded" not in (fault_alert["text"] or ""):
            raise RuntimeError(f"public Parts route did not surface lazy asset failure: {fault_alert}")
        qa.root_lazy_fault = False
        browser_call(session, "reload", log_dir=out, steps=steps)
        wait_selector(session, ".m1-canvas", log_dir=out, steps=steps)
        browser_call(session, "click", "#m1-tab-Parts", log_dir=out, steps=steps)
        wait_selector(session, ".m1-parts-catalogue-choice", log_dir=out, steps=steps)
        recovered = eval_json(session,
            "({workspace:document.querySelector('[role=tab][aria-selected=true]')?.textContent.trim(),"
            "choiceCount:document.querySelectorAll('.m1-parts-catalogue-choice').length,"
            "alert:document.querySelector('.m1-parts-load-error[role=alert]')?.textContent.trim()||null})",
            log_dir=out, steps=steps)
        if recovered["workspace"] != "Parts" or recovered["choiceCount"] < 1 or recovered["alert"]:
            raise RuntimeError(f"Parts did not recover after the real lazy module was restored: {recovered}")
        report["lazy_module_failure"] = {"cache_eviction": evicted, "public_failure": fault_alert,
                                           "public_recovery_after_server_restore_reload": recovered}
        # Verify observed server responses rather than infer them from UI state.
        with qa.lock:
            lazy_rows = [row for row in qa.requests if row["path"] == "/" + LAZY_ASSET]
        if not any(row["status"] == 503 and row.get("fault") for row in lazy_rows):
            raise RuntimeError(f"server did not record the controlled lazy failure: {lazy_rows}")
        if not any(row["status"] == 200 for row in lazy_rows):
            raise RuntimeError(f"restored public lazy request did not receive the real candidate asset: {lazy_rows}")
        report["lazy_asset_http_observations"] = lazy_rows

        root_tree_after = tree_digest(root_site, provenance["root"]["assets"])
        sub_tree_after = tree_digest(sub_site, provenance["subpath"]["assets"])
        if root_tree_after != root_tree_before or sub_tree_after != sub_tree_before:
            raise RuntimeError("immutable candidate asset tree changed during QA")
        if provenance_path.read_bytes() != provenance_bytes_before:
            raise RuntimeError("immutable candidate provenance changed during QA")
        report["immutable_build_unchanged"] = {"root_tree_sha256_before": root_tree_before,
            "root_tree_sha256_after": root_tree_after, "subpath_tree_sha256_before": sub_tree_before,
            "subpath_tree_sha256_after": sub_tree_after, "provenance_sha256": sha256(provenance_path)}
        report["claims"] = [
            "Real candidate root and /boardstudio/ service workers installed in one isolated browser profile and controlled their own scopes.",
            "Production Rust service-worker update replaced only the root cache; subpath and unrelated same-origin cache survived.",
            "Both route shells reopened through their respective workers while Chromium offline mode was enabled.",
            "The real Parts UI requested the actual lazy layout-generator module; a scoped test 503 produced the visible error and restoring the candidate asset allowed reload/re-entry recovery.",
            "The immutable root/subpath asset trees and provenance were byte/hash unchanged by the run."
        ]
        report["status"] = "passed"
        report["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
        save_json(out / "report.json", report)
        save_json(out / "commands.json", commands)
        return 0
    except Exception as error:
        report["status"] = "failed_or_incomplete"
        report["error"] = f"{type(error).__name__}: {error}"
        report["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
        save_json(out / "report.json", report)
        save_json(out / "commands.json", commands)
        print(report["error"], file=sys.stderr)
        return 1
    finally:
        if session:
            try:
                subprocess.run(["agent-browser", "--session", session, "close", "--json"],
                               cwd=REPO, text=True, capture_output=True, timeout=10)
            except Exception:
                pass
        if server is not None:
            server.shutdown()
            server.server_close()
        if thread is not None:
            thread.join(timeout=3)
        # The server request log is append-only; retain it on both pass and fail.


if __name__ == "__main__":
    raise SystemExit(main())

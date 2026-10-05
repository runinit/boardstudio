#!/usr/bin/env python3
"""Run only the F9.4 lazy-module failure/recovery probe against a final build."""
from __future__ import annotations

import argparse
import datetime as dt
import importlib.util
import json
from pathlib import Path
import secrets
import sys
import threading
import time

HERE = Path(__file__).resolve().parent
HARNESS_PATH = HERE / "offline-qualification.py"
spec = importlib.util.spec_from_file_location("f94_offline_qualification", HARNESS_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"could not load shared F9.4 server/browser helpers: {HARNESS_PATH}")
harness = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = harness
spec.loader.exec_module(harness)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("final_build", type=Path)
    parser.add_argument("port", type=int, help="local QA port; 0 chooses an ephemeral port")
    parser.add_argument("output_dir", type=Path, help="new directory outside the immutable build")
    parser.add_argument("--hold-on-recovery-timeout", type=int, default=0,
                        help="keep the named browser and local server alive this many seconds after a captured recovery timeout")
    args = parser.parse_args()
    build, out = args.final_build.resolve(), args.output_dir.resolve()
    if not (build / "provenance.json").is_file():
        parser.error("final_build must contain provenance.json")
    if not (0 <= args.port <= 65535):
        parser.error("port must be 0..65535")
    if out == build or build in out.parents:
        parser.error("output_dir must be outside the immutable build")
    if out.exists() and any(out.iterdir()):
        parser.error("output_dir must be new or empty")
    out.mkdir(parents=True, exist_ok=True)

    provenance_path = build / "provenance.json"
    provenance_bytes = provenance_path.read_bytes()
    provenance = json.loads(provenance_bytes)
    root_site = Path(provenance["root"]["site"]).resolve()
    manifest_path = build / "offline-manifest-root.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    tree_before = harness.tree_digest(root_site, provenance["root"]["assets"])
    session = "f94-lazy-" + secrets.token_hex(6)
    run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + secrets.token_hex(3)
    steps: list[dict] = []
    report = {
        "run_id": run_id,
        "status": "running",
        "build_id": provenance.get("build_id"),
        "source_commit": provenance.get("source_commit"),
        "build_dir": str(build),
        "root_site": str(root_site),
        "root_manifest_version": manifest["version"],
        "root_tree_sha256_before": tree_before,
        "provenance_sha256": harness.sha256(provenance_path),
        "browser_session": session,
        "probe": "evict actual lazy module; enable controlled server 503; reload before opening Parts; restore serving; reload and re-enter Parts",
        "claims": [],
        "limitations": [
            "Uses a uniquely named agent-browser session and never controls the selected in-app browser.",
            "The one-route HTTP 503 is a controlled missing-network-response simulation, not an assertion that offline emulation caused the failure.",
            "This is only the remaining lazy-module failure/recovery clause; root/subpath install, update, offline and cache-isolation results remain in the original run receipt.",
        ],
    }
    harness.save_json(out / "report.json", report)
    server = thread = None
    hold_browser = False
    try:
        # This probe needs the candidate's installed root worker but no update
        # worker. Its unused overlay paths stay inside the output directory.
        qa = harness.QAState(root_site, root_site, out, out)
        server = harness.Server(("127.0.0.1", args.port), harness.Handler)
        server.qa = qa
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        origin = f"http://127.0.0.1:{server.server_address[1]}"
        report["origin"] = origin
        harness.save_json(out / "report.json", report)

        installed = harness.open_workspace(session, origin + "/", "/", log_dir=out, steps=steps)
        report["installed_root"] = installed
        cache_name = "boardstudio-m1-offline-path-2f-" + manifest["version"]
        evicted = harness.eval_json(
            session,
            f"(async()=>{{const c=await caches.open({json.dumps(cache_name)});"
            f"const url=new URL({json.dumps(harness.LAZY_ASSET)},location.origin).href;"
            "return {cache:" + json.dumps(cache_name) + ",url,deleted:await c.delete(url)}})()",
            log_dir=out,
            steps=steps,
        )
        report["cache_eviction"] = evicted
        if not evicted["deleted"]:
            raise RuntimeError(f"candidate lazy asset was not in installed root cache: {evicted}")

        # Reload before entering Parts: the document that may have imported the
        # module is discarded, so the next UI route resolves from the evicted cache.
        qa.root_lazy_fault = True
        harness.browser_call(session, "reload", log_dir=out, steps=steps)
        harness.wait_selector(session, ".m1-canvas", log_dir=out, steps=steps)
        harness.browser_call(session, "click", "#m1-tab-Parts", log_dir=out, steps=steps)
        harness.wait_selector(session, ".m1-parts-load-error[role=alert]", log_dir=out, steps=steps)
        failure = harness.eval_json(
            session,
            "({workspace:document.querySelector('[role=tab][aria-selected=true]')?.textContent.trim(),"
            "alert:document.querySelector('.m1-parts-load-error[role=alert]')?.textContent.trim()})",
            log_dir=out,
            steps=steps,
        )
        if failure["workspace"] != "Parts" or "could not be loaded" not in (failure["alert"] or ""):
            raise RuntimeError(f"Parts did not display the lazy-module failure: {failure}")
        with qa.lock:
            failed_rows = [row for row in qa.requests if row["path"] == "/" + harness.LAZY_ASSET]
        if not any(row["status"] == 503 and row.get("fault") for row in failed_rows):
            raise RuntimeError(f"controlled lazy-module 503 was not observed: {failed_rows}")
        report["public_failure"] = failure
        report["failure_http_observations"] = failed_rows

        qa.root_lazy_fault = False
        harness.browser_call(session, "reload", log_dir=out, steps=steps)
        harness.wait_selector(session, ".m1-canvas", log_dir=out, steps=steps)
        harness.browser_call(session, "click", "#m1-tab-Parts", log_dir=out, steps=steps)
        try:
            harness.wait_expr(
                session,
                "document.querySelector('.m1-parts-catalogue-choice') || "
                "document.querySelector('.m1-parts-load-error[role=alert]') || "
                "document.querySelector('.m1-parts-preview')",
                log_dir=out,
                steps=steps,
            )
        except Exception:
            report["recovery_diagnostic"] = harness.eval_json(
                session,
                "({url:location.href,workspace:document.querySelector('[role=tab][aria-selected=true]')?.textContent.trim(),"
                "choices:[...document.querySelectorAll('.m1-parts-catalogue-choice')].map(e=>({text:e.textContent.trim(),visible:!!e.getClientRects().length})),"
                "options:[...document.querySelectorAll('[role=option]')].map(e=>({text:e.textContent.trim(),visible:!!e.getClientRects().length})),"
                "alert:[...document.querySelectorAll('.m1-parts-load-error[role=alert]')].map(e=>e.textContent.trim()),"
                "loading:[...document.querySelectorAll('.m1-parts-loading[role=status]')].map(e=>e.textContent.trim()),"
                "library:document.querySelector('.m1-parts-library')?.innerText.slice(0,900)||null,"
                "preview:document.querySelector('.m1-parts-preview')?.innerText.slice(0,500)||null})",
                log_dir=out,
                steps=steps,
            )
            report["recovery_accessibility_snapshot"] = harness.browser_call(
                session, "snapshot", "-i", log_dir=out, steps=steps
            )
            with qa.lock:
                report["recovery_request_log"] = [row for row in qa.requests
                                                  if row["path"] == "/" + harness.LAZY_ASSET]
            hold_browser = args.hold_on_recovery_timeout > 0
            raise
        recovery = harness.eval_json(
            session,
            "({workspace:document.querySelector('[role=tab][aria-selected=true]')?.textContent.trim(),"
            "choices:document.querySelectorAll('.m1-parts-catalogue-choice').length,"
            "alert:document.querySelector('.m1-parts-load-error[role=alert]')?.textContent.trim()||null,"
            "setupDismiss:!!document.querySelector('.m1-setup-guide__dismiss'),"
            "library:document.querySelector('.m1-parts-library')?.innerText.slice(0,600)||null,"
            "preview:document.querySelector('.m1-parts-preview')?.innerText.slice(0,300)||null})",
            log_dir=out,
            steps=steps,
        )
        if recovery["workspace"] == "Parts" and not recovery["alert"] and recovery["choices"] < 1 \
                and recovery["preview"] and recovery["setupDismiss"]:
            # The successful preview appears in setup-guide mode, where the
            # guide intentionally replaces the Objects library. Follow its
            # visible Back to objects action before asserting catalogue choices.
            harness.browser_call(session, "click", ".m1-setup-guide__dismiss", log_dir=out, steps=steps)
            harness.wait_selector(session, ".m1-parts-catalogue-choice", log_dir=out, steps=steps)
            recovery = harness.eval_json(
                session,
                "({workspace:document.querySelector('[role=tab][aria-selected=true]')?.textContent.trim(),"
                "choices:document.querySelectorAll('.m1-parts-catalogue-choice').length,"
                "visibleChoices:[...document.querySelectorAll('.m1-parts-catalogue-choice')].filter(e=>e.getClientRects().length>0).length,"
                "search:!![...document.querySelectorAll('input')].find(e=>e.getAttribute('aria-label')==='Search footprints'),"
                "library:document.querySelector('.m1-parts-library')?.innerText.slice(0,600)||null,"
                "alert:document.querySelector('.m1-parts-load-error[role=alert]')?.textContent.trim()||null})",
                log_dir=out,
                steps=steps,
            )
            report["public_setup_guide_dismissal"] = "Back to objects"
        if recovery["workspace"] != "Parts" or recovery["choices"] < 1 or recovery["alert"]:
            report["recovery_diagnostic"] = recovery
            report["recovery_accessibility_snapshot"] = harness.browser_call(
                session, "snapshot", "-i", log_dir=out, steps=steps
            )
            raise RuntimeError(f"Parts did not recover after the module response was restored: {recovery}")
        with qa.lock:
            all_rows = [row for row in qa.requests if row["path"] == "/" + harness.LAZY_ASSET]
        if not any(row["status"] == 200 for row in all_rows):
            raise RuntimeError(f"restored candidate module did not return HTTP 200: {all_rows}")
        report["public_recovery_after_restore_reload"] = recovery
        report["lazy_asset_http_observations"] = all_rows

        tree_after = harness.tree_digest(root_site, provenance["root"]["assets"])
        if tree_after != tree_before or provenance_path.read_bytes() != provenance_bytes:
            raise RuntimeError("immutable candidate root tree or provenance changed during probe")
        report["root_tree_sha256_after"] = tree_after
        report["immutable_build_unchanged"] = True
        report["claims"] = [
            "After deleting the actual layout-generator module from the installed root cache, reloading before opening Parts produced a recorded controlled HTTP 503 and the visible catalogue error.",
            "After restoring candidate serving and reloading before re-entering Parts, catalogue choices appeared and the server recorded HTTP 200.",
            "The immutable root asset tree and provenance remained unchanged.",
        ]
        report["status"] = "passed"
        report["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
        harness.save_json(out / "report.json", report)
        return 0
    except Exception as error:
        report["status"] = "failed_or_incomplete"
        report["error"] = f"{type(error).__name__}: {error}"
        report["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
        harness.save_json(out / "report.json", report)
        print(report["error"], file=sys.stderr)
        return 1
    finally:
        if hold_browser:
            print(json.dumps({"diagnostic_hold_seconds": args.hold_on_recovery_timeout,
                              "browser_session": session, "origin": report.get("origin"),
                              "output_dir": str(out)}, sort_keys=True), flush=True)
            time.sleep(args.hold_on_recovery_timeout)
        try:
            import subprocess
            subprocess.run(["agent-browser", "--session", session, "close", "--json"],
                           cwd=harness.REPO, text=True, capture_output=True, timeout=10)
        except Exception:
            pass
        if server is not None:
            server.shutdown()
            server.server_close()
        if thread is not None:
            thread.join(timeout=3)


if __name__ == "__main__":
    raise SystemExit(main())

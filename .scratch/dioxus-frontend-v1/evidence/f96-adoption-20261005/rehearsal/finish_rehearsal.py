#!/usr/bin/env python3
"""Finish rollback export/readback and candidate readoption from the live React phase."""
from __future__ import annotations
import argparse, datetime as dt, importlib, json, urllib.request
from pathlib import Path
import urllib.parse, time

base = importlib.import_module("continue_rehearsal")

def save_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--session", required=True)
    parser.add_argument("--origin", required=True)
    parser.add_argument("--candidate-upstream", required=True)
    parser.add_argument("--candidate-archive", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    output = args.output_dir.resolve(); output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()): raise SystemExit(f"output directory is not empty: {output}")
    origin = args.origin.rstrip("/") + "/"
    input_archive = args.candidate_archive.resolve()
    with __import__("zipfile").ZipFile(input_archive) as z:
        input_doc = json.loads(z.read("project.json"))
    project_id = input_doc["id"]
    steps, phases = [], []
    report = {"status":"running-from-react-export-boundary", "session":args.session,
              "origin":origin, "candidate_upstream":args.candidate_upstream,
              "input_archive":base.archive_info(input_archive),
              "prior_run":"run-worker-handoff-resumed3-20261005 stopped only because the public save menu remained closed after same-ID import",
              "menu_policy":"inspect aria-expanded and open only if closed; no upload replay before export"}
    failure = None
    def evaluate(expr): return base.evaluate(args.session, steps, output, expr)
    def wait(expr, timeout=120000): return base.wait_for(args.session, steps, output, expr, timeout)
    def browser(*args2): return base.browser(args.session, steps, output, *args2)
    try:
        state = evaluate("({url:location.href,title:document.title,react:!!document.querySelector('.wb-project-trigger'),controller:navigator.serviceWorker.controller?.scriptURL??null,triggerExpanded:document.querySelector('.wb-project-trigger')?.getAttribute('aria-expanded')??null,reactActive:localStorage.getItem('boardstudio-v2-active-project'),m1Active:localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root'),registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null})),databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys()})")
        report["resume_state"] = state
        if (state["url"] != origin or not state["react"] or not (state["controller"] or "").endswith("/sw.js") or state["reactActive"] != project_id):
            raise RuntimeError(f"not at the expected stable React rollback state: {state}")
        trigger = "document.querySelector('.wb-project-trigger')"
        if state["triggerExpanded"] != "true":
            browser("click", ".wb-project-trigger")
            wait(f"{trigger} && {trigger}.getAttribute('aria-expanded') === 'true'")
        menu = evaluate("({expanded:document.querySelector('.wb-project-trigger')?.getAttribute('aria-expanded')??null,copyAction:!!document.querySelector('button[title=\"Save project copy…\"]')})")
        phases.append({"react_save_menu_open":menu})
        if menu["expanded"] != "true" or not menu["copyAction"]:
            raise RuntimeError(f"public React save menu did not open: {menu}")

        rollback_archive = output / "reference-copy-from-candidate.boardstudio"
        browser("download", "button[title='Save project copy…']", str(rollback_archive))
        comparison = base.compare_project_archives(input_archive, rollback_archive)
        rollback_info = base.archive_info(rollback_archive)
        if (rollback_info["document"]["id"] != project_id
                or not comparison["project_document_within_accepted_numeric_tolerance"]
                or not comparison["asset_paths_equal"] or not comparison["asset_hashes_equal"]):
            raise RuntimeError(f"rollback archive copy differs: {comparison}")
        wait("document.querySelector('.wb-save-state.is-saved')")
        browser("reload")
        wait(f"document.querySelector('.wb-project-trigger') && document.querySelector('.wb-save-state.is-saved') && localStorage.getItem('boardstudio-v2-active-project') === {json.dumps(project_id)}")
        phases.append({"rollback_public_archive_export_and_reload":rollback_info,
                       "comparison":comparison,
                       "readback":evaluate("({title:document.title,projectName:document.querySelector('.wb-project-name')?.textContent??null,active:localStorage.getItem('boardstudio-v2-active-project'),controller:navigator.serviceWorker.controller?.scriptURL??null})")})

        switch = urllib.request.Request(origin + "__rehearsal__/switch/candidate", method="POST", data=b"")
        with urllib.request.urlopen(switch, timeout=5) as r: phases.append({"switch":json.loads(r.read())})
        browser("reload")
        wait("document.querySelector('.m1-canvas') || document.querySelector('.m1-library-landing')")
        wait("navigator.serviceWorker.controller && new URL(navigator.serviceWorker.controller.scriptURL).pathname.replace(/\\/$/, '') === '/service-worker.js'")
        deadline=time.monotonic()+120; settled=None
        while time.monotonic()<deadline:
            settled=evaluate("(async()=>{const rs=await navigator.serviceWorker.getRegistrations();const r=rs.find(x=>x.scope==="+json.dumps(origin)+");return r?{active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null}:null})()")
            if settled and (settled["active"] or "").endswith("/service-worker.js") and not settled["waiting"] and not settled["installing"]: break
            time.sleep(1)
        else: raise RuntimeError(f"M1 worker did not settle: {settled}")
        phases.append({"candidate_readoption_shell_worker":{"shell":True,"registration":settled,
                         "controller":evaluate("navigator.serviceWorker.controller?.scriptURL??null")}})

        has_input=evaluate("Boolean(document.querySelector('.m1-library-landing input[type=file][accept=\".boardstudio\"]'))")
        if not has_input: browser("click","details.m1-project-menu > summary")
        browser("upload","input[type='file'][accept='.boardstudio']",str(rollback_archive))
        wait(f"document.querySelector('.m1-project-menu') && localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root') === {json.dumps(project_id)}")
        wait("document.querySelector('.m1-save-state[data-state=\"saved\"]')")
        browser("click","details.m1-project-menu > summary")
        candidate_copy=output/"candidate-readopted-copy.boardstudio"
        browser("download","button.m1-project-copy-action",str(candidate_copy))
        final_comparison=base.compare_project_archives(input_archive,candidate_copy)
        final_info=base.archive_info(candidate_copy)
        if (final_info["document"]["id"] != project_id
                or not final_comparison["project_document_within_accepted_numeric_tolerance"]
                or not final_comparison["asset_paths_equal"] or not final_comparison["asset_hashes_equal"]):
            raise RuntimeError(f"final candidate archive differs: {final_comparison}")
        browser("reload")
        wait(f"document.querySelector('.m1-project-menu') && document.querySelector('.m1-save-state[data-state=\"saved\"]') && localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root') === {json.dumps(project_id)}")
        final_state=evaluate("({title:document.title,projectName:document.querySelector('.m1-project-name')?.textContent??null,active:localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root'),controller:navigator.serviceWorker.controller?.scriptURL??null,databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys()})")
        if final_state["projectName"] != input_doc.get("name"):
            raise RuntimeError(f"candidate project name did not read back: {final_state}")
        phases.append({"final_candidate_public_import_save_reload":final_info,
                       "comparison":final_comparison,"readback":final_state})
        report["status"]="completed-same-origin-cutover-rollback-readoption"
    except Exception as exc:
        failure=repr(exc); report["status"]="stopped-at-first-failed-boundary"; report["error"]=failure
        try:
            report["failure_state"]=evaluate("({url:location.href,title:document.title,controller:navigator.serviceWorker.controller?.scriptURL??null,react:!!document.querySelector('.wb-project-trigger'),m1:!!document.querySelector('.m1-project-menu'),reactActive:localStorage.getItem('boardstudio-v2-active-project'),m1Active:localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root'),registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null})),databases:(await indexedDB.databases()).map(x=>x.name),cacheNames:await caches.keys()})")
        except Exception as diag: report["failure_state_error"]=repr(diag)
    report["phases"]=phases
    report["browser_session_closed"]=False
    save_json(output/"report.json",report)
    return 1 if failure else 0

if __name__ == "__main__": raise SystemExit(main())

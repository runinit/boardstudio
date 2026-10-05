#!/usr/bin/env python3
"""Paired 1280x500 public control-reachability evidence; no edits/downloads/screenshots."""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
from pathlib import Path
import secrets
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[4]
WORKSPACES = ["Layout", "Parts", "PCB", "Keymap", "Keycaps", "Case", "Export"]


class Browser:
    def __init__(self, name: str, url: str, side: str, output: Path):
        self.name, self.url, self.side, self.output = name, url, side, output
        self.steps: list[dict] = []
        self.session = "f92-short-" + name + "-" + secrets.token_hex(4)

    def call(self, *args: str):
        argv = ["agent-browser", "--session", self.session, *args, "--json"]
        result = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=90)
        row = {"argv": argv, "exit": result.returncode, "stdout": result.stdout,
               "stderr": result.stderr, "time_utc": dt.datetime.now(dt.timezone.utc).isoformat()}
        self.steps.append(row)
        (self.output / f"{self.name}-steps.json").write_text(
            json.dumps(self.steps, indent=2) + "\n", encoding="utf-8")
        if result.returncode:
            raise RuntimeError(f"{self.name}: agent-browser {args} failed: {result.stderr[-800:]}")
        response = json.loads(result.stdout)
        if not response.get("success", True):
            raise RuntimeError(f"{self.name}: agent-browser {args}: {response}")
        return response.get("data", response)

    def evaluate(self, expression: str):
        data = self.call("eval", f"JSON.stringify({expression})")
        value = data.get("result", data)
        return json.loads(value) if isinstance(value, str) else value

    def wait(self, expression: str):
        self.call("wait", "--fn", expression)

    def snapshot(self, label: str):
        value = self.call("snapshot", "-i")
        (self.output / f"{self.name}-{label}-snapshot.json").write_text(
            json.dumps(value, indent=2) + "\n", encoding="utf-8")
        return value

    def load_fixture(self, fixture: Path):
        self.call("open", self.url)
        self.call("set", "viewport", "1280", "500")
        if self.side == "candidate":
            self.wait('Boolean(document.querySelector(\'input[type="file"][accept=".boardstudio"]\'))')
            self.call("upload", 'input[type="file"][accept=".boardstudio"]', str(fixture))
            self.wait('Boolean(document.querySelector("button[aria-label=\'Expand keys\']"))')
        else:
            self.wait('Boolean(document.querySelector(".wb-project-trigger") || document.querySelector(".wb-open-project"))')
            if self.evaluate('Boolean(document.querySelector(".wb-project-trigger"))'):
                self.call("click", ".wb-project-trigger")
            self.call("click", ".wb-open-project")
            self.call("upload", 'input[type="file"].wb-project-file-input', str(fixture))
            self.wait('Boolean(document.querySelector(".wb-canvas") && [...document.querySelectorAll("button,[role=button]")].some(e=>(e.getAttribute("aria-label")||e.textContent||"").includes("J1")))')
        state = self.evaluate("({url:location.href,width:innerWidth,height:innerHeight,"
                              "tabs:[...document.querySelectorAll('[role=tab]')].map(e=>({name:e.textContent.trim(),selected:e.getAttribute('aria-selected')})),"
                              "title:document.title})")
        return state

    def workspace(self, name: str):
        if name == "Export":
            if self.side == "candidate":
                self.call("click", "#m1-tab-Export")
                self.wait('document.querySelector("#m1-tab-Export")?.getAttribute("aria-pressed")==="true"')
            else:
                self.call("find", "role", "button", "click", "--name", "Export", "--exact")
                self.wait('document.querySelector(".wb-export-trigger")?.getAttribute("aria-pressed")==="true"')
        else:
            self.call("find", "role", "tab", "click", "--name", name, "--exact")
            self.wait(f"[...document.querySelectorAll('[role=tab]')].some(e=>e.textContent.trim()==={json.dumps(name)} && e.getAttribute('aria-selected')==='true')")
        guide_dismissed = False
        if name == "Parts":
            parts = self.evaluate("({search:!!document.querySelector('input[aria-label=\"Search footprints\"]'),"
                                  "back:[...document.querySelectorAll('button')].some(e=>e.textContent.trim()==='Back to objects')})")
            if not parts["search"] and parts["back"]:
                self.call("find", "role", "button", "click", "--name", "Back to objects", "--exact")
                self.wait("Boolean(document.querySelector('input[aria-label=\"Search footprints\"]'))")
                guide_dismissed = True
        result = self.inspect(name)
        result["public_setup_guide_dismissal"] = guide_dismissed
        return result

    def inspect(self, name: str):
        state = self.evaluate(
            "({url:location.href,width:innerWidth,height:innerHeight,"
            "workspace:document.querySelector('#m1-tab-Export[aria-pressed=true],.wb-export-trigger[aria-pressed=true]')?'Export':document.querySelector('[role=tab][aria-selected=true]')?.textContent.trim(),"
            "controls:[...document.querySelectorAll('button,input,select,textarea,[role=button],[role=option]')].map(e=>{"
            "const r=e.getBoundingClientRect(),s=getComputedStyle(e),cx=r.left+r.width/2,cy=r.top+r.height/2,"
            "inViewport=cx>=0&&cx<innerWidth&&cy>=0&&cy<innerHeight,h=inViewport?document.elementFromPoint(cx,cy):null;"
            "const label=e.getAttribute('aria-label')||e.getAttribute('title')||e.labels?.[0]?.innerText?.trim()||e.textContent?.trim()||'';"
            "const visible=!!e.getClientRects().length&&s.display!=='none'&&s.visibility!=='hidden'&&r.width>0&&r.height>0;"
            "return {tag:e.tagName.toLowerCase(),role:e.getAttribute('role'),type:e.getAttribute('type'),id:e.id||null,"
            "className:typeof e.className==='string'?e.className:null,label:label.replace(/\\s+/g,' ').slice(0,120),"
            "bounds:{x:r.x,y:r.y,width:r.width,height:r.height},visible,disabled:!!e.disabled,"
            "inViewport,centerHit:visible&&inViewport&&(h===e||e.contains(h))};})"
            ".filter(c=>c.inViewport)})")
        snapshot = self.snapshot(name.lower())
        return {"state": state, "snapshot_file": f"{self.name}-{name.lower()}-snapshot.json",
                "snapshot_success": snapshot.get("success", True)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate_url", default="http://127.0.0.1:34829/", nargs="?")
    parser.add_argument("reference_url", default="http://127.0.0.1:5175/", nargs="?")
    parser.add_argument("fixture", type=Path, default=Path(
        ".scratch/dioxus-frontend-v1/evidence/f86-integrated-20261005/fixed-candidate/candidate-joined.boardstudio"), nargs="?")
    parser.add_argument("output_dir", type=Path, default=Path(
        ".scratch/dioxus-frontend-v1/evidence/f92-short-height-20261005/run-20261005"), nargs="?")
    args = parser.parse_args()
    fixture = args.fixture.resolve()
    output = args.output_dir.resolve()
    if not fixture.is_file():
        parser.error(f"fixture missing: {fixture}")
    if output.exists() and any(output.iterdir()):
        parser.error(f"output directory must be new or empty: {output}")
    output.mkdir(parents=True, exist_ok=True)
    report = {"status": "incomplete", "candidate_url": args.candidate_url,
              "reference_url": args.reference_url, "fixture": str(fixture),
              "fixture_sha256": hashlib.sha256(fixture.read_bytes()).hexdigest(),
              "candidate_build_id": "frontend-layout-untouched-coordinate-20261005",
              "candidate_source_commit": "df6b04b9e8903381029a137fdfcef8d797305fb8",
              "reference_source_commit": "5a472a9426e6e38993361da402cd4ec730feb369",
              "viewport_css_px": {"width": 1280, "height": 500},
              "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
              "limitations": ["Reachability only: no controls are activated except workflow tabs and public setup-guide dismissal if needed.",
                              "No model edits, undo/redo, export/download, screenshots, or browser zoom."]}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    browsers = [Browser("candidate", args.candidate_url, "candidate", output),
                Browser("reference", args.reference_url, "reference", output)]
    try:
        for browser in browsers:
            report[f"{browser.name}_session"] = browser.session
            report[f"{browser.name}_baseline"] = browser.load_fixture(fixture)
            report[f"{browser.name}_workspaces"] = {}
            for name in WORKSPACES:
                report[f"{browser.name}_workspaces"][name] = browser.workspace(name)
            with (output / "report.json").open("w", encoding="utf-8") as stream:
                json.dump(report, stream, indent=2)
                stream.write("\n")
        report["status"] = "observed"
        report["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
        (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        return 0
    except Exception as error:
        report["status"] = "failed_or_incomplete"
        report["error"] = f"{type(error).__name__}: {error}"
        report["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
        (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(report["error"], file=sys.stderr)
        return 1
    finally:
        for browser in browsers:
            try:
                browser.call("close")
            except Exception:
                pass


if __name__ == "__main__":
    raise SystemExit(main())

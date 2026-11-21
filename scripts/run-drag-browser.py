#!/usr/bin/env python3
"""Exercise packaged BoardStudio drags, Undo and Case collision findings in Chrome."""
from __future__ import annotations

import argparse
import importlib
from pathlib import Path
import re
import sys
import time
from urllib.parse import urlsplit

import browser_tools

SITE = Path(__file__).resolve().parents[1] / "web/target/site"
CONTROL = "\ue009"
ESCAPE = "\ue00c"
ARROW_LEFT = "\ue012"
SAVED = "document.querySelector('.header-save')?.dataset.state === 'saved'"
SCENE = """[...document.querySelectorAll('[data-geometry="scene-part"]')]
    .map(el => [el.dataset.partId, el.getAttribute('transform')]).sort()"""
PART = '[data-geometry="scene-part"][data-part-id^="matrix/"][data-part-id$="/r1c2"]'
HIT_AREA = PART + ' [data-geometry="part-hit-area"]'
ROWS = '[role="slider"][aria-label="Rows"]'
FINDINGS = ('document.querySelector("[data-case-collision-findings]") ?? '
            """document.querySelector('[data-slot="collapsible"]:has(> [aria-label="Collision check"])')""")
VIEWER = ('document.querySelector("[data-case-viewer-canvas]") ?? '
          """document.querySelector('canvas[aria-label^="Interactive 3D Case preview"]')""")
PCB_PAIR = re.compile(r'\[(?:plate|bottom)[^\]]*\] ↔ PCB \[pcb-reference\].*'
                      r'\(surface (?:intersection (?:near|observed)|separation [0-9])')
READ_PROJECT = """
const done = arguments[0];
const request = indexedDB.open('boardstudio-m1-boardstudio');
request.onerror = () => done({error: String(request.error)});
request.onblocked = () => done({error: 'IndexedDB project read blocked'});
request.onsuccess = () => {
    const db = request.result;
    try {
        const query = db.transaction('projects').objectStore('projects').getAll();
        query.onsuccess = () => { db.close(); done({projects: query.result}); };
        query.onerror = () => { db.close(); done({error: String(query.error)}); };
    } catch (error) {
        db.close();
        done({error: String(error)});
    }
};
"""


def handler_for(site, served):
    base = importlib.import_module("serve-web").handler_for(site)

    class Handler(base):
        def send_head(self):
            path = urlsplit(self.path).path
            if path == "/boardstudio/service-worker.js":
                # Keep the packaged bootstrap while bypassing its bulk precache.
                self.send_response(200)
                self.send_header("Content-Type", "text/javascript")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return None
            source = super().send_head()
            if source and self.command == "GET":
                served.add(path)
            return source

        def log_message(self, *_args):
            pass

    return Handler


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def wait_for(probe, label, timeout=20):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if result := probe():
            return result
        time.sleep(0.1)
    raise RuntimeError(f"timed out waiting for {label}")


def input_source(kind, *events):
    device = {"type": kind, "id": kind, "actions": list(events)}
    if kind == "pointer":
        device["parameters"] = {"pointerType": "mouse"}
    return device


def input_actions(browser, kind, *events):
    browser.actions({"actions": [input_source(kind, *events)]})


def move(x, y, duration=0):
    return {"type": "pointerMove", "origin": "viewport", "x": x, "y": y, "duration": duration}


DOWN = {"type": "pointerDown", "button": 0}
UP = {"type": "pointerUp", "button": 0}


def key(browser, value, *types):
    input_actions(browser, "key", *({"type": kind, "value": value} for kind in types))


class Smoke:
    def __init__(self, browser):
        self.browser = browser

    def evaluate(self, expression, *args):
        return self.browser.execute_sync(f"return ({expression});", *args)

    def button(self, name, selector='button,[role="button"]'):
        return self.click(selector, name)

    def click(self, selector, name=None):
        x, y = self.point(selector, name, scroll=True)
        input_actions(self.browser, "pointer", move(x, y), DOWN, UP)

    def select_option(self, name, option):
        self.button(name)
        self.click(f'[role="listbox"][aria-label="{name}"] [role="option"]', option)
        wait_for(lambda: self.evaluate("""(() => {
            const trigger = [...document.querySelectorAll('button')].find(el => el.getAttribute('aria-label') === arguments[0]);
            return trigger?.textContent.trim() === arguments[1] && trigger.getAttribute('aria-expanded') === 'false';
        })()""", name, option), f"{name} selected {option}")

    def point(self, selector, name=None, *, scroll=False):
        return wait_for(lambda: self.browser.execute_sync("""
            const [selector, name, scroll] = arguments;
            for (const el of document.querySelectorAll(selector)) {
                const label = el.getAttribute('aria-label') || el.textContent.trim();
                if ((name !== null && label !== name) || !el.getClientRects().length ||
                    el.matches(':disabled,[aria-disabled=true]') || getComputedStyle(el).visibility !== 'visible') continue;
                if (scroll) el.scrollIntoView({block: 'nearest', inline: 'nearest'});
                const box = el.getBoundingClientRect();
                const x = Math.round(box.x + box.width / 2), y = Math.round(box.y + box.height / 2);
                if (el.contains(document.elementFromPoint(x, y))) return [x, y];
            }
            return null;
            """, selector, name, scroll), name or selector, 60)

    def onboarding(self, url):
        self.browser.get(url)
        self.point('button,[role="button"]', "Import project")
        self.button("Create new keyboard")
        self.button("Continue to layout")
        rows = wait_for(lambda: self.evaluate("document.querySelector(arguments[0])?.getAttribute('aria-valuenow')",
                                              ROWS), "matrix rows", 60)
        # The setup form at this base starts at 5×6; move down one row when needed.
        if rows == "5":
            self.browser.execute_sync("document.querySelector(arguments[0]).focus();", ROWS)
            key(self.browser, ARROW_LEFT, "keyDown", "keyUp")
        # Choc is covered by the packaged baked STEP inventory.
        self.click('[aria-label="New matrix assembly"] button:nth-of-type(2)')
        self.button("Create matrix")
        wait_for(lambda: self.evaluate("!!document.querySelector(arguments[0])", HIT_AREA), "created matrix", 60)
        self.button("Hide guide")
        self.saved()
        require(any((m["rows"], m["columns"]) == (4, 6) for m in self.project()["matrices"]),
                "onboarding did not persist a 4×6 matrix")

    def saved(self):
        wait_for(lambda: self.evaluate(SAVED), "project save")

    def project(self):
        value = self.browser.execute_async(READ_PROJECT, timeout=10)
        require("error" not in value, value.get("error"))
        require(len(value["projects"]) == 1, "expected one project in IndexedDB")
        return value["projects"][0]

    def persisted_after(self, revision):
        project = self.project()
        return project if project["revision"] > revision and self.evaluate(SAVED) else None

    def scope(self, name):
        label = "Select " + name
        if self.evaluate("[...document.querySelectorAll('button')].some(el => "
                         "el.getAttribute('aria-label') === arguments[0] && el.getClientRects().length)", label):
            self.button(label)
        else:
            self.click("#m1-layout-select-trigger")
            self.click('#m1-layout-select-menu [role="menuitemradio"], #m1-layout-select-menu button', name)
        self.click(HIT_AREA)

    def gesture(self, name, selector, scene, *, ctrl=False, cancel=False):
        self.browser.execute_sync("""
            window.dragEvents = [];
            if (window.dragObserver) return;
            window.dragObserver = true;
            for (const type of ['pointerdown', 'pointermove', 'pointerup', 'keydown', 'keyup']) {
                document.addEventListener(type, event => window.dragEvents.push({
                    type, trusted: event.isTrusted, key: event.key, ctrl: event.ctrlKey,
                    id: event.pointerId, pointer: event.pointerType, buttons: event.buttons,
                    part: event.target.closest?.('[data-part-id]')?.dataset.partId,
                    handle: event.target.closest?.('[data-transform-handle]')?.getAttribute('aria-label')
                }), true);
            }
            """)
        if ctrl:
            key(self.browser, CONTROL, "keyDown")
        x, y = self.point(selector)
        # ChromeDriver retains pointer capture when down and movement share one action sequence.
        pause = {"type": "pause", "duration": 0}
        start = [move(x, y), DOWN]
        keys = []
        if ctrl:
            # Release Ctrl just after pointerdown so the movement tests the latched parameter.
            start.append(pause)
            keys.append(input_source("key", pause, pause, {"type": "keyUp", "value": CONTROL}))
        steps = [move(x + dx, y + dy, 40) for dx, dy in ((8, 5), (17, 10), (26, 16), (36, 22))]
        self.browser.actions({"actions": [input_source("pointer", *start, *steps), *keys]})
        wait_for(lambda: self.evaluate(SCENE) != scene, f"{name} live preview", 10)
        captured = self.evaluate("""(() => {
            const down = window.dragEvents.find(event => event.type === 'pointerdown');
            return !!down && [...document.querySelectorAll('svg,[data-transform-handle]')]
                .some(el => el.hasPointerCapture(down.id));
        })()""")
        if cancel:
            key(self.browser, ESCAPE, "keyDown", "keyUp")
        input_actions(self.browser, "pointer", UP)
        events = self.evaluate("window.dragEvents")
        require(captured, f"{name}: missing pointer capture during preview")
        down = next((event for event in events if event["type"] == "pointerdown"), {})
        moves = [event for event in events if event["type"] == "pointermove" and event["buttons"] == 1]
        require(down.get("trusted") and down.get("pointer") == "mouse" and down.get("ctrl") == ctrl,
                f"{name}: expected trusted mouse pointerdown with Ctrl={ctrl}")
        target = (self.evaluate("document.querySelector(arguments[0]).dataset.partId", PART)
                  if selector == HIT_AREA else selector.split('"')[1])
        require(down.get("part" if selector == HIT_AREA else "handle") == target,
                f"{name}: pointerdown missed {target}")
        require(len(moves) >= 2 and all(event["trusted"] and not event["ctrl"] for event in moves)
                and any(event["type"] == "pointerup" and event["trusted"] for event in events),
                f"{name}: incomplete trusted pointer gesture")
        if cancel:
            require(any(event["type"] == "keydown" and event["key"] == "Escape" and event["trusted"]
                        for event in events), f"{name}: trusted Escape was not observed")

    def commit_and_undo(self, name, selector, matrix_id, field, *, ctrl=False):
        self.saved()
        before = self.project()
        scene = self.evaluate(SCENE)
        self.gesture(name, selector, scene, ctrl=ctrl)
        after = wait_for(lambda: self.persisted_after(before["revision"]), f"{name} durable revision")
        changed = next(matrix for matrix in after["matrices"] if matrix["id"] == matrix_id)
        original = next(matrix for matrix in before["matrices"] if matrix["id"] == matrix_id)
        verify_edit(name, field, original, changed)
        require(self.evaluate(SCENE) != scene, f"{name}: saved edit did not reach the scene")
        print(f"PASS {name}", flush=True)
        self.button("Undo")
        undone = wait_for(lambda: self.persisted_after(after["revision"]), f"{name} durable Undo")
        require(without_revision(undone) == without_revision(before), f"{name}: Undo changed saved project content")
        wait_for(lambda: self.evaluate(SCENE) == scene, f"{name} Undo restoring the scene")
        print(f"PASS {name} undo", flush=True)

    def drag(self):
        self.button("Fit board")
        self.scope("Part")
        part_id = self.evaluate("document.querySelector(arguments[0]).dataset.partId", PART)
        matrix_id = part_id.removeprefix("matrix/").rsplit("/", 1)[0]
        self.commit_and_undo("Part drag", HIT_AREA, matrix_id, "cells")
        self.scope("Column")
        self.saved()
        before = self.project()
        scene = self.evaluate(SCENE)
        self.gesture("Stagger cancel", '[aria-label="Drag to stagger"]', scene, cancel=True)
        wait_for(lambda: self.evaluate(SCENE) == scene, "Escape restoring the scene")
        self.saved()
        require(self.project() == before, "Stagger cancel wrote an IndexedDB revision")
        print("PASS Stagger cancel", flush=True)
        self.commit_and_undo("Ctrl Offset handle", '[aria-label="Drag to offset column"]',
                             matrix_id, "columnLayout", ctrl=True)

    def prepare_collision(self):
        self.button("Open Case workspace")
        before = self.project()
        self.button("Configure mechanical stack")
        configured = wait_for(lambda: (project if (project := self.persisted_after(before["revision"]))
                                       and mechanical_mounts(project) else None), "mechanical stack save", 60)
        if mechanical_mounts(configured) != ["rigid"]:
            self.select_option("Mount style", "Rigid mount")
            wait_for(lambda: (project if (project := self.persisted_after(configured["revision"]))
                              and mechanical_mounts(project) == ["rigid"] else None),
                     "rigid mount save", 60)
        # Wait for readiness instead of treating an initially disabled button as no action.
        self.button("Update preview")

    def collision(self):
        self.prepare_collision()
        wait_for(lambda: self.evaluate(
            """[...document.querySelectorAll('[role="status"]')].some(el =>
                el.textContent.includes('Exact case geometry ready.'))"""), "exact Case geometry", 120)
        self.button("Check collisions")
        layout = wait_for(lambda: self.evaluate(f"""(() => {{
            const panel = {FINDINGS};
            const viewer = {VIEWER};
            const assembly = panel?.closest('[aria-label="Case assembly"]');
            if (!panel || !viewer || !assembly || panel.querySelector('[role="status"]')) return null;
            const alerts = [...panel.querySelectorAll('[role="alert"]')].map(el => el.textContent);
            const findings = [...panel.querySelectorAll('p:not([role="alert"])')]
                .filter(el => !el.closest('[data-slot="collapsible"]:has(> [aria-label="Witness identity"])'))
                .map(el => el.textContent).filter(text => text.includes('↔'));
            if (!findings.length && !alerts.length) return null;
            const rect = el => el.getBoundingClientRect().toJSON();
            panel.scrollTop = panel.scrollHeight;
            const scrolled = panel.scrollTop > 0;
            panel.scrollTop = 0;
            return {{alerts, findings, scrolled, overflow: getComputedStyle(panel).overflowY,
                visible: getComputedStyle(viewer).visibility === 'visible',
                panel: rect(panel), viewer: rect(viewer), assembly: rect(assembly),
                viewport: [innerWidth, innerHeight]}};
        }})()"""), "completed collision findings", 120)
        require(not layout["alerts"], f"collision alerts: {layout['alerts']}")
        require(any(PCB_PAIR.search(text) for text in layout["findings"]),
                f"no generated case-body/PCB mesh observation in {len(layout['findings'])} findings")
        print("PASS Default collision PCB pair", flush=True)
        require(layout["scrolled"] and layout["overflow"] in ("scroll", "auto"),
                "Case findings did not overflow and scroll within their panel")
        panel, viewer, assembly = (layout[name] for name in ("panel", "viewer", "assembly"))
        width, height = layout["viewport"]
        require(layout["visible"] and viewer["width"] > 0 and viewer["height"] > 0
                and 0 <= viewer["left"] < viewer["right"] <= width
                and 0 <= viewer["top"] < viewer["bottom"] <= height,
                f"Case canvas was not visible inside the viewport: {viewer}")
        require(assembly["top"] <= panel["top"] and panel["bottom"] <= assembly["bottom"]
                and (viewer["bottom"] <= panel["top"] + 1 or panel["bottom"] <= viewer["top"] + 1),
                f"Case findings overlap the canvas or escape the assembly: {panel}, {viewer}")
        print("PASS Case viewer with overflowing collision findings", flush=True)


def without_revision(project):
    return {key: value for key, value in project.items() if key != "revision"}


def mechanical_mounts(project):
    configurations = [project.get("mechanical")]
    configurations.extend(instance.get("mechanical") for instance in (project.get("hardware") or {}).get("instances", []))
    return [configuration["mount"] for configuration in configurations if configuration is not None]


def verify_edit(name, field, before, after):
    require({key: value for key, value in before.items() if key != field}
            == {key: value for key, value in after.items() if key != field},
            f"{name}: changed unrelated fields in the saved matrix")
    if field == "cells":
        def cell_offsets(matrix):
            return {(cell["row"], cell["column"]): cell["offset"] for cell in matrix.get(field, [])
                    if cell.get("offset") and any(cell["offset"].values())}
        left, right = cell_offsets(before), cell_offsets(after)
        require({cell for cell in left.keys() | right.keys() if left.get(cell) != right.get(cell)} == {(1, 2)},
                f"{name}: expected only the selected cell offset to change")
    else:
        def column(matrix, index):
            cols = matrix.get(field, [])
            return cols[index] if index < len(cols) else {}
        left, right = column(before, 2), column(after, 2)
        require(left.get("offset") != right.get("offset") and "offset" in right
                and {key: value for key, value in left.items() if key != "offset"}
                == {key: value for key, value in right.items() if key != "offset"}
                and all(column(before, i) == column(after, i) for i in range(before["columns"]) if i != 2),
                f"{name}: expected only the latched column offset to change")


def run(suite, site):
    index = site / "site-subpath/boardstudio/index.html"
    if not index.is_file():
        raise RuntimeError(f"packaged site missing {index}; run python3 scripts/check.py build first")
    served = set()
    with browser_tools.serve(handler_for(site, served)) as url, browser_tools.WebDriverSession(timeout=600) as browser:
        smoke = Smoke(browser)
        try:
            smoke.onboarding(url + "/boardstudio/")
            if suite in ("all", "drag"):
                smoke.drag()
                require(any(path.endswith("/m1_core_worker_bg.wasm") for path in served.copy()),
                        "Core worker WASM was not served")
            if suite in ("all", "collision"):
                smoke.collision()
                for suffix in ("/m1_cad_worker_bg.wasm", ".bstep"):
                    require(any(path.endswith(suffix) for path in served.copy()), f"Case did not load {suffix}")
        except (OSError, RuntimeError, ValueError) as error:
            try:
                status = browser.execute_sync("""return ({
                    statuses: [...document.querySelectorAll('[role="status"]')].map(el => el.textContent.trim()),
                    alerts: [...document.querySelectorAll('[role="alert"]')].map(el => el.textContent.trim()),
                });""")
                error.add_note(f"rendered DOM status={status.get('statuses')} alerts={status.get('alerts')}")
            except (OSError, RuntimeError):
                pass
            try:
                browser.check_page_errors()
            except (OSError, RuntimeError) as page_error:
                error.add_note(str(page_error))
            raise
        browser.check_page_errors()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", default="all", choices=("all", "drag", "collision"))
    args = parser.parse_args(argv)
    try:
        run(args.suite, SITE)
    except (OSError, RuntimeError, ValueError) as error:
        print(f"FAIL {error}", *getattr(error, "__notes__", ()), sep="\n", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

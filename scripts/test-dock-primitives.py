#!/usr/bin/env python3
"""Build/mount ticket 009 alone and exercise it with trusted agent-browser input.

Run from any directory: python3 scripts/test-dock-primitives.py
Uses the existing lane target, pinned wasm-bindgen and Dioxus's cached Tailwind CLI.
"""
import functools
import http.server
import json
import os
from pathlib import Path
import subprocess
import threading
import importlib

wasm_bindgen_dir = importlib.import_module("run-wasm-tests").wasm_bindgen_dir

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target/dock-primitives-fixture"
SESSION = "dock-009"


def run(*args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, text=True, timeout=600, **kwargs)


def browser(*args):
    result = json.loads(run("agent-browser", "--session", SESSION, "--json", *map(str, args),
                            capture_output=True).stdout)
    assert result["success"], result
    return result["data"]


def evaluate(expression):
    return browser("eval", expression)["result"]


def check(expression):
    assert evaluate(expression), expression


def control(role, name):
    return f'document.querySelector({json.dumps(f"[role={role}][aria-label={json.dumps(name)}]")})'


def click(name):
    browser("find", "role", "button", "click", "--name", name, "--exact")


def width(name):
    return evaluate(f"+{control('separator', 'Resize ' + name)}.getAttribute('aria-valuenow')")


def saved(side):
    return evaluate(f"JSON.parse(localStorage.getItem('boardstudio:v2:panel:{side}'))")


def tests(url):
    browser("set", "viewport", 1440, 900)
    browser("open", url)
    browser("wait", "[role=separator][aria-label='Resize Objects']")
    evaluate("localStorage.setItem('boardstudio:v2:panel:left', JSON.stringify({mode:'autohide',width:250}));"
             "localStorage.setItem('boardstudio:v2:panel:right', JSON.stringify({mode:'collapsed',width:340}))")
    browser("reload")
    browser("wait", "[role=separator][aria-label='Resize Objects']")
    browser("snapshot", "-i")
    assert (width("Objects"), width("Inspector")) == (250, 340)
    for name in ["Objects", "Inspector"]:
        dock = control("complementary", name)
        check(f"getComputedStyle({dock}).position === 'relative' && {dock}.getBoundingClientRect().top >= 56")
    evaluate("window.dockEvents=[]; ['pointerdown','pointermove','pointerup','lostpointercapture'].forEach(type => "
             "document.addEventListener(type,e => { if(e.target.getAttribute('role')==='separator') "
             "dockEvents.push({type,trusted:e.isTrusted,id:e.pointerId,captured:e.target.hasPointerCapture(e.pointerId)}) }))")
    for name, side, delta, expected in [("Objects", "left", 80, 330), ("Inspector", "right", -60, 400)]:
        handle = control("separator", "Resize " + name)
        rect = evaluate(f"(() => {{const r={handle}.getBoundingClientRect(); return {{x:r.x+r.width/2,y:r.y+r.height/2}}}})()")
        before = saved(side)
        browser("mouse", "move", rect["x"], rect["y"])
        browser("mouse", "down")
        browser("mouse", "move", rect["x"] + delta / 2, rect["y"])
        browser("mouse", "move", rect["x"] + delta, rect["y"])
        assert width(name) == expected
        assert saved(side) == before, "drag preview must not write preferences"
        check("dockEvents.some(e => e.type==='pointermove' && e.trusted && e.captured)")
        browser("mouse", "up")
        assert saved(side) == {"mode": before["mode"], "width": expected}
        check(f"!{handle}.hasPointerCapture(dockEvents.findLast(e=>e.type==='pointerup').id)")
    check("dockEvents.filter(e=>e.type==='pointerdown' && e.trusted).length===2")
    for name, arrow, expected, minimum, maximum in [("Objects", "ArrowRight", 350, 200, 420), ("Inspector", "ArrowLeft", 420, 280, 480)]:
        browser("find", "role", "separator", "focus", "--name", "Resize " + name, "--exact")
        browser("press", arrow)
        assert width(name) == expected
        browser("press", "Home")
        assert width(name) == minimum
        browser("press", "End")
        assert width(name) == maximum
    browser("reload")
    browser("wait", "[role=separator][aria-label='Resize Objects']")
    assert (width("Objects"), width("Inspector")) == (420, 480)
    assert saved("left")["mode"] == "autohide" and saved("right")["mode"] == "collapsed"
    evaluate("localStorage.setItem('boardstudio:v2:panel:left','{\"mode\":\"pinned\",\"width\":9999}');"
             "localStorage.setItem('boardstudio:v2:panel:right','{\"mode\":\"collapsed\",\"width\":1}')")
    browser("reload")
    browser("wait", "[role=separator][aria-label='Resize Objects']")
    assert (width("Objects"), width("Inspector")) == (420, 280)
    for edge in ["Bottom", "Top", "Left", "Right"]:
        click(edge)
        click("Open settings")
        dialog = control("dialog", "Dock settings")
        browser("wait", "[role=dialog][aria-label='Dock settings']")
        check(f"{dialog}.getAttribute('aria-modal')==='false' && {dialog}===document.activeElement")
        check("getComputedStyle(document.body).overflow !== 'hidden' && !document.querySelector('[inert]')")
        check(f"{dialog}.getAttribute('data-side')==='{edge.lower()}'")
        check("document.querySelector('button[aria-haspopup=dialog]').getAttribute('aria-expanded')==='true'")
        if edge == "Bottom":
            browser("press", "Tab")
            check("document.activeElement.textContent==='Sheet action'")
            browser("press", "Tab")
            check("document.activeElement.textContent==='Close settings'")
            browser("press", "Tab")
            check(f"!{dialog}.contains(document.activeElement)")
            click("Canvas action")
            check(f"{control('status', 'Canvas clicks')}.textContent==='1' && !!{dialog}")
            browser("scroll", "down", 120)
            check("window.scrollY>0")
            browser("scroll", "up", 120)
        browser("press", "Escape")
        check(f"!{dialog} && document.activeElement.textContent==='Open settings'")
    click("Open settings")
    click("External close")
    check("!document.querySelector('[role=dialog]') && document.activeElement.textContent==='Open settings'")
    click("Open settings")
    click("Close settings")
    check("!document.querySelector('[role=dialog]') && document.activeElement.textContent==='Open settings'")
    assert browser("errors").get("errors", []) == []
    print("PASS: trusted per-side drag/capture/release, keyboard bounds, reload/persistence, in-flow geometry, four-edge non-modal Sheet, Tab/Escape and controlled focus return", flush=True)


def main():
    env = os.environ.copy()
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
    run("cargo", "build", "--locked", "-p", "boardstudio-web-ui-shared", "--example", "dock_primitives",
        "--target", "wasm32-unknown-unknown", env=env)
    OUT.mkdir(parents=True, exist_ok=True)
    run(str(wasm_bindgen_dir(ROOT) / "wasm-bindgen"), "--target", "web", "--out-dir", str(OUT),
        str(ROOT / "target/wasm32-unknown-unknown/debug/examples/dock_primitives.wasm"))
    run(str(Path.home() / ".local/share/.dx/tools/tailwindcss-v4.1.5/tailwindcss"),
        "-i", "web/tailwind.css", "-o", str(OUT / "tailwind.css"))
    (OUT / "index.html").write_text('<!doctype html><html lang="en"><meta charset="utf-8">'
        '<title>Dock primitives test</title><link rel="stylesheet" href="tailwind.css">'
        '<body><div id="main"></div><script type="module">import init from "./dock_primitives.js"; init();</script>')
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(OUT))
    with http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            tests(f"http://127.0.0.1:{server.server_port}")
        finally:
            browser("close")
            server.shutdown()


if __name__ == "__main__":
    main()

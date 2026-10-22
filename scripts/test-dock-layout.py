#!/usr/bin/env python3
"""Trusted browser checks of the production dock composition (ticket 011)."""
import functools
import http.server
import importlib
import os
from pathlib import Path
import threading

primitives = importlib.import_module("test-dock-primitives")
primitives.SESSION = "dock-011"
browser, evaluate, check, click = (primitives.browser, primitives.evaluate, primitives.check, primitives.click)
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target/dock-layout-fixture"


def tests(url):
    browser("set", "viewport", 900, 900)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[role=status][aria-label=\"Canvas clicks\"]')")
    browser("find", "role", "button", "click", "--name", "PCB", "--exact")
    click("Objects")
    dialogs = evaluate("[...document.querySelectorAll('[role=dialog]')].map(e=>({name:e.getAttribute('aria-label'),modal:e.getAttribute('aria-modal')}))")
    assert dialogs == [{"name": "Workspace panels", "modal": "false"}], f"expected one non-modal dock Sheet at 900px in PCB; actual={dialogs}"
    for workspace in ["Layout", "PCB", "Keymap", "Keycaps", "Case", "Parts"]:
        click(workspace)
        check("document.querySelectorAll('[role=dialog][aria-label=\"Workspace panels\"]').length===1")
        for role, name in [("tab", "Objects"), ("tab", "Inspector")]:
            browser("find", "role", role, "click", "--name", name, "--exact")
            check(f"document.querySelector('[role=tab][aria-selected=true]').textContent==='{name}'")
        clicks = evaluate("document.querySelector('[role=status][aria-label=\"Canvas clicks\"]').textContent")
        click("Canvas action")
        check(f"document.querySelector('[role=status][aria-label=\"Canvas clicks\"]').textContent==='{int(clicks)+1}'")
        check("!!document.querySelector('[role=dialog]') && getComputedStyle(document.body).overflow!=='hidden' && !document.querySelector('[inert]')")
    browser("find", "role", "textbox", "fill", "--name", "Dock draft", "--exact", "--", "retained draft")
    click("Open details")
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog][aria-label=\"Dock details\"]')")
    check("!!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]')")
    browser("press", "Escape")
    check("!document.querySelector('[role=dialog]') && document.activeElement.getAttribute('aria-label')==='Objects'")
    click("Inspector")
    check("document.querySelector('[aria-label=\"Dock draft\"]').value==='retained draft'")
    for key, expected in [("Home", "Objects"), ("End", "Inspector"), ("ArrowRight", "Objects")]:
        browser("focus", primitives.by_role_name("tab", "Inspector"))
        browser("press", key)
        check(f"document.activeElement.getAttribute('role')==='tab' && document.activeElement.textContent==='{expected}'")
        browser("press", "Enter")
        check(f"document.activeElement.getAttribute('aria-selected')==='true'")
    browser("press", "Space")
    click("Close panels")
    check("!document.querySelector('[role=dialog]') && document.activeElement.getAttribute('aria-label')==='Inspector'")
    browser("set", "viewport", 1440, 900)
    evaluate("localStorage.setItem('boardstudio:v2:panel:left',JSON.stringify({mode:'autohide',width:250}));localStorage.setItem('boardstudio:v2:panel:right',JSON.stringify({mode:'autohide',width:340}))")
    browser("reload")
    browser("wait", "--fn", "!!document.querySelector('[role=separator][aria-label=\"Resize Objects\"]')")
    for side, width in [("left", 250), ("right", 340)]:
        assert primitives.saved(side) == {"mode": "pinned", "width": width}
    for name in ["Objects", "Inspector"]:
        dock = primitives.control("complementary", name)
        check(f"getComputedStyle({dock}).position==='relative' && getComputedStyle({dock}).borderTopWidth==='0px'")
        click(f"Collapse {name.lower()}")
        check(f"!{dock}")
        click(f"Show {name.lower()}")
        check(f"!!{dock}")
    for name, side, arrow, width in [("Objects", "left", "ArrowRight", 270), ("Inspector", "right", "ArrowLeft", 360)]:
        browser("focus", primitives.by_role_name("separator", "Resize " + name))
        browser("press", arrow)
        assert primitives.saved(side) == {"mode": "pinned", "width": width}
    browser("reload")
    browser("wait", "--fn", "!!document.querySelector('[role=separator][aria-label=\"Resize Objects\"]')")
    assert (primitives.width("Objects"), primitives.width("Inspector")) == (270, 360)
    for width in [980, 979, 390]:
        browser("set", "viewport", width, 844)
        if width == 980:
            check("document.querySelectorAll('[role=complementary]').length===2 && !document.querySelector('[role=dialog]')")
        else:
            click("Objects")
            dialog = primitives.control("dialog", "Workspace panels")
            check(f"{dialog}.getBoundingClientRect().top>=300 && {dialog}.getBoundingClientRect().right<={width}")
            click("Canvas action")
            browser("press", "Escape")
    for theme in ["original", "shadcn"]:
        for appearance in ["light", "dark"]:
            evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
            click("Inspector")
            browser("screenshot", str(ROOT / f".evidence/011-narrow-{theme}-{appearance}.png"))
            browser("press", "Escape")
    assert browser("errors").get("errors", []) == []
    print("PASS: all six Workspace dock routes, one non-modal Sheet, canvas input, tabs keyboard floor, nested Escape/focus, caller draft, migration, per-side modes/widths and breakpoint geometry", flush=True)


def main():
    env = os.environ.copy()
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
    primitives.run("cargo", "build", "--locked", "-p", "boardstudio-web-ui-shared", "--example", "dock_layout",
                   "--target", "wasm32-unknown-unknown", env=env)
    OUT.mkdir(parents=True, exist_ok=True)
    primitives.run(str(primitives.wasm_bindgen_dir(ROOT) / "wasm-bindgen"), "--target", "web", "--out-dir", str(OUT),
                   str(ROOT / "target/wasm32-unknown-unknown/debug/examples/dock_layout.wasm"))
    primitives.run(str(Path.home() / ".local/share/.dx/tools/tailwindcss-v4.1.5/tailwindcss"),
                   "-i", "web/tailwind.css", "-o", str(OUT / "tailwind.css"))
    if not (OUT / "assets").exists():
        (OUT / "assets").symlink_to(ROOT / "web/assets", target_is_directory=True)
    (OUT / "index.html").write_text('<!doctype html><html lang="en"><meta charset="utf-8">'
        '<title>Dock integration test</title><link rel="stylesheet" href="assets/m1.css">'
        '<link rel="stylesheet" href="tailwind.css"><body><div id="main"></div>'
        '<script type="module">import init from "./dock_layout.js"; init();</script>')
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(OUT))
    with http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            tests(f"http://127.0.0.1:{server.server_port}")
        finally:
            browser("close")
            server.shutdown()


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Trusted browser checks of the production dock composition (ticket 011)."""
import functools
import argparse
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


def production_tests(url):
    """Exercise the actual Editor-owned callbacks and mobile Header composition."""
    browser("set", "viewport", 1440, 900)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
    if evaluate("!!document.querySelector('[aria-label=\"Keyboard setup\"]')"):
        click("Hide guide")
        browser("wait", "--fn", "!document.querySelector('[aria-label=\"Keyboard setup\"]')")
    click("Project")
    browser("find", "role", "menuitem", "click", "--name", "New project", "--exact")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Keyboard setup\"]')")
    click("4 Case (optional)")
    check("!!document.querySelector('[role=region][aria-label=\"Case workspace\"]')")
    click("Hide guide")

    def navigate(workspace):
        name = "Open " + workspace + " workspace"
        if not any(item.get("name") == name for item in browser("snapshot", "-i")["refs"].values()):
            click("Workbench menu")
        click(name)
        browser("wait", "--fn", f"!!document.querySelector('[role=region][aria-label=\"{workspace} workspace\"]')")

    def sheet_open():
        return evaluate("!!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]')")

    def close_sheet():
        if sheet_open():
            click("Close panels")
            browser("wait", "--fn", "!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]')")

    def add_view():
        was_sheet_open = sheet_open()
        click("Add object")
        browser("wait", "--fn", "!!document.querySelector('[role=dialog][aria-label=Add]')")
        check("document.getElementById('m1-objects-panel').contains(document.querySelector('[role=dialog][aria-label=Add]'))")
        browser("focus", primitives.by_role_name("button", "← Back to objects"))
        browser("press", "Escape")
        browser("wait", "--fn", "!document.querySelector('[role=dialog][aria-label=Add]')")
        assert sheet_open() == was_sheet_open
        check("document.activeElement.id==='m1-add-object-trigger'")

    navigate("Layout")
    for name in ["objects", "inspector"]:
        label = "Show " + name
        if any(item.get("name") == label for item in browser("snapshot", "-i")["refs"].values()):
            click(label)
    browser("set", "viewport", 980, 900)
    click("Collapse objects")
    click("Collapse inspector")
    left_width = primitives.saved("left")["width"]
    click("Browse parts")
    browser("wait", "--fn", "!!document.querySelector('[role=region][aria-label=\"Parts workspace\"]')")
    assert primitives.saved("left") == {"mode": "pinned", "width": left_width}
    assert not sheet_open()
    navigate("Layout")
    click("Collapse objects")
    browser("set", "viewport", 979, 900)
    close_sheet()
    click("Browse parts")
    browser("wait", "--fn", "!!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]')")
    assert primitives.saved("left") == {"mode": "collapsed", "width": left_width}
    check("document.querySelector('[role=tab][aria-selected=true]').textContent==='Objects'")
    close_sheet()
    navigate("Layout")
    click("Add key matrix")
    for name in ["Rows", "Columns"]:
        browser("focus", primitives.by_role_name("slider", name))
        browser("press", "Home")
    click("Create matrix")
    browser("wait", "--fn", "!document.querySelector('[role=region][aria-label=\"New matrix\"]')")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Open Position and rotation properties for Key\"]')")
    close_sheet()
    browser("set", "viewport", 980, 900)
    right_before = primitives.saved("right")
    assert right_before["mode"] == "collapsed"
    click("Open Position and rotation properties for Key")
    assert primitives.saved("right") == {"mode": "pinned", "width": right_before["width"]}
    assert not sheet_open()
    click("Collapse inspector")
    browser("set", "viewport", 979, 900)
    close_sheet()
    click("Open Position and rotation properties for Key")
    browser("wait", "--fn", "!!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]')")
    assert primitives.saved("right") == right_before
    check("document.querySelector('[role=tab][aria-selected=true]').textContent==='Inspector'")
    check("!!document.querySelector('#m1-inspector-panel-content [aria-label=\"Matrix name\"]')")
    close_sheet()
    for workspace in ["Layout", "PCB", "Keymap", "Keycaps", "Case", "Parts"]:
        navigate(workspace)
        if not sheet_open():
            click("Workbench menu")
            click("Objects")
        browser("wait", "--fn", "!!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]')")
        check("document.querySelectorAll('#workspace-panel-sheet').length===1")
        if workspace == "Layout":
            browser("find", "role", "tab", "click", "--name", "Objects", "--exact")
            add_view()
        for name in ["Objects", "Inspector"]:
            browser("find", "role", "tab", "click", "--name", name, "--exact")
        check("document.querySelectorAll('[role=dialog][aria-label=\"Workspace panels\"]').length===1")
        close_sheet()
    browser("set", "viewport", 390, 844)
    for retained_exit in [True, False]:
        click("Workbench menu")
        opener = primitives.by_role_name("button", "Objects")
        assert browser("get", "attr", opener, "aria-controls")["value"] == "workspace-panel-sheet"
        if retained_exit:
            evaluate("window.menuExit=document.getElementById('header-mobile-commands').animate([{opacity:1},{opacity:0}],{duration:1000000});menuExit.pause()")
        click("Objects")
        check("document.getElementById('workspace-panel-sheet').getAttribute('role')==='dialog'")
        browser("find", "role", "tab", "click", "--name", "Inspector", "--exact")
        if retained_exit:
            check("document.getElementById('objects-sheet-toggle').isConnected && !!document.getElementById('objects-sheet-toggle').closest('[inert], [aria-hidden=true]')")
        else:
            browser("wait", "--fn", "!document.getElementById('header-mobile-commands')")
        if retained_exit:
            click("Close panels")
        else:
            browser("press", "Escape")
        browser("wait", "--fn", "!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]')")
        check("document.activeElement.getAttribute('aria-label')==='Workbench menu'")
        if retained_exit:
            evaluate("menuExit.finish()")
            browser("wait", "--fn", "!document.getElementById('header-mobile-commands')")
    click("Workbench menu")
    click("Inspector")
    browser("wait", "--fn", "!document.getElementById('header-mobile-commands')")
    browser("screenshot", str(ROOT / ".evidence/011-production-narrow.png"))
    close_sheet()
    browser("set", "viewport", 1440, 900)
    navigate("Layout")
    for name in ["objects", "inspector"]:
        label = "Show " + name
        if any(item.get("name") == label for item in browser("snapshot", "-i")["refs"].values()):
            click(label)
    add_view()
    check("(() => {const left=document.querySelector('[role=complementary][aria-label=Objects]').getBoundingClientRect(),right=document.querySelector('[role=complementary][aria-label=Inspector]').getBoundingClientRect(),canvas=document.querySelector('[role=region][aria-label=\"Layout workspace\"]').getBoundingClientRect();return left.right<=canvas.left && canvas.right<=right.left && canvas.width>=280})()")
    browser("screenshot", str(ROOT / ".evidence/011-production-desktop.png"))
    (ROOT / ".evidence/011-production-browser-errors.json").write_text(primitives.json.dumps(browser("errors")))
    print("PASS: packaged six-Workspace Sheet, real Case stage role/name, browse Parts and pin Inspector callbacks at979/980 with saved widths, legacy focus IDs, mobile opener connected/inert and unmounted fallbacks", flush=True)


def tests(url):
    browser("set", "viewport", 900, 900)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[role=status][aria-label=\"Canvas clicks\"]')")
    browser("find", "role", "button", "click", "--name", "PCB", "--exact")
    body_overflow = evaluate("getComputedStyle(document.body).overflow")
    click("Objects")
    dialogs = evaluate("[...document.querySelectorAll('[role=dialog]')].map(e=>({name:e.getAttribute('aria-label'),modal:e.getAttribute('aria-modal')}))")
    assert dialogs == [{"name": "Workspace panels", "modal": "false"}], f"expected one non-modal dock Sheet at 900px in PCB; actual={dialogs}"
    def popup_state(name, expanded):
        ref = primitives.by_role_name("button", name)
        for attribute, expected in [("aria-expanded", str(expanded).lower()), ("aria-controls", "workspace-panel-sheet"), ("aria-haspopup", "dialog")]:
            assert browser("get", "attr", ref, attribute)["value"] == expected
        if expanded:
            check("document.querySelectorAll('#workspace-panel-sheet').length===1 && document.getElementById('workspace-panel-sheet').getAttribute('role')==='dialog'")
    popup_state("Objects", True)
    click("Objects options")
    browser("wait", "--fn", "!!document.querySelector('[role=dialog][aria-label=\"Objects panel options\"]')")
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog][aria-label=\"Objects panel options\"]')")
    check("!!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]') && document.activeElement.getAttribute('aria-label')==='Objects options'")
    for workspace in ["Layout", "PCB", "Keymap", "Keycaps", "Case", "Parts"]:
        click(workspace)
        check("document.querySelectorAll('[role=dialog][aria-label=\"Workspace panels\"]').length===1")
        for role, name in [("tab", "Objects"), ("tab", "Inspector")]:
            browser("find", "role", role, "click", "--name", name, "--exact")
            check(f"document.querySelector('[role=tab][aria-selected=true]').textContent==='{name}'")
            check("(() => {const tab=document.querySelector('[role=tab][aria-selected=true]');return document.getElementById(tab.getAttribute('aria-controls')).getAttribute('aria-labelledby')===tab.id})()")
        clicks = evaluate("document.querySelector('[role=status][aria-label=\"Canvas clicks\"]').textContent")
        click("Canvas action")
        check(f"document.querySelector('[role=status][aria-label=\"Canvas clicks\"]').textContent==='{int(clicks)+1}'")
        assert evaluate("getComputedStyle(document.body).overflow") == body_overflow
        check("!!document.querySelector('[role=dialog]') && !document.querySelector('[inert]')")
    browser("find", "role", "textbox", "fill", "--name", "Dock draft", "--exact", "retained draft")
    click("Open details")
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog][aria-label=\"Dock details\"]')")
    check("!!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]')")
    browser("press", "Escape")
    check("!document.querySelector('[role=dialog]') && document.activeElement.getAttribute('aria-label')==='Objects'")
    popup_state("Objects", False)
    click("Inspector")
    popup_state("Inspector", True)
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
    browser("find", "role", "textbox", "fill", "--name", "Local dock draft", "--exact", "local draft")
    evaluate("window.localDraftNode=document.querySelector('[aria-label=\"Local dock draft\"]')")
    for name in ["Objects", "Inspector"]:
        dock = primitives.control("complementary", name)
        check(f"getComputedStyle({dock}).position==='relative' && getComputedStyle({dock}).borderTopWidth==='0px'")
        click(f"Collapse {name.lower()}")
        check(f"!{dock}.checkVisibility()")
        check(f"document.activeElement.getAttribute('aria-label')==='Show {name.lower()}'")
        if name == "Inspector":
            check("!!localDraftNode.closest('[hidden][inert]')")
            evaluate("localDraftNode.focus()")
            check("document.activeElement.getAttribute('aria-label')==='Show inspector'")
        click(f"Show {name.lower()}")
        check(f"{dock}.checkVisibility()")
        check(f"document.activeElement.getAttribute('aria-label')==='Collapse {name.lower()}'")
    check("localDraftNode===document.querySelector('[aria-label=\"Local dock draft\"]') && localDraftNode.value==='local draft'")
    for name, side, delta, expected in [("Objects", "left", 40, 290), ("Inspector", "right", -20, 360)]:
        handle = primitives.control("separator", "Resize " + name)
        rect = evaluate(f"(() => {{const r={handle}.getBoundingClientRect();return {{x:r.x+r.width/2,y:r.y+r.height/2}}}})()")
        before = primitives.saved(side)
        browser("mouse", "move", rect["x"], rect["y"])
        browser("mouse", "down")
        browser("mouse", "move", rect["x"] + delta, rect["y"])
        assert primitives.width(name) == expected
        assert primitives.saved(side) == before
        browser("mouse", "up")
        assert primitives.saved(side) == {"mode": "pinned", "width": expected}
    for name, side, arrow, width in [("Objects", "left", "ArrowRight", 310), ("Inspector", "right", "ArrowLeft", 380)]:
        browser("focus", primitives.by_role_name("separator", "Resize " + name))
        browser("press", arrow)
        assert primitives.saved(side) == {"mode": "pinned", "width": width}
    browser("reload")
    browser("wait", "--fn", "!!document.querySelector('[role=separator][aria-label=\"Resize Objects\"]')")
    assert (primitives.width("Objects"), primitives.width("Inspector")) == (310, 380)
    for width in [980, 979, 390]:
        browser("set", "viewport", width, 844)
        print("boundary before", width, evaluate("({buttons:[...document.querySelectorAll('[aria-controls=workspace-panel-sheet]')].map(e=>[e.getAttribute('aria-label'),e.getAttribute('aria-expanded')]),dialogs:document.querySelectorAll('[role=dialog]').length})"), flush=True)
        if width == 980:
            check("document.querySelectorAll('[role=complementary]').length===2 && !document.querySelector('[role=dialog]')")
        else:
            click("Objects")
            print("boundary clicked", width, evaluate("({buttons:[...document.querySelectorAll('[aria-controls=workspace-panel-sheet]')].map(e=>[e.getAttribute('aria-label'),e.getAttribute('aria-expanded')]),dialogs:document.querySelectorAll('[role=dialog]').length})"), flush=True)
            browser("wait", "--fn", "!!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]')")
            dialog = primitives.control("dialog", "Workspace panels")
            check(f"{dialog}.getBoundingClientRect().top>=300 && {dialog}.getBoundingClientRect().right<={width}")
            click("Canvas action")
            browser("press", "Escape")
            browser("wait", "--fn", "!document.querySelector('[role=dialog]')")
            popup_state("Objects", False)
    click("Objects")
    click("External close panels")
    check("!document.querySelector('[role=dialog]') && document.activeElement.textContent==='External close panels'")
    for theme in ["original", "shadcn"]:
        for appearance in ["light", "dark"]:
            evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
            click("Inspector")
            browser("screenshot", str(ROOT / f".evidence/011-narrow-{theme}-{appearance}.png"))
            browser("press", "Escape")
    assert browser("errors").get("errors", []) == []
    print("PASS: all six Workspace dock routes, one non-modal Sheet, canvas input, tabs keyboard floor, nested Escape/focus, caller draft, migration, per-side modes/widths and breakpoint geometry", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", help="check a running packaged production page instead of building the dock fixture")
    args = parser.parse_args()
    if args.page_url:
        primitives.SESSION = "dock-011-production"
        try:
            production_tests(args.page_url)
        except Exception:
            (ROOT / ".evidence/011-production-failure-snapshot.json").write_text(primitives.json.dumps(browser("snapshot", "-i"), indent=2))
            raise
        finally:
            browser("close")
        return
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

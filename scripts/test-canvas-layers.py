#!/usr/bin/env python3
"""016 shared slice: trusted input on the actual CanvasLayers and LayerPanel owners."""
import functools
from datetime import datetime
import http.server
import importlib
import os
from pathlib import Path
import threading

driver = importlib.import_module("test-dock-primitives")
ROOT = Path(__file__).resolve().parents[1]
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
driver.SESSION = f"canvas-layers-{ROOT.name}"
OUT = TARGET / "canvas-layers-fixture"
CAPTURES = ROOT / ".evidence/016" / datetime.now().strftime("layer-captures-%Y%m%d-%H%M%S")
CAPTURES.mkdir(parents=True)
browser, check, evaluate = driver.browser, driver.check, driver.evaluate


def focus(role, name):
    browser("focus", driver.by_role_name(role, name))


def pressed(name):
    return browser("get", "attr", driver.by_role_name("button", name), "aria-pressed")["value"]


def tests(url):
    browser("open", url)
    browser("wait", "--fn", "document.getElementById('layers-016')?.getAttribute('aria-expanded')==='false'")
    check("document.getElementById('layer-list-016').hidden && document.getElementById('layer-list-016').inert")
    check("document.getElementById('layer-notes-016').getAttribute('aria-expanded')==='true'")
    focus("button", "Canvas visibility")
    browser("press", "Enter")
    browser("wait", "--fn", "document.getElementById('layers-016').getAttribute('aria-expanded')==='true'")
    check("document.getElementById(document.activeElement.getAttribute('aria-controls'))===document.getElementById('layer-list-016')")
    print("opened layer content:", evaluate('({hidden:document.getElementById("layer-list-016").hidden,inert:document.getElementById("layer-list-016").inert,attr:document.getElementById("layer-list-016").getAttribute("inert")})'), flush=True)
    check('!document.getElementById("layer-list-016").hidden && !document.getElementById("layer-list-016").inert')
    focus("button", "Hide Switches")
    print("actual dock styles:", evaluate('({position:getComputedStyle(document.getElementById("layer-list-016")).position,eyeHeight:document.activeElement.getBoundingClientRect().height,eyeWidth:document.activeElement.getBoundingClientRect().width,triggerHeight:document.getElementById("layers-016").getBoundingClientRect().height})'), flush=True)
    check('getComputedStyle(document.getElementById("layer-list-016")).position==="static" && document.activeElement.getBoundingClientRect().height >= 44 && document.activeElement.getBoundingClientRect().width >= 44')
    for key in ["Enter", "Space"]:
        focus("button", "Hide Switches")
        evaluate("window.layerEye=document.activeElement;window.layerEvents=[];window.recordLayer=e=>layerEvents.push(e.isTrusted);layerEye.addEventListener('click',recordLayer)")
        browser("press", key)
        assert pressed("Show Switches") == "false"
        check("layerEvents.length===1 && layerEvents[0] && document.querySelector('[aria-label=\"Hidden layers\"]').textContent.includes('Keys')")
        evaluate("layerEye.removeEventListener('click',recordLayer)")
        browser("press", key)
        assert pressed("Hide Switches") == "true"
        check("!document.querySelector('[aria-label=\"Hidden layers\"]').textContent.includes('Keys')")
    focus("button", "Hide Switches")
    driver.click("Hide Switches")
    assert pressed("Show Switches") == "false"
    driver.click("Show Switches")
    assert pressed("Hide Switches") == "true"
    focus("button", "Hide Switches")
    browser("press", "Home")
    check("document.activeElement.getAttribute('aria-label')==='Hide Switches'")
    browser("press", "End")
    check("document.activeElement.getAttribute('aria-label')==='Hide Switches'")
    browser("wait", "--fn", "!!document.getElementById(document.activeElement.getAttribute('aria-describedby'))")
    browser("press", "Escape")
    browser("wait", "--fn", "document.getElementById('layers-016').getAttribute('aria-expanded')==='false'")
    check("document.activeElement.id==='layers-016' && document.getElementById('layer-list-016').inert")
    browser("press", "Tab")
    check("!document.activeElement.closest('[inert]')")
    focus("button", "Canvas visibility")
    browser("press", "Space")
    focus("button", "Hide Module")
    browser("press", "Enter")
    check("document.querySelector('[aria-label=\"Hidden modules\"]').textContent.includes('module')")
    focus("button", "Show Footprints")
    browser("press", "Space")
    check("document.querySelector('[aria-label=\"Footprints visible\"]').textContent==='true'")
    browser("fill", driver.by_role_name("textbox", "Layer note"), "Uncommitted layer note")
    focus("textbox", "Layer note")
    evaluate("window.retainedNote=document.activeElement;window.retainedList=document.getElementById('layer-notes-content-016')")
    focus("button", "Layer notes")
    browser("press", "Space")
    check("retainedNote.isConnected && retainedList.inert && retainedList.hidden")
    browser("press", "Enter")
    focus("textbox", "Layer note")
    check("document.activeElement===retainedNote && retainedNote.value==='Uncommitted layer note'")
    disabled = browser("get", "box", driver.by_role_name("button", "Hide disabled layer"))
    browser("mouse", "move", round(disabled["x"] + disabled["width"] / 2), round(disabled["y"] + disabled["height"] / 2))
    browser("mouse", "down")
    browser("mouse", "up")
    check("document.querySelector('[aria-label=\"Disabled callbacks\"]').textContent==='0'")
    focus("button", "After layers")
    browser("press", "Shift+Tab")
    check("document.activeElement.getAttribute('aria-label')!=='Hide disabled layer'")
    driver.click("Mount or unmount layers")
    check("!document.getElementById('layers-016') && !retainedNote.isConnected")
    driver.click("Mount or unmount layers")
    browser("wait", "--fn", "!!document.getElementById('layers-016')")
    check("document.getElementById('layers-016').getAttribute('aria-expanded')==='false'")
    focus("textbox", "Layer note")
    check("document.activeElement.value==='Local draft'")
    check("document.querySelector('[aria-label=\"Hidden modules\"]').textContent.includes('module') && document.querySelector('[aria-label=\"Footprints visible\"]').textContent==='true'")
    focus("button", "Canvas visibility")
    browser("press", "Enter")
    focus("button", "Hide Switches")
    check('document.activeElement.getAttribute("data-slot")==="tooltip-trigger" && document.activeElement.closest("[data-slot=toggle-group]").getAttribute("aria-label")==="Switches visibility" && !!document.activeElement.closest("[data-slot=item]")')
    check('document.activeElement.getBoundingClientRect().height >= 44 && document.activeElement.getBoundingClientRect().width >= 44 && document.getElementById("layers-016").getBoundingClientRect().height >= 44')
    browser("fill", driver.by_role_name("textbox", "Layer note"), "Draft across themes and widths")
    focus("textbox", "Layer note")
    evaluate("window.themeNote=document.activeElement")
    focus("button", "After layers")
    for width in [1440, 1100, 390]:
        browser("set", "viewport", width, 900)
        for theme in ["original", "shadcn"]:
            for appearance in ["light", "dark"]:
                evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
                driver.click("Hide Switches")
                assert pressed("Show Switches") == "false"
                focus("button", "Show Switches")
                browser("press", "Space")
                assert pressed("Hide Switches") == "true"
                browser("press", "Escape")
                check('document.activeElement.id==="layers-016" && document.getElementById("layer-list-016").hidden && document.getElementById("layer-list-016").inert && document.getElementById("layer-list-016").getBoundingClientRect().height===0')
                browser("press", "Tab")
                check('!document.activeElement.closest("[inert]")')
                focus("button", "Canvas visibility")
                browser("press", "Enter")
                focus("button", "Layer notes")
                browser("press", "Space")
                check('themeNote.isConnected && document.getElementById("layer-notes-content-016").hidden && document.getElementById("layer-notes-content-016").inert')
                browser("press", "Enter")
                focus("textbox", "Layer note")
                check('document.activeElement===themeNote && themeNote.value==="Draft across themes and widths"')
                focus("button", "Hide Switches")
                check('getComputedStyle(document.getElementById("layer-list-016")).position==="static" && document.activeElement.getBoundingClientRect().height >= 44 && document.activeElement.getBoundingClientRect().width >= 44 && document.getElementById("layers-016").getBoundingClientRect().height >= 44')
                check('document.getElementById("layer-list-016").getBoundingClientRect().top >= document.getElementById("layers-016").getBoundingClientRect().bottom && document.getElementById("layer-list-016").parentElement.getBoundingClientRect().height >= document.getElementById("layer-list-016").getBoundingClientRect().height + document.getElementById("layers-016").getBoundingClientRect().height')
                focus("button", "After layers")
                browser("screenshot", str(CAPTURES / f"{width}-{theme}-{appearance}.png"))
    print("Captures:", CAPTURES, flush=True)
    assert browser("errors").get("errors", []) == []
    print("PASS shared layers: trusted Enter/Space, owner visibility, independent module/footprint targets, Escape/focus, hidden/inert Tab exclusion, retained local child draft/node, disabled refusal, unmount/remount, 12 theme/width captures", flush=True)


def main():
    env = os.environ.copy()
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
    driver.run("cargo", "build", "--locked", "-p", "boardstudio-web-ui-shared", "--example", "canvas_layers", "--target", "wasm32-unknown-unknown", env=env)
    OUT.mkdir(parents=True, exist_ok=True)
    driver.run(str(driver.wasm_bindgen_dir(ROOT) / "wasm-bindgen"), "--target", "web", "--out-dir", str(OUT), str(TARGET / "wasm32-unknown-unknown/debug/examples/canvas_layers.wasm"))
    driver.run(str(Path.home() / ".local/share/.dx/tools/tailwindcss-v4.1.5/tailwindcss"), "-i", "web/tailwind.css", "-o", str(OUT / "tailwind.css"))
    if not (OUT / "assets").exists():
        (OUT / "assets").symlink_to(ROOT / "web/assets", target_is_directory=True)
    (OUT / "index.html").write_text('<!doctype html><html lang="en"><meta charset="utf-8"><title>Shared layer controls</title><link rel="stylesheet" href="assets/workbench-base.css"><link rel="stylesheet" href="tailwind.css"><style>body{height:auto;overflow:auto;padding:16px}#main{display:grid;gap:12px;max-width:360px}</style><body><div id="main"></div><script type="module">import init from "./canvas_layers.js";init();</script>')
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(OUT))
    with http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            tests(f"http://127.0.0.1:{server.server_port}")
        except Exception:
            browser("screenshot", str(CAPTURES / "failure.png"))
            raise
        finally:
            browser("close")
            server.shutdown()


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Ticket 015: trusted keyboard and owner callbacks on the production strips."""
import functools
import http.server
import importlib
import os
from pathlib import Path
import threading

driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "layout-015"
browser, evaluate, check, click = driver.browser, driver.evaluate, driver.check, driver.click
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target/layout-toolbar-fixture"


def tests(url):
    browser("set", "viewport", 1440, 900)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Select Column\"]')")
    click("Select Column")
    browser("press", "Escape")
    scope = driver.by_role_name("button", "Select Column")
    browser("focus", scope)
    browser("press", "Home")
    check("document.activeElement.getAttribute('aria-label')==='Select Matrix'")
    browser("press", "End")
    check("document.activeElement.getAttribute('aria-label')==='Select Part'")
    browser("press", "ArrowLeft")
    check("document.activeElement.getAttribute('aria-label')==='Select Key'")
    browser("press", "Space")
    browser("press", "Escape")
    click("Activate Stagger column; selection changes to Column")
    check("document.querySelector('[aria-label=\"Owner state\"]').textContent.startsWith('Column:Some(Stagger)')")
    actions = driver.control("toolbar", "Actions for Column")
    check(f"{actions}.querySelectorAll('[tabindex=\"0\"]').length===1")
    browser("focus", driver.by_role_name("button", "Open Position and rotation properties for Column"))
    browser("press", "End")
    check("document.activeElement.getAttribute('aria-label')==='Activate Splay for the selected column'")
    browser("press", "Home")
    browser("press", "Enter")
    check("document.querySelector('[aria-label=\"Owner state\"]').textContent.includes(':1:')")
    click("Transform")
    browser("wait", "--fn", "document.activeElement?.getAttribute('role')==='menuitem'")
    browser("press", "End")
    browser("press", "Home")
    browser("press", "o")
    browser("press", "Enter")
    check("document.querySelector('[aria-label=Transform]').getAttribute('aria-expanded')==='false'")
    check("document.activeElement.getAttribute('aria-label')==='Transform'")
    click("Snap settings")
    click("Snap increment")
    browser("press", "End")
    browser("press", "Enter")
    check("document.querySelector('[aria-label=\"Owner state\"]').textContent.includes(':-0.1:')")
    check("document.activeElement.getAttribute('aria-label')==='Snap increment'")
    check("document.querySelector('[aria-label=\"Snap gap\"]').getAttribute('min')==='0' && document.querySelector('[aria-label=\"Snap gap\"]').getAttribute('step')==='any'")
    gap = driver.by_role_name("spinbutton", "Snap gap")
    browser("fill", gap, "0.125")
    check("document.querySelector('[aria-label=\"Snap gap\"]').validity.valid")
    browser("fill", gap, "-0.125")
    check("document.querySelector('[aria-label=\"Snap gap\"]').validity.rangeUnderflow")
    browser("press", "Escape")
    check("document.activeElement.getAttribute('aria-label')==='Snap settings'")
    before = evaluate("document.querySelector('[aria-label=\"Owner state\"]').textContent")
    click("Geometry snap")
    assert evaluate("document.querySelector('[aria-label=\"Owner state\"]').textContent") != before
    # Theme changes must leave the same metrics; viewport changes must not hide labels or scroll strips.
    for width in [1440, 980, 390]:
        browser("set", "viewport", width, 900)
        sizes = []
        for theme in ["original", "shadcn"]:
            for appearance in ["light", "dark"]:
                evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
                evaluate("document.fonts.ready.then(()=>true)")
                check("[...document.querySelectorAll('[role=toolbar]')].every(e=>e.scrollWidth<=e.clientWidth)")
                sizes.append(evaluate("[...document.querySelectorAll('[role=toolbar]')].map(e=>[e.getBoundingClientRect().width,e.getBoundingClientRect().height])"))
                browser("screenshot", str(ROOT / f".evidence/015/{width}-{theme}-{appearance}.png"))
        assert all(size == sizes[0] for size in sizes), (width, sizes)
    assert browser("errors").get("errors", []) == []
    print("PASS: production strips, trusted scope/action/menu/Snap keyboard, owner callbacks, precise numeric validity, three widths and four theme appearances", flush=True)


def main():
    env = os.environ.copy()
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
    driver.run("cargo", "build", "--locked", "-p", "boardstudio-web-layout", "--example", "toolbar_strips", "--target", "wasm32-unknown-unknown", env=env)
    OUT.mkdir(parents=True, exist_ok=True)
    driver.run(str(driver.wasm_bindgen_dir(ROOT) / "wasm-bindgen"), "--target", "web", "--out-dir", str(OUT), str(ROOT / "target/wasm32-unknown-unknown/debug/examples/toolbar_strips.wasm"))
    driver.run(str(Path.home() / ".local/share/.dx/tools/tailwindcss-v4.1.5/tailwindcss"), "-i", "web/tailwind.css", "-o", str(OUT / "tailwind.css"))
    if not (OUT / "assets").exists():
        (OUT / "assets").symlink_to(ROOT / "web/assets", target_is_directory=True)
    (OUT / "index.html").write_text('<!doctype html><html lang="en"><meta charset="utf-8"><title>Layout strips</title><link rel="stylesheet" href="assets/m1.css"><link rel="stylesheet" href="tailwind.css"><style>body{margin:0}.toolbar-fixture{padding:12px;box-sizing:border-box}</style><body><div id="main"></div><script type="module">import init from "./toolbar_strips.js";init();</script>')
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

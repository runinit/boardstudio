#!/usr/bin/env python3
"""016: both actual Add owners, copied Secondary look and trusted open/cancel input."""
import argparse
from datetime import datetime
import functools
import http.server
import importlib
import os
from pathlib import Path
import threading


driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "layout-add-016"
browser, evaluate, check, click = driver.browser, driver.evaluate, driver.check, driver.click
ROOT = Path(__file__).resolve().parents[1]
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
OUT = TARGET / "layout-add-fixture"


def focus_add():
    browser("focus", driver.by_role_name("button", "Add object"))
    evaluate("window.actualAdd=document.activeElement")


def settle():
    browser("wait", "--fn", "!actualAdd.getAnimations().some(a=>a.playState==='running')")


def prepare(url, editor):
    browser("set", "viewport", 1440, 900)
    browser("open", url)
    if editor:
        browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
        click("Project")
        browser("find", "role", "menuitem", "click", "--name", "New project", "--exact")
        browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Keyboard setup\"]')")
        click("4 Case (optional)")
        click("Hide guide")
        click("Open Layout workspace")
    browser("wait", "--fn", "!!document.getElementById('m1-add-object-trigger')")


def tests(url, editor, captures):
    prepare(url, editor)
    failures = []
    for width in [1440, 1100, 390]:
        browser("set", "viewport", width, 900)
        for theme in ["original", "shadcn"]:
            for appearance in ["light", "dark"]:
                evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
                if editor and width < 980:
                    if not evaluate("!!document.getElementById('workspace-panel-sheet')?.matches('[role=dialog]')"):
                        click("Workbench menu")
                        click("Objects")
                    browser("find", "role", "tab", "click", "--name", "Objects", "--exact")
                    browser("wait", "--fn", "document.getElementById('m1-add-object-trigger')?.checkVisibility()")
                browser("mouse", "move", 0, 0)
                focus_add()
                browser("press", "Tab")
                browser("press", "Shift+Tab")
                check("document.activeElement===actualAdd && actualAdd.matches(':focus-visible')")
                settle()
                default = evaluate("""(()=>{
                    const s=getComputedStyle(actualAdd), r=actualAdd.getBoundingClientRect();
                    const paint=document.createElement('span');paint.style.background='var(--secondary)';
                    paint.style.color='var(--secondary-foreground)';document.body.append(paint);
                    const expected=getComputedStyle(paint);
                    const result={bg:s.backgroundColor,ink:s.color,expectedBg:expected.backgroundColor,
                        expectedInk:expected.color,slot:actualAdd.getAttribute('data-slot'),
                        width:r.width,height:r.height,focus:s.outlineStyle==='none'&&s.boxShadow.includes('3px')};
                    paint.style.background='color-mix(in oklab,var(--secondary) 80%,transparent)';
                    result.expectedHover=getComputedStyle(paint).backgroundColor;paint.remove();return result;
                })()""")
                browser("hover", driver.by_role_name("button", "Add object"))
                settle()
                hover = evaluate("getComputedStyle(actualAdd).backgroundColor")
                # Neither owner exposes a disabled branch: probe native disabled refusal explicitly.
                evaluate("window.addClicks=[];window.recordAdd=e=>addClicks.push(e.isTrusted);actualAdd.addEventListener('click',recordAdd);actualAdd.disabled=true")
                settle()
                point = evaluate("(()=>{const r=actualAdd.getBoundingClientRect();return {x:Math.round(r.x+r.width/2),y:Math.round(r.y+r.height/2),opacity:getComputedStyle(actualAdd).opacity}})()")
                browser("mouse", "move", point["x"], point["y"])
                browser("mouse", "down")
                browser("mouse", "up")
                check("addClicks.length===0 && actualAdd.getAttribute('aria-expanded')==='false'")
                evaluate("actualAdd.disabled=false")
                browser("mouse", "move", 0, 0)
                for key in ["Enter", "Space"]:
                    focus_add()
                    before = evaluate("addClicks.length")
                    browser("press", key)
                    browser("wait", "--fn", "actualAdd.getAttribute('aria-expanded')==='true'")
                    check(f"addClicks.length==={before + 1} && addClicks.every(Boolean)")
                    region = driver.by_role_name("region", "Add")
                    assert region
                    browser("focus", driver.by_role_name("button", "← Back to objects"))
                    browser("press", "Escape")
                    browser("wait", "--fn", "actualAdd.getAttribute('aria-expanded')==='false' && document.activeElement===actualAdd")
                evaluate("actualAdd.removeEventListener('click',recordAdd)")
                result = {"default": default, "hover": hover, "disabled": point["opacity"]}
                print("Editor" if editor else "Standalone Objects", width, theme, appearance, result, flush=True)
                checks = {"Secondary default": default["bg"] == default["expectedBg"] and default["ink"] == default["expectedInk"],
                          "Secondary hover": hover == default["expectedHover"], "focus": default["focus"],
                          "copied slot": default["slot"] == "button", "native disabled CSS": point["opacity"] == "0.5",
                          "size": default["height"] >= (44 if width < 760 else 40) and default["width"] >= 44}
                failures.extend((width, theme, appearance, name) for name, passed in checks.items() if not passed)
                browser("screenshot", str(captures / f"{'editor' if editor else 'objects'}-{width}-{theme}-{appearance}.png"))
                if editor and width < 980:
                    click("Close panels")
    print("Add entry failures:", "Editor" if editor else "Standalone Objects", failures, flush=True)
    return failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", required=True)
    args = parser.parse_args()
    env = os.environ.copy()
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
    driver.run("cargo", "build", "--locked", "-p", "boardstudio-web-layout", "--example", "layout_add",
               "--features", "test-support", "--target", "wasm32-unknown-unknown", env=env)
    OUT.mkdir(parents=True, exist_ok=True)
    driver.run(str(driver.wasm_bindgen_dir(ROOT) / "wasm-bindgen"), "--target", "web", "--out-dir", str(OUT), str(TARGET / "wasm32-unknown-unknown/debug/examples/layout_add.wasm"))
    driver.run(str(Path.home() / ".local/share/.dx/tools/tailwindcss-v4.1.5/tailwindcss"), "-i", "web/tailwind.css", "-o", str(OUT / "tailwind.css"))
    if not (OUT / "assets").exists():
        (OUT / "assets").symlink_to(ROOT / "web/assets", target_is_directory=True)
    (OUT / "index.html").write_text('<!doctype html><html lang="en"><meta charset="utf-8"><title>Actual standalone Objects</title><link rel="stylesheet" href="assets/m1.css"><link rel="stylesheet" href="tailwind.css"><style>body{height:auto;overflow:auto;padding:16px}#main{max-width:360px}</style><body><div id="main"></div><script type="module">import init from "./layout_add.js";init();</script>')
    captures = ROOT / ".evidence/016" / datetime.now().strftime("add-entries-%Y%m%d-%H%M%S")
    captures.mkdir(parents=True)
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(OUT))
    with http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            failures = tests(f"http://127.0.0.1:{server.server_port}", False, captures)
            failures += tests(args.page_url, True, captures)
            print("Captures:", captures, flush=True)
            assert not failures, failures
        except Exception:
            browser("screenshot", str(captures / "failure.png"))
            print(browser("errors"), browser("snapshot", "-i")["snapshot"], flush=True)
            raise
        finally:
            browser("close")
            server.shutdown()


if __name__ == "__main__":
    main()

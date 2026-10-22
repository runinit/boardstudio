#!/usr/bin/env python3
"""Ticket 015: trusted keyboard and owner callbacks on the production strips."""
import functools
import argparse
import http.server
import importlib
import os
import subprocess
from pathlib import Path
import threading
from datetime import datetime

driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "layout-015"
browser, evaluate, check, click = driver.browser, driver.evaluate, driver.check, driver.click
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target/layout-toolbar-fixture"


def held_space(url):
    """agent-browser press releases immediately; CDP proves native keyup activation."""
    evaluate("window.heldButton=document.activeElement; window.heldEvents=[]; window.recordHeld=e=>heldEvents.push(e); ['keydown','keyup','click'].forEach(type=>heldButton.addEventListener(type,recordHeld))")
    endpoint = subprocess.run(["agent-browser", "--session", driver.SESSION, "get", "cdp-url"], check=True, text=True, capture_output=True).stdout.strip()
    result = subprocess.run(["node", "--input-type=module", "-e", """
        import assert from 'node:assert/strict';
        const [endpoint,url] = process.argv.slice(1), ws = new WebSocket(endpoint);
        await new Promise((resolve,reject)=>{ws.onopen=resolve;ws.onerror=reject});
        try {
        let next=0; const pending=new Map();
        ws.onmessage=event=>{const reply=JSON.parse(event.data),entry=pending.get(reply.id);if(entry){pending.delete(reply.id);reply.error?entry.reject(Error(JSON.stringify(reply.error))):entry.resolve(reply.result)}};
        const send=(method,params={},sessionId)=>new Promise((resolve,reject)=>{const id=++next;pending.set(id,{resolve,reject});ws.send(JSON.stringify({id,method,params,sessionId}))});
        const pages=(await send('Target.getTargets')).targetInfos.filter(t=>t.type==='page'&&t.url.startsWith(url));
        assert.equal(pages.length,1);
        const {sessionId}=await send('Target.attachToTarget',{targetId:pages[0].targetId,flatten:true});
        const state=async()=> (await send('Runtime.evaluate',{expression:"({clicks:heldEvents.filter(e=>e.type==='click').length,trusted:heldEvents.every(e=>e.isTrusted),keys:heldEvents.filter(e=>e.type==='keydown').map(e=>({trusted:e.isTrusted,prevented:e.defaultPrevented}))})",returnByValue:true},sessionId)).result.value;
        try {
            await send('Input.dispatchKeyEvent',{type:'keyDown',key:' ',code:'Space',windowsVirtualKeyCode:32},sessionId);
            const down=await state(); console.log('held Space keydown',JSON.stringify(down));
            assert.equal(down.clicks,0); assert.deepEqual(down.keys,[{trusted:true,prevented:false}]);
        } finally {
            await send('Input.dispatchKeyEvent',{type:'keyUp',key:' ',code:'Space',windowsVirtualKeyCode:32},sessionId);
        }
        const up=await state(); console.log('held Space keyup',JSON.stringify(up)); assert.equal(up.clicks,1); assert.equal(up.trusted,true);
        } finally { ws.close(); }
    """, endpoint, url], text=True, capture_output=True, timeout=60)
    print(result.stdout, result.stderr, flush=True)
    evaluate("['keydown','keyup','click'].forEach(type=>heldButton.removeEventListener(type,recordHeld))")
    assert result.returncode == 0, "Held Space must remain native, unprevented, and activate exactly once on keyup"


def production_tests(url):
    """Check native keyboard and one action/Undo through the real Layout owner."""
    browser("set", "viewport", 1440, 900)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
    click("Project")
    browser("find", "role", "menuitem", "click", "--name", "New project", "--exact")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Keyboard setup\"]')")
    click("4 Case (optional)")
    click("Hide guide")
    click("Open Layout workspace")
    click("Add object")
    browser("find", "role", "button", "click", "--name", "Matrix…")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"New matrix\"]')")
    click("Create matrix")
    browser("wait", "--fn", "!document.querySelector('[aria-label=\"New matrix\"]')")
    browser("focus", driver.by_role_name("button", "Select Column"))
    browser("press", "Enter")
    browser("press", "Escape")
    browser("focus", driver.by_role_name("button", "Select Column"))
    held_space(url)
    check("document.querySelector('[aria-label=\"Select Column\"]').getAttribute('aria-expanded')==='true'")
    browser("press", "Escape")
    button = driver.by_role_name("button", "Activate Stagger for the selected column")
    browser("focus", button)
    held_space(url)
    browser("wait", "--fn", "!!document.querySelector('[role=button][aria-label=\"Drag to stagger\"]')")
    handle = driver.by_role_name("button", "Drag to stagger")
    browser("focus", handle)
    before = evaluate("document.activeElement.getAttribute('aria-description')")
    browser("press", "ArrowUp")
    browser("wait", "--fn", f"document.querySelector('[role=button][aria-label=\"Drag to stagger\"]').getAttribute('aria-description')!=={driver.json.dumps(before)}")
    click("Undo")
    browser("wait", "--fn", f"document.querySelector('[role=button][aria-label=\"Drag to stagger\"]').getAttribute('aria-description')==={driver.json.dumps(before)}")
    browser("focus", driver.by_role_name("button", "Transform"))
    browser("press", "Space")
    browser("wait", "--fn", "!!document.activeElement.closest('[role=menu]')")
    browser("focus", driver.by_role_name("menuitemradio", "Activate Offset for the selected column"))
    browser("press", "Space")
    check("document.activeElement.id==='m1-layout-transform-trigger' && document.activeElement.getAttribute('aria-expanded')==='false'")
    browser("wait", "--fn", "!!document.querySelector('[role=button][aria-label=\"Drag to offset column\"]')")
    browser("focus", driver.by_role_name("button", "Drag to offset column"))
    before = evaluate("document.activeElement.getAttribute('aria-description')")
    browser("press", "ArrowRight")
    browser("wait", "--fn", f"document.querySelector('[role=button][aria-label=\"Drag to offset column\"]').getAttribute('aria-description')!=={driver.json.dumps(before)}")
    click("Undo")
    browser("wait", "--fn", f"document.querySelector('[role=button][aria-label=\"Drag to offset column\"]').getAttribute('aria-description')==={driver.json.dumps(before)}")
    browser("focus", driver.by_role_name("button", "Align"))
    held_space(url)
    browser("wait", "--fn", "document.activeElement.closest('#m1-layout-align-menu')!==null")
    browser("focus", driver.by_role_name("button", "Close Align"))
    held_space(url)
    check("document.activeElement.id==='m1-layout-align-trigger' && document.activeElement.getAttribute('aria-expanded')==='false'")
    print("PASS: packaged Layout native scope/tool Space and real nudge/Undo outcome", flush=True)


def tests(url):
    captures = ROOT / ".evidence/015" / datetime.now().strftime("captures-%Y%m%d-%H%M%S")
    captures.mkdir(parents=True)
    print("Screenshot directory:", captures, flush=True)
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
    actions = "[...document.querySelectorAll('[role=toolbar][aria-label=\"Actions for Column\"]')].find(e=>!e.closest('[inert],[aria-hidden=true]'))"
    check(f"{actions}.querySelectorAll('[tabindex=\"0\"]').length===1")
    browser("focus", driver.by_role_name("button", "Open Position and rotation properties for Column"))
    browser("press", "End")
    check("document.activeElement.getAttribute('aria-label')==='Activate Splay for the selected column'")
    browser("press", "Home")
    browser("press", "Enter")
    check("document.querySelector('[aria-label=\"Owner state\"]').textContent.includes(':1:')")
    browser("focus", driver.by_role_name("button", "Open Position and rotation properties for Column"))
    browser("press", "Space")
    check("document.querySelector('[aria-label=\"Owner state\"]').textContent.includes(':2:')")
    browser("focus", driver.by_role_name("button", "Activate Stagger for the selected column"))
    browser("wait", "--fn", "document.getElementById(document.activeElement.getAttribute('aria-describedby'))?.textContent.includes('Ctrl')")
    check("document.getElementById(document.activeElement.getAttribute('aria-describedby')).textContent.includes('Alt') && document.getElementById(document.activeElement.getAttribute('aria-describedby')).textContent.includes('Shift')")
    browser("press", "Escape")
    click("Transform")
    browser("wait", "--fn", "document.activeElement?.getAttribute('role')==='menuitem'")
    browser("press", "End")
    browser("press", "Home")
    browser("press", "o")
    browser("press", "Enter")
    check("document.querySelector('[aria-label=Transform]').getAttribute('aria-expanded')==='false'")
    check("document.activeElement.getAttribute('aria-label')==='Transform'")
    click("Align")
    combo = driver.by_role_name("combobox", "Alignment reference")
    browser("fill", combo, "Reference 12")
    browser("press", "ArrowDown")
    browser("press", "Enter")
    check("document.querySelector('[aria-label=\"Reference state\"]').textContent==='12'")
    browser("wait", "--fn", "!document.querySelector('[role=listbox][aria-label=\"Alignment reference\"]')")
    browser("press", "Escape")
    check("document.activeElement.getAttribute('aria-label')==='Align'")
    check("[...document.querySelectorAll('[role=dialog][data-state=closed]')].every(e=>e.hasAttribute('inert'))")
    browser("press", "Tab")
    check("document.activeElement.getAttribute('aria-label')==='Transform' && !document.activeElement.closest('[inert]')")
    browser("press", "Shift+Tab")
    check("document.activeElement.getAttribute('aria-label')==='Align'")
    click("Few references")
    click("Align")
    click("Alignment reference")
    browser("press", "Home")
    browser("press", "Enter")
    check("document.querySelector('[aria-label=\"Reference state\"]').textContent==='0'")
    browser("wait", "--fn", "!document.querySelector('[role=listbox][aria-label=\"Alignment reference\"]')")
    browser("press", "Escape")
    click("Snap settings")
    browser("focus", driver.by_role_name("button", "Close Snap"))
    browser("press", "Shift+Tab")
    print("Nonmodal backward Tab:", evaluate("({id:document.activeElement.id,name:document.activeElement.getAttribute('aria-label'),open:document.getElementById('m1-layout-snap-trigger').getAttribute('aria-expanded')})"), flush=True)
    check("document.activeElement.id==='m1-layout-snap-trigger'")
    browser("focus", driver.by_role_name("spinbutton", "Snap gap"))
    browser("press", "Tab")
    print("Nonmodal forward Tab:", evaluate("({id:document.activeElement.id,name:document.activeElement.getAttribute('aria-label'),open:document.getElementById('m1-layout-snap-trigger').getAttribute('aria-expanded')})"), flush=True)
    check("document.activeElement.getAttribute('aria-label')==='Geometry snap' && !document.activeElement.closest('[role=dialog]')")
    check("document.getElementById('m1-layout-snap-trigger').getAttribute('aria-expanded')==='false'")
    click("Snap settings")
    check("document.activeElement.getAttribute('aria-label')==='Snap increment'")
    click("Snap increment")
    browser("press", "End")
    browser("press", "Enter")
    check("document.querySelector('[aria-label=\"Owner state\"]').textContent.includes(':-0.1:')")
    check("document.activeElement.getAttribute('aria-label')==='Snap increment'")
    browser("wait", "--fn", "!document.querySelector('[role=listbox][aria-label=\"Snap increment\"]')")
    check("document.querySelector('[aria-label=\"Snap gap\"]').getAttribute('min')==='0' && document.querySelector('[aria-label=\"Snap gap\"]').getAttribute('step')==='any'")
    gap = driver.by_role_name("spinbutton", "Snap gap")
    browser("fill", gap, "0.125")
    check("document.querySelector('[aria-label=\"Snap gap\"]').validity.valid")
    browser("fill", gap, "-0.125")
    check("document.querySelector('[aria-label=\"Snap gap\"]').validity.rangeUnderflow")
    check("document.querySelector('[aria-label=\"Owner state\"]').textContent.endsWith(':-0.125')")
    browser("press", "Escape")
    check("document.activeElement.getAttribute('aria-label')==='Snap settings'")
    before = evaluate("document.querySelector('[aria-label=\"Owner state\"]').textContent")
    click("Geometry snap")
    assert evaluate("document.querySelector('[aria-label=\"Owner state\"]').textContent") != before
    for key in ["Enter", "Space"]:
        browser("focus", driver.by_role_name("button", "Geometry snap"))
        browser("wait", "--fn", "document.getElementById(document.activeElement.getAttribute('aria-describedby'))?.textContent==='Geometry snap'")
        calls = int(evaluate("document.querySelector('[aria-label=\"Snap calls\"]').textContent"))
        browser("press", key)
        assert int(evaluate("document.querySelector('[aria-label=\"Snap calls\"]').textContent")) == calls + 1
    browser("wait", "--fn", "!document.getElementById('m1-layout-snap-menu')")
    click("Snap settings")
    check("document.activeElement.getAttribute('aria-label')==='Snap increment'")
    evaluate("window.retainedPopup=document.getElementById('m1-layout-snap-menu'); window.retainedAnimation=retainedPopup.animate([{opacity:1},{opacity:0}],1000); retainedAnimation.pause()")
    browser("press", "Escape")
    check("retainedPopup===document.getElementById('m1-layout-snap-menu') && retainedPopup.hasAttribute('inert') && retainedPopup.getAttribute('data-state')==='closed'")
    browser("press", "Tab")
    check("document.activeElement.getAttribute('aria-label')==='Geometry snap' && !document.activeElement.closest('[inert]')")
    click("Snap settings")
    print("retained reopen:", evaluate("({same:retainedPopup===document.getElementById('m1-layout-snap-menu'),connected:retainedPopup.isConnected,inert:retainedPopup.hasAttribute('inert'),state:retainedPopup.getAttribute('data-state'),animation:retainedAnimation.playState,focus:document.activeElement.getAttribute('aria-label')})"), flush=True)
    check("retainedPopup===document.getElementById('m1-layout-snap-menu') && !retainedPopup.hasAttribute('inert') && document.activeElement.getAttribute('aria-label')==='Snap increment'")
    click("Few references")
    evaluate("retainedAnimation.finish(); retainedPopup.getAnimations().forEach(a=>a.finish())")
    browser("wait", "--fn", "!document.getElementById('m1-layout-snap-menu')")
    check("document.activeElement.textContent==='Few references'")
    click("Snap settings")
    click("Close Snap")
    check("document.activeElement.getAttribute('aria-label')==='Snap settings'")
    for key in ["Enter", "Space"]:
        click("Snap settings")
        browser("focus", driver.by_role_name("button", "Close Snap"))
        browser("wait", "--fn", "document.getElementById(document.activeElement.getAttribute('aria-describedby'))?.textContent==='Close Snap'")
        evaluate("window.closeClicks=0; window.closeButton=document.activeElement; window.countClose=()=>window.closeClicks++; closeButton.addEventListener('click',countClose)")
        browser("press", key)
        print("Close key:", key, "clicks:", evaluate("closeClicks"), flush=True)
        evaluate("closeButton.removeEventListener('click',countClose)")
        check("closeClicks===1 && document.getElementById('m1-layout-snap-trigger').getAttribute('aria-expanded')==='false' && document.activeElement.id==='m1-layout-snap-trigger'")
    # Themes preserve control height and spacing; font-dependent widths may differ.
    for width in [1440, 1100, 980, 390]:
        browser("set", "viewport", width, 900)
        sizes = []
        for theme in ["original", "shadcn"]:
            for appearance in ["light", "dark"]:
                evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
                evaluate("document.fonts.ready.then(()=>true)")
                check("[...document.querySelectorAll('[role=toolbar]')].every(e=>e.scrollWidth<=e.clientWidth)")
                sizes.append(evaluate("[...document.querySelectorAll('[role=toolbar]')].filter(e=>!e.closest('[inert],[aria-hidden=true]')).map(e=>[e.getBoundingClientRect().height,getComputedStyle(e).gap,getComputedStyle(e).padding])"))
                check("[...document.querySelectorAll('[role=toolbar]')].filter(e=>!e.closest('[inert],[aria-hidden=true]')).every(e=>{const r=e.getBoundingClientRect();return r.left>=0 && r.right<=innerWidth})")
                before = evaluate("['Select Matrix','Snap settings'].map(n=>{const e=[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')===n);const r=e.getBoundingClientRect();return [r.x,r.y]})")
                for selected in ["Key", "Row", "Part", "Matrix", "Column", "Matrix", "Part", "Row", "Key", "Column"]:
                    click("Select " + selected)
                    after = evaluate("['Select Matrix','Snap settings'].map(n=>{const e=[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')===n);const r=e.getBoundingClientRect();return [r.x,r.y]})")
                    assert after == before, ("stationary with scope popup open", selected, before, after)
                    browser("press", "Escape")
                    after = evaluate("['Select Matrix','Snap settings'].map(n=>{const e=[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')===n);const r=e.getBoundingClientRect();return [r.x,r.y]})")
                    assert after == before, ("stationary with scope popup closed", selected, before, after)
                click("Clear selection")
                after = evaluate("['Select Matrix','Snap settings'].map(n=>{const e=[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')===n);const r=e.getBoundingClientRect();return [r.x,r.y]})")
                assert after == before, ("stationary empty action slot", before, after)
                point = evaluate("(()=>{const r=document.querySelector('.layout-action-slot').getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2}})()")
                count = evaluate("document.querySelector('[aria-label=\"Canvas clicks\"]').textContent")
                browser("mouse", "move", point["x"], point["y"])
                browser("mouse", "down")
                browser("mouse", "up")
                assert int(evaluate("document.querySelector('[aria-label=\"Canvas clicks\"]').textContent")) == int(count) + 1
                click("Restore selection")
                browser("wait", "--fn", "!document.querySelector('[role=dialog][data-state=closed]')")
                browser("screenshot", str(captures / f"{width}-{theme}-{appearance}.png"))
        assert all(size == sizes[0] for size in sizes), (width, sizes)
    assert browser("errors").get("errors", []) == []
    print("PASS: production strips, trusted scope/action/menu/Snap keyboard, owner callbacks, precise numeric validity, four widths and four theme appearances", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", help="check the packaged application instead of the fixture")
    args = parser.parse_args()
    if args.page_url:
        try:
            production_tests(args.page_url)
        except Exception:
            capture = ROOT / ".evidence/015" / datetime.now().strftime("production-failure-%Y%m%d-%H%M%S.png")
            browser("screenshot", str(capture))
            print("Production failure screenshot:", capture, flush=True)
            raise
        finally:
            browser("close")
        return
    env = os.environ.copy()
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
    driver.run("cargo", "build", "--locked", "-p", "boardstudio-web-layout", "--example", "toolbar_strips", "--target", "wasm32-unknown-unknown", env=env)
    OUT.mkdir(parents=True, exist_ok=True)
    driver.run(str(driver.wasm_bindgen_dir(ROOT) / "wasm-bindgen"), "--target", "web", "--out-dir", str(OUT), str(ROOT / "target/wasm32-unknown-unknown/debug/examples/toolbar_strips.wasm"))
    driver.run(str(Path.home() / ".local/share/.dx/tools/tailwindcss-v4.1.5/tailwindcss"), "-i", "web/tailwind.css", "-o", str(OUT / "tailwind.css"))
    if not (OUT / "assets").exists():
        (OUT / "assets").symlink_to(ROOT / "web/assets", target_is_directory=True)
    (OUT / "index.html").write_text('<!doctype html><html lang="en"><meta charset="utf-8"><title>Layout strips</title><link rel="stylesheet" href="assets/m1.css"><link rel="stylesheet" href="tailwind.css"><style>body{margin:0}.toolbar-fixture{position:relative;padding:12px;box-sizing:border-box}</style><body><div id="main"></div><script type="module">import init from "./toolbar_strips.js";init();</script>')
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

#!/usr/bin/env python3
"""016: actual Add/Mirror operations, retained local draft, precise geometry and Undo."""
import argparse
from datetime import datetime
import importlib
import json
from pathlib import Path
import subprocess


driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "layout-adoption-016"
browser, evaluate, check, click = driver.browser, driver.evaluate, driver.check, driver.click
ROOT = Path(__file__).resolve().parents[1]


def objects_sheet():
    if not evaluate("!!document.getElementById('workspace-panel-sheet')?.matches('[role=dialog]')"):
        click("Workbench menu")
        click("Objects")
    browser("find", "role", "tab", "click", "--name", "Objects", "--exact")


def activate_with_space(name):
    browser("focus", driver.by_role_name("button", name))
    evaluate("window.spaceOwner=document.activeElement;window.spaceClicks=[];spaceOwner.addEventListener('click',e=>spaceClicks.push(e.isTrusted),{once:true})")
    browser("press", "Space")
    print("Native Space", name, evaluate("({clicks:spaceClicks,connected:spaceOwner.isConnected})"), flush=True)
    check("spaceClicks.length===1 && spaceClicks[0]")


def geometry():
    # Drawing geometry is inspected structurally; interactive controls use role/name.
    return evaluate("[...document.querySelectorAll('.m1-matrix-key')].map(e=>({matrix:e.dataset.matrixId,transform:e.getAttribute('transform'),width:e.getAttribute('width'),height:e.getAttribute('height')}))")


def space_key(event_type):
    # agent-browser press releases immediately; its documented CDP bridge can hold one trusted key.
    cdp = browser("get", "cdp-url")["cdpUrl"]
    page = browser("get", "url")["url"]
    script = """
        import {readFileSync} from 'node:fs';
        const {url,page,type}=JSON.parse(readFileSync(0,'utf8'));
        const ws=new WebSocket(url),pending=new Map();let id=0;
        const timeout=setTimeout(()=>{console.error('CDP key dispatch timed out');process.exit(1)},10000);
        ws.onmessage=({data})=>{const m=JSON.parse(data);if(m.id){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(m.error):p.resolve(m.result)}};
        const send=(method,params={},sessionId)=>new Promise((resolve,reject)=>{const next=++id;pending.set(next,{resolve,reject});ws.send(JSON.stringify({id:next,method,params,sessionId}))});
        await new Promise((resolve,reject)=>{ws.onopen=resolve;ws.onerror=reject});
        try {
            const {targetInfos}=await send('Target.getTargets');
            const pages=targetInfos.filter(t=>t.type==='page'&&t.url===page);
            if(pages.length!==1)throw Error('Expected exactly one owned Layout page');
            const {sessionId}=await send('Target.attachToTarget',{targetId:pages[0].targetId,flatten:true});
            await send('Input.dispatchKeyEvent',{type,key:' ',code:'Space',windowsVirtualKeyCode:32,nativeVirtualKeyCode:32},sessionId);
        } finally {clearTimeout(timeout);ws.close()}
    """
    subprocess.run(["node", "--input-type=module", "-e", script],
                   input=json.dumps({"url": cdp, "page": page, "type": event_type}),
                   text=True, check=True, timeout=15)


def held_space_floor():
    canvas_name = "Keyboard layout; drag components, hold Shift for range selection, hold Space and drag to pan, or use position controls"
    browser("focus", driver.by_role_name("group", canvas_name))
    evaluate("window.layoutCanvas=document.activeElement")
    point = evaluate("(()=>{const r=layoutCanvas.getBoundingClientRect();return {x:Math.round(r.x+r.width/2),y:Math.round(r.y+r.height*.75)}})()")
    check(f"document.elementFromPoint({point['x']},{point['y']}).closest('[role=group]')===layoutCanvas")

    def drag():
        browser("mouse", "move", point["x"], point["y"])
        browser("mouse", "down")
        try:
            browser("mouse", "move", point["x"] + 30, point["y"] + 20)
        finally:
            browser("mouse", "up")

    browser("focus", driver.by_role_name("button", "Group objects by"))
    evaluate("window.heldSelect=document.activeElement;window.openTransitions=[];window.heldClicks=[];window.heldKeys=[];window.selectObserver=new MutationObserver(ms=>openTransitions.push(...ms.map(m=>m.oldValue)));selectObserver.observe(heldSelect,{attributes:true,attributeFilter:['aria-expanded'],attributeOldValue:true});heldSelect.addEventListener('click',e=>heldClicks.push(e.isTrusted));window.recordHeldKey=e=>{if(e.code==='Space')heldKeys.push({type:e.type,trusted:e.isTrusted,prevented:e.defaultPrevented})};window.addEventListener('keydown',recordHeldKey);window.addEventListener('keyup',recordHeldKey)")
    before = evaluate("layoutCanvas.getAttribute('viewBox')")
    space_key("keyDown")
    try:
        assert driver.by_role_name("listbox", "Group objects by")
        drag()
        check("heldSelect.textContent.trim()==='By layout' && heldClicks.length===0 && openTransitions.filter(v=>v==='false').length===1 && !heldKeys.some(e=>e.type==='keydown')")
        assert evaluate("layoutCanvas.getAttribute('viewBox')") == before, "Select-held Space must not arm canvas pan"
    finally:
        space_key("keyUp")
    check("heldKeys.length===1 && heldKeys[0].type==='keyup' && heldKeys[0].trusted && heldClicks.length===0")
    browser("press", "Escape")
    browser("focus", driver.by_role_name("group", canvas_name))
    space_key("keyDown")
    try:
        drag()
        assert evaluate("layoutCanvas.getAttribute('viewBox')") != before, "blank-canvas Space pan must still work"
    finally:
        space_key("keyUp")
    after = evaluate("layoutCanvas.getAttribute('viewBox')")
    drag()
    assert evaluate("layoutCanvas.getAttribute('viewBox')") == after, "release must stop canvas pan"
    print("Held Select Space: one open, no native click/window keydown/pan; canvas positive pan and release", evaluate("({keys:heldKeys,transitions:openTransitions,clicks:heldClicks})"), before, after, flush=True)
    evaluate("selectObserver.disconnect();window.removeEventListener('keydown',recordHeldKey);window.removeEventListener('keyup',recordHeldKey)")
    click("Fit board")


def tests(url, captures):
    browser("set", "viewport", 1440, 900)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
    click("Project")
    browser("find", "role", "menuitem", "click", "--name", "New project", "--exact")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Keyboard setup\"]')")
    click("4 Case (optional)")
    click("Hide guide")
    click("Open Layout workspace")
    browser("focus", driver.by_role_name("button", "Group objects by"))
    evaluate("window.selectSpaces=[];window.recordSelectSpace=e=>{if(e.code==='Space')selectSpaces.push({trusted:e.isTrusted,prevented:e.defaultPrevented,name:e.target.getAttribute('aria-label')})};window.addEventListener('keydown',recordSelectSpace)")
    browser("press", "Space")
    assert driver.by_role_name("listbox", "Group objects by")
    print("Group Select Space window propagation", evaluate("selectSpaces"), flush=True)
    check("selectSpaces.length===0")
    evaluate("window.removeEventListener('keydown',recordSelectSpace)")
    browser("press", "Escape")
    check("document.activeElement.getAttribute('aria-label')==='Group objects by'")
    held_space_floor()
    click("Add object")
    assert driver.by_role_name("region", "Add")
    check("!document.querySelector('[role=dialog][aria-label=Add]') && document.getElementById('m1-objects-panel').contains(document.querySelector('[role=region][aria-label=Add]'))")
    activate_with_space("Matrix… Rows, columns & key assemblies")
    for name in ["Rows", "Columns"]:
        browser("focus", driver.by_role_name("slider", name))
        browser("press", "Home")
    activate_with_space("Create matrix")
    browser("wait", "--fn", "document.querySelectorAll('.m1-matrix-key').length===1")
    original = geometry()
    assert original[0]["transform"] == "translate(0 0) rotate(0)" and original[0]["width"] == original[0]["height"] == "18", original
    click("Add object")
    click("Mirror existing half… Create a linked half from an existing layout")
    browser("fill", driver.by_role_name("spinbutton", "Mirror axis X"), "12.123456789")
    browser("focus", driver.by_role_name("spinbutton", "Mirror axis X"))
    evaluate("window.axisDraft=document.activeElement")
    check("axisDraft.closest('[role=region]').getAttribute('aria-label')==='Mirror existing half' && !axisDraft.closest('[role=dialog][aria-label=\"Mirror existing half\"]') && document.getElementById('m1-objects-panel').contains(axisDraft)")
    check("axisDraft.labels.length===1 && axisDraft.labels[0].control===axisDraft && document.querySelectorAll('[id=\"'+axisDraft.id+'\"]').length===1")
    for width in [1440, 1100, 390]:
        browser("set", "viewport", width, 900)
        for theme in ["original", "shadcn"]:
            for appearance in ["light", "dark"]:
                evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
                if width < 980:
                    objects_sheet()
                browser("focus", driver.by_role_name("spinbutton", "Mirror axis X"))
                check("document.activeElement===axisDraft && axisDraft.value==='12.123456789'")
                flow = evaluate("""(()=>{
                    const region=axisDraft.closest('[role=region]'),card=axisDraft.closest('[data-slot=card]');
                    const r=region.getBoundingClientRect(),c=card.getBoundingClientRect(),f=axisDraft.form.getBoundingClientRect();
                    return {position:getComputedStyle(card).position,overflow:getComputedStyle(card).overflow,
                        regionHeight:r.height,cardHeight:c.height,formHeight:f.height,scroll:card.scrollTop};
                })()""")
                print("Mirror dock flow", width, theme, appearance, flow, flush=True)
                assert flow["position"] == "static" and flow["overflow"] == "visible", flow
                assert flow["regionHeight"] >= flow["cardHeight"] >= flow["formHeight"] > 100 and flow["scroll"] == 0, flow
                assert geometry() == original, "theme/width changes must preserve accepted geometry"
                browser("screenshot", str(captures / f"mirror-{width}-{theme}-{appearance}.png"))
                if width < 980:
                    click("Close panels")
                    browser("wait", "--fn", "axisDraft.closest('[inert]')!==null")
                    check("axisDraft.isConnected && axisDraft.value==='12.123456789'")
                    evaluate("axisDraft.focus()")
                    check("document.activeElement!==axisDraft")
    browser("set", "viewport", 1440, 900)
    browser("focus", driver.by_role_name("spinbutton", "Mirror axis X"))
    check("document.activeElement===axisDraft && axisDraft.value==='12.123456789'")
    activate_with_space("Create linked half")
    browser("wait", "--fn", "document.querySelectorAll('.m1-matrix-key').length===2")
    paired = geometry()
    assert original[0] in paired, paired
    mirrored = [key for key in paired if key["matrix"] != original[0]["matrix"]]
    assert len(mirrored) == 1 and mirrored[0]["transform"] == "translate(24.246913578 0) rotate(0)", paired
    assert mirrored[0]["width"] == mirrored[0]["height"] == "18", paired
    browser("fill", driver.by_role_name("spinbutton", "Origin X"), "7.125")
    browser("press", "Enter")
    browser("wait", "--fn", "[...document.querySelectorAll('.m1-matrix-key')].some(e=>e.getAttribute('transform')==='translate(7.125 0) rotate(0)')")
    moved = geometry()
    assert {key["transform"] for key in moved} == {"translate(7.125 0) rotate(0)", "translate(17.121913578 0) rotate(0)"}, moved
    assert all(key["width"] == key["height"] == "18" for key in moved)
    click("Undo")
    browser("wait", "--fn", "[...document.querySelectorAll('.m1-matrix-key')].some(e=>e.getAttribute('transform')==='translate(24.246913578 0) rotate(0)')")
    assert geometry() == paired, "one Undo restores the exact measurements and linked geometry"
    click("Undo")
    browser("wait", "--fn", "document.querySelectorAll('.m1-matrix-key').length===1")
    assert geometry() == original, "one Undo removes only the linked half"
    click("Undo")
    browser("wait", "--fn", "document.querySelectorAll('.m1-matrix-key').length===0")
    print("PASS actual Add/Mirror dock owners, retained precise local draft and hidden/inert focus across 12 combinations; exact linked geometry/measurements and three separate Undo outcomes", captures, flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", required=True)
    args = parser.parse_args()
    captures = ROOT / ".evidence/016" / datetime.now().strftime("layout-adoption-%Y%m%d-%H%M%S")
    captures.mkdir(parents=True)
    try:
        tests(args.page_url, captures)
    except Exception:
        browser("screenshot", str(captures / "failure.png"))
        print(browser("snapshot", "-i")["snapshot"], flush=True)
        raise
    finally:
        browser("close")


if __name__ == "__main__":
    main()

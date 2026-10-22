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
    check("heldButton.closest('#m1-layout-command-pill')!==null")
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


def snap_checkbox_tests(url, *, fixture=False):
    """Native settings buttons and label text retain checkbox behavior at their owner."""
    failures = []
    captures = ROOT / ".evidence/015" / datetime.now().strftime("checkboxes-%Y%m%d-%H%M%S")
    captures.mkdir(parents=True)

    def checked(name):
        return browser("get", "attr", driver.by_role_name("checkbox", name), "aria-checked")["value"]

    def geometry(enabled):
        if checked("Geometry snap") != str(enabled).lower():
            browser("find", "role", "checkbox", "click", "--name", "Geometry snap", "--exact")
        assert checked("Geometry snap") == str(enabled).lower()

    def observe(name):
        browser("focus", driver.by_role_name("checkbox", name))
        evaluate(f"window.snapBox=document.querySelector('#m1-layout-snap-menu [role=checkbox][aria-label={driver.json.dumps(name)}]'); window.snapClicks=[];window.snapChanges=0;window.snapClick=e=>snapClicks.push(e.isTrusted);snapBox.addEventListener('click',snapClick);window.snapObserver=new MutationObserver(records=>snapChanges+=records.filter(r=>r.oldValue!==snapBox.getAttribute('aria-checked')).length);snapObserver.observe(snapBox,{{attributes:true,attributeFilter:['aria-checked'],attributeOldValue:true}})")
        return int(evaluate("document.querySelector('[aria-label=\"Snap calls\"]').textContent")) if fixture else None

    def finish(name, before, calls, expected):
        state = evaluate("({clicks:snapClicks,changes:snapChanges,value:snapBox.getAttribute('aria-checked')})")
        print("Checkbox outcome:", name, before, state, flush=True)
        evaluate("snapBox.removeEventListener('click',snapClick);snapObserver.disconnect()")
        good = state == {"clicks": [True] if expected else [], "changes": expected,
                         "value": ("false" if before == "true" else "true") if expected else before}
        if fixture:
            good = good and int(evaluate("document.querySelector('[aria-label=\"Snap calls\"]').textContent")) == calls + expected
        if not good:
            failures.append((name, before, state, expected))
            browser("screenshot", str(captures / f"{len(failures)}-{name.replace(' ', '-')}.png"))

    click("Snap settings")
    browser("wait", "--fn", "document.activeElement.getAttribute('aria-label')==='Snap increment'")
    for name in ["Geometry snap", "Envelope gap"]:
        geometry(True)
        before = checked(name)
        calls = observe(name)
        try:
            held_space(url)
        except AssertionError as error:
            # Collect the two independent Space regressions before failing the run.
            failures.append((name, "held Space", str(error)))
            browser("screenshot", str(captures / f"{len(failures)}-{name.replace(' ', '-')}-Space.png"))
        finish(name, before, calls, 1)
        geometry(True)
        before = checked(name)
        calls = observe(name)
        browser("press", "Enter")
        finish(name, before, calls, 0)

    for name in ["Geometry snap", "Envelope gap"]:
        geometry(True)
        before = checked(name)
        calls = observe(name)
        point = evaluate("(()=>{const label=snapBox.closest('label'),text=[...label.childNodes].find(n=>n.nodeType===Node.TEXT_NODE&&n.textContent.trim()===snapBox.getAttribute('aria-label'));const range=document.createRange();range.selectNodeContents(text);const r=range.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2,for:label.htmlFor,id:snapBox.id}})()")
        print("Label TEXT target:", name, point, flush=True)
        browser("mouse", "move", round(point["x"]), round(point["y"]))
        browser("mouse", "down")
        browser("mouse", "up")
        finish(name + " label text", before, calls, 1)
        if not point["id"] or point["for"] != point["id"]:
            failures.append((name, "label association", point))

    geometry(False)
    check("document.querySelector('#m1-layout-snap-menu [role=checkbox][aria-label=\"Envelope gap\"]').disabled")
    before = checked("Envelope gap")
    # A disabled native button cannot take focus; Tab skips it to the numeric field.
    calls = observe("Envelope gap")
    check("document.activeElement!==snapBox")
    browser("focus", driver.by_role_name("checkbox", "Geometry snap"))
    browser("press", "Tab")
    check("document.activeElement.getAttribute('aria-label')==='Snap gap'")
    browser("press", "Space")
    point = evaluate("(()=>{const r=snapBox.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2}})()")
    browser("mouse", "move", round(point["x"]), round(point["y"]))
    browser("mouse", "down")
    browser("mouse", "up")
    # Label text must also respect the same disabled owner.
    point = evaluate("(()=>{const label=snapBox.closest('label'),text=[...label.childNodes].find(n=>n.nodeType===Node.TEXT_NODE&&n.textContent.trim()===snapBox.getAttribute('aria-label'));const range=document.createRange();range.selectNodeContents(text);const r=range.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2}})()")
    browser("mouse", "move", round(point["x"]), round(point["y"]))
    browser("mouse", "down")
    browser("mouse", "up")
    finish("disabled Envelope gap pointer/label/Space", before, calls, 0)
    geometry(True)
    values = [checked(name) for name in ["Geometry snap", "Envelope gap"]]
    browser("press", "Escape")
    click("Snap settings")
    assert [checked(name) for name in ["Geometry snap", "Envelope gap"]] == values, "Owner values must survive reopening"
    browser("press", "Escape")
    print("Checkbox failures:", failures, "evidence:", captures, flush=True)
    assert not failures, failures


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
    click("Collapse inspector")
    browser("wait", "--fn", "document.querySelector('[aria-label=\"Show inspector\"]')?.checkVisibility()")
    browser("focus", driver.by_role_name("button", "Select Key"))
    browser("press", "Enter")
    browser("focus", driver.by_role_name("button", "Open Inspector"))
    held_space(url)
    browser("wait", "--fn", "document.querySelector('[aria-label=\"Collapse inspector\"]')?.checkVisibility()")
    check("document.querySelector('[aria-label=\"Select Key\"]').getAttribute('aria-expanded')==='false'")
    click("Align")
    check("document.querySelector('#m1-layout-align-menu .layout-alignment-actions button').disabled")
    point = evaluate("(()=>{const b=document.querySelector('#m1-layout-align-menu .layout-alignment-actions button'),r=b.getBoundingClientRect();window.disabledClicks=0;b.addEventListener('click',()=>disabledClicks++);return {x:r.x+r.width/2,y:r.y+r.height/2}})()")
    browser("mouse", "move", round(point["x"]), round(point["y"]))
    browser("mouse", "down")
    browser("mouse", "up")
    check("disabledClicks===0 && document.getElementById('m1-layout-align-trigger').getAttribute('aria-expanded')==='true'")
    browser("press", "Escape")
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
    browser("wait", "--fn", "!document.querySelector('#m1-layout-align-menu .layout-alignment-actions button').disabled")
    browser("focus", driver.by_role_name("button", "Left"))
    held_space(url)
    browser("wait", "--fn", "[...document.querySelectorAll('[role=status]')].some(e=>e.textContent.startsWith('Aligned selection'))")
    check("document.activeElement.id==='m1-layout-align-trigger' && document.activeElement.getAttribute('aria-expanded')==='false'")
    click("Undo")
    browser("focus", driver.by_role_name("button", "Align"))
    held_space(url)
    browser("wait", "--fn", "document.activeElement.closest('#m1-layout-align-menu')!==null")
    browser("focus", driver.by_role_name("button", "Relationships"))
    held_space(url)
    print("Inspector tabs:", evaluate("[...document.querySelectorAll('[role=tab]')].map(e=>({name:e.textContent,selected:e.getAttribute('aria-selected')}))"), flush=True)
    browser("wait", "--fn", "[...document.querySelectorAll('[role=tab]')].some(e=>e.textContent==='Relations' && e.getAttribute('aria-selected')==='true' && e.checkVisibility())")
    assert browser("get", "attr", driver.by_role_name("tab", "Relations"), "aria-selected")["value"] == "true"
    check("document.activeElement.id==='m1-layout-align-trigger' && document.activeElement.getAttribute('aria-expanded')==='false'")
    browser("focus", driver.by_role_name("button", "Align"))
    held_space(url)
    browser("wait", "--fn", "document.activeElement.closest('#m1-layout-align-menu')!==null")
    browser("focus", driver.by_role_name("button", "Close Align"))
    held_space(url)
    check("document.activeElement.id==='m1-layout-align-trigger' && document.activeElement.getAttribute('aria-expanded')==='false'")
    geometry = driver.by_role_name("button", "Geometry snap")
    before = browser("get", "attr", geometry, "aria-pressed")["value"]
    for expected in ["false" if before == "true" else "true", before]:
        browser("focus", driver.by_role_name("button", "Geometry snap"))
        held_space(url)
        assert browser("get", "attr", driver.by_role_name("button", "Geometry snap"), "aria-pressed")["value"] == expected
    snap_checkbox_tests(url)

    def check_tool_hint(capture=None):
        browser("wait", "--fn", "document.getElementById(document.activeElement.getAttribute('aria-describedby'))?.textContent.includes('Alt')")
        check("(()=>{const t=document.getElementById(document.activeElement.getAttribute('aria-describedby')),r=t.getBoundingClientRect();return t.textContent.includes('Shift') && r.left>=0 && r.right<=innerWidth && r.top>=0 && r.bottom<=innerHeight && t.checkVisibility()})()")
        print("Hint bounds:", evaluate("(()=>{const b=document.activeElement,t=document.getElementById(b.getAttribute('aria-describedby')),g=b.closest('[role=toolbar]');return {label:b.getAttribute('aria-label'),text:t.textContent,role:t.getAttribute('role'),class:t.className,parentSame:t.offsetParent===g,parent:t.offsetParent?.outerHTML.slice(0,180),group:g.getBoundingClientRect().toJSON(),hint:t.getBoundingClientRect().toJSON(),scroll:t.scrollWidth,client:t.clientWidth}})()"), flush=True)
        check("(()=>{const b=document.activeElement,t=document.getElementById(b.getAttribute('aria-describedby')),g=b.closest('[role=toolbar]'),r=t.getBoundingClientRect(),a=g.getBoundingClientRect();return t.getAttribute('role')==='tooltip' && t.classList.contains('layout-tool-hint') && t.offsetParent===g && r.left+1/64>=a.left && r.right<=a.right+1/64 && r.width<=a.width && t.scrollWidth<=t.clientWidth && (t.textContent.includes('Ctrl') === b.getAttribute('aria-label').includes('Stagger'))})()")
        check("(()=>{const t=document.getElementById(document.activeElement.getAttribute('aria-describedby')),r=t.getBoundingClientRect();for(let p=t.parentElement;p;p=p.parentElement){const s=getComputedStyle(p),a=p.getBoundingClientRect();if (/(hidden|clip|auto|scroll)/.test(s.overflowX) && (r.left<a.left || r.right>a.right)) return false;if (/(hidden|clip|auto|scroll)/.test(s.overflowY) && (r.top<a.top || r.bottom>a.bottom)) return false;}return true})()")
        print("Tool hint:", evaluate("({control:document.activeElement.getAttribute('aria-label'),text:document.getElementById(document.activeElement.getAttribute('aria-describedby')).textContent})"), flush=True)
        evaluate("window.widthFocus=document.activeElement;window.widthTooltipId=widthFocus.getAttribute('aria-describedby');window.widthControlState=JSON.stringify([...document.querySelectorAll('.layout-toolbar-strips button')].filter(e=>!e.closest('[hidden],[inert],[aria-hidden=true]') && e.checkVisibility()).map(e=>[e.getAttribute('aria-label'),e.getAttribute('aria-expanded'),e.getAttribute('aria-pressed')]));")
        check("!document.querySelector('.layout-toolbar-strips :is([role=menu],[role=dialog])[data-state=open]')")
        check("[...document.querySelectorAll('.layout-toolbar-strips button')].filter(e=>!e.closest('[inert],[aria-hidden=true]') && e.checkVisibility()).every(e=>{const r=e.getBoundingClientRect();return r.left>=0 && r.right<=innerWidth})")
        if capture is not None:
            browser("screenshot", str(capture))
        browser("press", "Escape")
        browser("wait", "--fn", "!document.getElementById(widthTooltipId)")
        print("Tooltip dismissal state:", evaluate("({sameFocus:document.activeElement===widthFocus,before:JSON.parse(widthControlState),after:[...document.querySelectorAll('.layout-toolbar-strips button')].filter(e=>!e.closest('[hidden],[inert],[aria-hidden=true]') && e.checkVisibility()).map(e=>[e.getAttribute('aria-label'),e.getAttribute('aria-expanded'),e.getAttribute('aria-pressed')]),open:[...document.querySelectorAll('.layout-toolbar-strips :is([role=menu],[role=dialog])[data-state=open]')].map(e=>e.id)})"), flush=True)
        check("document.activeElement===widthFocus && JSON.stringify([...document.querySelectorAll('.layout-toolbar-strips button')].filter(e=>!e.closest('[hidden],[inert],[aria-hidden=true]') && e.checkVisibility()).map(e=>[e.getAttribute('aria-label'),e.getAttribute('aria-expanded'),e.getAttribute('aria-pressed')]))===widthControlState && !document.querySelector('.layout-toolbar-strips :is([role=menu],[role=dialog])[data-state=open]')")

    captures = ROOT / ".evidence/015" / datetime.now().strftime("page-captures-%Y%m%d-%H%M%S")
    captures.mkdir(parents=True)
    print("Packaged strip screenshots:", captures, flush=True)
    for width in [1440, 1100, 980, 1018, 1020, 1022, 390]:
        browser("set", "viewport", width, 900)
        if width == 390:
            # Normal Inspector-sheet setup, so the strip captures remain unobscured.
            click("Close panels")
            browser("wait", "--fn", "![...document.querySelectorAll('[role=dialog][aria-label=\"Workspace panels\"]')].some(e=>!e.closest('[hidden],[inert],[aria-hidden=true]') && e.checkVisibility())")
        for theme in ["original", "shadcn"]:
            for appearance in ["light", "dark"]:
                evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
                evaluate("document.fonts.ready.then(()=>true)")
                anchors = "['m1-layout-snap-trigger'].map(id=>{const r=document.getElementById(id).getBoundingClientRect();return [r.x,r.y]}).concat([...document.querySelectorAll('.layout-scope-strip')].map(e=>{const r=e.getBoundingClientRect();return [r.x,r.y]}))"
                before = evaluate(anchors)
                hinted_scopes = set()
                for selected in ["Key", "Row", "Part", "Matrix", "Column", "Matrix", "Part", "Row", "Key", "Column"]:
                    click("Select " + selected)
                    assert evaluate(anchors) == before, (width, theme, "open", selected, before, evaluate(anchors))
                    browser("press", "Escape")
                    assert evaluate(anchors) == before, (width, theme, "closed", selected, before, evaluate(anchors))
                    if selected not in hinted_scopes:
                        hinted_scopes.add(selected)
                        labels = evaluate("[...document.querySelectorAll('.layout-action-slot button[aria-label^=Activate]')].filter(e=>!e.closest('[inert],[aria-hidden=true]') && e.checkVisibility()).map(e=>e.getAttribute('aria-label'))")
                        for label in labels:
                            browser("focus", driver.by_role_name("button", label))
                            check_tool_hint(captures / f"{width}-{theme}-{appearance}-stagger.png" if width == 390 and selected == "Column" and 'Stagger' in label else None)
                browser("focus", driver.by_role_name("button", "Select Column"))
                browser("press", "Home")
                check("document.activeElement.getAttribute('aria-label')==='Select Matrix'")
                browser("press", "End")
                check("document.activeElement.getAttribute('aria-label')==='Select Part'")
                browser("focus", driver.by_role_name("button", "Open Position and rotation properties for Column"))
                browser("press", "End")
                check("document.activeElement.getAttribute('aria-label')==='Activate Splay for the selected column'")
                check_tool_hint(captures / f"{width}-{theme}-{appearance}-tooltip.png")
                browser("screenshot", str(captures / f"{width}-{theme}-{appearance}.png"))
                metrics = evaluate("[...document.querySelectorAll('.layout-scope-strip,.layout-context-strip,.layout-toolbar-strips [role=toolbar]')].filter(e=>!e.closest('[inert],[aria-hidden=true]')).map(e=>({name:e.getAttribute('aria-label')||e.className,scroll:e.scrollWidth,client:e.clientWidth,left:e.getBoundingClientRect().left,right:e.getBoundingClientRect().right}))")
                assert all(m["scroll"] <= m["client"] and m["left"] >= 0 and m["right"] <= width for m in metrics), (width, theme, appearance, metrics)
    print("PASS: packaged Layout native scope/tool Space and real nudge/Undo outcome", flush=True)


def tests(url):
    captures = ROOT / ".evidence/015" / datetime.now().strftime("captures-%Y%m%d-%H%M%S")
    captures.mkdir(parents=True)
    print("Screenshot directory:", captures, flush=True)
    browser("set", "viewport", 1440, 900)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Select Column\"]')")
    snap_checkbox_tests(url, fixture=True)
    click("Clear selection")
    click("Transform")
    browser("wait", "--fn", "document.activeElement.id==='m1-layout-transform-menu'")
    check("!document.activeElement.querySelector('[role^=menuitem]:not([aria-disabled=true])')")
    # Updating availability in place must leave enabled navigation to the copied menu.
    evaluate("[...document.querySelectorAll('button')].find(e=>e.textContent==='Restore selection').click()")
    browser("wait", "--fn", "!!document.querySelector('#m1-layout-transform-menu [role=menuitem]')")
    browser("press", "Home")
    check("document.activeElement.getAttribute('role')==='menuitem' && document.activeElement.getAttribute('aria-disabled')==='false'")
    browser("press", "End")
    check("document.activeElement.textContent==='Origin'")
    browser("press", "Escape")
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
    for width in [1440, 1100, 980, 390, 374, 376, 378, 336]:
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
    print("PASS: production strips, empty/enabled Transform focus, trusted keyboard, owner callbacks, numeric validity, requested and wrap-boundary widths x four theme appearances", flush=True)


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

#!/usr/bin/env python3
"""Ticket 012: role/name checks against the packaged production workbench."""
import argparse
import atexit
import json
import re
from pathlib import Path
import subprocess
from uuid import uuid4


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url")
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    session = "palette-012-" + uuid4().hex

    def browser(*command):
        result = subprocess.run(["agent-browser", "--session", session, *command],
                                text=True, capture_output=True, timeout=90)
        print(" ".join(command), result.stdout, result.stderr, flush=True)
        if result.returncode:
            for diagnostic in (("snapshot",), ("console",), ("errors",), ("screenshot", str((args.evidence / "failure.png").resolve()))):
                capture = subprocess.run(["agent-browser", "--session", session, *diagnostic], text=True, capture_output=True, timeout=30)
                print("DIAGNOSTIC", diagnostic, capture.stdout, capture.stderr, flush=True)
        assert result.returncode == 0, f"{command}: {result.stdout}{result.stderr}"
        return result.stdout

    def action(role, name, verb="click", text=None):
        if verb == "focus":
            refs = role_refs(role, name)
            assert len(refs) == 1, (role, name, refs)
            browser("focus", "@" + refs[0])
            return
        command = ["find", "role", role, verb]
        if text is not None:
            command.append(text)
        browser(*command, "--name", name, "--exact")

    def role_refs(role, name):
        snapshot = browser("snapshot", "-i")
        return re.findall(r'^\s*- ' + re.escape(role) + r' "' + re.escape(name) + r'" \[[^\]\n]*\bref=(e\d+)', snapshot, re.MULTILINE)

    def check(js):
        browser("eval", f"(() => {{ {js} return true; }})()")

    def opened():
        # First meaningful RED: the real incumbent Search button does nothing.
        browser("wait", '[role="dialog"][aria-labelledby]')
        check("""
            const dialog = document.querySelector('[role=dialog]');
            if (dialog.getAttribute('aria-modal') !== 'true') throw Error('Palette must be modal');
            for (const attr of ['aria-labelledby', 'aria-describedby']) {
                if (!document.getElementById(dialog.getAttribute(attr))) throw Error(attr + ' must resolve');
            }
            if (document.activeElement.getAttribute('role') !== 'combobox') throw Error('Search must receive focus');
        """)

    def choose(name):
        browser("press", "Control+k")
        opened()
        action("combobox", "Search commands and objects", "fill", name)
        if name == "Snap settings":
            check("if (document.querySelectorAll('[role=option][aria-label=\"Snap settings\"]').length !== 1) throw Error('Snap command must have one owner');")
        action("option", name)
        browser("wait", "--fn", "!document.querySelector('[role=combobox][aria-label=\"Search commands and objects\"]')")

    def pan_canvas():
        canvas = '[role="group"][aria-label^="Keyboard layout;"]'
        browser("focus", canvas)
        check("""
            const canvas = document.activeElement;
            if (!canvas.getAttribute('aria-label')?.startsWith('Keyboard layout;')) throw Error('Pan test must focus the canvas');
            window.palettePanBefore = canvas.getAttribute('viewBox');
            window.palettePanEvents = [];
            for (const type of ['keydown', 'keyup', 'pointerdown', 'pointerup']) canvas.addEventListener(type, e => palettePanEvents.push({type, trusted:e.isTrusted}));
        """)
        box = json.loads(browser("eval", "(() => { const b = document.activeElement.getBoundingClientRect(); return {x:b.x, y:b.y, width:b.width, height:b.height}; })()"))
        endpoint = browser("get", "cdp-url").strip()
        # agent-browser press releases the key immediately; CDP holds it across the drag.
        result = subprocess.run(["node", "--input-type=module", "-e", """
            import assert from 'node:assert/strict';
            const [endpoint, url, box] = process.argv.slice(1);
            const ws = new WebSocket(endpoint);
            await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
            let next = 0;
            const pending = new Map();
            ws.onmessage = event => {
                const reply = JSON.parse(event.data);
                if (pending.has(reply.id)) {
                    const {resolve, reject} = pending.get(reply.id);
                    pending.delete(reply.id);
                    reply.error ? reject(Error(JSON.stringify(reply.error))) : resolve(reply.result);
                }
            };
            const send = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
                const id = ++next;
                pending.set(id, {resolve, reject});
                ws.send(JSON.stringify({id, method, params, sessionId}));
            });
            const targets = (await send('Target.getTargets')).targetInfos.filter(t => t.type === 'page' && t.url.startsWith(url));
            assert.equal(targets.length, 1, 'pan only the current palette test page');
            const {sessionId} = await send('Target.attachToTarget', {targetId: targets[0].targetId, flatten: true});
            const input = (method, params) => send('Input.' + method, params, sessionId);
            const bounds = JSON.parse(box);
            const x = bounds.x + bounds.width * 0.55, y = bounds.y + bounds.height * 0.55;
            try {
                await input('dispatchKeyEvent', {type:'keyDown', key:' ', code:'Space', windowsVirtualKeyCode:32});
                await input('dispatchMouseEvent', {type:'mouseMoved', x, y});
                await input('dispatchMouseEvent', {type:'mousePressed', x, y, button:'left', buttons:1, clickCount:1});
                await input('dispatchMouseEvent', {type:'mouseMoved', x:x+75, y:y+40, button:'left', buttons:1});
                await input('dispatchMouseEvent', {type:'mouseReleased', x:x+75, y:y+40, button:'left', buttons:0, clickCount:1});
            } finally {
                await input('dispatchKeyEvent', {type:'keyUp', key:' ', code:'Space', windowsVirtualKeyCode:32});
                ws.close();
            }
        """, endpoint, args.url, json.dumps(box)], text=True, capture_output=True, timeout=60)
        print("Held-Space canvas pan", result.stdout, result.stderr, flush=True)
        assert result.returncode == 0, "Trusted held-Space input failed"
        browser("wait", "--fn", f"document.querySelector({json.dumps(canvas)}).getAttribute('viewBox') !== palettePanBefore")
        check("""
            for (const type of ['keydown', 'keyup', 'pointerdown', 'pointerup']) {
                if (!palettePanEvents.some(e => e.type === type && e.trusted)) throw Error('Pan missing trusted ' + type);
            }
            if (document.querySelector('[aria-label="Snap settings"]').getAttribute('aria-expanded') !== 'false') throw Error('Canvas pan changed Snap');
        """)

    browser("set", "viewport", "1440", "960")
    browser("open", args.url)
    atexit.register(browser, "close")
    browser("wait", '[aria-label="Search commands"]')
    check("""
        const search = document.querySelector('[aria-label="Search commands"]');
        if (search.getAttribute('aria-haspopup') !== 'dialog' || search.getAttribute('aria-expanded') !== 'false' || search.hasAttribute('aria-controls')) throw Error('Closed Search popup ARIA');
    """)
    action("button", "Search commands")
    opened()
    check("""
        const search = document.querySelector('[aria-label="Search commands"]');
        const popup = document.getElementById(search.getAttribute('aria-controls'));
        if (search.getAttribute('aria-expanded') !== 'true' || !popup?.querySelector('[role=dialog]')) throw Error('Open Search popup ARIA must resolve');
    """)
    check("""
        const rename = document.querySelector('[role=option][aria-label="Rename project"]');
        if (rename?.getAttribute('aria-disabled') !== 'true') throw Error('Rename without a project must be unavailable');
    """)
    action("combobox", "Search commands and objects", "fill", "Workspace settings")
    browser("press", "ArrowDown")
    check("""
        const input = document.activeElement;
        if (!document.getElementById(input.getAttribute('aria-activedescendant'))) throw Error('Active option must resolve');
    """)
    browser("press", "Enter")
    browser("wait", '[role="menuitemradio"]')
    browser("press", "Escape")
    browser("press", "Control+k")
    opened()
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog]')")
    check("if (document.activeElement.getAttribute('aria-label') !== 'Settings') throw Error('Shortcut must return focus to opener');")
    check("if (document.querySelector('[aria-label=\"Search commands\"]').getAttribute('aria-expanded') !== 'false') throw Error('Closed Search ARIA must refresh without dropping command owners');")
    browser("press", "Meta+k")
    opened()
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog]')")
    action("button", "Project")
    action("menuitem", "New project")
    for name in ("Continue to layout", "Hide guide", "Cancel"):
        browser("wait", "--text", name)
        action("button", name)
    browser("press", "Control+k")
    opened()
    action("combobox", "Search commands and objects", "fill", "PCB")
    action("option", "PCB")
    browser("wait", '[aria-label="Open PCB workspace"][aria-current="page"]')
    browser("press", "Control+k")
    opened()
    check("""
        const names = [...document.querySelectorAll('[role=option]')].map(e => e.getAttribute('aria-label'));
        if (names.includes('Select Matrix') || names.includes('Update preview')) throw Error('Other workspace commands must be hidden');
        if (names.some(n => /Delete|Unlink/.test(n))) throw Error('Destructive command');
    """)
    browser("press", "Tab")
    check("if (!document.querySelector('[role=dialog]').contains(document.activeElement)) throw Error('Tab escaped');")
    browser("press", "Shift+Tab")
    check("if (!document.querySelector('[role=dialog]').contains(document.activeElement)) throw Error('Shift Tab escaped');")
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog]')")
    choose("Layout")
    check("""
        const triggers = document.querySelectorAll('[aria-label="Snap settings"]');
        if (triggers.length !== 1 || !triggers[0].checkVisibility() || document.querySelector('.m1-layout-snap-dock')) throw Error('Canonical canvas Snap trigger must be visible and unique, without a dock duplicate');
    """)
    # Keep the palette open while crossing the dock breakpoint: dispatch must read it now.
    check("""
        localStorage.setItem('boardstudio:v2:panel:left', JSON.stringify({mode:'pinned',width:253}));
        localStorage.setItem('boardstudio:v2:panel:right', JSON.stringify({mode:'pinned',width:347}));
    """)
    browser("reload")
    browser("wait", '[aria-label="Saved locally"]')

    def panels(left, right, tab=None):
        browser("wait", "--fn", f"JSON.parse(localStorage.getItem('boardstudio:v2:panel:left')).mode === '{left}' && JSON.parse(localStorage.getItem('boardstudio:v2:panel:right')).mode === '{right}'")
        check(f"""
            const left = JSON.parse(localStorage.getItem('boardstudio:v2:panel:left'));
            const right = JSON.parse(localStorage.getItem('boardstudio:v2:panel:right'));
            if (left.mode !== '{left}' || left.width !== 253 || right.mode !== '{right}' || right.width !== 347) throw Error('Dock command changed the wrong mode or persisted width: ' + JSON.stringify({{left,right}}));
        """)
        if tab:
            browser("wait", '[role=dialog][aria-label="Workspace panels"]')
            check(f"if (document.querySelector('[role=tab][aria-selected=true]').textContent !== '{tab}') throw Error('Dock command selected wrong side');")
        else:
            browser("wait", "--fn", "!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]')")

    def dock(label, before, after):
        browser("set", "viewport", str(before), "960")
        browser("press", "Control+k")
        opened()
        action("combobox", "Search commands and objects", "fill", label)
        check(f"if (!document.querySelector('[role=option][aria-label={json.dumps(label)}]')) throw Error('Missing View command: {label}');")
        browser("press", "ArrowDown")
        browser("set", "viewport", str(after), "960")
        check(f"if (document.getElementById(document.activeElement.getAttribute('aria-activedescendant'))?.getAttribute('aria-label') !== {json.dumps(label)}) throw Error('Selected View command must survive the viewport change');")
        browser("press", "Enter")
        browser("wait", "--fn", "!document.querySelector('[role=combobox][aria-label=\"Search commands and objects\"]')")

    dock("Show/hide Objects", 980, 979)
    panels("pinned", "pinned", "Objects")
    dock("Show/hide Inspector", 979, 979)
    panels("pinned", "pinned", "Inspector")
    dock("Show/hide Objects", 979, 979)
    panels("pinned", "pinned", "Objects")
    dock("Show/hide Objects", 979, 979)
    panels("pinned", "pinned")
    dock("Show/hide Inspector", 979, 979)
    panels("pinned", "pinned", "Inspector")
    dock("Show/hide Inspector", 979, 979)
    panels("pinned", "pinned")
    dock("Show/hide Objects", 979, 980)
    panels("collapsed", "pinned")
    dock("Show/hide Inspector", 980, 980)
    panels("collapsed", "collapsed")
    dock("Show/hide Objects", 980, 980)
    panels("pinned", "collapsed")
    dock("Show/hide Inspector", 980, 980)
    panels("pinned", "pinned")
    dock("Show/hide Objects", 980, 980)
    panels("collapsed", "pinned")
    choose("Add object")
    check("if (!document.querySelector('[role=dialog][aria-label=Add]')?.checkVisibility()) throw Error('Add command must reveal its collapsed Objects owner');")
    panels("pinned", "pinned")
    action("button", "← Back to objects")
    browser("wait", "--fn", "document.activeElement.id === 'm1-add-object-trigger'")
    dock("Show/hide Inspector", 980, 979)
    panels("pinned", "pinned", "Inspector")
    choose("Add object")
    panels("pinned", "pinned", "Objects")
    check("if (!document.querySelector('[role=dialog][aria-label=Add]')?.checkVisibility()) throw Error('Compact Add must reveal Objects over Inspector');")
    action("button", "← Back to objects", "focus")
    check("if (!document.activeElement.closest('[role=dialog][aria-label=Add]')) throw Error('Add Escape must originate in its non-modal task surface');")
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog][aria-label=Add]') && document.activeElement.id === 'm1-add-object-trigger'")
    panels("pinned", "pinned", "Objects")
    dock("Show/hide Objects", 979, 979)
    panels("pinned", "pinned")
    browser("set", "viewport", "980", "960")
    browser("reload")
    browser("wait", '[aria-label="Saved locally"]')
    panels("pinned", "pinned")
    browser("set", "viewport", "1440", "960")
    # Canvas Snap and the palette use the same menu and settings owner.
    choose("Snap settings")
    browser("wait", '[aria-label="Snap settings"][aria-expanded="true"]')
    browser("wait", "--fn", "document.activeElement.getAttribute('aria-label') === 'Snap increment'")
    check("if (document.querySelectorAll('#m1-layout-snap-menu').length !== 1) throw Error('Snap command must open one menu');")
    browser("select", '[aria-label="Snap increment"]', "0.5")
    action("checkbox", "Geometry snap")
    check("""
        const checkbox = [...document.querySelectorAll('[role=checkbox], input[type=checkbox]')].find(e => e.closest('label')?.textContent.trim() === 'Geometry snap');
        const enabled = checkbox.checked ?? checkbox.getAttribute('aria-checked') === 'true';
        if (document.querySelector('button[aria-label="Geometry snap"]').getAttribute('aria-pressed') !== String(enabled)) throw Error('Snap options must use the native geometry owner');
    """)
    action("checkbox", "Geometry snap")
    action("spinbutton", "Snap gap", "fill", "0.75")
    browser("press", "Escape")
    browser("wait", '[aria-label="Snap settings"][aria-expanded="false"]')
    browser("wait", "--fn", "document.activeElement.getAttribute('aria-label') === 'Snap settings'")
    action("button", "Snap settings")
    browser("wait", '[aria-label="Snap increment"]')
    check("if (document.querySelector('[aria-label=\"Snap increment\"]').value !== '0.5' || document.querySelector('[aria-label=\"Snap gap\"]').value !== '0.75') throw Error('Snap settings callback values must survive closing/reopening');")
    action("button", "Snap settings", "focus")
    check("""
        window.paletteSnapEvents = [];
        for (const type of ['keydown', 'keyup', 'click']) document.activeElement.addEventListener(type, e => paletteSnapEvents.push({type, key:e.key, trusted:e.isTrusted}));
    """)
    browser("press", "Enter")
    browser("wait", '[aria-label="Snap settings"][aria-expanded="false"]')
    check("if (document.activeElement.getAttribute('aria-label') !== 'Snap settings') throw Error('Snap must retain focus after Enter');")
    check("if (paletteSnapEvents.filter(e=>e.type==='click').length !== 1) throw Error('Enter must toggle once');")
    browser("press", "Space")
    browser("wait", '[aria-label="Snap settings"][aria-expanded="true"]')
    check("""
        if (paletteSnapEvents.filter(e=>e.type==='click').length !== 2) throw Error('Space must toggle exactly once');
        if (!paletteSnapEvents.some(e=>e.type==='keyup' && e.key===' ' && e.trusted)) throw Error('Space must use trusted keyup activation');
    """)
    browser("wait", "--fn", "document.activeElement.getAttribute('aria-label') === 'Snap increment'")
    browser("press", "Control+k")
    opened()
    action("combobox", "Search commands and objects", "fill", "Position & rotation")
    check("""
        const option = document.querySelector('[role=option][aria-label="Position & rotation"]');
        if (option?.getAttribute('aria-disabled') !== 'true' || !option.textContent.includes('Select a matrix')) throw Error('Unavailable tools must explain why');
    """)
    browser("press", "Enter")
    check("if (!document.querySelector('[role=dialog]')) throw Error('Unavailable command must not dispatch');")
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog]')")
    check("if (document.querySelector('[aria-label=\"Snap settings\"]').getAttribute('aria-expanded') !== 'true') throw Error('Palette Escape closed the underlying tool');")
    browser("press", "Escape")
    browser("wait", '[aria-label="Snap settings"][aria-expanded="false"]')
    browser("wait", "--fn", "document.activeElement.getAttribute('aria-label') === 'Snap settings'")
    pan_canvas()
    # Open the existing Library rename from a different workspace.
    choose("Rename project")
    browser("wait", '[role="textbox"][aria-label="Project name"], input[aria-label="Project name"]')
    check("if (document.activeElement.getAttribute('aria-label') !== 'Project name') throw Error('Rename must focus the real draft field');")
    action("textbox", "Project name", "fill", "Palette committed name")
    browser("press", "Space")
    check("if (document.activeElement.value !== 'Palette committed name ') throw Error('Space must keep typing inside a field');")
    browser("press", "Control+k")
    opened()
    browser("wait", "--fn", "document.querySelector('[aria-label=Project]').textContent.trim() === 'Palette committed name'")
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog]')")
    check("if (document.activeElement.getAttribute('aria-label') !== 'Project name') throw Error('Field shortcut must return to its opener');")
    action("textbox", "Project name", "fill", "Cancelled name")
    browser("press", "Escape")
    check("if (document.querySelector('[aria-label=Project]').textContent.trim() !== 'Palette committed name') throw Error('Escape must preserve the owner cancel rule');")
    choose("PCB")
    choose("Open demo")
    browser("wait", '[aria-label="Demo keyboards"]')
    check("if (document.activeElement.textContent.trim() !== 'Open demo') throw Error('Demo must focus the existing chooser');")
    # The existing chooser still opens a real demo copy.
    demo_refs = role_refs("button", "Open demo copy")
    assert demo_refs, "The real demo chooser must have actions"
    browser("click", "@" + demo_refs[0])
    browser("wait", '[aria-label="Open Layout workspace"]')
    browser("wait", "--fn", "document.querySelector('[aria-label=Project]').textContent.trim() !== 'Palette committed name'")
    choose("Layout")
    choose("Add object")
    browser("wait", "--text", "Matrix…")
    browser("find", "role", "button", "click", "--name", "Matrix…")
    browser("wait", '[aria-label="New matrix"]')
    action("button", "Create matrix")
    browser("wait", "--fn", "!document.querySelector('[aria-label=\"New matrix\"]')")
    browser("press", "Control+k")
    opened()
    check("if (!document.querySelector('[role=group][aria-label=Objects] [role=option]')) throw Error('Demo objects must be searchable');")
    objects = json.loads(browser("eval", "[...document.querySelectorAll('[role=group][aria-label=Objects] [role=option]')].map(e => e.getAttribute('aria-label'))"))
    browser("screenshot", str((args.evidence / "palette-open.png").resolve()))
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog]')")
    for kind, selector in (
        ("Matrix ", '[aria-label="Matrix inspector"]'),
        ("Key ", '[aria-label="Key enabled"]'),
        ("Part ", '[aria-label="Component inspector"]'),
        ("outline", '[aria-label="Board outline"]'),
    ):
        matches = [name for name in objects
                   if (name.endswith(" Outline") if kind == "outline" else name.startswith(kind))]
        assert matches, f"Demo must exercise {kind} object search"
        choose(matches[0])
        browser("wait", selector)
        if kind == "Part ":
            check(f"if (document.querySelector('[aria-label=\"Component inspector\"] h2').textContent !== {json.dumps(matches[0][5:])}) throw Error('Part search must select its actual owner');")
    choose("Align")
    browser("wait", '[aria-label="Close Align"]')
    browser("press", "Escape")
    choose("Transform")
    browser("wait", '[aria-label="Close Transform"]')
    browser("press", "Escape")
    choose("Export")
    browser("press", "Control+k")
    opened()
    action("combobox", "Search commands and objects", "fill", "Export ZMK firmware")
    check("""
        const option = document.querySelector('[role=option][aria-label="Export ZMK firmware"]');
        if (option?.getAttribute('aria-disabled') !== 'true' || !option.textContent.includes('resolve controller wiring')) throw Error('Export must mirror the actual blocker');
    """)
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog]')")
    for theme in ("Original", "shadcn"):
        action("button", "Settings")
        action("menuitemradio", theme)
        browser("wait", "--fn", "!document.querySelector('[role=menu]')")
        for appearance in ("Light", "Dark"):
            action("button", "Settings")
            action("menuitemradio", appearance)
            browser("wait", "--fn", f"!document.querySelector('[role=menu]') && document.documentElement.dataset.theme === '{theme.lower()}' && document.documentElement.dataset.appearance === '{appearance.lower()}'")
            browser("press", "Control+k")
            opened()
            check("""
                const dialog = document.querySelector('[role=dialog]').getBoundingClientRect();
                if (dialog.left < 0 || dialog.right > innerWidth || dialog.top < 0 || dialog.bottom > innerHeight) throw Error('Palette must fit the viewport');
            """)
            browser("screenshot", str((args.evidence / f"palette-{theme.lower()}-{appearance.lower()}.png").resolve()))
            browser("press", "Escape")
            browser("wait", "--fn", "!document.querySelector('[role=dialog]')")
    browser("set", "viewport", "375", "800")
    browser("press", "Control+k")
    opened()
    check("""
        const dialog = document.querySelector('[role=dialog]').getBoundingClientRect();
        if (dialog.left < 0 || dialog.right > innerWidth || dialog.top < 0 || dialog.bottom > innerHeight) throw Error('Narrow palette must fit the viewport');
    """)
    browser("screenshot", str((args.evidence / "palette-narrow.png").resolve()))
    browser("press", "Escape")
    browser("screenshot", str((args.evidence / "palette-closed.png").resolve()))
    print("PASS: production palette dialog, combobox, navigation, context, focus and keyboard", flush=True)


if __name__ == "__main__":
    main()

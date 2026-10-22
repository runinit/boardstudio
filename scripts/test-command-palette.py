#!/usr/bin/env python3
"""Ticket 012: role/name checks against the packaged production workbench."""
import argparse
import atexit
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
        action("option", name)
        browser("wait", "--fn", "!document.querySelector('[role=dialog]')")

    browser("set", "viewport", "1440", "960")
    browser("open", args.url)
    atexit.register(browser, "close")
    browser("wait", '[aria-label="Search commands"]')
    action("button", "Search commands")
    opened()
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
    # Snap's normal trigger and the palette share the same controlled owner.
    choose("Snap settings")
    browser("wait", '[aria-label="Snap"][aria-expanded="true"]')
    action("button", "Snap", "focus")
    check("if (document.activeElement.getAttribute('aria-label') !== 'Snap') throw Error('Keyboard test must focus Snap');")
    check("""
        window.paletteSnapEvents = [];
        for (const type of ['keydown', 'keyup', 'click']) document.activeElement.addEventListener(type, e => paletteSnapEvents.push({type, key:e.key, trusted:e.isTrusted}));
    """)
    browser("press", "Enter")
    browser("wait", '[aria-label="Snap"][aria-expanded="false"]')
    check("if (document.activeElement.getAttribute('aria-label') !== 'Snap') throw Error('Snap must retain focus after Enter');")
    check("if (paletteSnapEvents.filter(e=>e.type==='click').length !== 1) throw Error('Enter must toggle once');")
    browser("press", "Space")
    browser("wait", '[aria-label="Snap"][aria-expanded="true"]')
    check("""
        if (document.activeElement.getAttribute('aria-label') !== 'Snap') throw Error('Snap must retain focus after Space');
        if (paletteSnapEvents.filter(e=>e.type==='click').length !== 2) throw Error('Space must toggle exactly once');
        if (!paletteSnapEvents.some(e=>e.type==='keyup' && e.key===' ' && e.trusted)) throw Error('Space must use trusted keyup activation');
    """)
    browser("press", "Control+k")
    opened()
    action("combobox", "Search commands and objects", "fill", "Position & rotation")
    check("""
        const option = document.querySelector('[role=option][aria-label="Position & rotation"]');
        if (option?.getAttribute('aria-disabled') !== 'true' || !option.textContent.includes('Select a matrix')) throw Error('Unavailable tools must explain why');
    """)
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[role=dialog]')")
    check("if (document.querySelector('[aria-label=Snap]').getAttribute('aria-expanded') !== 'true') throw Error('Palette Escape closed the underlying tool');")
    # Open the existing Library rename from a different workspace.
    choose("Rename project")
    browser("wait", '[role="textbox"][aria-label="Project name"], input[aria-label="Project name"]')
    check("if (document.activeElement.getAttribute('aria-label') !== 'Project name') throw Error('Rename must focus the real draft field');")
    action("textbox", "Project name", "fill", "Palette committed name")
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
    browser("press", "Control+k")
    opened()
    check("if (!document.querySelector('[role=group][aria-label=Objects] [role=option]')) throw Error('Demo objects must be searchable');")
    browser("screenshot", str((args.evidence / "palette-open.png").resolve()))
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
    browser("screenshot", str((args.evidence / "palette-closed.png").resolve()))
    print("PASS: production palette dialog, combobox, navigation, context, focus and keyboard", flush=True)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Ticket 010: exercise the mounted production header with agent-browser."""
import argparse
import atexit
from pathlib import Path
import subprocess
from uuid import uuid4


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url")
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    session = "header-010-" + uuid4().hex

    def browser(*command):
        result = subprocess.run(["agent-browser", "--session", session, *command],
                                text=True, capture_output=True, timeout=90)
        if result.returncode:
            diagnostic = subprocess.run(["agent-browser", "--session", session, "eval",
                "JSON.stringify({focus:document.activeElement.outerHTML, popups:[...document.querySelectorAll('main > header [data-state]')].map(e=>({slot:e.dataset.slot,state:e.dataset.state,label:e.getAttribute('aria-label')}))})"],
                text=True, capture_output=True, timeout=30)
            (args.evidence / "failure-dom.txt").write_text(diagnostic.stdout + diagnostic.stderr)
            subprocess.run(["agent-browser", "--session", session, "screenshot", str((args.evidence / "failure.png").resolve())], capture_output=True, timeout=30)
        assert result.returncode == 0, f"{command}: {result.stdout}{result.stderr}"
        return result.stdout

    def action(role, name, verb="click"):
        browser("find", "role", role, verb, "--name", name, "--exact")

    def check(js):
        return browser("eval", f"(() => {{ {js} return true; }})()")

    browser("set", "viewport", "1440", "960")
    browser("open", args.url)
    atexit.register(browser, "close")
    browser("wait", '[aria-label="Create new keyboard"]')
    # This is the first RED on the incumbent header (64px, bottom border).
    check("""
        const header = document.querySelector('main > header');
        const css = getComputedStyle(header);
        if (header.getBoundingClientRect().height !== 56) throw Error('Header must be 56px; got ' + header.getBoundingClientRect().height);
        if (parseFloat(css.borderBottomWidth)) throw Error('Header must be borderless');
    """)
    action("button", "Project")
    action("menuitem", "New project")
    for name in ("Continue to layout", "Hide guide", "Cancel"):
        browser("wait", "--text", name)
        action("button", name)
    browser("wait", "--text", "Browse parts")
    for workspace in ("PCB", "Keymap", "Keycaps", "Case", "Parts", "Layout"):
        action("button", f"Open {workspace} workspace")
        browser("wait", "--fn", f"document.querySelector('[aria-label=\"Open {workspace} workspace\"]').getAttribute('aria-current') === 'page'")
    action("button", "Search commands")
    check("if (document.querySelector('[role=dialog]')) throw Error('Search must stay inert until 012');")
    action("button", "Project")
    browser("wait", '[role="menu"][aria-label="Project"]')
    check("""
        if (document.activeElement.getAttribute('role') !== 'menuitem') throw Error('Project initial focus');
        for (const name of ['New project', 'Open project…', 'Save project copy…', 'Browse projects', 'Setup guide', 'Undo', 'Redo', 'Export', 'Workspace settings']) {
            if (![...document.querySelectorAll('[role=menuitem]')].some(e => e.textContent.trim() === name)) throw Error('Missing action: ' + name);
        }
    """)
    browser("press", "End")
    check("if (document.activeElement.textContent.trim() !== 'Workspace settings') throw Error('Project End');")
    browser("press", "Enter")
    browser("wait", '[role=menuitemradio]')
    browser("press", "Escape")
    check("if (document.activeElement.getAttribute('aria-label') !== 'Settings') throw Error('Workspace settings opens the real theme control');")
    action("button", "Project")
    browser("press", "Home")
    browser("press", "b")
    check("if (document.activeElement.textContent.trim() !== 'Browse projects') throw Error('Project typeahead');")
    browser("press", "Escape")
    check("if (document.activeElement.getAttribute('aria-label') !== 'Project') throw Error('Project return focus');")
    browser("wait", '[aria-label="Saved locally"]')
    check("""
        const button = document.querySelector('[aria-label="Saved locally"]');
        if (button.getAttribute('aria-haspopup') !== 'dialog' || button.getAttribute('aria-expanded') !== 'false') throw Error('Save popup trigger state');
    """)
    action("button", "Saved locally")
    browser("wait", "--text", "Changes are saved in this browser.")
    browser("press", "Escape")
    check("if (document.activeElement.getAttribute('aria-label') !== 'Saved locally') throw Error('Save return focus');")
    original_name = browser("get", "text", '[aria-label="Project"]').strip()
    action("button", "Project")
    action("menuitem", "Browse projects")
    browser("wait", "--text", "Your keyboards")
    action("button", "Preview " + original_name)
    browser("find", "text", "Rename project", "click", "--exact")
    browser("find", "role", "textbox", "fill", "Header outcome", "--name", "Project name", "--exact")
    browser("press", "Tab")
    browser("wait", "--fn", "document.querySelector('[aria-label=Project]').textContent.trim() === 'Header outcome'")
    action("button", "Project")
    action("menuitem", "Return to editor")
    action("button", "Undo")
    browser("wait", "--fn", f"document.querySelector('[aria-label=Project]').textContent.trim() === {original_name!r}")
    action("button", "Redo")
    browser("wait", "--fn", "document.querySelector('[aria-label=Project]').textContent.trim() === 'Header outcome'")
    browser("press", "Control+z")
    browser("wait", "--fn", f"document.querySelector('[aria-label=Project]').textContent.trim() === {original_name!r}")
    browser("press", "Control+Shift+z")
    browser("wait", "--fn", "document.querySelector('[aria-label=Project]').textContent.trim() === 'Header outcome'")
    action("button", "Export")
    browser("wait", "--fn", "document.querySelector('[aria-label=Export][aria-pressed=true]') !== null")
    action("button", "Export")
    browser("wait", "--fn", "document.querySelector('[aria-label=\"Open Layout workspace\"][aria-current=page]') !== null")
    action("button", "Project")
    download = (args.evidence / "header-copy.boardstudio").resolve()
    browser("download", '[role=menuitem][aria-label="Save project copy…"]', str(download))
    assert download.stat().st_size > 0, "Save project copy must produce a real file"
    action("button", "Project")
    action("menuitem", "New project")
    for name in ("Continue to layout", "Hide guide", "Cancel"):
        browser("wait", "--text", name)
        action("button", name)
    check("if (document.querySelector('[aria-label=Project]').textContent.trim() === 'Header outcome') throw Error('New project must replace the previous owner');")
    browser("upload", "#header-open-project", str(download))
    browser("wait", "--fn", "document.querySelector('[aria-label=Project]').textContent.trim() === 'Header outcome'")
    browser("wait", '[aria-label="Saved locally"]')
    action("button", "Settings")
    browser("press", "End")
    check("if (document.activeElement.textContent.trim() !== 'shadcn') throw Error('Settings End');")
    browser("press", "Home")
    browser("press", "ArrowDown")
    browser("press", "ArrowDown")
    check("if (document.activeElement.textContent.trim() !== 'Dark') throw Error('Settings arrows');")
    browser("press", "Enter")
    browser("wait", "--fn", "document.documentElement.dataset.appearance === 'dark'")
    check("if (document.activeElement.getAttribute('aria-label') !== 'Settings') throw Error('Settings activation return focus');")
    observations = []
    for theme, theme_label in (("original", "Original"), ("shadcn", "shadcn")):
        action("button", "Settings")
        action("menuitemradio", theme_label)
        for appearance, label in (("light", "Light"), ("dark", "Dark")):
            action("button", "Settings")
            action("menuitemradio", label)
            browser("wait", "--fn", f"document.documentElement.dataset.theme === '{theme}' && document.documentElement.dataset.appearance === '{appearance}'")
            for width in (1440, 1024, 768, 390):
                browser("set", "viewport", str(width), "960")
                if width <= 1250:
                    action("button", "Workbench menu")
                    browser("wait", '[aria-label="Open Layout workspace"]')
                    check("""
                        const trigger = document.querySelector('[aria-label="Workbench menu"]');
                        if (trigger.getAttribute('aria-haspopup') !== 'dialog' || trigger.getAttribute('aria-expanded') !== 'true' || !document.getElementById(trigger.getAttribute('aria-controls'))) throw Error('Mobile popup trigger state');
                    """)
                if width == 390:
                    action("button", "Open PCB workspace")
                    browser("wait", "--fn", "document.querySelector('[aria-label=\"Workbench menu\"]').getAttribute('aria-expanded') === 'false'")
                    check("if (document.activeElement.getAttribute('aria-label') !== 'Workbench menu') throw Error('Mobile navigation return focus');")
                    action("button", "Workbench menu")
                    check("if (document.querySelector('[aria-label=\"Open PCB workspace\"]').getAttribute('aria-current') !== 'page') throw Error('Mobile workspace outcome');")
                    action("button", "Open Layout workspace")
                    browser("wait", "--fn", "document.querySelector('[aria-label=\"Workbench menu\"]').getAttribute('aria-expanded') === 'false'")
                    action("button", "Workbench menu")
                    action("button", "Settings")
                    browser("wait", '[role="menuitemradio"]')
                    browser("wait", "--fn", "document.getAnimations().every(a => a.effect.getComputedTiming().iterations === Infinity || (a.playState !== 'running' && !a.pending))")
                    check("""
                        for (const menu of document.querySelectorAll('[role=menu]')) {
                            const r = menu.getBoundingClientRect();
                            if (r.left < 0 || r.right > innerWidth) throw Error('Mobile Settings overflow');
                        }
                        const popup = document.getElementById('header-mobile-commands').getBoundingClientRect();
                        for (const item of document.querySelectorAll('[role=menuitemradio]')) {
                            const r = item.getBoundingClientRect();
                            if (r.top < popup.top || r.bottom > popup.bottom) throw Error('Mobile Settings choice clipped: ' + item.textContent.trim());
                        }
                    """)
                    browser("screenshot", str((args.evidence / f"{theme}-{appearance}-settings-{width}.png").resolve()))
                    browser("press", "Escape")
                    check("if (document.activeElement.getAttribute('aria-label') !== 'Settings') throw Error('Nested Settings return focus');")
                browser("wait", "--fn", "[...document.querySelectorAll('main > header [data-slot=popover-content], main > header [data-slot=dropdown-menu-content]')].every(e => e.dataset.state !== 'closed' && getComputedStyle(e).opacity === '1') && document.getAnimations().every(a => a.effect.getComputedTiming().iterations === Infinity || (a.playState !== 'running' && !a.pending))")
                observations.append(browser("eval", """(() => {
                    const h = document.querySelector('main > header'), r = h.getBoundingClientRect(), c = getComputedStyle(h);
                    if (r.height !== 56 || parseFloat(c.borderBottomWidth)) throw Error('Header geometry');
                    if (r.left < 0 || r.right > innerWidth) throw Error('Header overflow');
                    for (const e of h.querySelectorAll('[aria-label]')) {
                        if (!e.checkVisibility() || e.closest('[role=menu]')) continue;
                        const b = e.getBoundingClientRect();
                        if (b.left < r.left || b.right > r.right + 1) throw Error('Control overflow: ' + e.getAttribute('aria-label'));
                    }
                    return {width:innerWidth, height:r.height, theme:document.documentElement.dataset.theme, appearance:document.documentElement.dataset.appearance};
                })()"""))
                browser("screenshot", str((args.evidence / f"{theme}-{appearance}-{width}.png").resolve()))
                if width <= 1250:
                    browser("press", "Escape")
                    check("if (document.activeElement.getAttribute('aria-label') !== 'Workbench menu') throw Error('Mobile return focus');")
            browser("set", "viewport", "1440", "960")
            browser("reload")
            browser("wait", '[aria-label="Saved locally"]')
            browser("wait", "--fn", f"document.documentElement.dataset.theme === '{theme}' && document.documentElement.dataset.appearance === '{appearance}'")
            action("button", "Settings")
            check(f"""
                for (const name of ['{theme_label}', '{label}']) {{
                    const item = [...document.querySelectorAll('[role=menuitemradio]')].find(e => e.textContent.trim() === name);
                    if (item?.getAttribute('aria-checked') !== 'true') throw Error('Preference not retained: ' + name);
                }}
            """)
            browser("press", "Escape")
    browser("set", "viewport", "390", "960")
    action("button", "Project")
    action("menuitem", "Workspace settings")
    browser("wait", '[role=menuitemradio]')
    browser("press", "Escape")
    check("if (document.activeElement.getAttribute('aria-label') !== 'Settings') throw Error('Compact Project settings route');")
    browser("press", "Escape")
    browser("set", "viewport", "1440", "960")
    browser("set", "media", "light")
    action("button", "Settings")
    action("menuitemradio", "System")
    browser("wait", "--fn", "document.documentElement.dataset.appearance === 'light' && localStorage.getItem('boardstudio:v2:appearance') === 'system'")
    browser("set", "media", "dark")
    browser("wait", "--fn", "document.documentElement.dataset.appearance === 'dark'")
    browser("eval", "localStorage.removeItem('boardstudio:v2:appearance'); localStorage.setItem('boardstudio:v2:theme', 'light')")
    browser("reload")
    browser("wait", '[aria-label="Saved locally"]')
    browser("wait", "--fn", "document.documentElement.dataset.appearance === 'light' && document.documentElement.dataset.theme === 'shadcn'")
    action("button", "Settings")
    action("menuitemradio", "Dark")
    browser("reload")
    browser("wait", '[aria-label="Saved locally"]')
    browser("wait", "--fn", "document.documentElement.dataset.appearance === 'dark' && localStorage.getItem('boardstudio:v2:theme') === 'light' && localStorage.getItem('boardstudio:v2:appearance') === 'dark'")
    (args.evidence / "geometry.txt").write_text("\n".join(observations) + "\n")
    print("PASS: production header navigation, action-menu keyboard floor, new/rename/Undo/Redo/Export/copy/import outcomes, inert Search, persisted Theme/Appearance, mobile focus and geometry at four widths")


if __name__ == "__main__":
    main()

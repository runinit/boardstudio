#!/usr/bin/env python3
"""Ticket 014: production shell keyboard, camera and theme/width checks."""
import argparse
import json
from pathlib import Path
import re
import subprocess
from uuid import uuid4


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url")
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    session = "shell-014-" + uuid4().hex

    def browser(*command):
        result = subprocess.run(["agent-browser", "--session", session, *command],
                                text=True, capture_output=True, timeout=90)
        assert result.returncode == 0, (command, result.stdout, result.stderr)
        return result.stdout

    def action(role, name, verb="click"):
        return browser("find", "role", role, verb, "--name", name, "--exact")

    def camera(name, verb="click", toolbar="Canvas view controls"):
        snapshot = json.loads(browser("snapshot", "-i", "--json", "--selector",
            f'[role="toolbar"][aria-label="{toolbar}"]'))["data"]
        refs = re.findall(r'^\s*- button "' + re.escape(name) + r'" \[[^\]\n]*\bref=(e\d+)',
                          snapshot["snapshot"], re.MULTILINE)
        assert len(refs) == 1, (name, snapshot)
        assert snapshot["refs"][refs[0]]["role"] == "button"
        assert snapshot["refs"][refs[0]]["name"] == name
        return browser(verb, "@" + refs[0])

    def check(js):
        browser("eval", "(() => { " + js + "; return true; })()")

    try:
        browser("set", "viewport", "1440", "960")
        browser("open", args.url)
        browser("wait", '[aria-label="Create new keyboard"]')
        action("button", "Project")
        action("menuitem", "New project")
        for name in ("Continue to layout", "Hide guide", "Cancel"):
            browser("wait", "--text", name)
            action("button", name)
        browser("wait", "--text", "Browse parts")
        camera("Fit board", "focus")
        check("""
            const toolbar = document.activeElement.closest('[role=toolbar]');
            if (toolbar.getAttribute('aria-label') !== 'Canvas view controls') throw Error('camera owner');
            const items = [...toolbar.querySelectorAll('[data-slot=toolbar-button]')];
            if (items.filter(e => !e.disabled && e.tabIndex === 0).length !== 1) throw Error('single camera tab stop');
            if (!items.find(e => e.textContent.trim() === 'Fit selection').disabled) throw Error('empty selection disabled');
        """)
        browser("press", "End")
        check("if (document.activeElement.getAttribute('aria-label') !== 'Zoom in') throw Error('camera End')")
        browser("wait", '[role=tooltip]')
        check("""
            const ids = document.activeElement.getAttribute('aria-describedby');
            if (!ids || !ids.split(' ').some(id => document.getElementById(id)?.getAttribute('role') === 'tooltip')) throw Error('zoom tooltip description');
        """)
        browser("press", "Escape")
        check("if (document.activeElement.getAttribute('aria-label') !== 'Zoom in') throw Error('tooltip keeps focus')")
        browser("press", "Home")
        check("if (document.activeElement.textContent.trim() !== 'Fit board') throw Error('camera Home')")
        browser("press", "ArrowRight")
        check("if (document.activeElement.getAttribute('aria-label') !== 'Zoom out') throw Error('camera skips disabled selection')")
        before = browser("get", "text", '[role="toolbar"][aria-label="Canvas view controls"]')
        camera("Zoom in")
        percent = re.search(r"\d+%", before).group()
        browser("wait", "--fn", f"!document.querySelector('[role=toolbar][aria-label=\"Canvas view controls\"]').textContent.includes('{percent}')")
        after = browser("get", "text", '[role="toolbar"][aria-label="Canvas view controls"]')
        assert before != after, ("zoom reaches the existing camera owner", before, after)
        camera("Fit board")
        # Keep the empty state visible in all twelve requested combinations.
        for theme in ("original", "shadcn"):
            for appearance in ("light", "dark"):
                browser("eval", f"localStorage.setItem('boardstudio:v2:workbench-theme', '{theme}'); localStorage.setItem('boardstudio:v2:appearance', '{appearance}')")
                browser("reload")
                browser("wait", "--fn", f"document.documentElement.dataset.theme === '{theme}' && document.documentElement.dataset.appearance === '{appearance}' && [...document.querySelectorAll('[data-slot=empty-title]')].some(e => e.textContent === 'Build your keyboard layout')")
                for width, height in ((1440, 960), (1100, 900), (390, 844)):
                    browser("set", "viewport", str(width), str(height))
                    check("""
                        const root = document.documentElement;
                        if (root.scrollWidth > root.clientWidth) throw Error('shell horizontal overflow');
                        const controls = [...document.querySelectorAll('[role=toolbar][aria-label="Canvas view controls"] [data-slot=toolbar-button], [data-slot=empty] [data-slot=button]')];
                        for (const el of controls) {
                            const r = el.getBoundingClientRect();
                            if (r.width <= 0 || r.left < 0 || r.right > innerWidth || r.bottom > innerHeight) throw Error('clipped shell control: ' + el.textContent);
                        }
                    """)
                    path = args.evidence / f"shell-{theme}-{appearance}-{width}.png"
                    browser("screenshot", str(path.resolve()))
        browser("set", "viewport", "1440", "960")
        # The shell dispatches through the existing matrix owner and palette registry.
        action("button", "Add key matrix")
        browser("wait", '[role=region][aria-label="New matrix"]')
        action("button", "Create matrix")
        browser("wait", "--fn", "![...document.querySelectorAll('[data-slot=empty-title]')].some(e => e.textContent === 'Build your keyboard layout')")
        camera("Fit selection")
        browser("set", "viewport", "1100", "900")
        camera("Undo", "focus", "Footer edit history")
        browser("press", "End")
        check("if (document.activeElement.getAttribute('aria-label') !== 'Redo') throw Error('history End')")
        browser("press", "Home")
        check("if (document.activeElement.getAttribute('aria-label') !== 'Undo') throw Error('history Home')")
        camera("Undo", toolbar="Footer edit history")
        browser("wait", "--text", "Build your keyboard layout")
        camera("Redo", toolbar="Footer edit history")
        browser("wait", "--fn", "![...document.querySelectorAll('[data-slot=empty-title]')].some(e => e.textContent === 'Build your keyboard layout')")
        browser("press", "Control+k")
        browser("wait", '[role=combobox][aria-label="Search commands and objects"]')
        browser("press", "Escape")
        (args.evidence / "final-snapshot.txt").write_text(browser("snapshot", "-i"))
        (args.evidence / "result.json").write_text(json.dumps({"result": "passed", "captures": 12,
            "limits": "Chromium keyboard and DOM checks; no screen reader, first-paint timing or human review"}, indent=2) + "\n")
        print("PASS: shell camera keyboard/callbacks, palette, empty action and 12 Theme/Appearance/width captures")
    except BaseException:
        (args.evidence / "failure-snapshot.txt").write_text(browser("snapshot", "-i"))
        browser("screenshot", str((args.evidence / "failure.png").resolve()))
        raise
    finally:
        browser("close")


if __name__ == "__main__":
    main()

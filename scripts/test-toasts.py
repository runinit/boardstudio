#!/usr/bin/env python3
"""Ticket 013: one-shot notifications on the packaged production shell."""
import argparse
import atexit
from pathlib import Path
import re
import subprocess
from uuid import uuid4


def stack_interoperability(browser, action, check, evidence):
    action("button", "Export")
    for width, height in ((1440, 960), (1024, 960), (390, 960), (390, 640)):
        browser("set", "viewport", str(width), str(height))
        compact = width <= 1250
        for theme, theme_label in (("original", "Original"), ("shadcn", "shadcn")):
            for appearance, appearance_label in (("light", "Light"), ("dark", "Dark")):
                if compact:
                    action("button", "Workbench menu")
                for label in (theme_label, appearance_label):
                    action("button", "Settings")
                    action("menuitemradio", label)
                if compact:
                    action("button", "Workbench menu")
                    browser("wait", "--fn", "document.getElementById('header-mobile-trigger').getAttribute('aria-expanded') === 'false'")
                # Exercise actual shell exports, in a burst shorter than the five-second expiry.
                browser("eval", """(async () => {
                    const before = new Set(document.querySelectorAll('[role=status][data-type]'));
                    const download = [...document.querySelectorAll('button')].find(e => e.textContent.trim() === 'Download .boardstudio project');
                    if (!download || download.disabled) throw Error('Project-copy action unavailable');
                    for (let i = 0; i < 7; i++) {
                        download.click();
                        const deadline = performance.now() + 2000;
                        while ([...document.querySelectorAll('[role=status][data-type]')].filter(e => !before.has(e)).length < (i + 1) * 2) {
                            if (performance.now() > deadline) throw Error('Real export did not settle');
                            await new Promise(requestAnimationFrame);
                        }
                    }
                    window.stackToasts = [...document.querySelectorAll('[role=status][data-type]')].filter(e => !before.has(e));
                    if (stackToasts.length !== 14) throw Error('Lost export results');
                    if (stackToasts.some(e => e.contains(document.activeElement))) throw Error('Result announcement stole focus');
                })()""")
                if compact:
                    action("button", "Workbench menu")
                action("button", "Settings")
                # Original failure: a live Toast intercepted this actual menu click.
                action("menuitemradio", theme_label)
                if compact:
                    action("button", "Workbench menu")
                    browser("wait", "--fn", "document.getElementById('header-mobile-trigger').getAttribute('aria-expanded') === 'false'")
                check(f"""
                    if (document.documentElement.dataset.theme !== '{theme}' || document.documentElement.dataset.appearance !== '{appearance}') throw Error('Theme outcome');
                    if (stackToasts.some(e => !e.isConnected)) throw Error('Results disappeared before interaction');
                    for (const title of ['Export started.', 'Project copy downloaded.']) {{
                        if (stackToasts.filter(e => document.getElementById(e.getAttribute('aria-labelledby'))?.textContent === title).length !== 7) throw Error('Lost result multiplicity: ' + title);
                    }}
                    const region = stackToasts[0].closest('[role=region]');
                    const bounds = region.getBoundingClientRect();
                    if (bounds.top < 0 || bounds.bottom > innerHeight || bounds.left < 0 || bounds.right > innerWidth || bounds.height > innerHeight / 2) throw Error('Notification stack must stay in a bounded viewport: ' + JSON.stringify(bounds));
                    if (region.scrollHeight <= region.clientHeight) throw Error('Burst must exercise scrolling');
                    for (const toast of stackToasts) {{
                        if (toast.getAttribute('aria-atomic') !== 'true' || toast.getAttribute('tabindex') !== '-1') throw Error('Passive atomic announcement lost');
                        const close = toast.querySelector('[aria-label=close]');
                        close.focus();
                        const rect = close.getBoundingClientRect();
                        const hit = document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2);
                        if (document.activeElement !== close || !close.contains(hit)) throw Error('Keyboard focus must reveal every dismiss button');
                    }}
                    window.stackDismissed = document.activeElement.closest('[role=status]');
                """)
                browser("press", "Shift+Tab")
                browser("press", "Tab")
                check("if (document.activeElement.closest('[role=status]') !== stackDismissed || document.activeElement.getAttribute('aria-label') !== 'close') throw Error('Dismiss button must stay reachable by Tab');")
                browser("press", "Enter")
                check("if (stackDismissed.isConnected || stackToasts.filter(e => e.isConnected).length !== 13) throw Error('Keyboard dismissal must remove exactly one result');")
                browser("press", "F6")
                check("if (document.activeElement.getAttribute('role') !== 'region' || !document.activeElement.getAttribute('aria-label').endsWith(' notifications')) throw Error('F6 notification access lost');")
                browser("screenshot", str((evidence / f"stack-{width}-{height}-{theme}-{appearance}.png").resolve()))
    print("PASS: 14-result bursts preserve multiplicity and atomic status; Settings, scrolling, Tab/Enter dismissal and F6 work in both Themes/Appearances at desktop, compact and narrow sizes")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url")
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--delivery-only", action="store_true")
    parser.add_argument("--stack-only", action="store_true")
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    session = "toasts-013-" + uuid4().hex

    def browser(*command):
        result = subprocess.run(["agent-browser", "--session", session, *command],
                                text=True, capture_output=True, timeout=90)
        if result.returncode:
            diagnostic = subprocess.run(["agent-browser", "--session", session, "snapshot"],
                                        text=True, capture_output=True, timeout=30)
            (args.evidence / "failure-dom.txt").write_text(diagnostic.stdout + diagnostic.stderr)
            subprocess.run(["agent-browser", "--session", session, "screenshot",
                            str((args.evidence / "failure.png").resolve())],
                           capture_output=True, timeout=30)
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
    action("button", "Project")
    action("menuitem", "New project")
    for name in ("Continue to layout", "Hide guide", "Cancel"):
        browser("wait", "--text", name)
        action("button", name)
    browser("wait", '[aria-label="Saved locally"]')
    if args.stack_only:
        stack_interoperability(browser, action, check, args.evidence)
        return
    if args.delivery_only:
        action("button", "Export")
        check("window.refusedDownloads = 0; URL.createObjectURL = () => { refusedDownloads++; throw Error('013 refused download'); };")
        action("button", "Download .boardstudio project")
        browser("wait", "--fn", "refusedDownloads === 1")
        check("""
            const failure = [...document.querySelectorAll('[role=status]')].filter(e => e.textContent.includes('013 refused download'));
            if (failure.length !== 1 || failure[0].dataset.type !== 'error') throw Error('Refused project copy must remain an error Toast after settlement; observed statuses: ' + [...document.querySelectorAll('[role=status], [role=alert]')].map(e => e.textContent).join(' | '));
            if ([...document.querySelectorAll('[role=status]')].some(e => e.textContent.includes('Project copy downloaded.'))) throw Error('Refused copy falsely reported success');
        """)
        print("PASS: refused project-copy delivery settles as one error Toast without false success")
        return
    # A real settled save must produce an announced, dismissible notification.
    check("""
        const toast = [...document.querySelectorAll('[role=status]')].find(e =>
            document.getElementById(e.getAttribute('aria-labelledby'))?.textContent === 'Saved locally.');
        if (!toast) throw Error('Settled save must announce Saved locally. as a Toast; no named status exists');
        if (toast.getAttribute('aria-atomic') !== 'true' || toast.getAttribute('tabindex') !== '-1') throw Error('Passive atomic announcement');
        if (toast.contains(document.activeElement)) throw Error('Save notification stole focus');
        if (document.querySelector('[role=alertdialog]')) throw Error('Notification must not be modal');
        window.savedToast = toast;
    """)
    close_refs = re.findall(r'button "close" \[ref=(e\d+)\]', browser("snapshot", "-i"))
    assert close_refs, "The announced Toast exposes a close button"
    browser("focus", "@" + close_refs[0])
    browser("press", "Shift+Tab")
    browser("press", "Tab")
    check("if (document.activeElement.getAttribute('aria-label') !== 'close') throw Error('Dismiss button must be reachable by Tab');")
    browser("press", "Enter")
    browser("wait", "--fn", "!savedToast.isConnected")
    check("window.remainingSaveToasts = [...document.querySelectorAll('[role=status]')];")
    action("button", "Export")
    browser("wait", "--text", "Export package")
    check("if ([...document.querySelectorAll('[role=status]')].some(e => e.textContent.includes('Saved locally.') && !remainingSaveToasts.includes(e))) throw Error('Route change redrained save');")
    action("button", "Download .boardstudio project")
    browser("wait", "--fn", "[...document.querySelectorAll('[role=status]')].some(e => e.textContent.includes('Export started.'))")
    browser("wait", "--fn", "[...document.querySelectorAll('[role=status]')].some(e => e.textContent.includes('Project copy downloaded.'))")
    check("""
        for (const message of ['Export started.', 'Project copy downloaded.']) {
            if ([...document.querySelectorAll('[role=status]')].filter(e => document.getElementById(e.getAttribute('aria-labelledby'))?.textContent === message).length !== 1) throw Error('Lost or duplicate export notification: ' + message);
        }
        const portable = [...document.querySelectorAll('[aria-label="Portable project"]')][0];
        if (portable.querySelector('[role=status]')) throw Error('One-shot result remains inline');
    """)
    browser("screenshot", str((args.evidence / "export-results.png").resolve()))
    # Refuse a real browser download; keep the operation and its error handling intact.
    check("window.originalObjectURL = URL.createObjectURL; URL.createObjectURL = () => { throw Error('013 refused download'); };")
    action("button", "Download .boardstudio project")
    browser("wait", "--fn", "[...document.querySelectorAll('[role=status]')].some(e => e.textContent.includes('013 refused download'))")
    check("""
        URL.createObjectURL = originalObjectURL;
        const failures = [...document.querySelectorAll('[role=status]')].filter(e => e.textContent.includes('013 refused download'));
        if (failures.length !== 1 || failures[0].dataset.type !== 'error') throw Error('Download failure must Toast once with error severity');
        if ([...document.querySelectorAll('[role=alert]')].some(e => e.textContent.includes('013 refused download'))) throw Error('One-shot download failure remains inline');
    """)
    print("PASS: save, project-copy and download failure Toast once; atomic status, passive focus, Tab/Enter dismissal and route changes")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Ticket 013: one-shot notifications on the packaged production shell."""
import argparse
import atexit
from pathlib import Path
import subprocess
from uuid import uuid4


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url")
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--delivery-only", action="store_true")
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
    action("button", "close", "focus")
    browser("press", "Shift+Tab")
    browser("press", "Tab")
    check("if (document.activeElement.getAttribute('aria-label') !== 'close') throw Error('Dismiss button must be reachable by Tab');")
    browser("press", "Enter")
    browser("wait", "--fn", "!savedToast.isConnected")
    action("button", "Export")
    browser("wait", "--text", "Export package")
    check("if ([...document.querySelectorAll('[role=status]')].some(e => e.textContent.includes('Saved locally.'))) throw Error('Route change redrained save');")
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

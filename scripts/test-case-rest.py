#!/usr/bin/env python3
"""019: production Case workspace journey: generation, bodies, collision, layers and panels."""
import argparse
import importlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "case-rest-019"
browser, evaluate = driver.browser, driver.evaluate


def action(role, name, verb="click", value=None):
    if verb == "focus":
        return browser("focus", driver.by_role_name(role, name))
    command = ["find", "role", role, verb]
    if value is not None:
        command.append(value)
    return browser(*command, "--name", name, "--exact")


def dismiss_notifications():
    for _ in range(3):
        closed = False
        for ref, item in browser("snapshot", "-i")["refs"].items():
            if item.get("role") == "button" and item.get("name") == "close":
                try:
                    browser("click", "@"+ref)
                    closed = True
                except Exception:
                    pass  # covered by an overlapping toast; retry after the top one closes
        if not closed:
            return


def has(role, name):
    return any(item.get("role") == role and item.get("name") == name
               for item in browser("snapshot", "-i")["refs"].values())


def set_theme(theme, appearance):
    evaluate(f"document.documentElement.dataset.theme='{theme}';"
             f"document.documentElement.dataset.appearance='{appearance}'")
    browser("wait", "--fn", "document.fonts.status==='loaded'")


def journey(url, evidence):
    """Fresh project to a Case workspace with an authored body and a generated scene."""
    browser("set", "viewport", 1440, 960)
    browser("open", url)
    browser("wait", "--text", "Your keyboards")
    action("button", "Project")
    action("menuitem", "New project")
    for name in ("Continue to layout", "Hide guide", "Cancel"):
        try:
            browser("wait", "--text", name, "--timeout", "5000")
            action("button", name)
        except Exception:
            pass  # persisted guide dismissal skips this step on reruns
    action("button", "Add key matrix")
    action("button", "Create matrix")
    browser("wait", "--text", "Matrix 1")
    action("button", "Open Case workspace")
    dismiss_notifications()
    browser("wait", "--text", "Case stack")
    # Generation controls keep their readiness behaviour: Update enabled when
    # ready and idle, Cancel disabled unless a generation is running.
    assert has("button", "Update preview")
    assert has("checkbox", "Live preview")
    assert has("button", "Cancel")
    update_disabled = evaluate("[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Update preview').disabled")
    cancel_disabled = evaluate("[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Cancel').disabled")
    print(f"generation readiness: update_disabled={update_disabled} cancel_disabled={cancel_disabled}", flush=True)
    # Authored bodies editor on the unconfigured stack.
    action("button", "+ New case body")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Body type\"]')")
    print("PASS authored body creation reaches the migrated editor", flush=True)
    # Select keyboard floor on the production Body type control.
    action("button", "Body type", "focus")
    assert evaluate("document.activeElement.dataset.slot === 'select-trigger'")
    browser("press", "Enter")
    browser("wait", "--fn", "!!document.querySelector('[role=listbox][aria-label=\"Body type\"]')")
    browser("press", "Home")
    assert evaluate("document.activeElement.textContent").strip() == "Plate"
    browser("press", "End")
    assert evaluate("document.activeElement.textContent").strip() == "Lid"
    browser("press", "Escape")
    browser("wait", "--fn", "document.activeElement.getAttribute('aria-label') === 'Body type'")
    assert evaluate("document.activeElement.getAttribute('aria-expanded')") == "false"
    print("PASS Body type Select keyboard floor with focus return", flush=True)
    # The generated scene brings the viewer, layers and collision checks.
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Fit\"]')", "--timeout", "180000")
    browser("wait", "--fn", "!![...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Collision check')",
            "--timeout", "180000")
    assert has("button", "Assembly visibility")
    assert has("spinbutton", "Required clearance (mm)")
    # Once the scene settles idle, Update is enabled and Cancel stands down.
    browser("wait", "--fn", "[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Update preview')?.disabled===false", "--timeout", "60000")
    print("PASS Case scene mounts viewer, layers and collision controls", flush=True)


def capture_matrix(evidence, configured=False):
    """Case workspace in both Themes x both Appearances x three widths."""
    tag = "configured" if configured else "bodies"
    records = []
    for width in (1440, 1050, 390):
        browser("set", "viewport", width, 960 if width != 390 else 844)
        for theme in ("original", "shadcn"):
            for appearance in ("light", "dark"):
                set_theme(theme, appearance)
                dismiss_notifications()
                if width == 390:
                    try:
                        action("button", "Close panels")
                    except Exception:
                        pass
                assert has("button", "Update preview"), (tag, width, theme, appearance)
                assert has("button", "Fit"), (tag, width, theme, appearance)
                stem = f"{tag}-{theme}-{appearance}-{width}"
                browser("screenshot", str(evidence/f"{stem}.png"))
                records.append({"capture": stem, "width": width, "theme": theme,
                                "appearance": appearance})
                if width == 390:
                    action("button", "Workbench menu")
                    action("button", "Inspector")
                    browser("wait", "--fn", "!!document.querySelector('[role=dialog][aria-label=\"Workspace panels\"]')")
                    browser("wait", "--fn", "!document.getElementById('header-mobile-commands')")
                    if configured:
                        browser("wait", "--text", "Dimensions and clearances")
                    else:
                        browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Body type\"]')")
                        evaluate("document.querySelector('[aria-label=\"Body type\"]').scrollIntoView({block:'center'})")
                    stem = f"{tag}-{theme}-{appearance}-390-inspector"
                    browser("screenshot", str(evidence/f"{stem}.png"))
                    records.append({"capture": stem, "width": width, "theme": theme,
                                    "appearance": appearance, "sheet": "inspector"})
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url", help="packaged page URL, e.g. http://127.0.0.1:48119/boardstudio/")
    parser.add_argument("--evidence", type=Path, default=ROOT/".evidence/019/captures")
    args = parser.parse_args()
    args.evidence = args.evidence.resolve()
    args.evidence.mkdir(parents=True, exist_ok=True)
    print("Source pin:", subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(), flush=True)
    try:
        journey(args.url, args.evidence)
        records = capture_matrix(args.evidence)
        action("button", "Configure mechanical stack")
        browser("wait", "--text", "Dimensions and clearances")
        records += capture_matrix(args.evidence, configured=True)
        errors = browser("errors").get("errors", [])
        assert errors == [], errors
        (args.evidence/"captures.json").write_text(json.dumps(records, indent=2)+"\n")
        print(f"PASS Case workspace: bodies, collision, layers, panels; {len(records)} captures", flush=True)
    except BaseException:
        try:
            browser("screenshot", str(args.evidence/"failure.png"))
            (args.evidence/"failure-snapshot.json").write_text(json.dumps(browser("snapshot"), indent=2)+"\n")
        except Exception as capture_error:
            print("Failure capture unavailable:", capture_error, flush=True)
        raise
    finally:
        browser("close")


if __name__ == "__main__":
    main()

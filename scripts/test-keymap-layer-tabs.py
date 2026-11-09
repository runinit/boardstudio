#!/usr/bin/env python3
"""Exercise layer tab activation with trusted keyboard input on a checked page."""
import argparse
import importlib
import json

driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "keymap-layer-tabs"
browser, evaluate = driver.browser, driver.evaluate


def action(role, name):
    browser("find", "role", role, "click", "--name", name, "--exact")


def run(url):
    browser("set", "viewport", 1440, 900)
    browser("open", url)
    browser("wait", "--text", "Your keyboards")
    action("button", "Create new keyboard")
    browser("wait", "--text", "Continue to layout")
    action("button", "Continue to layout")
    browser("wait", "--text", "Hide guide")
    action("button", "Hide guide")
    action("button", "Open Keymap workspace")
    browser("wait", "--text", "Add layer")
    action("button", "Add layer")
    browser("wait", "--fn", "document.querySelectorAll('[role=tablist][aria-label=\"Keymap layers\"] [role=tab]').length >= 2")
    tabs = "document.querySelectorAll('[role=tablist][aria-label=\"Keymap layers\"] [role=tab]')"
    names = evaluate(f"[...{tabs}].map(tab => tab.getAttribute('aria-label'))")
    initially_selected = evaluate(f"[...{tabs}].findIndex(tab => tab.getAttribute('aria-selected') === 'true')")
    assert initially_selected in (0, 1), initially_selected
    for index, key in ((1 - initially_selected, "Enter"), (initially_selected, "Space")):
        assert evaluate(f"{tabs}[{index}].getAttribute('aria-selected')") == "false"
        browser("focus", driver.by_role_name("tab", names[index]))
        browser("press", key)
        browser("wait", "--fn", f"{tabs}[{index}].getAttribute('aria-selected') === 'true'")
        selected = evaluate(f"{tabs}[{index}].getAttribute('aria-controls')")
        assert evaluate(f"document.getElementById({json.dumps(selected)}).getAttribute('role') === 'tabpanel' && "
                        f"document.getElementById({json.dumps(selected)}).getAttribute('aria-labelledby') === {tabs}[{index}].id")
        print(f"PASS {key} activates {names[index]} and its panel", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url", help="checked page URL")
    args = parser.parse_args()
    try:
        run(args.url)
    finally:
        browser("close")


if __name__ == "__main__":
    main()

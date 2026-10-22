#!/usr/bin/env python3
"""017: trusted keyboard on production Case settings and their real local draft owners."""
import argparse
import functools
import http.server
import importlib
import json
import os
from pathlib import Path
import subprocess
import threading

ROOT = Path(__file__).resolve().parents[1]
driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "case-settings-017"
browser, evaluate = driver.browser, driver.evaluate


def action(role, name, verb="click", value=None):
    if verb == "focus":
        return browser("focus", driver.by_role_name(role, name))
    command = ["find", "role", role, verb]
    if value is not None:
        command.append(value)
    return browser(*command, "--name", name, "--exact")


def status(name):
    return browser("get", "text", driver.by_role_name("status", name))["text"]


def tests(url, evidence):
    browser("set", "viewport", 1440, 960)
    browser("open", url)
    browser("wait", "--text", "Case construction")
    for name in ("Plate method", "Mount style", "Bottom construction"):
        action("button", name, "focus")
        assert evaluate("document.activeElement.dataset.slot === 'select-trigger'")
        browser("press", "Enter")
        browser("press", "Home")
        browser("press", "End")
        assert evaluate("document.activeElement.getAttribute('role') === 'option'")
        browser("press", "Escape")
        assert evaluate("document.activeElement.getAttribute('aria-label')") == name
        assert evaluate("document.activeElement.getAttribute('aria-expanded')") == "false"
    action("button", "Plate method")
    browser("press", "c")
    browser("press", "Enter")
    assert "Cnc" in status("Last request")
    assert evaluate("document.activeElement.getAttribute('aria-label')") == "Plate method"
    for name in ("Add middle frame", "Integrate plate frame into case"):
        before = int(status("Requests"))
        action("checkbox", name, "focus")
        browser("press", "Space")
        assert int(status("Requests")) == before + 1
        assert evaluate("document.activeElement.getAttribute('aria-checked')") == "true"
        browser("press", "Enter")
        assert int(status("Requests")) == before + 1, "checkbox Enter must do nothing"
        point = evaluate("""(()=>{const box=document.activeElement,label=box.labels[0];
            if(!box.id || label.htmlFor!==box.id) throw Error('associated unique label required');
            const r=label.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2}})()""")
        browser("mouse", "move", point["x"], point["y"])
        browser("mouse", "down")
        browser("mouse", "up")
        assert int(status("Requests")) == before + 2, "label must activate exactly once"
    precise = "1.2345678901234567"
    before = int(status("Requests"))
    action("spinbutton", "Plate thickness", "fill", precise)
    assert int(status("Requests")) == before, "draft stays in actual DimensionField"
    browser("press", "Enter")
    assert status("Accepted plate thickness") == precise
    assert int(status("Requests")) == before + 1
    action("spinbutton", "Plate thickness", "fill", "2.75")
    browser("press", "Escape")
    browser("press", "Tab")
    assert status("Accepted plate thickness") == precise
    assert int(status("Requests")) == before + 1
    action("spinbutton", "Plate thickness", "fill", "0.05")
    browser("press", "Tab")
    action("spinbutton", "Plate thickness", "focus")
    assert evaluate("document.activeElement.value") == "0.05"
    assert evaluate("document.activeElement.getAttribute('aria-invalid')") == "true"
    assert evaluate("document.getElementById(document.activeElement.getAttribute('aria-describedby')).textContent.length>0")
    assert int(status("Requests")) == before + 1
    browser("press", "Escape")
    action("button", "Switch fit profiles", "focus")
    browser("press", "Space")
    assert evaluate("document.activeElement.getAttribute('aria-expanded')") == "true"
    assert evaluate("document.getElementById(document.activeElement.getAttribute('aria-controls'))!==null")
    action("button", "Read KiCad layers")
    browser("wait", "--text", "Custom geometry layers")
    action("checkbox", "Select geometry fp_rect:0")
    action("button", "Purpose for Dwgs.User")
    browser("press", "End")
    browser("press", "Enter")
    action("button", "Switch fit profiles", "focus")
    browser("press", "Enter")
    assert evaluate("document.activeElement.getAttribute('aria-expanded')") == "false"
    assert evaluate("document.getElementById(document.activeElement.getAttribute('aria-controls')).hidden")
    browser("press", "Space")
    action("checkbox", "Select geometry fp_rect:0", "focus")
    assert evaluate("document.activeElement.getAttribute('aria-checked')") == "true", "real local extraction selection survives collapse"
    action("button", "Purpose for Dwgs.User", "focus")
    assert evaluate("document.activeElement.textContent.includes('Drawing guide')"), "purpose draft stays with geometry owner"
    # Re-select plate cutout and apply through the real Core extraction port.
    browser("press", "Enter")
    browser("press", "Home")
    browser("press", "Enter")
    action("button", "Apply selected geometry")
    assert "UpdateProfile" in status("Last request") and "-9.5" in status("Last request")
    action("spinbutton", "Plate cutout geometry contour 1 vertex 1 X", "fill", "-9.123456789012345")
    assert "-9.123456789012345" in status("Last request")
    action("spinbutton", "Access openings volume 1 Height", "fill", "6.123456789012345")
    browser("press", "Enter")
    assert "6.123456789012345" in status("Last request")
    action("button", "Library fit profile")
    action("option", "MX switch")
    action("combobox", "Assign library fit profile to", "fill", "Part 12")
    browser("press", "ArrowDown")
    assert evaluate("document.activeElement.hasAttribute('aria-activedescendant')")
    browser("press", "Enter")
    assert "part-12" in status("Last request") and "AssignSwitchProfile" in status("Last request")
    action("combobox", "Assign library fit profile to", "focus")
    browser("press", "ArrowDown")
    browser("press", "Escape")
    assert evaluate("document.activeElement.getAttribute('role')") == "combobox"
    action("button", "Fixture layer 1.50 mm Z 0.00", "focus")
    browser("press", "Space")
    assert status("Selected layer") == "fixture-layer"
    browser("press", "Space")
    assert status("Selected layer") == ""
    action("button", "Toggle editing")
    for role, name in [("button", "Plate method"), ("checkbox", "Add middle frame"), ("spinbutton", "Plate thickness")]:
        assert not browser("is", "enabled", driver.by_role_name(role, name))["enabled"]
    action("button", "Toggle editing")
    observations = []
    for theme in ("original", "shadcn"):
        for appearance in ("light", "dark"):
            evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
            browser("wait", "--fn", "document.fonts.status==='loaded'")
            action("spinbutton", "Plate thickness", "focus")
            metrics = evaluate("""(()=>{const input=document.activeElement,group=input.parentElement,label=input.labels[0],s=getComputedStyle(input);
                return {slot:input.dataset.slot,height:group.getBoundingClientRect().height,labelFor:label.htmlFor,id:input.id,
                font:s.fontFamily,border:s.borderTopWidth,invalid:input.getAttribute('aria-invalid')}})()""")
            assert metrics["slot"] == "input-group-control" and metrics["height"] == 40, metrics
            assert metrics["labelFor"] == metrics["id"] and metrics["id"]
            observations.append({"theme": theme, "appearance": appearance, **metrics})
            browser("screenshot", str(evidence / f"settings-{theme}-{appearance}.png"))
    for width in (1050, 390):
        browser("set", "viewport", width, 844)
        browser("screenshot", str(evidence / f"settings-{width}.png"))
    assert browser("errors").get("errors", []) == []
    (evidence / "observations.json").write_text(json.dumps(observations, indent=2)+"\n")
    print("PASS production Case Select/Combobox/Checkbox/Collapsible/Toggle controls; precise native entry, validation, retained local geometry drafts, real Core extraction; both Themes/Appearances and mid/narrow captures", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, default=ROOT/".evidence/017/browser")
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
    driver.run("cargo", "build", "--locked", "-p", "boardstudio-web-case", "--example", "settings", "--target", "wasm32-unknown-unknown", env=env)
    out = ROOT/"target/case-settings-fixture"
    out.mkdir(parents=True, exist_ok=True)
    driver.run(str(driver.wasm_bindgen_dir(ROOT)/"wasm-bindgen"), "--target", "web", "--out-dir", str(out), str(ROOT/"target/wasm32-unknown-unknown/debug/examples/settings.wasm"))
    driver.run(str(Path.home()/".local/share/.dx/tools/tailwindcss-v4.1.5/tailwindcss"), "-i", "web/tailwind.css", "-o", str(out/"tailwind.css"))
    if not (out/"assets").exists():
        (out/"assets").symlink_to(ROOT/"web/assets", target_is_directory=True)
    (out/"index.html").write_text('<!doctype html><html lang="en"><meta charset="utf-8"><title>Case settings 017</title><link rel="stylesheet" href="assets/m1.css"><link rel="stylesheet" href="tailwind.css"><style>body{overflow:auto;height:auto;min-height:100vh}#main{box-sizing:border-box;max-width:480px;width:100%;padding:16px;background:var(--card);color:var(--foreground)}output{display:block}</style><body><div id="main"></div><script type="module">import init from "./settings.js";init()</script>')
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(out))
    with http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            tests(f"http://127.0.0.1:{server.server_port}", args.evidence)
        finally:
            browser("close")
            server.shutdown()


if __name__ == "__main__":
    main()

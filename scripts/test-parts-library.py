#!/usr/bin/env python3
"""020: the packaged Parts owners, trusted role/name input, retained controls and theme captures."""
import argparse
import importlib
import json
from pathlib import Path
from uuid import uuid4


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url")
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    driver = importlib.import_module("test-dock-primitives")
    driver.SESSION = "parts-020-" + uuid4().hex
    browser, evaluate, check = driver.browser, driver.evaluate, driver.check

    def action(role, name, verb="click", *values):
        return browser("find", "role", role, verb, *values, "--name", name, "--exact")

    def focus(role, name):
        browser("focus", driver.by_role_name(role, name))

    def attr(role, name, attribute):
        return browser("get", "attr", driver.by_role_name(role, name), attribute)["value"]

    def has(role, name):
        return any(item.get("role") == role and item.get("name") == name
                   for item in browser("snapshot", "-i")["refs"].values())

    def expanded(name):
        return attr("button", name, "aria-expanded") == "true"

    def retained(name, inner_role, inner_name):
        focus(inner_role, inner_name)
        evaluate("window.partsRetained=document.activeElement")
        action("button", name)
        assert not expanded(name)
        check("partsRetained.isConnected && !partsRetained.checkVisibility()")
        evaluate("partsRetained.focus()")
        check("document.activeElement!==partsRetained")
        browser("press", "Tab")
        check("!partsRetained.parentElement.closest('[data-open=false]')?.contains(document.activeElement)")
        action("button", name)
        assert expanded(name)
        focus(inner_role, inner_name)
        check("document.activeElement===partsRetained")

    def checkbox(name):
        focus("checkbox", name)
        before = attr("checkbox", name, "aria-checked")
        browser("press", "Enter")
        assert attr("checkbox", name, "aria-checked") == before, (name, "Enter changed checkbox")
        browser("press", "Space")
        assert attr("checkbox", name, "aria-checked") != before, (name, "Space did not change checkbox")
        evaluate("window.partsCheckbox=document.activeElement")
        check("partsCheckbox.id && [...document.querySelectorAll('label[for]')].filter(e=>e.htmlFor===partsCheckbox.id).length===1")
        # Native label activation through a trusted pointer at its text node.
        point = evaluate("(()=>{const label=[...document.querySelectorAll('label[for]')].find(e=>e.htmlFor===partsCheckbox.id);const text=[...label.childNodes].find(n=>n.nodeType===Node.TEXT_NODE&&n.textContent.trim());const range=document.createRange();range.selectNodeContents(text);const r=range.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2}})()")
        browser("mouse", "move", point["x"], point["y"])
        browser("mouse", "down")
        browser("mouse", "up")
        assert attr("checkbox", name, "aria-checked") == before, (name, "label activation")

    try:
        browser("set", "viewport", 1440, 960)
        browser("open", args.url)
        browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
        action("button", "Project")
        action("menuitem", "New project")
        browser("wait", "--text", "Keyboard setup")
        action("button", "4 Case (optional)")
        action("button", "Hide guide")
        action("button", "Open Parts workspace")
        browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Search footprints\"]')")
        action("button", "MX Solder")
        browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Switch orientation\"]')")
        focus("button", "Switch orientation")
        browser("press", "Enter")
        assert has("listbox", "Switch orientation")
        browser("press", "Home")
        browser("press", "End")
        browser("press", "n")
        browser("press", "Enter")
        check("document.activeElement.getAttribute('aria-label')==='Switch orientation' && document.activeElement.textContent.includes('North-facing')")
        browser("press", "Space")
        browser("press", "Escape")
        assert not expanded("Switch orientation")
        check("document.activeElement.getAttribute('aria-label')==='Switch orientation'")
        retained("Key assemblies", "button", "Switch orientation")
        check("document.activeElement.textContent.includes('North-facing')")
        action("searchbox", "Search footprints", "fill", "MX switch")
        action("button", "MX switch")
        browser("wait", "--fn", "!!document.querySelector('[aria-label=\"2D footprint\"]')")
        focus("button", "2D footprint")
        browser("press", "ArrowRight")
        check("document.activeElement.getAttribute('aria-label')==='3D model' && document.activeElement.getAttribute('aria-pressed')==='false'")
        browser("press", "Enter")
        browser("wait", "--fn", "!!document.querySelector('[aria-label=\"View controls\"]')")
        action("button", "View controls")
        action("button", "exploded")
        focus("slider", "Explode amount")
        browser("press", "Home")
        assert float(attr("slider", "Explode amount", "aria-valuenow")) == 0
        browser("press", "ArrowRight")
        fraction = float(attr("slider", "Explode amount", "aria-valuenow"))
        assert abs(fraction - 0.1) < 1e-7, fraction
        retained("View controls", "slider", "Explode amount")
        assert float(attr("slider", "Explode amount", "aria-valuenow")) == fraction
        browser("press", "End")
        assert float(attr("slider", "Explode amount", "aria-valuenow")) == 10
        action("button", "section")
        focus("slider", "Section position")
        browser("press", "Home")
        assert float(attr("slider", "Section position", "aria-valuenow")) == -100
        browser("press", "End")
        assert float(attr("slider", "Section position", "aria-valuenow")) == 100
        action("button", "Section plane")
        assert has("listbox", "Section plane")
        browser("press", "Home")
        browser("press", "Enter")
        check("document.activeElement.getAttribute('aria-label')==='Section plane' && document.activeElement.textContent.includes('XY')")
        checkbox("Show section plane")
        checkbox("Show hidden lines")
        retained("View controls", "checkbox", "Show hidden lines")
        action("button", "View controls")
        focus("button", "View controls")
        browser("wait", "--fn", "!!document.getElementById(document.activeElement.getAttribute('aria-describedby'))")
        browser("press", "Escape")
        check("document.activeElement.getAttribute('aria-label')==='View controls'")
        for width, height in [(1440, 960), (1100, 900), (390, 844)]:
            browser("set", "viewport", width, height)
            if width < 980 and has("button", "Close panels"):
                action("button", "Close panels")
            for theme in ["original", "shadcn"]:
                for appearance in ["light", "dark"]:
                    evaluate(f"document.documentElement.dataset.theme={json.dumps(theme)};document.documentElement.dataset.appearance={json.dumps(appearance)}")
                    evaluate("document.fonts.ready.then(()=>true)")
                    action("button", "View controls")
                    focus("checkbox", "Show hidden lines")
                    metrics = evaluate("(()=>{const content=document.activeElement.closest('[data-slot=collapsible-content]');const toolbar=content.parentElement.parentElement;const selected=[...toolbar.querySelectorAll('[data-slot=toggle-group-item][aria-pressed=true]')];const probe=document.createElement('div');probe.style.backgroundColor='var(--background)';document.body.append(probe);const expected=getComputedStyle(probe).backgroundColor;probe.remove();return {legacy:toolbar.classList.contains('m1-case-view-toolbar'),bounds:[content.getBoundingClientRect().left,content.getBoundingClientRect().right],scroll:toolbar.scrollWidth,client:toolbar.clientWidth,expected,selected:selected.map(e=>({background:getComputedStyle(e).backgroundColor,border:getComputedStyle(e).borderWidth,height:e.getBoundingClientRect().height}))}})()")
                    assert not metrics["legacy"] and metrics["scroll"] <= metrics["client"], metrics
                    assert 0 <= metrics["bounds"][0] < metrics["bounds"][1] <= width, metrics
                    assert all(e["background"] == metrics["expected"] and e["border"] == "0px" and e["height"] >= 36 for e in metrics["selected"]), metrics
                    print("Computed preview styles:", width, theme, appearance, metrics, flush=True)
                    browser("screenshot", str((args.evidence / f"preview-{width}-{theme}-{appearance}.png").resolve()))
                    action("button", "View controls")
                    if width < 980:
                        action("button", "Workbench menu")
                        action("button", "Objects")
                        action("tab", "Objects")
                    focus("searchbox", "Search footprints")
                    check("document.activeElement.getAttribute('data-slot')==='input-group-control' && document.activeElement.getBoundingClientRect().width>0")
                    check("document.documentElement.scrollWidth<=document.documentElement.clientWidth")
                    browser("screenshot", str((args.evidence / f"parts-{width}-{theme}-{appearance}.png").resolve()))
                    if width < 980:
                        action("button", "Close panels")
        action("button", "2D footprint")
        browser("wait", "--fn", "document.querySelector('[aria-label=\"2D footprint\"]').getAttribute('aria-pressed')==='true'")
        print("PASS: packaged Parts Select, preview ToggleGroup, retained production owner values/identity, sliders, checkbox Space/Enter/labels, and 12 theme/width captures", flush=True)
    except Exception:
        browser("screenshot", str((args.evidence / "failure.png").resolve()))
        raise
    finally:
        browser("close")


if __name__ == "__main__":
    main()

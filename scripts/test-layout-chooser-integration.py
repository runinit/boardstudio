#!/usr/bin/env python3
"""016: actual long Layout/Parts choices, native labels, placement, Cancel and Undo."""
import argparse
from datetime import datetime
import importlib
import json
from pathlib import Path


flow = importlib.import_module("test-layout-adoption")
driver = flow.driver
driver.SESSION = "layout-chooser-016"
browser, evaluate, check, click = driver.browser, driver.evaluate, driver.check, driver.click
ROOT = Path(__file__).resolve().parents[1]
CANVAS = "Keyboard layout; drag components, hold Shift for range selection, hold Space and drag to pan, or use position controls"


def native_label(name, implicit):
    browser("focus", driver.by_role_name("combobox", name))
    evaluate("window.choiceInput=document.activeElement")
    check("choiceInput.labels.length===1 && choiceInput.labels[0].control===choiceInput && choiceInput.id!=='' && document.querySelectorAll('[id=\"'+choiceInput.id+'\"]').length===1")
    assert evaluate("choiceInput.labels[0].hasAttribute('for')") != implicit
    # Native labels have no role: hit their associated text, outside the actual input.
    point = evaluate("(()=>{const label=choiceInput.labels[0],text=[...label.childNodes].find(n=>n.nodeType===Node.TEXT_NODE&&n.textContent.trim());if(!text)throw Error('No associated label text');const range=document.createRange();range.selectNodeContents(text);const r=range.getBoundingClientRect();return {x:Math.round(r.x+r.width/2),y:Math.round(r.y+r.height/2)}})()")
    browser("press", "Tab")
    check("document.activeElement!==choiceInput")
    evaluate("window.labelClicks=[];choiceInput.labels[0].addEventListener('click',e=>labelClicks.push(e.isTrusted),{once:true})")
    browser("mouse", "move", point["x"], point["y"])
    browser("mouse", "down")
    browser("mouse", "up")
    check("document.activeElement===choiceInput && labelClicks.length===1 && labelClicks[0]")
    browser("press", "Escape")
    browser("wait", "--fn", "choiceInput.getAttribute('aria-expanded')==='false'")


def choose_long(name, text):
    browser("fill", driver.by_role_name("combobox", name), text)
    browser("press", "ArrowDown")
    browser("wait", "--fn", "!!document.getElementById(choiceInput.getAttribute('aria-activedescendant'))")
    selected = evaluate("document.getElementById(choiceInput.getAttribute('aria-activedescendant')).textContent.trim()")
    assert text.lower() in selected.lower(), (text, selected)
    browser("press", "Enter")
    browser("wait", "--fn", f"choiceInput.getAttribute('aria-expanded')==='false' && choiceInput.value==={json.dumps(selected)}")
    check("document.activeElement===choiceInput")
    print("Actual long choice", name, selected, flush=True)


def parts():
    return evaluate("[...document.querySelectorAll('[data-part-id]')].map(e=>({id:e.dataset.partId,transform:e.getAttribute('transform').trim()})).sort((a,b)=>a.id.localeCompare(b.id))")


def start_part():
    click("Add object")
    browser("fill", driver.by_role_name("searchbox", "Search parts"), "reset")
    refs = browser("snapshot", "-i")["refs"]
    names = sorted(item["name"] for item in refs.values() if item["role"] == "button" and item["name"].lower().startswith("add ") and "reset" in item["name"].lower())
    assert names, refs
    flow.activate_with_space(names[0])
    browser("wait", "--fn", "!!document.querySelector('.m1-part-placement-preview')")
    print("Actual component placement", names[0], flush=True)
    browser("focus", driver.by_role_name("group", CANVAS))
    evaluate("window.placementCanvas=document.activeElement")


def tests(url, captures):
    flow.new_layout_project(url)
    for count in range(1, 14):
        flow.create_single_matrix(count)
    original = flow.geometry()
    original_parts = parts()
    click("Add object")
    click("Mirror existing half… Create a linked half from an existing layout")
    native_label("Source layouts", implicit=False)
    choose_long("Source layouts", "Matrix 13")
    browser("fill", driver.by_role_name("spinbutton", "Mirror axis X"), "12.123456789")
    browser("press", "Escape")
    check("!document.querySelector('[role=region][aria-label=\"Mirror existing half\"]')")
    assert flow.geometry() == original and parts() == original_parts
    check("document.activeElement.getAttribute('aria-label')==='Add object'")
    click("Add object")
    click("Mirror existing half… Create a linked half from an existing layout")
    click("Cancel")
    check("!document.querySelector('[role=region][aria-label=\"Mirror existing half\"]') && document.activeElement.getAttribute('aria-label')==='Add object'")
    assert flow.geometry() == original and parts() == original_parts
    click("Add object")
    click("Mirror existing half… Create a linked half from an existing layout")
    native_label("Source layouts", implicit=False)
    choose_long("Source layouts", "Matrix 13")
    browser("fill", driver.by_role_name("spinbutton", "Mirror axis X"), "12.123456789")
    flow.activate_with_space("Create linked half")
    browser("wait", "--fn", "document.querySelectorAll('.m1-matrix-key').length===14")
    added = [key for key in flow.geometry() if key not in original]
    assert len(added) == 1 and added[0]["transform"] == "translate(24.246913578 0) rotate(0)", added
    click("Undo")
    browser("wait", "--fn", "document.querySelectorAll('.m1-matrix-key').length===13")
    assert flow.geometry() == original and parts() == original_parts
    click("Add object")
    native_label("Part placement layout", implicit=True)
    choose_long("Part placement layout", "Board / ungrouped")
    browser("screenshot", str(captures / "long-implicit-placement-choice.png"))
    browser("press", "Escape")
    start_part()
    browser("press", "ArrowRight")
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('.m1-part-placement-preview')")
    check("document.activeElement===placementCanvas")
    assert flow.geometry() == original and parts() == original_parts
    start_part()
    before = evaluate("document.querySelector('.m1-part-placement-preview').getAttribute('transform')")
    browser("press", "ArrowRight")
    browser("wait", "--fn", f"document.querySelector('.m1-part-placement-preview').getAttribute('transform')!=={json.dumps(before)}")
    preview = evaluate("document.querySelector('.m1-part-placement-preview').getAttribute('transform')")
    browser("press", "Enter")
    browser("wait", "--fn", f"!document.querySelector('.m1-part-placement-preview') && document.querySelectorAll('[data-part-id]').length==={len(original_parts)+1}")
    committed = parts()
    added_parts = [part for part in committed if part not in original_parts]
    assert len(added_parts) == 1 and added_parts[0]["transform"] == preview + " rotate(0)", (preview, added_parts)
    assert all(part in committed for part in original_parts) and flow.geometry() == original
    click("Undo")
    browser("wait", "--fn", f"document.querySelectorAll('[data-part-id]').length==={len(original_parts)}")
    assert parts() == original_parts
    click("Redo")
    browser("wait", "--fn", f"document.querySelectorAll('[data-part-id]').length==={len(committed)}")
    assert parts() == committed
    click("Undo")
    browser("wait", "--fn", f"document.querySelectorAll('[data-part-id]').length==={len(original_parts)}")
    assert parts() == original_parts and flow.geometry() == original
    browser("screenshot", str(captures / "placement-undo.png"))
    print("PASS actual13-layout explicit/implicit long Combobox labels, filter/active option/controlled value/focus, Mirror Cancel/precise one-source commit/Undo, Parts keyboard Cancel/commit and exact Undo/Redo", captures, flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", required=True)
    args = parser.parse_args()
    captures = ROOT / ".evidence/016" / datetime.now().strftime("layout-chooser-integration-%Y%m%d-%H%M%S")
    captures.mkdir(parents=True)
    try:
        tests(args.page_url, captures)
    except Exception:
        browser("screenshot", str(captures / "failure.png"))
        print(browser("snapshot", "-i")["snapshot"], flush=True)
        raise
    finally:
        browser("close")


if __name__ == "__main__":
    main()

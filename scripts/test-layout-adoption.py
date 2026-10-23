#!/usr/bin/env python3
"""016: actual Add/Mirror operations, retained local draft, precise geometry and Undo."""
import argparse
from datetime import datetime
import importlib
from pathlib import Path


driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "layout-adoption-016"
browser, evaluate, check, click = driver.browser, driver.evaluate, driver.check, driver.click
ROOT = Path(__file__).resolve().parents[1]


def objects_sheet():
    if not evaluate("!!document.getElementById('workspace-panel-sheet')?.matches('[role=dialog]')"):
        click("Workbench menu")
        click("Objects")
    browser("find", "role", "tab", "click", "--name", "Objects", "--exact")


def activate_with_space(name):
    browser("focus", driver.by_role_name("button", name))
    evaluate("window.spaceOwner=document.activeElement;window.spaceClicks=[];spaceOwner.addEventListener('click',e=>spaceClicks.push(e.isTrusted),{once:true})")
    browser("press", "Space")
    print("Native Space", name, evaluate("({clicks:spaceClicks,connected:spaceOwner.isConnected})"), flush=True)
    check("spaceClicks.length===1 && spaceClicks[0]")


def geometry():
    # Drawing geometry is inspected structurally; interactive controls use role/name.
    return evaluate("[...document.querySelectorAll('.m1-matrix-key')].map(e=>({matrix:e.dataset.matrixId,transform:e.getAttribute('transform'),width:e.getAttribute('width'),height:e.getAttribute('height')}))")


def tests(url, captures):
    browser("set", "viewport", 1440, 900)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
    click("Project")
    browser("find", "role", "menuitem", "click", "--name", "New project", "--exact")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Keyboard setup\"]')")
    click("4 Case (optional)")
    click("Hide guide")
    click("Open Layout workspace")
    click("Add object")
    assert driver.by_role_name("region", "Add")
    check("!document.querySelector('[role=dialog][aria-label=Add]') && document.getElementById('m1-objects-panel').contains(document.querySelector('[role=region][aria-label=Add]'))")
    activate_with_space("Matrix… Rows, columns & key assemblies")
    for name in ["Rows", "Columns"]:
        browser("focus", driver.by_role_name("slider", name))
        browser("press", "Home")
    activate_with_space("Create matrix")
    browser("wait", "--fn", "document.querySelectorAll('.m1-matrix-key').length===1")
    original = geometry()
    assert original[0]["transform"] == "translate(0 0) rotate(0)" and original[0]["width"] == original[0]["height"] == "18", original
    click("Add object")
    click("Mirror existing half… Create a linked half from an existing layout")
    browser("fill", driver.by_role_name("spinbutton", "Mirror axis X"), "12.123456789")
    browser("focus", driver.by_role_name("spinbutton", "Mirror axis X"))
    evaluate("window.axisDraft=document.activeElement")
    check("axisDraft.closest('[role=region]').getAttribute('aria-label')==='Mirror existing half' && !axisDraft.closest('[role=dialog][aria-label=\"Mirror existing half\"]') && document.getElementById('m1-objects-panel').contains(axisDraft)")
    check("axisDraft.labels.length===1 && axisDraft.labels[0].control===axisDraft && document.querySelectorAll('[id=\"'+axisDraft.id+'\"]').length===1")
    for width in [1440, 1100, 390]:
        browser("set", "viewport", width, 900)
        for theme in ["original", "shadcn"]:
            for appearance in ["light", "dark"]:
                evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
                if width < 980:
                    objects_sheet()
                browser("focus", driver.by_role_name("spinbutton", "Mirror axis X"))
                check("document.activeElement===axisDraft && axisDraft.value==='12.123456789'")
                assert geometry() == original, "theme/width changes must preserve accepted geometry"
                browser("screenshot", str(captures / f"mirror-{width}-{theme}-{appearance}.png"))
                if width < 980:
                    click("Close panels")
                    browser("wait", "--fn", "axisDraft.closest('[inert]')!==null")
                    check("axisDraft.isConnected && axisDraft.value==='12.123456789'")
                    evaluate("axisDraft.focus()")
                    check("document.activeElement!==axisDraft")
    browser("set", "viewport", 1440, 900)
    browser("focus", driver.by_role_name("spinbutton", "Mirror axis X"))
    check("document.activeElement===axisDraft && axisDraft.value==='12.123456789'")
    activate_with_space("Create linked half")
    browser("wait", "--fn", "document.querySelectorAll('.m1-matrix-key').length===2")
    paired = geometry()
    assert original[0] in paired, paired
    mirrored = [key for key in paired if key["matrix"] != original[0]["matrix"]]
    assert len(mirrored) == 1 and mirrored[0]["transform"] == "translate(24.246913578 0) rotate(0)", paired
    assert mirrored[0]["width"] == mirrored[0]["height"] == "18", paired
    browser("fill", driver.by_role_name("spinbutton", "Origin X"), "7.125")
    browser("press", "Enter")
    browser("wait", "--fn", "[...document.querySelectorAll('.m1-matrix-key')].some(e=>e.getAttribute('transform')==='translate(7.125 0) rotate(0)')")
    moved = geometry()
    assert {key["transform"] for key in moved} == {"translate(7.125 0) rotate(0)", "translate(17.121913578 0) rotate(0)"}, moved
    assert all(key["width"] == key["height"] == "18" for key in moved)
    click("Undo")
    browser("wait", "--fn", "[...document.querySelectorAll('.m1-matrix-key')].some(e=>e.getAttribute('transform')==='translate(24.246913578 0) rotate(0)')")
    assert geometry() == paired, "one Undo restores the exact measurements and linked geometry"
    click("Undo")
    browser("wait", "--fn", "document.querySelectorAll('.m1-matrix-key').length===1")
    assert geometry() == original, "one Undo removes only the linked half"
    click("Undo")
    browser("wait", "--fn", "document.querySelectorAll('.m1-matrix-key').length===0")
    print("PASS actual Add/Mirror dock owners, retained precise local draft and hidden/inert focus across 12 combinations; exact linked geometry/measurements and three separate Undo outcomes", captures, flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", required=True)
    args = parser.parse_args()
    captures = ROOT / ".evidence/016" / datetime.now().strftime("layout-adoption-%Y%m%d-%H%M%S")
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

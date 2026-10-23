#!/usr/bin/env python3
"""016: computed styles and node identity on actual Matrix and transform fields."""
import argparse
from datetime import datetime
import importlib
from pathlib import Path


driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "layout-fields-016"
browser, evaluate, check, click = driver.browser, driver.evaluate, driver.check, driver.click
ROOT = Path(__file__).resolve().parents[1]


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
    click("Matrix… Rows, columns & key assemblies")
    for name in ["Rows", "Columns"]:
        browser("focus", driver.by_role_name("slider", name))
        browser("press", "Home")
    click("Create matrix")
    browser("wait", "--fn", "document.querySelectorAll('.m1-matrix-key').length===1")
    evaluate("window.fieldNodes={}")
    failures = []
    for width in [1440, 1100, 390]:
        browser("set", "viewport", width, 900)
        for theme in ["original", "shadcn"]:
            for appearance in ["light", "dark"]:
                evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
                if width < 980:
                    click("Workbench menu")
                    click("Inspector")
                    browser("find", "role", "tab", "click", "--name", "Inspector", "--exact")
                for role, name in [("textbox", "Matrix name"), ("spinbutton", "Origin X")]:
                    browser("focus", driver.by_role_name(role, name))
                    evaluate("window.actualField=document.activeElement")
                    browser("press", "Tab")
                    browser("press", "Shift+Tab")
                    check("document.activeElement===actualField && actualField.matches(':focus-visible')")
                    browser("wait", "--fn", "!actualField.getAnimations().some(a=>a.playState==='running')")
                    result = evaluate("""(()=>{
                        const input=actualField,group=input.closest('[data-slot=input-group]');
                        const s=getComputedStyle(input),g=getComputedStyle(group),r=input.getBoundingClientRect();
                        const paint=document.createElement('span');paint.style.background='var(--field)';document.body.append(paint);
                        const expected=getComputedStyle(paint).backgroundColor;paint.remove();
                        const legacyMatches=[];
                        for(const sheet of document.styleSheets){
                            let rules;try{rules=sheet.cssRules}catch{continue}
                            for(const rule of rules){
                                if(rule.selectorText?.startsWith('.m1-matrix-field-input')&&input.matches(rule.selectorText))
                                    legacyMatches.push({selector:rule.selectorText,style:rule.style.cssText});
                            }
                        }
                        const name=input.getAttribute('aria-label'),same=!fieldNodes[name]||fieldNodes[name]===input;
                        fieldNodes[name]=input;
                        // Unadopted CSS reference only; no substitute owner, handlers or draft.
                        const legacy=document.createElement('input');legacy.setAttribute('aria-hidden','true');legacy.tabIndex=-1;
                        legacy.style.position='absolute';legacy.style.left='-10000px';input.parentElement.append(legacy);
                        const old=getComputedStyle(legacy),reference={border:old.borderTopWidth,background:old.backgroundColor};legacy.remove();
                        return {slot:input.getAttribute('data-slot'),background:s.backgroundColor,border:s.borderTopWidth,
                            radius:s.borderTopLeftRadius,outline:s.outlineStyle,font:s.fontSize,height:r.height,
                            groupBackground:g.backgroundColor,expectedBackground:expected,groupFocus:g.boxShadow.includes('3px'),
                            sameNode:same,label:input.labels.length===1&&input.labels[0].control===input,reference,legacyMatches};
                    })()""")
                    print(width, theme, appearance, name, result, flush=True)
                    checks = {"copied input": result["slot"] == "input-group-control",
                              "transparent input": result["background"] == "rgba(0, 0, 0, 0)",
                              "border": result["border"] == "0px", "square input": result["radius"] == "0px",
                              "focus": result["outline"] == "none" and result["groupFocus"],
                              "field background": result["groupBackground"] == result["expectedBackground"],
                              "font": result["font"] == "16px",
                              "target": result["height"] >= (44 if width < 760 else 40),
                              "node and label": result["sameNode"] and result["label"],
                              "unadopted reference": result["reference"]["border"] == "1px"}
                    failures.extend((width, theme, appearance, name, check_name) for check_name, passed in checks.items() if not passed)
                browser("screenshot", str(captures / f"{width}-{theme}-{appearance}.png"))
                if width < 980:
                    click("Close panels")
    print("Actual field style failures:", failures, "captures:", captures, flush=True)
    assert not failures, failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", required=True)
    args = parser.parse_args()
    captures = ROOT / ".evidence/016" / datetime.now().strftime("layout-field-styles-%Y%m%d-%H%M%S")
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

#!/usr/bin/env python3
"""016: actual perimeter row alignment/selection and owned Secondary action styles."""
import argparse
from datetime import datetime
import importlib
from pathlib import Path


flow = importlib.import_module("test-layout-adoption")
fields = importlib.import_module("test-layout-field-styles")
driver = flow.driver
driver.SESSION = "layout-point-controls-016"
browser, evaluate, check, click = driver.browser, driver.evaluate, driver.check, driver.click
ROOT = Path(__file__).resolve().parents[1]


def dismiss_notifications():
    refs = browser("snapshot", "-i")["refs"]
    count = sum(item["role"] == "button" and item["name"] == "close" for item in refs.values())
    for _ in range(count):
        refs = browser("snapshot", "-i")["refs"]
        close = [ref for ref, item in refs.items() if item["role"] == "button" and item["name"] == "close"]
        if close:
            browser("click", "@" + close[0])


def secondary(name):
    browser("focus", driver.by_role_name("button", name))
    evaluate("window.secondaryAction=document.activeElement")
    browser("press", "Tab")
    browser("press", "Shift+Tab")
    check("document.activeElement===secondaryAction && secondaryAction.matches(':focus-visible')")
    browser("wait", "--fn", "!secondaryAction.getAnimations().some(a=>a.playState==='running')")
    result = evaluate("""(()=>{
        const s=getComputedStyle(secondaryAction),paint=document.createElement('span');
        paint.style.background='var(--secondary)';paint.style.color='var(--secondary-foreground)';
        document.body.append(paint);const expected=getComputedStyle(paint);
        const result={slot:secondaryAction.dataset.slot,background:s.backgroundColor,color:s.color,
            expectedBackground:expected.backgroundColor,expectedColor:expected.color,
            border:s.borderTopWidth,padding:s.padding,outline:s.outlineStyle,ring:s.boxShadow.includes('3px')};
        paint.remove();return result;
    })()""")
    print("Actual Secondary", name, result, flush=True)
    return result["slot"] == "button" and result["background"] == result["expectedBackground"] and result["color"] == result["expectedColor"] and result["border"] == "0px" and result["padding"] == "8px 16px" and result["outline"] == "none" and result["ring"]


def inline_error(name, submit):
    browser("focus", driver.by_role_name("spinbutton", name))
    evaluate("window.invalidField=document.activeElement;window.acceptedFieldText=invalidField.value")
    browser("fill", driver.by_role_name("spinbutton", name), "")
    if submit:
        browser("press", "Enter")
    browser("wait", "--fn", "document.querySelectorAll('[data-slot=field-error]').length===1")
    check("document.querySelector('[data-slot=field-error]').getAttribute('role')==='alert'")
    result = evaluate("(()=>{const error=document.querySelector('[data-slot=field-error]'),s=getComputedStyle(error),paint=document.createElement('span');paint.style.color='var(--destructive)';document.body.append(paint);const result={font:s.fontSize,color:s.color,expected:getComputedStyle(paint).color,text:error.textContent};paint.remove();return result})()")
    print("Actual inline error", name, result, flush=True)
    browser("press", "Escape")
    browser("wait", "--fn", "!document.querySelector('[data-slot=field-error]') && invalidField.value===acceptedFieldText")
    return result["font"] == "14px" and result["color"] == result["expected"]


def tests(url, captures):
    flow.new_layout_project(url)
    flow.create_single_matrix(1)
    original = flow.geometry()
    click("Group objects by")
    browser("find", "role", "option", "click", "--name", "By part", "--exact")
    click("SW1 switch mx · front")
    dismiss_notifications()
    failures = []
    for owner in ["component", "outline", "points"]:
        browser("set", "viewport", 1440, 900)
        if owner == "outline":
            click("Outline Generated")
            dismiss_notifications()
        elif owner == "points":
            click("Edit perimeter points")
        for width in [1440, 1100, 390]:
            browser("set", "viewport", width, 900)
            for theme in ["original", "shadcn"]:
                for appearance in ["light", "dark"]:
                    evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
                    if width < 980:
                        fields.inspector_sheet()
                    actions = ["Edit electrical connections"] if owner == "component" else ["Edit perimeter points"]
                    if owner in ["component", "outline"]:
                        if not inline_error("X mm" if owner == "component" else "Outline margin", owner == "component"):
                            failures.append((width, theme, appearance, "copied inline error"))
                        if width < 980:
                            fields.inspector_sheet()
                    if owner == "points":
                        for index in [1, 2]:
                            browser("focus", driver.by_role_name("button", f"Select outline point {index}"))
                            evaluate(f"window.point{index}=document.activeElement")
                            browser("press", "Space")
                        result = evaluate("""(()=>{
                            const header=point2.closest('.m1-outline-point-list').querySelector('.m1-outline-point-columns');
                            return {first:point1.getAttribute('aria-pressed'),second:point2.getAttribute('aria-pressed'),
                                display:getComputedStyle(point2).display,header:[...header.children].map(e=>e.getBoundingClientRect().x),
                                row:[...point2.children].map(e=>e.getBoundingClientRect().x)};
                        })()""")
                        print("Actual point rows", width, theme, appearance, result, flush=True)
                        if result["first"] != "false" or result["second"] != "true":
                            failures.append((width, theme, appearance, "point selection"))
                        if result["display"] != "grid" or any(abs(a-b) > 1 for a, b in zip(result["header"], result["row"])):
                            failures.append((width, theme, appearance, "point columns"))
                        actions = ["Done", "Insert after 2", "Remove point 2"]
                    for name in actions:
                        if not secondary(name):
                            failures.append((width, theme, appearance, name))
                    browser("screenshot", str(captures / f"{owner}-{width}-{theme}-{appearance}.png"))
                    if width < 980:
                        click("Close panels")
    assert flow.geometry() == original
    print("Actual point and Secondary failures:", failures, "captures:", captures, flush=True)
    assert not failures, failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", required=True)
    args = parser.parse_args()
    captures = ROOT / ".evidence/016" / datetime.now().strftime("layout-point-controls-%Y%m%d-%H%M%S")
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

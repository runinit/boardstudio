#!/usr/bin/env python3
"""016 review: real picker hit targets, action/error tokens, Tooltip and failure ARIA."""
import argparse
from datetime import datetime
import importlib
import json
import os
from pathlib import Path
import traceback

flow = importlib.import_module("test-layout-adoption")
points = importlib.import_module("test-layout-point-controls")
driver = flow.driver
driver.SESSION = "layout-review-016"
browser, evaluate, check, click = driver.browser, driver.evaluate, driver.check, driver.click
ROOT = Path(__file__).resolve().parents[1]


def prepare(url):
    flow.new_layout_project(url)
    flow.create_single_matrix(1)
    points.dismiss_notifications()


def fault_boundary():
    # Fail one real Core worker request using its existing rejected-reply envelope.
    # Actual control drafts, accepted projection, callbacks and failure attribution remain in the app.
    evaluate("window.failedCoreRequests=[];window.failCore=null;window.originalWorkerPost=Worker.prototype.postMessage;Worker.prototype.postMessage=function(m,...args){if(m?.kind==='core'&&window.failCore){const error=window.failCore;window.failCore=null;failedCoreRequests.push({request:m.request_id,epoch:m.executor_epoch,frame:JSON.parse(m.frame).kind});queueMicrotask(()=>this.dispatchEvent(new MessageEvent('message',{data:{kind:'core',request_id:m.request_id,executor_epoch:m.executor_epoch,frame:JSON.stringify({kind:'error',id:m.request_id,message:error,revision:0})}})));return;}return originalWorkerPost.call(this,m,...args)}")


def choose(name, role, label):
    browser("focus", driver.by_role_name(role, name))
    if role == "combobox":
        browser("fill", driver.by_role_name(role, name), label)
        browser("press", "ArrowDown")
    else:
        browser("press", "Space")
    refs = browser("snapshot", "-i")["refs"]
    names = [item["name"] for item in refs.values() if item["role"] == "option"]
    assert label in names, (label, names)
    if role != "combobox":
        browser("press", "Home")
        for _ in range(names.index(label)):
            browser("press", "ArrowDown")
    browser("press", "Enter")
    browser("wait", "--fn", "!document.querySelector('[role=listbox][data-state=open]')")


def colors(element):
    return evaluate(f"""(()=>{{
        const e={element},s=getComputedStyle(e),paint=document.createElement('span');
        paint.style.background='var(--secondary)';paint.style.color='var(--secondary-foreground)';document.body.append(paint);
        const t=getComputedStyle(paint),r={{bg:s.backgroundColor,ink:s.color,expectedBg:t.backgroundColor,expectedInk:t.color,
            ring:s.boxShadow.includes('3px'),outline:s.outlineStyle}};
        paint.style.background='color-mix(in oklab,var(--secondary) 80%,transparent)';r.expectedHover=getComputedStyle(paint).backgroundColor;
        paint.style.color='var(--destructive)';r.expectedError=getComputedStyle(paint).color;paint.remove();return r;
    }})()""")


def styles(url, captures):
    prepare(url)
    fault_boundary()
    original = flow.geometry()
    evaluate("window.failCore='016 key-size failure'")
    choose("Key size", "combobox", "2u")
    browser("wait", "--fn", "!!document.querySelector('[data-slot=field-error]')")
    check("failedCoreRequests.length===1 && failedCoreRequests[0].frame==='edit'")
    assert flow.geometry() == original
    browser("focus", driver.by_role_name("combobox", "Key size"))
    evaluate("window.keySizeControl=document.activeElement;window.sizeError=keySizeControl.closest('section').querySelector('[data-slot=field-error]')")
    check("sizeError.textContent.includes('016 key-size failure')")
    points.dismiss_notifications()
    click("Add object")
    failures = []
    for theme in ["original", "shadcn"]:
        for appearance in ["light", "dark"]:
            evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
            browser("mouse", "move", 0, 0)
            browser("focus", driver.by_role_name("button", "← Back to objects"))
            evaluate("window.backAction=document.activeElement")
            browser("press", "Tab")
            browser("press", "Shift+Tab")
            check("document.activeElement===backAction && backAction.matches(':focus-visible')")
            browser("wait", "--fn", "!backAction.getAnimations().some(a=>a.playState==='running')")
            default = colors("backAction")
            error = colors("sizeError")
            browser("hover", driver.by_role_name("button", "← Back to objects"))
            browser("wait", "--fn", "!backAction.getAnimations().some(a=>a.playState==='running')")
            hover = evaluate("getComputedStyle(backAction).backgroundColor")
            print("Review action/error", theme, appearance, default, error, hover, flush=True)
            if default["bg"] != default["expectedBg"] or default["ink"] != default["expectedInk"] or not default["ring"] or default["outline"] != "none" or hover != default["expectedHover"]:
                failures.append((theme, appearance, "Back Secondary"))
            if error["ink"] != error["expectedError"]:
                failures.append((theme, appearance, "Key size FieldError"))
            browser("screenshot", str(captures / f"styles-{theme}-{appearance}.png"))
    click("← Back to objects")
    check("document.activeElement.getAttribute('aria-label')==='Add object'")
    choose("Key size", "combobox", "2u")
    browser("wait", "--fn", "!keySizeControl.closest('section').querySelector('[data-slot=field-error]')")
    browser("wait", "--fn", "document.querySelector('.m1-matrix-key').getAttribute('width')!=='18'")
    print("Review style failures", failures, flush=True)
    assert not failures, failures


def tooltip(url, captures):
    browser("set", "viewport", 1440, 900)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
    click("Project")
    browser("find", "role", "menuitem", "click", "--name", "New project", "--exact")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Keyboard setup\"]')")
    click("Continue to layout")
    browser("focus", driver.by_role_name("button", "Previous step"))
    evaluate("window.previousStep=document.activeElement")
    browser("press", "Tab")
    browser("press", "Shift+Tab")
    browser("wait", "--fn", "!!document.querySelector('[role=tooltip][data-state=open]')")
    assert driver.by_role_name("tooltip", "Previous step")
    check("document.activeElement===previousStep && previousStep.getAttribute('aria-describedby').split(/\\s+/).some(id=>document.getElementById(id)?.textContent==='Previous step')")
    browser("press", "Escape")
    browser("press", "Tab")
    browser("hover", driver.by_role_name("button", "Previous step"))
    browser("wait", "--fn", "!!document.querySelector('[role=tooltip][data-state=open]')")
    browser("screenshot", str(captures / "previous-step-tooltip.png"))
    browser("focus", driver.by_role_name("button", "Previous step"))
    evaluate("window.previousClicks=[];previousStep.addEventListener('click',e=>previousClicks.push(e.isTrusted),{once:true})")
    browser("press", "Space")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Project name\"]')")
    check("previousClicks.length===1 && previousClicks[0]")
    click("Continue to layout")
    browser("focus", driver.by_role_name("button", "Previous step"))
    browser("press", "Enter")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Project name\"]')")
    print("PASS Previous step focus/hover Tooltip and native Space/Enter stage callbacks", flush=True)


def pickers(url, captures):
    prepare(url)
    browser("focus", driver.by_role_name("combobox", "Key size"))
    evaluate("window.picker=document.activeElement;window.pickerNode=picker;window.pickerContent=picker.closest('[data-slot=collapsible-content]')")
    browser("press", "ArrowDown")
    target = driver.by_role_name("option", "2u")
    box = browser("get", "box", target)
    x, y = round(box["x"]+box["width"]/2), round(box["y"]+box["height"]/2)
    result = evaluate(f"(()=>{{const c=pickerContent.getBoundingClientRect(),hit=document.elementFromPoint({x},{y});return {{bottom:c.bottom,optionY:{y},overflow:getComputedStyle(pickerContent).overflow,hit:hit?.closest('[role=option]')?.textContent.trim()??null}}}})()")
    print("Actual picker boundary", result, flush=True)
    browser("screenshot", str(captures / "key-size-picker-boundary.png"))
    assert result["optionY"] > result["bottom"] and result["hit"] == "2u", result
    browser("mouse", "move", x, y)
    browser("mouse", "down")
    browser("mouse", "up")
    browser("wait", "--fn", "picker.value==='2u' && picker.getAttribute('aria-expanded')==='false'")
    click("Key size")
    check("picker===pickerNode && picker.isConnected && picker.closest('[hidden][inert]') && !picker.checkVisibility()")
    evaluate("picker.focus()")
    check("document.activeElement!==picker")
    click("Key size")
    check("picker===pickerNode && picker.value==='2u' && !picker.closest('[hidden],[inert]')")
    print("PASS actual picker beyond disclosure boundary, native selection and retained closed hidden/inert owner", flush=True)


def validation(url, captures):
    prepare(url)
    fault_boundary()
    failures = []

    def rejected_choice(name, role, label):
        browser("focus", driver.by_role_name(role, name))
        evaluate("window.rejectedControl=document.activeElement")
        message = "016 " + name + " failure"
        evaluate(f"window.failCore={json.dumps(message)}")
        choose(name, role, label)
        browser("wait", "--fn", "!!document.querySelector('[data-slot=field-error]')")
        result = evaluate("(()=>{const ids=(rejectedControl.getAttribute('aria-describedby')||'').split(/\\s+/).filter(Boolean);return {invalid:rejectedControl.getAttribute('aria-invalid'),descriptions:ids.map(id=>({id,count:document.querySelectorAll('[id=\\\"'+id+'\\\"]').length,text:document.getElementById(id)?.textContent})),focus:document.activeElement===rejectedControl}})()")
        print("Actual failed choice ARIA", name, result, flush=True)
        browser("screenshot", str(captures / (name.replace(" ", "-") + "-error.png")))
        if result["invalid"] != "true" or not result["focus"] or not any(d["count"] == 1 and message in (d["text"] or "") for d in result["descriptions"]):
            failures.append((name, "failed native ARIA/focus relationship"))
        points.dismiss_notifications()
        choose(name, role, label)
        browser("wait", "--fn", "!document.querySelector('[data-slot=field-error]')")
        check("rejectedControl.getAttribute('aria-invalid')!=='true' && !rejectedControl.hasAttribute('aria-describedby')")
        print("Accepted choice clears failure", name, flush=True)

    rejected_choice("Mirror matrix", "button", "X axis")
    click("Column 1 1 keys")
    rejected_choice("Splay origin", "button", "Custom point")
    click("Matrix 1 1 keys")
    click("Key assembly switch mx")
    rejected_choice("Diode direction", "button", "Columns to rows")
    browser("focus", driver.by_role_name("combobox", "Matrix part definition"))
    current = browser("get", "value", driver.by_role_name("combobox", "Matrix part definition"))["value"]
    browser("fill", driver.by_role_name("combobox", "Matrix part definition"), "")
    browser("press", "ArrowDown")
    refs = browser("snapshot", "-i")["refs"]
    alternatives = [item["name"] for item in refs.values() if item["role"] == "option" and item["name"] != current and not item.get("disabled")]
    assert alternatives, refs
    browser("press", "Escape")
    rejected_choice("Matrix part definition", "combobox", alternatives[0])
    check("failedCoreRequests.length===4 && failedCoreRequests.every(m=>m.frame==='edit')")
    print("Choice validation failures", failures, flush=True)
    assert not failures, failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", required=True)
    parser.add_argument("--only", choices=["styles", "tooltip", "pickers", "validation"])
    args = parser.parse_args()
    os.environ["AGENT_BROWSER_ARGS"] = "\n".join(filter(None,[os.environ.get("AGENT_BROWSER_ARGS"),"--blink-settings=primaryHoverType=2"]))
    captures = ROOT / ".evidence/016" / datetime.now().strftime("review-r1-%Y%m%d-%H%M%S")
    captures.mkdir(parents=True)
    failures = []
    try:
        for name in [args.only] if args.only else ["styles", "tooltip", "pickers", "validation"]:
            try:
                globals()[name](args.page_url, captures)
            except Exception:
                traceback.print_exc()
                failures.append(name)
                browser("screenshot", str(captures / f"{name}-failure.png"))
                print(browser("snapshot", "-i")["snapshot"], flush=True)
        print("Review correction failures", failures, "captures", captures, flush=True)
        assert not failures, failures
    finally:
        browser("close")


if __name__ == "__main__":
    main()

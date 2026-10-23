#!/usr/bin/env python3
"""016: computed styles on the actual Editor-owned Layout view controls."""
import argparse
from datetime import datetime
import importlib
from pathlib import Path

driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "layout-view-016"
browser, evaluate, click = driver.browser, driver.evaluate, driver.click
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
    failures = []
    for width in [1440, 1100, 390]:
        browser("set", "viewport", width, 900)
        for theme in ["original", "shadcn"]:
            for appearance in ["light", "dark"]:
                evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
                if width < 980:
                    if not evaluate("!!document.getElementById('workspace-panel-sheet')?.matches('[role=dialog]')"):
                        click("Workbench menu")
                        click("Objects")
                    browser("find", "role", "tab", "click", "--name", "Objects", "--exact")
                    browser("wait", "--fn", "document.getElementById('m1-add-object-trigger')?.checkVisibility()")
                browser("focus", driver.by_role_name("button", "Add object"))
                evaluate("window.actualSecondary=document.activeElement")
                browser("wait", "--fn", "!actualSecondary.getAnimations().some(a=>a.playState==='running')")
                secondary = evaluate("""(()=>{
                    const action=document.activeElement, style=getComputedStyle(action);
                    const paint=document.createElement('span');
                    paint.style.background='var(--secondary)';paint.style.color='var(--secondary-foreground)';
                    document.body.append(paint);const expected=getComputedStyle(paint);
                    const result={slot:action.getAttribute('data-slot'),filled:action.classList.contains('bg-secondary'),
                        background:style.backgroundColor,color:style.color,
                        expectedBackground:expected.backgroundColor,expectedColor:expected.color};
                    paint.remove();return result;
                })()""")
                if width < 980:
                    click("Close panels")
                browser("focus", driver.by_role_name("button", "2D"))
                evaluate("window.actualView=document.activeElement;window.actualViewGroup=actualView.closest('.m1-layout-view-group')")
                # Exercise real keyboard focus on the production owner; no substitute state.
                browser("press", "Tab")
                browser("press", "Shift+Tab")
                driver.check("document.activeElement===actualView && actualView.matches(':focus-visible')")
                evaluate("window.viewClicks=[];window.recordView=e=>viewClicks.push(e.isTrusted);actualView.addEventListener('click',recordView)")
                browser("press", "Space")
                print("Actual 2D Space", width, theme, appearance, evaluate("({clicks:viewClicks,pressed:actualView.getAttribute('aria-pressed')})"), flush=True)
                driver.check("viewClicks.length===1 && viewClicks[0] && actualView.getAttribute('aria-pressed')==='true'")
                evaluate("actualView.removeEventListener('click',recordView)")
                browser("wait", "--fn", "!actualView.getAnimations().some(a=>a.playState==='running')")
                result = evaluate("""(()=>{
                    const style=getComputedStyle(actualView), rect=actualView.getBoundingClientRect();
                    const paint=document.createElement('span');paint.style.background='var(--background)';document.body.append(paint);
                    const expectedBackground=getComputedStyle(paint).backgroundColor;paint.remove();
                    const copied={font:style.fontSize,padding:style.padding,bg:style.backgroundColor,
                        outline:style.outlineStyle,shadow:style.boxShadow,width:rect.width,height:rect.height,
                        pressed:actualView.getAttribute('aria-pressed'),slot:actualView.getAttribute('data-slot')};
                    // CSS disabled-state probe on the actual node; it is not an owner-disabled outcome.
                    const wasDisabled=actualView.disabled;actualView.disabled=true;
                    const disabled={opacity:getComputedStyle(actualView).opacity,pointerEvents:getComputedStyle(actualView).pointerEvents};
                    actualView.disabled=wasDisabled;
                    // The old, unadopted branch remains a native style reference only.
                    const legacyOwner=document.createElement('div');legacyOwner.className='m1-layout-view-group board-tool-pill';
                    const legacy=document.createElement('button');legacy.textContent='Legacy Layout style reference';legacyOwner.append(legacy);
                    actualViewGroup.parentElement.append(legacyOwner);
                    const old=getComputedStyle(legacy), legacyStyle={font:old.fontSize,padding:old.padding,bg:old.backgroundColor};
                    legacy.setAttribute('aria-pressed','true');const legacyPressed=getComputedStyle(legacy).backgroundColor;
                    legacyOwner.remove();
                    return {copied,disabled,legacyStyle,legacyPressed,expectedBackground};
                })()""")
                print(width, theme, appearance, {"view": result, "secondary action": secondary}, flush=True)
                browser("screenshot", str(captures / f"{width}-{theme}-{appearance}.png"))
                copied = result["copied"]
                checks = {
                    "actual secondary action": secondary["slot"] == "button" and secondary["filled"],
                    "secondary fill": secondary["background"] == secondary["expectedBackground"],
                    "secondary ink": secondary["color"] == secondary["expectedColor"],
                    "copied font": copied["font"] == "14px",
                    # The copy sets px-2; Chromium's UA rule supplies 1px block padding.
                    "copied padding": copied["padding"] == "1px 8px",
                    "copied pressed color": copied["pressed"] == "true" and copied["bg"] == result["expectedBackground"],
                    "copied focus": copied["outline"] == "none" and "3px" in copied["shadow"],
                    "touch rectangle": copied["width"] >= 44 and copied["height"] >= 44,
                    "copied slot coexistence": copied["slot"] in ["toggle-group-item", "tooltip-trigger"],
                    "disabled CSS": result["disabled"] == {"opacity": "0.5", "pointerEvents": "none"},
                    "unadopted font": result["legacyStyle"]["font"] == "12px",
                    "unadopted padding": result["legacyStyle"]["padding"] == "10px",
                    "unadopted pressed": result["legacyPressed"] != result["legacyStyle"]["bg"],
                }
                failures.extend((width, theme, appearance, name) for name, passed in checks.items() if not passed)
    secondary_failures = [failure for failure in failures if failure[3].startswith(("secondary", "actual secondary"))]
    view_failures = [failure for failure in failures if failure not in secondary_failures]
    print("View style failures:", view_failures, flush=True)
    print("Owned Secondary Add object failures:", secondary_failures, "captures:", captures, flush=True)
    assert not view_failures, view_failures
    assert not secondary_failures, secondary_failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", required=True, help="packaged production page built from this lane")
    args = parser.parse_args()
    captures = ROOT / ".evidence/016" / datetime.now().strftime("view-controls-%Y%m%d-%H%M%S")
    captures.mkdir(parents=True)
    try:
        tests(args.page_url, captures)
    except Exception:
        browser("screenshot", str(captures / "failure.png"))
        raise
    finally:
        browser("close")


if __name__ == "__main__":
    main()

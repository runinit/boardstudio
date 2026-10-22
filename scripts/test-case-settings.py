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
    selector = f"[role=status][aria-label={json.dumps(name)}]"
    return evaluate(f"document.querySelector({json.dumps(selector)}).textContent")


def styles(url, evidence):
    browser("set", "viewport", 1440, 960)
    browser("open", url)
    browser("wait", "--text", "Case construction")
    failures, observations = [], []
    for theme in ("original", "shadcn"):
        for appearance in ("light", "dark"):
            evaluate(f"document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.appearance='{appearance}'")
            browser("wait", "--fn", "document.fonts.status==='loaded'")
            action("spinbutton", "Plate thickness", "focus")
            metrics = evaluate('''(()=>{const input=document.activeElement,group=input.parentElement,label=input.labels[0],s=getComputedStyle(input);
                return {slot:input.dataset.slot,height:group.getBoundingClientRect().height,labelFor:label.htmlFor,id:input.id,
                    font:s.fontFamily,border:s.borderTopWidth,bg:s.backgroundColor,boxHeight:input.getBoundingClientRect().height}})()''')
            action("checkbox", "Add middle frame", "focus")
            metrics["checkbox"] = evaluate("({height:document.activeElement.getBoundingClientRect().height,border:getComputedStyle(document.activeElement).borderTopWidth,labelHeight:document.activeElement.labels[0].getBoundingClientRect().height})")
            observations.append({"theme":theme,"appearance":appearance,**metrics})
            if not (metrics["slot"] == "input-group-control" and metrics["height"] == 40 and metrics["border"] == "0px" and metrics["labelFor"] == metrics["id"] and metrics["id"] and metrics["checkbox"]["height"] == 16 and metrics["checkbox"]["border"] == "0px" and metrics["checkbox"]["labelHeight"] >= 44):
                failures.append(observations[-1])
            browser("screenshot", str(evidence/f"styles-{theme}-{appearance}.png"))
    (evidence/"style-observations.json").write_text(json.dumps(observations,indent=2)+"\n")
    assert not failures, failures
    print("PASS actual adopted numeric and checkbox computed styles in four Theme/Appearance combinations",flush=True)


def tests(url, evidence):
    browser("set", "viewport", 1440, 960)
    browser("open", url)
    browser("wait", "--text", "Case construction")
    for name in ("Plate method", "Mount style", "Bottom construction"):
        action("button", name, "focus")
        assert evaluate("document.activeElement.dataset.slot === 'select-trigger'")
        browser("press", "Enter")
        lists = [item for item in browser("snapshot")["refs"].values() if item["role"] == "listbox" and item["name"] == name]
        assert len(lists) == 1, (name, lists)
        browser("press", "ArrowDown")
        browser("press", "ArrowUp")
        browser("press", "Home")
        browser("press", "End")
        assert evaluate("document.activeElement.getAttribute('role') === 'option'")
        browser("press", "Escape")
        assert evaluate("document.activeElement.getAttribute('aria-label')") == name
        assert evaluate("document.activeElement.getAttribute('aria-expanded')") == "false"
        browser("press", "Tab")
        assert evaluate("document.activeElement.getAttribute('role') !== 'option'"), "closed retained options stay out of Tab order"
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
    refs = browser("snapshot", "-i")["refs"]
    geometry_names = [item["name"] for item in refs.values() if item["role"] == "checkbox" and item["name"].startswith("Select geometry ")]
    assert len(geometry_names) == 1, geometry_names
    geometry_name = geometry_names[0]
    action("checkbox", geometry_name)
    evaluate("window.geometryCheckbox=document.activeElement")
    action("button", "Purpose for Dwgs.User")
    browser("press", "End")
    browser("press", "Enter")
    before_collapse = int(status("Requests"))
    action("spinbutton", "Access openings volume 1 Height", "fill", "0.05")
    browser("press", "Tab")
    action("spinbutton", "Access openings volume 1 Height", "focus")
    assert evaluate("document.activeElement.getAttribute('aria-invalid')") == "true"
    evaluate("window.profileVolumeDraft=document.activeElement")
    action("button", "Switch fit profiles", "focus")
    browser("press", "Enter")
    assert evaluate("document.activeElement.getAttribute('aria-expanded')") == "false"
    assert evaluate("getComputedStyle(document.getElementById(document.activeElement.getAttribute('aria-controls'))).display === 'none'")
    browser("press", "Tab")
    assert evaluate("!profileVolumeDraft.closest('[data-slot=collapsible-content]').contains(document.activeElement)"), "closed retained profile content stays out of Tab order"
    action("button", "Switch fit profiles", "focus")
    browser("press", "Space")
    action("spinbutton", "Access openings volume 1 Height", "focus")
    assert evaluate("document.activeElement===profileVolumeDraft && document.activeElement.value==='0.05' && document.activeElement.getAttribute('aria-invalid')==='true'"), "real local invalid draft and input identity survive collapse"
    assert int(status("Requests")) == before_collapse, "invalid retained draft never submits"
    browser("press", "Escape")
    action("checkbox", geometry_name, "focus")
    assert evaluate("document.activeElement.getAttribute('aria-checked')") == "true", "real local extraction selection survives collapse"
    assert evaluate("document.activeElement===geometryCheckbox"), "collapse preserves the actual local control identity"
    action("button", "Purpose for Dwgs.User", "focus")
    assert evaluate("document.activeElement.textContent.includes('Drawing guide')"), "purpose draft stays with geometry owner"
    # Re-select plate cutout and apply through the real Core extraction port.
    browser("press", "Enter")
    browser("press", "Home")
    browser("press", "Enter")
    action("button", "Apply selected geometry")
    assert "UpdateProfile" in status("Last request") and "-9.5" in status("Last request")
    action("spinbutton", "Plate cutout geometry contour 1 vertex 1 X", "fill", "-9.123456789012345")
    assert repr(float("-9.123456789012345")) in status("Last request"), "exact parsed f64 reaches the profile callback"
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
    action("button", "Twelve profile targets")
    action("button", "Assign library fit profile to")
    browser("press", "End")
    browser("press", "Enter")
    assert "part-12" in status("Last request"), "12 choices use Select; 13 use searchable Combobox"
    action("button", "Fixture layer, 1.50 mm, Z 0.00", "focus")
    browser("press", "Space")
    assert status("Selected layer") == "fixture-layer"
    browser("press", "Space")
    assert status("Selected layer") == ""
    browser("press", "ArrowDown")
    assert evaluate("document.activeElement.getAttribute('aria-label')") == "Second layer, 1.60 mm, Z 2.00"
    browser("press", "Enter")
    assert status("Selected layer") == "second-layer"
    browser("press", "Home")
    assert evaluate("document.activeElement.getAttribute('aria-label')") == "Fixture layer, 1.50 mm, Z 0.00"
    browser("press", "End")
    assert evaluate("document.activeElement.getAttribute('aria-label')") == "Second layer, 1.60 mm, Z 2.00"
    assert evaluate("[...document.activeElement.parentElement.children].filter(e=>e.tabIndex===0).length") == 1
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


def page_tests(url, evidence):
    """The packaged app, its Runtime, durable save and real Undo/Redo owners."""
    browser("set", "viewport", 1440, 960)
    browser("open", url)
    browser("wait", "--text", "Your keyboards")
    action("button", "Project")
    action("menuitem", "New project")
    for name in ("Continue to layout", "Hide guide", "Cancel"):
        browser("wait", "--text", name)
        action("button", name)
    action("button", "Add key matrix")
    action("button", "Create matrix")
    browser("wait", "--text", "Matrix 1")
    action("button", "Open Case workspace")
    action("button", "Configure mechanical stack")
    browser("wait", "--text", "Dimensions and clearances")
    action("spinbutton", "Wall thickness", "focus")
    original = evaluate("document.activeElement.value")
    assert evaluate("document.activeElement.dataset.slot") == "input-group-control"
    precise = "2.1234567890123457"

    def saved_wall(value, owner=None):
        # Observe the real durable document, not the previous operation's save indicator.
        expression = """(async()=>{
            const database='boardstudio-m1-'+(location.pathname.startsWith('/boardstudio/')?'boardstudio':'root');
            const id=localStorage.getItem('boardstudio-m1-active-project:'+database);
            if(!id) return false;
            const doc=await new Promise((resolve,reject)=>{
                const request=indexedDB.open(database);
                request.onerror=()=>reject(request.error);
                request.onupgradeneeded=()=>{request.transaction.abort();reject(Error('expected existing project database'))};
                request.onsuccess=()=>{const db=request.result,transaction=db.transaction('projects','readonly');
                    const read=transaction.objectStore('projects').get(id);
                    read.onsuccess=()=>resolve(read.result);read.onerror=()=>reject(read.error);
                    transaction.oncomplete=()=>db.close();transaction.onabort=()=>{db.close();reject(transaction.error)};};
            });
            if(!doc) return false;
            const owners=[['project',doc.mechanical],['shared',doc.hardware?.sharedConstruction],
                ...(doc.hardware?.instances??[]).map(instance=>['instance:'+instance.id,instance.mechanical])];
            const matches=owners.filter(([name,config])=>config?.wallThickness===EXPECTED && (OWNER===null || name===OWNER));
            if(matches.length!==1) return false;
            window.caseSavedWall={id:doc.id,revision:doc.revision,owner:matches[0][0],wallThickness:matches[0][1].wallThickness};
            return true;
        })()""".replace('EXPECTED', json.dumps(float(value))).replace('OWNER', json.dumps(owner))
        browser("wait", "--fn", expression)
        return evaluate("window.caseSavedWall")

    action("spinbutton", "Wall thickness", "fill", precise)
    browser("press", "Escape")
    browser("wait", "--fn", f"document.activeElement.value === {json.dumps(original)}")
    action("spinbutton", "Wall thickness", "fill", "0.05")
    browser("press", "Tab")
    action("spinbutton", "Wall thickness", "focus")
    assert evaluate("document.activeElement.getAttribute('aria-invalid')") == "true"
    browser("press", "Escape")
    action("spinbutton", "Wall thickness", "fill", precise)
    browser("press", "Enter")
    saved = saved_wall(precise)
    action("spinbutton", "Wall thickness", "focus")
    browser("wait", "--fn", f"document.activeElement.value === {json.dumps(precise)}")
    action("button", "Project", "focus")
    browser("press", "Control+z")
    action("spinbutton", "Wall thickness", "focus")
    browser("wait", "--fn", f"document.activeElement.value === {json.dumps(original)}")
    undone = saved_wall(original, saved["owner"])
    assert undone["id"] == saved["id"] and undone["revision"] > saved["revision"]
    action("button", "Project", "focus")
    browser("press", "Control+Shift+z")
    action("spinbutton", "Wall thickness", "focus")
    browser("wait", "--fn", f"document.activeElement.value === {json.dumps(precise)}")
    redone = saved_wall(precise, saved["owner"])
    assert redone["id"] == saved["id"] and redone["revision"] > undone["revision"]
    for theme in ("Original", "shadcn"):
        action("button", "Settings")
        action("menuitemradio", theme)
        for appearance in ("Light", "Dark"):
            action("button", "Settings")
            action("menuitemradio", appearance)
            browser("wait", "--fn", "document.fonts.status==='loaded'")
            action("spinbutton", "Wall thickness", "focus")
            assert evaluate("document.activeElement.value") == precise
            for ref, item in browser("snapshot", "-i")["refs"].items():
                if item.get("role") == "button" and item.get("name") == "close":
                    browser("click", "@"+ref)
            browser("screenshot", str(evidence/f"page-{theme.lower()}-{appearance.lower()}.png"))
    (evidence/"page-result.json").write_text(json.dumps({"original":original,"precise":precise,"nativeEscape":True,"validation":True,"undo":True,"redo":True,"saved":saved,"undone":undone,"redone":redone,"themes":4,"browserErrors":browser("errors").get("errors",[])},indent=2)+"\n")
    print("PASS packaged Case Runtime: native precision/validation/Escape, durable save, exact Undo/Redo, four Theme/Appearance captures",flush=True)


def capture_failure(evidence):
    try:
        browser("screenshot", str(evidence/"failure.png"))
        (evidence/"failure-snapshot.json").write_text(json.dumps(browser("snapshot"),indent=2)+"\n")
    except Exception as capture_error:
        print("Failure capture unavailable:", capture_error, flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, default=ROOT/".evidence/017/browser")
    parser.add_argument("--styles-only", action="store_true")
    parser.add_argument("--page-url")
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    print("Source pin:", subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(), flush=True)
    if args.page_url:
        try:
            page_tests(args.page_url,args.evidence)
        except BaseException:
            capture_failure(args.evidence)
            raise
        finally:
            browser("close")
        return
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
            url = f"http://127.0.0.1:{server.server_port}"
            styles(url, args.evidence)
            if not args.styles_only:
                tests(url, args.evidence)
        except BaseException:
            capture_failure(args.evidence)
            raise
        finally:
            browser("close")
            server.shutdown()


if __name__ == "__main__":
    main()

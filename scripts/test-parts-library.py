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
    parser.add_argument("--sizing-baseline", type=Path)
    args = parser.parse_args()
    sizing_baseline = json.loads(args.sizing_baseline.read_text()) if args.sizing_baseline else []
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
        hit = evaluate(f"(()=>{{const target=document.elementFromPoint({round(point['x'])},{round(point['y'])}),label=[...document.querySelectorAll('label[for]')].find(e=>e.htmlFor===partsCheckbox.id);return label.contains(target);}})()")
        assert hit, (name, point, "native label pointer target is occluded")
        evaluate("window.partsLabelClicks=[];partsCheckbox.addEventListener('click',e=>partsLabelClicks.push({trusted:e.isTrusted,target:e.target.getAttribute('aria-label')}));window.partsPointerEvents=[];['pointerdown','pointerup','click'].forEach(type=>document.addEventListener(type,e=>partsPointerEvents.push({type,tag:e.target.tagName,text:e.target.textContent?.slice(0,60),name:e.target.getAttribute('aria-label'),trusted:e.isTrusted,x:e.clientX,y:e.clientY}),{capture:true,once:true}))")
        browser("mouse", "move", round(point["x"]), round(point["y"]))
        browser("mouse", "down")
        browser("mouse", "up")
        browser("wait", "--fn", f"partsCheckbox.getAttribute('aria-checked')==='{before}'", "--timeout", 5000)
        assert attr("checkbox", name, "aria-checked") == before, (name, "label activation")
        print("Native label activation:", name, evaluate("partsLabelClicks"), flush=True)

    def slider_geometry(name, width):
        focus("slider", name)
        metrics = evaluate("""(()=>{const thumb=document.activeElement,root=thumb.closest('[data-slot=slider]'),
            track=root.querySelector('[data-slot=slider-track]'),t=thumb.getBoundingClientRect(),r=track.getBoundingClientRect();
            const clips=[];
            for(let e=thumb.parentElement;e;e=e.parentElement){
                if(/hidden|clip|auto|scroll/.test(getComputedStyle(e).overflow)){
                    const b=e.getBoundingClientRect();clips.push([b.left,b.top,b.right,b.bottom]);
                }
            }
            return {value:+thumb.getAttribute('aria-valuenow'),min:+thumb.getAttribute('aria-valuemin'),max:+thumb.getAttribute('aria-valuemax'),
                center:[t.x+t.width/2,t.y+t.height/2],track:[r.x,r.y,r.width,r.height],thumb:[t.left,t.top,t.right,t.bottom],clips,
                row:root.getBoundingClientRect().height,disabled:thumb.disabled,ariaDisabled:thumb.getAttribute('aria-disabled'),
                position:getComputedStyle(thumb).position,overflow:getComputedStyle(track).overflow,
                scroll:thumb.closest('.m1-case-view').scrollTop};})()""")
        expected = metrics["track"][0] + (metrics["value"]-metrics["min"])/(metrics["max"]-metrics["min"])*metrics["track"][2]
        assert abs(metrics["center"][0]-expected) < 0.1, metrics
        assert abs(metrics["center"][1]-(metrics["track"][1]+metrics["track"][3]/2)) < 0.1, metrics
        assert metrics["position"] == "absolute" and metrics["overflow"] == "visible", metrics
        assert metrics["row"] == (44 if width <= 960 else 40), metrics
        assert not metrics["disabled"] and metrics["ariaDisabled"] == "false" and metrics["scroll"] == 0, metrics
        # Focus ring is four pixels; require it inside every clipping ancestor, including the scrollable popup.
        left, top, right, bottom = metrics["thumb"]
        assert all(left-4 >= c[0]-0.1 and top-4 >= c[1]-0.1 and right+4 <= c[2]+0.1 and bottom+4 <= c[3]+0.1
                   for c in metrics["clips"]), metrics
        return metrics

    try:
        browser("set", "viewport", 1440, 960)
        browser("open", args.url)
        browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
        action("button", "Project")
        action("menuitem", "New project")
        browser("wait", "--fn", "document.querySelector('[aria-label=\"Keyboard setup\"]')?.checkVisibility()")
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
        action("button", "Case layers and colors")
        action("button", "Select PCB")
        assert attr("button", "Select PCB", "aria-pressed") == "true"
        focus("button", "Toggle visibility for PCB")
        visible = attr("button", "Toggle visibility for PCB", "aria-pressed")
        browser("press", "Space")
        assert attr("button", "Toggle visibility for PCB", "aria-pressed") != visible
        action("button", "Toggle visibility for PCB")
        assert attr("button", "Toggle visibility for PCB", "aria-pressed") == visible
        # agent-browser omits Chromium's native color well from both snapshots; Tab from its named predecessor.
        browser("press", "Tab")
        original_color = evaluate("document.activeElement.value")
        check("document.activeElement.type==='color' && document.activeElement.getAttribute('aria-label')==='Color for PCB' && document.activeElement.id && [...document.querySelectorAll('label[for]')].some(e=>e.htmlFor===document.activeElement.id)")
        # Native color dialog UI is outside this check; this exercises the retained native input callback.
        evaluate("window.partsColor=document.activeElement;partsColor.value='#123456';partsColor.dispatchEvent(new Event('input',{bubbles:true}))")
        browser("wait", "--fn", "![...document.querySelectorAll('[data-slot=button]')].find(e=>e.textContent.trim()==='Reset color').disabled")
        action("button", "Reset color")
        assert evaluate("partsColor.value") == original_color
        assert attr("button", "Reset color", "disabled") is not None
        action("button", "Case layers and colors")
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
        track = evaluate("(()=>{const r=document.activeElement.closest('[data-slot=slider]').getBoundingClientRect();return {x:r.left+r.width/4,y:r.top+r.height/2}})()")
        browser("mouse", "move", round(track["x"]), round(track["y"]))
        browser("mouse", "down")
        browser("mouse", "up")
        assert -51 <= float(attr("slider", "Section position", "aria-valuenow")) <= -49
        focus("slider", "Section position")
        browser("press", "End")
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
        for theme in ["original", "shadcn"]:
            for appearance in ["light", "dark"]:
                browser("set", "viewport", 1440, 960)
                action("button", "Settings")
                action("menuitemradio", "Original" if theme == "original" else "shadcn")
                action("button", "Settings")
                action("menuitemradio", appearance.title())
                browser("wait", "--fn", f"document.documentElement.dataset.theme==='{theme}' && document.documentElement.dataset.appearance==='{appearance}'")
                evaluate("document.fonts.ready.then(()=>true)")
                for width, height in [(1440, 960), (1100, 900), (390, 844)]:
                    browser("set", "viewport", width, height)
                    if width < 980 and has("button", "Close panels"):
                        action("button", "Close panels")
                    for name in ("Fit sample", "View controls"):
                        focus("button", name)
                        icon = evaluate("(()=>{const button=document.activeElement,svg=button.querySelector('svg'),r=svg.getBoundingClientRect(),b=button.getBoundingClientRect(),s=getComputedStyle(svg);return {width:r.width,height:r.height,fill:s.fill,stroke:s.stroke,strokeWidth:parseFloat(s.strokeWidth),bounded:r.left>=b.left&&r.right<=b.right&&r.top>=b.top&&r.bottom<=b.bottom,targetWidth:b.width,targetHeight:b.height}})()")
                        assert icon["width"] == 16 and icon["height"] == 16 and icon["bounded"], icon
                        assert icon["fill"] == "none" and icon["stroke"] not in ("none", "rgba(0, 0, 0, 0)") and icon["strokeWidth"] == 1.5, icon
                        if width <= 960:
                            assert icon["targetWidth"] >= 44 and icon["targetHeight"] >= 44, icon
                    action("button", "View controls")
                    for mode, name, minimum, maximum, step, quarter in [
                        ("exploded", "Explode amount", 0, 10, 0.1, 2.5),
                        ("section", "Section position", -100, 100, 1, -50),
                    ]:
                        action("button", mode)
                        focus("slider", name)
                        for key, expected_value in [("Home", minimum), ("ArrowRight", minimum+step), ("End", maximum)]:
                            browser("press", key)
                            geometry = slider_geometry(name, width)
                            assert abs(geometry["value"]-expected_value) < 1e-6, geometry
                            print("Slider geometry:", width, theme, appearance, name, key, geometry, flush=True)
                        point = geometry["track"]
                        browser("mouse", "move", round(point[0]+point[2]/4), round(point[1]+point[3]/2))
                        browser("mouse", "down")
                        browser("mouse", "up")
                        geometry = slider_geometry(name, width)
                        assert abs(geometry["value"]-quarter) <= step, geometry
                        retained("View controls", "slider", name)
                        assert float(attr("slider", name, "aria-valuenow")) == geometry["value"]
                        browser("press", "End")
                    for role, name in [("button", "Top view"), ("checkbox", "Show hidden lines")]:
                        focus(role, name)
                        check("""(()=>{const active=document.activeElement,root=active.closest('.m1-case-view'),
                            content=active.closest('[data-slot=collapsible-content]'),toolbar=content.parentElement.parentElement,
                            r=active.getBoundingClientRect(),t=toolbar.getBoundingClientRect(),v=root.getBoundingClientRect();
                            return root.scrollTop===0 && t.top>=v.top && t.bottom<=v.bottom
                                && active.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2));})()""")
                        browser("press", "Tab")
                        check("document.querySelector('.m1-parts-sample-viewer .m1-case-view').scrollTop===0")
                        browser("press", "Shift+Tab")
                    mode_names = ("shaded", "wireframe", "hybrid")
                    before_modes = [attr("button", name, "aria-pressed") for name in mode_names]
                    focus("button", "Top view")
                    tab_order = []
                    for _ in range(20):
                        state = evaluate("""(()=>{const active=document.activeElement,r=active.getBoundingClientRect();
                            return {name:active.getAttribute('aria-label')||active.textContent.trim(),
                                visible:active.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)),
                                scroll:active.closest('.m1-case-view')?.scrollTop,
                                position:getComputedStyle(active).position,parentOverflow:getComputedStyle(active.parentElement).overflow,
                                rect:[r.left,r.top,r.right,r.bottom],parentRect:(()=>{const p=active.parentElement.getBoundingClientRect();return [p.left,p.top,p.right,p.bottom]})(),
                                hit:document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)?.outerHTML};})()""")
                        assert state["visible"] and state["scroll"] == 0, state
                        tab_order.append(state["name"])
                        if state["name"] == "Show hidden lines":
                            break
                        browser("press", "Tab")
                    assert tab_order == ["Top view", "Bottom view", "Isometric view", "Rotate left", "Rotate right",
                                         "Zoom in", "Zoom out", tab_order[7], "section", "Section plane", "Section position",
                                         "Show section plane", "Show hidden lines"], tab_order
                    assert tab_order[7] in mode_names, tab_order
                    assert [attr("button", name, "aria-pressed") for name in mode_names] == before_modes
                    print("Reachable popup Tab order:", width, theme, appearance, tab_order, flush=True)
                    before_pointer = attr("checkbox", "Show hidden lines", "aria-checked")
                    action("checkbox", "Show hidden lines")
                    assert attr("checkbox", "Show hidden lines", "aria-checked") != before_pointer
                    action("checkbox", "Show hidden lines")
                    assert attr("checkbox", "Show hidden lines", "aria-checked") == before_pointer
                    focus("checkbox", "Show hidden lines")
                    metrics = evaluate("""(() => {
                        const content=document.activeElement.closest('[data-slot=collapsible-content]');
                        const toolbar=content.parentElement.parentElement;
                        const selected=[...toolbar.querySelectorAll('[data-slot=toggle-group-item][aria-pressed=true]')];
                        const probe=document.createElement('div');
                        probe.style.backgroundColor='var(--background)'; document.body.append(probe);
                        const expected=getComputedStyle(probe).backgroundColor; probe.remove();
                        const bounds=content.getBoundingClientRect();
                        const root=toolbar.closest('.m1-case-view'),canvas=root.querySelector('canvas');
                        const rect=e=>{const r=e.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height}};
                        const clips=[];
                        for(let e=content.parentElement;e;e=e.parentElement){
                            if(/hidden|clip|auto|scroll/.test(getComputedStyle(e).overflowY)){
                                const r=e.getBoundingClientRect();clips.push({class:e.className,top:r.top,bottom:r.bottom,scroll:e.scrollTop});
                            }
                        }
                        const targets=[...toolbar.querySelectorAll('[data-slot=button],[data-slot=toggle-group-item],[data-slot=collapsible-trigger],[data-slot=select-trigger]')]
                            .filter(e=>e.checkVisibility()).map(e=>({name:e.getAttribute('aria-label')||e.textContent.trim(),height:e.getBoundingClientRect().height,width:e.getBoundingClientRect().width}));
                        const checks=[...content.querySelectorAll('[role=checkbox]')].map(e=>e.closest('label').getBoundingClientRect().height);
                        const sliders=[...content.querySelectorAll('[data-slot=slider]')].map(e=>e.getBoundingClientRect().height);
                        return {legacy:toolbar.classList.contains('m1-case-view-toolbar'),
                            position:getComputedStyle(toolbar).position,display:getComputedStyle(toolbar).display,wrap:getComputedStyle(toolbar).flexWrap,
                            popupPosition:getComputedStyle(content).position,popupOwner:content.offsetParent===toolbar,
                            nativeColor:{connected:partsColor.isConnected,type:partsColor.type,slot:partsColor.getAttribute('data-slot'),background:getComputedStyle(partsColor).backgroundColor},
                            bounds:[bounds.left,bounds.right,bounds.top,bounds.bottom],
                            clips,root:rect(root),canvas:rect(canvas),toolbar:rect(toolbar),canvasPixels:[canvas.width,canvas.height],rootScroll:root.scrollTop,
                            scroll:toolbar.scrollWidth,client:toolbar.clientWidth,
                            expected,targets,checks,sliders,
                            selected:selected.map(e=>({background:getComputedStyle(e).backgroundColor,border:getComputedStyle(e).borderWidth,height:e.getBoundingClientRect().height}))};
                    })()""")
                    assert not metrics["legacy"] and metrics["scroll"] <= metrics["client"], metrics
                    assert (metrics["position"], metrics["display"], metrics["wrap"]) == ("relative", "flex", "wrap"), metrics
                    assert metrics["popupPosition"] == "absolute" and metrics["popupOwner"], metrics
                    assert metrics["nativeColor"]["connected"] and metrics["nativeColor"]["type"] == "color" and metrics["nativeColor"]["slot"] is None, metrics
                    assert 0 <= metrics["bounds"][0] < metrics["bounds"][1] <= width, metrics
                    assert 0 <= metrics["bounds"][2] < metrics["bounds"][3] <= height, metrics
                    assert all(metrics["bounds"][2] >= clip["top"] and metrics["bounds"][3] <= clip["bottom"] for clip in metrics["clips"]), metrics
                    assert metrics["rootScroll"] == 0 and metrics["toolbar"]["height"] + 8 <= 80, metrics
                    if sizing_baseline:
                        baseline = next(e for e in sizing_baseline if (e["theme"], e["appearance"], e["width"]) == (theme, appearance, width))
                        for element in ("root", "canvas", "toolbar"):
                            assert metrics[element] == baseline[element], (element, metrics[element], baseline[element])
                        assert metrics["canvasPixels"] == baseline["canvasPixels"], (metrics, baseline)
                    assert metrics["selected"], metrics
                    assert all(e["background"] == metrics["expected"] and e["border"] == "0px" and e["height"] >= 36 for e in metrics["selected"]), metrics
                    if width <= 960:
                        assert all(e["height"] >= 44 and e["width"] >= 44 for e in metrics["targets"]), metrics
                        assert all(height >= 44 for height in metrics["checks"] + metrics["sliders"]), metrics
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
                        action("tab", "Inspector")
                        focus("button", "Place component")
                        check("document.activeElement.getAttribute('data-slot')==='button' && !document.activeElement.disabled && document.activeElement.getBoundingClientRect().width>0")
                        browser("screenshot", str((args.evidence / f"inspector-{width}-{theme}-{appearance}.png").resolve()))
                        action("button", "Close panels")
        action("button", "2D footprint")
        browser("wait", "--fn", "document.querySelector('[aria-label=\"2D footprint\"]').getAttribute('aria-pressed')==='true'")
        print("PASS: packaged Parts Select, preview ToggleGroup, retained production owner values/identity, sliders, checkbox Space/Enter/labels, and 28 library/preview/Inspector theme-width captures", flush=True)
    except Exception:
        print("Failure interaction diagnostics:", evaluate("({pointer:window.partsPointerEvents,labelClicks:window.partsLabelClicks,checkbox:window.partsCheckbox?.outerHTML,active:document.activeElement?.outerHTML})"), flush=True)
        browser("screenshot", str((args.evidence / "failure.png").resolve()))
        raise
    finally:
        browser("close")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""002/009: real-browser narrow viewport proof for Layout ToolRow.

Exploratory acceptance probe, not ticket closure. Requires a packaged page
built from this checkout (python3 scripts/build-web.py --fast + serve-web.py).
No fixture fallback: a fake page cannot prove viewport acceptance.

Usage:
  python3 scripts/test-workbench-viewport.py --page-url http://127.0.0.1:4173/boardstudio/
"""
import argparse
import importlib
import json
from pathlib import Path

driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "workbench-viewport-320"
browser, evaluate, check = driver.browser, driver.evaluate, driver.check
ROOT = Path(__file__).resolve().parents[1]

ENTRIES = ["Select", "Transform", "Align", "Snap", "View", "Objects", "Inspect"]
SECTIONS = ["Select", "Transform", "Align", "Snap", "View"]


def viewport_state():
    return evaluate(
        "({w: innerWidth, h: innerHeight, "
        "vv: {w: visualViewport.width, h: visualViewport.height, "
        "l: visualViewport.offsetLeft, t: visualViewport.offsetTop, s: visualViewport.scale}, "
        "fonts: document.fonts.status})"
    )


def wait_for_settled():
    # Deterministic: fonts ready, no running animations, stable row rect across two frames.
    browser(
        "wait", "--fn",
        "document.fonts.status==='loaded' && "
        "document.getAnimations().filter(a=>a.playState==='running').length===0 && "
        "!!document.querySelector('.workbench-tool-row')",
    )
    before = evaluate(
        "JSON.stringify(document.querySelector('.workbench-tool-row').getBoundingClientRect().toJSON())"
    )
    browser("wait", "--fn", "new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))")
    after = evaluate(
        "JSON.stringify(document.querySelector('.workbench-tool-row').getBoundingClientRect().toJSON())"
    )
    assert before == after, f"row rect not settled: {before} vs {after}"


def entry_report(name):
    return evaluate(
        "(() => {"
        f"const btn=[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==={json.dumps(name)});"
        f"if(!btn) return {{name:{json.dumps(name)}, found:false}};"
        "const b=btn.getBoundingClientRect();"
        "const cx=b.x+b.width/2, cy=b.y+b.height/2;"
        "const inside=cx>=0&&cx<=innerWidth&&cy>=0&&cy<=innerHeight;"
        "const el=document.elementFromPoint(cx,cy);"
        "const hit=el===btn||(el&&btn.contains(el));"
        f"return {{name:{json.dumps(name)}, found:true,"
        " rect:{x:b.x,y:b.y,w:b.width,h:b.height}, center:{x:cx,y:cy}, inside, hit,"
        " el: el?el.tagName+':'+(el.getAttribute('aria-label')||el.className||'').toString().slice(0,120):null,"
        " occluder: (!hit&&el)?el.outerHTML.slice(0,400):null};"
        "})()"
    )


def panel_report(panel_id, close_label):
    return evaluate(
        "(() => {"
        f"const panel=document.querySelector({json.dumps('#'+panel_id+':not([hidden])')});"
        f"const closeBtn=[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==={json.dumps(close_label)});"
        "if(!panel) return {open:false};"
        "const p=panel.getBoundingClientRect();"
        "const c=closeBtn?closeBtn.getBoundingClientRect():null;"
        "let closeHit=null, closeEl=null, closeOccluder=null;"
        "if(closeBtn&&c){"
        " const cx=c.x+c.width/2, cy=c.y+c.height/2;"
        " const el=document.elementFromPoint(cx,cy);"
        " closeHit=el===closeBtn||(el&&closeBtn.contains(el));"
        " closeEl=el?el.tagName+':'+(el.getAttribute('aria-label')||''):null;"
        " closeOccluder=(!closeHit&&el)?el.outerHTML.slice(0,400):null;"
        "}"
        "return {open:true,"
        " panel:{x:p.x,y:p.y,w:p.width,h:p.height},"
        " close: c?{x:c.x,y:c.y,w:c.width,h:c.height}:null,"
        " closeHit, closeEl, closeOccluder};"
        "})()"
    )


def overflow_state():
    return evaluate(
        "({sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth,"
        " sh: document.documentElement.scrollHeight, ch: document.documentElement.clientHeight})"
    )


def spacing_report():
    # Independent measured spacing oracle: architecture requires adjacent icon hit
    # regions separated by at least 4px with no overlap. Measured from rendered
    # bounding rects in DOM order, sorted left-to-right for robustness.
    return evaluate(
        "(() => {"
        f"const names={json.dumps(SECTIONS)};"
        "const rects=names.map(n=>{"
        "const btn=[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')===n);"
        "if(!btn) return {name:n,found:false};"
        "const b=btn.getBoundingClientRect();"
        "return {name:n,found:true,x:b.x,y:b.y,w:b.width,h:b.height};"
        "});"
        "const ordered=[...rects].filter(r=>r.found).sort((a,b)=>a.x-b.x);"
        "const gaps=[];"
        "for(let i=1;i<ordered.length;i++){"
        "const prev=ordered[i-1],cur=ordered[i];"
        "gaps.push({pair:prev.name+'>'+cur.name,gap:cur.x-(prev.x+prev.w)});"
        "}"
        "return {rects,gaps};"
        "})()"
    )


def production_viewport_tests(url, width=320, height=480):
    browser("set", "viewport", width, height)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
    # Current packaged flow (verified 2026-11-11): Project -> New project -> Hide guide.
    # Do not assume legacy "4 Case"/"Open Layout workspace" steps; Layout is default.
    browser("find", "role", "button", "click", "--name", "Project", "--exact")
    browser("find", "role", "menuitem", "click", "--name", "New project", "--exact")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Keyboard setup\"]')")
    browser("find", "role", "button", "click", "--name", "Hide guide", "--exact")
    browser("wait", "--fn", "!!document.querySelector('.workbench-tool-row')")

    state = viewport_state()
    print(f"VIEWPORT {width}x{height}: {json.dumps(state)}", flush=True)
    assert state["w"] == width and state["h"] == height, (
        f"browser viewport must be exactly {width}x{height}, got {state['w']}x{state['h']}"
    )
    wait_for_settled()

    row = evaluate(
        "(() => {const rows=[...document.querySelectorAll('.workbench-tool-row')];"
        "const r=rows[0].getBoundingClientRect();"
        "return {count:rows.length,x:r.x,y:r.y,w:r.width,h:r.height}})()"
    )
    print(f"ROW: {json.dumps(row)}", flush=True)
    assert row["count"] == 1, f"expected one .workbench-tool-row, got {row['count']}"
    assert abs(row["h"] - 42.0) < 0.5, f"row must be 42px high, got {row['h']}"

    failures = []
    for name in ENTRIES:
        rep = entry_report(name)
        print(f"ENTRY {name}: {json.dumps(rep)}", flush=True)
        if not rep.get("found"):
            failures.append((name, "missing"))
            continue
        r = rep["rect"]
        if not (r["w"] > 0 and r["h"] > 0):
            failures.append((name, f"non-positive rect {r}"))
        if not rep["inside"]:
            failures.append((name, f"center {rep['center']} outside {width}x{height}"))
        if not rep["hit"]:
            failures.append((name, f"elementFromPoint hit {rep['el']}, occluder {rep['occluder']}"))

    spacing = spacing_report()
    print(f"SPACING: {json.dumps(spacing)}", flush=True)
    if len(spacing["rects"]) != 5 or any(not r.get("found") for r in spacing["rects"]):
        failures.append(("spacing", f"section triggers missing {spacing['rects']}"))
    else:
        for g in spacing["gaps"]:
            if g["gap"] + 1e-6 < 4.0:
                failures.append(("spacing", f"adjacent {g['pair']} separated by {g['gap']:.3f}px, require >=4px"))

    over = overflow_state()
    print(f"OVERFLOW: {json.dumps(over)}", flush=True)
    if not (over["sw"] <= over["cw"] and over["sh"] <= over["ch"]):
        failures.append(("page", f"page overflow {over}"))

    # Panel opens via real accessibility click; Close still needs true hit test.
    for utility, panel_id, close_label in [
        ("Objects", "workbench-objects", "Close Objects"),
        ("Inspect", "workbench-inspector", "Close Inspector"),
    ]:
        browser("find", "role", "button", "click", "--name", utility, "--exact")
        browser("wait", "--fn", f"!!document.querySelector('#{panel_id}:not([hidden])')")
        wait_for_settled()
        rep = panel_report(panel_id, close_label)
        print(f"PANEL {utility}: {json.dumps(rep)}", flush=True)
        if not rep.get("open"):
            failures.append((utility, "panel did not open"))
            continue
        p = rep["panel"]
        if not (p["x"] >= 0 and p["y"] >= 0 and p["x"] + p["w"] <= width + 0.5 and p["y"] + p["h"] <= height + 0.5):
            failures.append((utility, f"panel bounds {p} outside {width}x{height}"))
        if not rep["closeHit"]:
            failures.append((close_label, f"Close hit {rep['closeEl']}, occluder {rep['closeOccluder']}"))
        over = overflow_state()
        print(f"OVERFLOW {utility}: {json.dumps(over)}", flush=True)
        if not (over["sw"] <= over["cw"] and over["sh"] <= over["ch"]):
            failures.append((utility, f"page overflow with panel open {over}"))

    errors = browser("errors").get("errors", [])
    print(f"BROWSER ERRORS: {json.dumps(errors)}", flush=True)
    if errors:
        failures.append(("browser", f"errors {errors}"))

    print(f"FAILURES ({len(failures)}): {json.dumps(failures, indent=2)}", flush=True)
    assert not failures, failures
    print(f"PASS: real {width}x{height} viewport, 42px row, seven entry true hit tests, Objects/Inspect panels", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", required=True, help="packaged production page URL (no fixture mode)")
    parser.add_argument("--width", type=int, default=320)
    parser.add_argument("--height", type=int, default=480)
    args = parser.parse_args()
    try:
        production_viewport_tests(args.page_url, args.width, args.height)
    finally:
        browser("close")


if __name__ == "__main__":
    main()

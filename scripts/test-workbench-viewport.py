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
import time
from pathlib import Path

driver = importlib.import_module("test-dock-primitives")
driver.SESSION = "workbench-viewport-320"
browser, evaluate, check = driver.browser, driver.evaluate, driver.check
ROOT = Path(__file__).resolve().parents[1]

ENTRIES = ["Select", "Transform", "Align", "Snap", "View", "Objects", "Inspect"]
SECTIONS = ["Select", "Transform", "Align", "Snap", "View"]
SECTION_KEYS = {"Select": "select", "Transform": "transform", "Align": "align", "Snap": "snap", "View": "view"}
MIN_HIT = 32.0
MIN_GAP = 4.0
# Ticket 002 Step 3 density table: at row content-box W >= this, every marked action shows its label;
# below it only pressed/checked actions do. Provisional until calibration accepts or revises it.
FULL_LABEL_W = 850


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


def seven_spacing_report():
    # Open-state spacing oracle: all seven stationary entry hit regions sorted
    # left-to-right, adjacent gaps >=4px, no overlap. Extends spacing_report
    # (five sections) without weakening it.
    return evaluate(
        "(() => {"
        f"const names={json.dumps(ENTRIES)};"
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


def open_state_report():
    return evaluate(
        "(()=>[...document.querySelectorAll('[data-workbench-section]')].map(g=>({"
        "key:g.getAttribute('data-workbench-section'),"
        "open:g.getAttribute('data-open')"
        "})))()"
    )


def action_lane_report(key):
    # Truthful lane inspection: reveal/track geometry + contained controls.
    # Does not require actions where valid domain selection isn't present;
    # caller logs and asserts only lane room + overflow semantics.
    return evaluate(
        "(()=>{"
        f"const key={json.dumps(key)};"
        "const g=document.querySelector('[data-workbench-section=\"'+key+'\"]');"
        "if(!g) return {key,missing:true};"
        "const rev=g.querySelector('.workbench-tool-reveal');"
        "const track=g.querySelector('.workbench-tool-track');"
        "const gr=rev.getBoundingClientRect();"
        "const tr=track?track.getBoundingClientRect():null;"
        "const actions=track?[...track.querySelectorAll('button,p,select,input')].map(e=>{"
        "const b=e.getBoundingClientRect();"
        "return {tag:e.tagName,label:(e.getAttribute('aria-label')||e.textContent||'').slice(0,80),"
        "cls:(e.className||'').toString().slice(0,200),slot:e.getAttribute('data-slot'),isGuidance:e.classList.contains('layout-selection-guidance'),"
        "x:b.x,y:b.y,w:b.width,h:b.height,disabled:e.hasAttribute('disabled')};"
        "}):[];"
        "return {key,open:g.getAttribute('data-open'),"
        "actionLabels:g.getAttribute('data-action-labels'),"
        "ariaHidden:rev.getAttribute('aria-hidden'),"
        "rev:{x:gr.x,y:gr.y,w:gr.width,h:gr.height},"
        "track:tr?{x:tr.x,y:tr.y,w:tr.width,h:tr.height}:null,"
        "clientW:track?track.clientWidth:null,scrollW:track?track.scrollWidth:null,"
        "overflowX:track?getComputedStyle(track).overflowX:null,"
        "actions};"
        "})()"
    )


def usable_action_report(key):
    # Sound usable-action oracle: enabled, >=32x32, fully within every clipping ancestor+viewport, center topmost.
    # Tries programmatic scrollIntoView for each enabled size-ok candidate, then re-checks.
    # Programmatic scroll only; keyboard focus scroll and native touch scrolling remain unproved (do not claim).
    # Never counts effectively disabled/inert, clipped, or offscreen geometry. Restores scrollLeft.
    return evaluate(
        "(()=>{"
        f"const key={json.dumps(key)};"
        "const g=document.querySelector('[data-workbench-section=\"'+key+'\"]');"
        "if(!g) return {key,missing:true};"
        "const row=document.querySelector('.workbench-tool-row');"
        "const track=g.querySelector('.workbench-tool-track');"
        "if(!row||!track) return {key,missing:true};"
        "const rr=row.getBoundingClientRect();"
        "const getTrack=()=>track.getBoundingClientRect();"
        "const rowBox={x:rr.x,y:rr.y,w:rr.width,h:rr.height};"
        "const origScroll=track.scrollLeft;"
        "const cands=[...track.querySelectorAll('button:not(.layout-selection-guidance)')];"
        "const isEffDisabled=(e)=>{ if(e.disabled) return true; if(e.hasAttribute('disabled')) return true; if(e.closest('[disabled]')) return true; if(e.closest('fieldset[disabled]')) return true; if(e.getAttribute('aria-disabled')==='true') return true; return false; };"
        "const isEffInert=(e)=>{ if(e.hasAttribute('inert')) return true; if(e.closest('[inert]')) return true; if(e.closest('[hidden]')) return true; if(e.closest('[aria-hidden=true]')) return true; try{ const cs=getComputedStyle(e); if(cs.display==='none'||cs.visibility==='hidden'||cs.visibility==='collapse') return true; }catch(_){} return false; };"
        "const getClippers=(e)=>{ const out=[]; let p=e.parentElement; while(p){ try{ const cs=getComputedStyle(p); const ox=cs.overflowX, oy=cs.overflowY; if(ox==='hidden'||ox==='auto'||ox==='scroll'||ox==='clip'||oy==='hidden'||oy==='auto'||oy==='scroll'||oy==='clip'){ const r=p.getBoundingClientRect(); out.push({tag:p.tagName,cls:(p.className||'').toString().slice(0,60),x:r.x,y:r.y,w:r.width,h:r.height,ox,oy}); } }catch(_){} p=p.parentElement; } return out; };"
        "const withinAll=(b,clippers)=>{ for(const c of clippers){ if(!(b.x+1e-6>=c.x&&b.y+1e-6>=c.y&&b.x+b.width-1e-6<=c.x+c.w&&b.y+b.height-1e-6<=c.y+c.h)) return false; } return true; };"
        "const detail=(e)=>{"
        "const b=e.getBoundingClientRect();"
        "const cx=b.x+b.width/2, cy=b.y+b.height/2;"
        "const t=getTrack();"
        "const withinTrack=b.x+1e-6>=t.x&&b.y+1e-6>=t.y&&b.x+b.width-1e-6<=t.x+t.width&&b.y+b.height-1e-6<=t.y+t.height;"
        "const withinRow=b.x+1e-6>=rr.x&&b.y+1e-6>=rr.y&&b.x+b.width-1e-6<=rr.x+rr.width&&b.y+b.height-1e-6<=rr.y+rr.height;"
        "const inside=cx>=0&&cx<=innerWidth&&cy>=0&&cy<=innerHeight;"
        "const withinViewport=b.x+1e-6>=0&&b.y+1e-6>=0&&b.x+b.width-1e-6<=innerWidth&&b.y+b.height-1e-6<=innerHeight;"
        "const effDis=isEffDisabled(e);"
        "const effInert=isEffInert(e);"
        "const clippers=getClippers(e);"
        "const withinAllClip=withinAll(b,clippers);"
        "const el=document.elementFromPoint(cx,cy);"
        "const hit=el===e||(el&&e.contains(el));"
        "return {label:(e.getAttribute('aria-label')||e.textContent||'').slice(0,80),x:b.x,y:b.y,w:b.width,h:b.height,disabled:e.hasAttribute('disabled'),effDis,effInert,cx,cy,withinTrack,withinRow,inside,withinViewport,withinAllClip,clipCount:clippers.length,hit,el:el?el.tagName+':'+(el.getAttribute('aria-label')||el.className||'').toString().slice(0,80):null};"
        "};"
        "const snap=cands.map(detail);"
        "const isUsable=(r)=>!r.effDis&&!r.effInert&&r.w+1e-6>=32&&r.h+1e-6>=32&&r.withinTrack&&r.withinRow&&r.withinViewport&&r.withinAllClip&&r.inside&&r.hit;"
        "let usable=snap.find(isUsable)||null;"
        "let scrolled=null;"
        "if(!usable){"
        "for(const e of cands){"
        "if(isEffDisabled(e)||isEffInert(e)) continue;"
        "const b0=e.getBoundingClientRect();"
        "if(b0.width+1e-6<32||b0.height+1e-6<32) continue;"
        "if(b0.width<0.5&&b0.height<0.5) continue;"
        "try{e.scrollIntoView({block:'nearest',inline:'nearest',behavior:'auto'});}catch(_){e.scrollIntoView();}"
        "const r=detail(e);"
        "if(isUsable(r)){usable=r;scrolled=r.label;break;}"
        "}"
        "}"
        "const ft=getTrack();"
        "const out={key,row:rowBox,track:{x:ft.x,y:ft.y,w:ft.width,h:ft.height,clientW:track.clientWidth,scrollW:track.scrollWidth,scrollLeft:track.scrollLeft,origScroll},candidates:snap,usable,scrolled};"
        "try{track.scrollLeft=origScroll;}catch(_){}"
        "return out;"
        "})()"
    )


def guidance_report(key):
    # Truthful guidance oracle: P box within row/track AND text fully readable (no truncation)
    # OR accessible via discoverable keyboard-accessible tooltip/compact surface with proof.
    # Do not label truncated ellipsis as readable. Returns truncation + alternative details.
    return evaluate(
        "(()=>{"
        f"const key={json.dumps(key)};"
        "const g=document.querySelector('[data-workbench-section=\"'+key+'\"]');"
        "if(!g) return {key,missing:true};"
        "const row=document.querySelector('.workbench-tool-row');"
        "const track=g.querySelector('.workbench-tool-track');"
        "const rr=row.getBoundingClientRect();"
        "const tr=track.getBoundingClientRect();"
        "const p=track.querySelector('.layout-selection-guidance');"
        "if(!p) return {key,hasGuidance:false,row:{x:rr.x,y:rr.y,w:rr.width,h:rr.height},track:{x:tr.x,y:tr.y,w:tr.width,h:tr.height}};"
        "const b=p.getBoundingClientRect();"
        "const withinRow=b.x+1e-6>=rr.x&&b.y+1e-6>=rr.y&&b.x+b.width-1e-6<=rr.x+rr.width&&b.y+b.height-1e-6<=rr.y+rr.height;"
        "const withinTrack=b.x+1e-6>=tr.x&&b.y+1e-6>=tr.y&&b.x+b.width-1e-6<=tr.x+tr.width&&b.y+b.height-1e-6<=tr.y+tr.height;"
        "const inner=p.querySelector('.layout-selection-guidance-text');"
        "const sw=inner?inner.scrollWidth:p.scrollWidth, cw=inner?inner.clientWidth:p.clientWidth, truncated=sw>cw+1;"
        "const cs=getComputedStyle(p);"
        "const tipBtn=track.querySelector('[data-transform-guidance-tip],button[aria-label*=\"Select a\"]');"
        "const tooltip=g.querySelector('[data-slot=tooltip]');"
        "const popover=g.querySelector('[data-slot=popover]');"
        "let tip=null;"
        "if(tipBtn){ const tb=tipBtn.getBoundingClientRect(); tip={tag:tipBtn.tagName,label:(tipBtn.getAttribute('aria-label')||'').slice(0,120),x:tb.x,y:tb.y,w:tb.width,h:tb.height}; }"
        "return {key,hasGuidance:true,tag:p.tagName,slot:p.getAttribute('data-slot'),text:(p.textContent||'').slice(0,120),rect:{x:b.x,y:b.y,w:b.width,h:b.height},withinRow,withinTrack,scrollW:sw,clientW:cw,truncated,overflow:cs.overflowX+','+cs.overflowY,whiteSpace:cs.whiteSpace,textOverflow:cs.textOverflow,tabindex:p.getAttribute('tabindex'),title:(p.getAttribute('title')||'').slice(0,120),hasTooltip:!!tooltip,hasPopover:!!popover,tip,row:{x:rr.x,y:rr.y,w:rr.width,h:rr.height},track:{x:tr.x,y:tr.y,w:tr.width,h:tr.height}};"
        "})()"
    )


def view_icon_report():
    # View icon hit regions must be >=32x32 where applicable; excludes hidden 0x0.
    return evaluate(
        "(()=>{"
        "const g=document.querySelector('[data-workbench-section=\"view\"]');"
        "if(!g) return {missing:true};"
        "const track=g.querySelector('.workbench-tool-track');"
        "const btns=[...track.querySelectorAll('button')].map(e=>{"
        "const b=e.getBoundingClientRect();"
        "return {label:(e.getAttribute('aria-label')||e.textContent||'').slice(0,80),x:b.x,y:b.y,w:b.width,h:b.height,disabled:e.hasAttribute('disabled')};"
        "});"
        "const visible=btns.filter(r=>r.w>0.5&&r.h>0.5);"
        "const small=visible.filter(r=>r.w+1e-6<32||r.h+1e-6<32);"
        "return {all:btns,visible,small};"
        "})()"
    )


def canvas_visibility_report():
    # Canvas-specific usable oracle: #m1-layers-trigger must be whole >=32x32, enabled,
    # not inert, fully within every clipping ancestor+viewport, center topmost, via lane scroll.
    # Does not assume 96px offscreen box fits; proves scroll can expose whole control.
    # Programmatic scroll only; keyboard/touch scrolling unproved.
    return evaluate(
        "(()=>{"
        "const g=document.querySelector('[data-workbench-section=\"view\"]');"
        "if(!g) return {missing:true};"
        "const row=document.querySelector('.workbench-tool-row');"
        "const track=g.querySelector('.workbench-tool-track');"
        "const btn=document.querySelector('#m1-layers-trigger');"
        "if(!row||!track||!btn) return {missing:true};"
        "const rr=row.getBoundingClientRect();"
        "const getTrack=()=>track.getBoundingClientRect();"
        "const isEffDisabled=(e)=>{ if(e.disabled) return true; if(e.hasAttribute('disabled')) return true; if(e.closest('[disabled]')) return true; return false; };"
        "const isEffInert=(e)=>{ if(e.hasAttribute('inert')) return true; if(e.closest('[inert]')) return true; if(e.closest('[hidden]')) return true; return false; };"
        "const getClippers=(e)=>{ const out=[]; let p=e.parentElement; while(p){ try{ const cs=getComputedStyle(p); const ox=cs.overflowX, oy=cs.overflowY; if(ox==='hidden'||ox==='auto'||ox==='scroll'||ox==='clip'||oy==='hidden'||oy==='auto'||oy==='scroll'||oy==='clip'){ const r=p.getBoundingClientRect(); out.push({tag:p.tagName,cls:(p.className||'').toString().slice(0,60),x:r.x,y:r.y,w:r.width,h:r.height}); } }catch(_){} p=p.parentElement; } return out; };"
        "const withinAll=(b,clippers)=>{ for(const c of clippers){ if(!(b.x+1e-6>=c.x&&b.y+1e-6>=c.y&&b.x+b.width-1e-6<=c.x+c.w&&b.y+b.height-1e-6<=c.y+c.h)) return false; } return true; };"
        "const detail=()=>{"
        "const b=btn.getBoundingClientRect();"
        "const cx=b.x+b.width/2, cy=b.y+b.height/2;"
        "const t=getTrack();"
        "const withinTrack=b.x+1e-6>=t.x&&b.y+1e-6>=t.y&&b.x+b.width-1e-6<=t.x+t.width&&b.y+b.height-1e-6<=t.y+t.height;"
        "const withinRow=b.x+1e-6>=rr.x&&b.y+1e-6>=rr.y&&b.x+b.width-1e-6<=rr.x+rr.width&&b.y+b.height-1e-6<=rr.y+rr.height;"
        "const withinViewport=b.x+1e-6>=0&&b.y+1e-6>=0&&b.x+b.width-1e-6<=innerWidth&&b.y+b.height-1e-6<=innerHeight;"
        "const inside=cx>=0&&cx<=innerWidth&&cy>=0&&cy<=innerHeight;"
        "const clippers=getClippers(btn);"
        "const withinAllClip=withinAll(b,clippers);"
        "const el=document.elementFromPoint(cx,cy);"
        "const hit=el===btn||(el&&btn.contains(el));"
        "return {label:(btn.getAttribute('aria-label')||'').slice(0,80),x:b.x,y:b.y,w:b.width,h:b.height,effDis:isEffDisabled(btn),effInert:isEffInert(btn),cx,cy,withinTrack,withinRow,withinViewport,withinAllClip,clipCount:clippers.length,inside,hit,el:el?el.tagName+':'+(el.getAttribute('aria-label')||'').slice(0,60):null,track:{x:t.x,y:t.y,w:t.width,h:t.height}};"
        "};"
        "const before=detail();"
        "const isUsable=(r)=>!r.effDis&&!r.effInert&&r.w+1e-6>=32&&r.h+1e-6>=32&&r.withinTrack&&r.withinRow&&r.withinViewport&&r.withinAllClip&&r.inside&&r.hit;"
        "let after=null, scrolled=false, strategy=null;"
        "const origScroll=track.scrollLeft;"
        "if(!isUsable(before)){"
        "try{btn.scrollIntoView({block:'nearest',inline:'nearest',behavior:'auto'});}catch(_){try{btn.scrollIntoView();}catch(_){}}"
        "after=detail();"
        "scrolled=true; strategy='nearest';"
        "if(!isUsable(after)){ try{ btn.scrollIntoView({block:'nearest',inline:'start',behavior:'auto'}); after=detail(); strategy='start'; }catch(_){} }"
        "if(after&&!isUsable(after)){ try{ btn.scrollIntoView({block:'nearest',inline:'center',behavior:'auto'}); after=detail(); strategy='center'; }catch(_){} }"
        "if(after&&!isUsable(after)){ try{ btn.scrollIntoView({block:'nearest',inline:'end',behavior:'auto'}); after=detail(); strategy='end'; }catch(_){} }"
        "}"
        "const usable=isUsable(before)?before:(after&&isUsable(after)?after:null);"
        # Oversized-control reach (ticket 002 item 6): the first lane scroll that leaves the
        # enabled control in the row and hit at its centre, never a whole-control claim.
        "const reaches=(r)=>!r.effDis&&!r.effInert&&r.h+1e-6>=32&&r.withinRow&&r.inside&&r.hit;"
        "let reach=reaches(before)?before:null;"
        "for(const inline of ['nearest','start','center','end']){ if(reach) break; try{ btn.scrollIntoView({block:'nearest',inline,behavior:'auto'}); const d=detail(); if(reaches(d)) reach=Object.assign(d,{strategy:inline}); }catch(_){} }"
        "try{track.scrollLeft=origScroll;}catch(_){}"
        "return {before,after,scrolled,strategy,usable:!!usable,usableDetail:usable,reach};"
        "})()"
    )


def layers_list_report():
    # Layers list placement oracle (ticket 002 390 right-edge defect): with View
    # open, the trigger scrolled into the lane and the list open, the fixed list
    # must sit fully inside the viewport with >=8px side margins, below the
    # trigger, its first choice hit-testable at centre, and no page overflow.
    # Caller opens/closes the list; this only measures. Programmatic trigger
    # click; trigger pointer reach is covered by CANVAS where the trigger fits.
    return evaluate(
        "(()=>{" +
        "const trig=document.querySelector('#m1-layers-trigger');" +
        "const list=document.querySelector('#m1-layers-list');" +
        "if(!trig||!list) return {missing:true};" +
        "const open=trig.getAttribute('aria-expanded')==='true';" +
        "const b=list.getBoundingClientRect();" +
        "const tb=trig.getBoundingClientRect();" +
        "const first=list.querySelector('button');" +
        "let choice=null;" +
        "if(first){"+
        "const fb=first.getBoundingClientRect();" +
        "const cx=fb.x+fb.width/2, cy=fb.y+fb.height/2;" +
        "const el=(cx>=0&&cx<=innerWidth&&cy>=0&&cy<=innerHeight)?document.elementFromPoint(cx,cy):null;" +
        "choice={label:(first.getAttribute('aria-label')||'').slice(0,40)," +
        "x:fb.x,y:fb.y,w:fb.width,h:fb.height,cx,cy," +
        "hit:el===first||(el&&first.contains(el))," +
        "el:el?el.tagName+':'+(el.getAttribute('aria-label')||'').slice(0,40):null};" +
        "}" +
        "return {open,vw:innerWidth," +
        "list:{x:b.x,y:b.y,w:b.width,h:b.height,right:b.x+b.width}," +
        "trigger:{x:tb.x,y:tb.y,w:tb.width,h:tb.height,bottom:tb.y+tb.height}," +
        "first:choice," +
        "overflow:{sw:document.documentElement.scrollWidth,cw:document.documentElement.clientWidth}};" +
        "})()"
    )


def check_layers_geometry(tag, failures):
    # Layers list placement asserts shared by every Layers observation: fully
    # inside the viewport with >=8px side margins, below the trigger, first
    # choice hit-testable at centre, no page horizontal overflow, and
    # horizontally anchored to the trigger (clamped to 8px side margins).
    layers = layers_list_report()
    print(f"LAYERS {tag}: {json.dumps(layers)}", flush=True)
    if layers.get("missing") or not layers.get("open"):
        failures.append((f"layers-{tag}", f"Layers list not open, full {layers}"))
        return
    lay, ltrig = layers["list"], layers["trigger"]
    if lay["x"] + 1e-6 < 8.0:
        failures.append((f"layers-{tag}", f"list left edge {lay['x']:.1f}px, require >=8px margin, full {layers}"))
    if lay["right"] - 1e-6 > layers["vw"] - 8.0:
        failures.append((f"layers-{tag}", f"list right edge {lay['right']:.1f}px in {layers['vw']}px viewport, require >=8px margin, full {layers}"))
    if lay["y"] + 1e-6 < ltrig["bottom"]:
        failures.append((f"layers-{tag}", f"list top {lay['y']:.1f}px above trigger bottom {ltrig['bottom']:.1f}px, full {layers}"))
    max_left = max(layers["vw"] - lay["w"] - 8.0, 8.0)
    expected = min(max(ltrig["x"], 8.0), max_left)
    if abs(lay["x"] - expected) > 2.0:
        failures.append((f"layers-{tag}", f"list x {lay['x']:.1f}px not anchored to trigger x {ltrig['x']:.1f}px (clamped {expected:.1f}px), full {layers}"))
    first = layers.get("first") or {}
    if not first.get("hit"):
        failures.append((f"layers-{tag}", f"first choice {first.get('label')} not hit-testable at centre ({first.get('cx')},{first.get('cy')}), hit {first.get('el')}, full {layers}"))
    over = layers["overflow"]
    if over["sw"] > over["cw"]:
        failures.append((f"layers-{tag}", f"page horizontal overflow with list open {over}"))


def reference_detail_report():
    # Alignment Reference detail geometry: dialog rect, trigger rect, Close hit,
    # nested state. The trigger rect anchors the clamped-placement assertion.
    return evaluate(
        "(()=>{" +
        "const m=document.querySelector('#m1-layout-align-menu');" +
        "if(!m) return {present:false};" +
        "const b=m.getBoundingClientRect();" +
        "const rt=document.querySelector('#m1-layout-align-trigger');" +
        "const rtb=rt?rt.getBoundingClientRect():null;" +
        "const c=[...m.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Close Alignment reference');" +
        "const cb=c.getBoundingClientRect();" +
        "const cx=cb.x+cb.width/2, cy=cb.y+cb.height/2;" +
        "const el=(cx>=0&&cx<=innerWidth&&cy>=0&&cy<=innerHeight)?document.elementFromPoint(cx,cy):null;" +
        "const n=[...m.querySelectorAll('button,input')].find(e=>e.getAttribute('aria-label')==='Alignment reference');" +
        "let nested=null;" +
        "if(n){const nb=n.getBoundingClientRect();" +
        "const nx=nb.x+nb.width/2, ny=nb.y+nb.height/2;" +
        "const nel=(nx>=0&&nx<=innerWidth&&ny>=0&&ny<=innerHeight)?document.elementFromPoint(nx,ny):null;" +
        "nested={form:n.tagName,disabled:n.hasAttribute('disabled'),x:nb.x,y:nb.y,w:nb.width,h:nb.height,hit:nel===n||n.contains(nel)};}" +
        "return {present:true,state:m.getAttribute('data-state')," +
        " menu:{x:b.x,y:b.y,w:b.width,h:b.height,right:b.x+b.width,bottom:b.y+b.height}," +
        " trigger:rtb?{x:rtb.x,y:rtb.y,w:rtb.width,h:rtb.height,bottom:rtb.y+rtb.height}:null," +
        " close:{x:cb.x,y:cb.y,w:cb.width,h:cb.height,hit:el===c||c.contains(el)}," +
        " nested,focus:(document.activeElement&&document.activeElement.getAttribute('aria-label')||'').slice(0,60)," +
        " vw:innerWidth,vh:innerHeight};" +
        "})()"
    )


def tooltip_report(group_key):
    # Open tooltip measurement inside one section group: bounds, text, viewport, hit.
    return evaluate(
        "(()=>{" +
        f"const g=document.querySelector('[data-workbench-section=\"{group_key}\"]');" +
        "const tip=g?g.querySelector('[data-slot=tooltip][data-state=open] [data-slot=tooltip-content]'):null;" +
        "if(!tip) return {open:false};" +
        "const b=tip.getBoundingClientRect();" +
        "const cx=b.x+b.width/2, cy=b.y+b.height/2;" +
        "const el=(cx>=0&&cx<=innerWidth&&cy>=0&&cy<=innerHeight)?document.elementFromPoint(cx,cy):null;" +
        "return {open:true,x:b.x,y:b.y,w:b.width,h:b.height," +
        " text:(tip.textContent||'').trim().slice(0,120)," +
        " withinViewport:b.x+1e-6>=0&&b.y+1e-6>=0&&b.x+b.width-1e-6<=innerWidth&&b.y+b.height-1e-6<=innerHeight," +
        " hit:el===tip||(el&&tip.contains(el))};" +
        "})()"
    )


def active_focus_state():
    # Focus visibility for O's no-invisible-focus checks: connected, nonzero,
    # not display/visibility-hidden, not inert (self or ancestor), inside the
    # viewport and topmost at its centre (catches lane/viewport clipping).
    # Body keeps the neutral-visible fallback; callers check its aria separately.
    return evaluate(
        "(()=>{const a=document.activeElement;if(!a) return {tag:null};"
        "const b=a.getBoundingClientRect();"
        "let p=a,hidden=false,inert=false;"
        "while(p){const cs=getComputedStyle(p);"
        "if(cs.display==='none'||cs.visibility==='hidden'||cs.visibility==='collapse')hidden=true;"
        "if(p.hasAttribute&&p.hasAttribute('inert'))inert=true;"
        "try{if(p.inert===true)inert=true;}catch(_){}p=p.parentElement;}"
        "const cx=b.x+b.width/2,cy=b.y+b.height/2;"
        "const inViewport=cx>=0&&cx<=innerWidth&&cy>=0&&cy<=innerHeight;"
        "const el=inViewport?document.elementFromPoint(cx,cy):null;"
        "const hit=el===a||(el&&a.contains(el));"
        "const elDesc=el?el.tagName+':'+((el.getAttribute&&(el.getAttribute('aria-label')||el.getAttribute('role')||el.id||el.className))||'').toString().slice(0,80):null;"
        "const visible=(b.width>0&&b.height>0&&a.isConnected&&!hidden&&!inert&&inViewport&&hit)||a===document.body;"
        "return {tag:a.tagName,aria:(a.getAttribute('aria-label')||'').slice(0,60),w:b.width,h:b.height,el:elDesc,"
        "connected:a.isConnected,hidden:!!hidden,inert:!!inert,inViewport:!!inViewport,hit:!!hit,visible:!!visible};})()"
    )


def check_layers_open(tag, failures):
    # Shared Layers list placement proof (page flow + layers scenario): scroll
    # the trigger into the lane, open with a programmatic click, measure settled
    # (tooltips closed so hit testing sees the list), then close/restore scroll.
    # Programmatic trigger click; trigger pointer reach is covered by CANVAS.
    layers_scroll = evaluate(
        "(()=>{const t=document.querySelector('#m1-layers-trigger');" +
        "const track=document.querySelector('[data-workbench-section=\"view\"] .workbench-tool-track');" +
        "const orig=track.scrollLeft;" +
        "t.scrollIntoView({block:'nearest',inline:'nearest',behavior:'auto'});" +
        "return orig;})()"
    )
    evaluate("document.querySelector('#m1-layers-trigger').click()")
    browser("wait", "--fn", "document.querySelector(\"#m1-layers-trigger\").getAttribute(\"aria-expanded\")===\"true\"")
    wait_for_settled()
    evaluate("(()=>{ document.querySelectorAll('[data-slot=tooltip-trigger], [data-slot=tooltip] > button, .layout-selection-guidance').forEach(t=>{ try{ if(document.activeElement===t) t.blur(); t.dispatchEvent(new MouseEvent('mouseleave',{bubbles:true})); }catch(_){} }); })()")
    browser("wait", "--fn", "document.querySelectorAll('[data-slot=tooltip][data-state=open]').length===0")
    check_layers_geometry(tag, failures)
    evaluate("document.querySelector('#m1-layers-trigger').click()")
    browser("wait", "--fn", "document.querySelector(\"#m1-layers-trigger\").getAttribute(\"aria-expanded\")===\"false\"")
    evaluate(f"(()=>{{const track=document.querySelector('[data-workbench-section=\"view\"] .workbench-tool-track');track.scrollLeft={layers_scroll};}})()")


def check_placement_raf_race(tag, failures):
    # F1: open Layers, close before the hook's pending RAF fires, let frames
    # run, assert no page error, no stale positioning, cancel called, reopen
    # works. The close is driven by a style-mutation observer in the same
    # microtask checkpoint as the hook's effect (which writes inline left/top
    # then schedules its RAF synchronously): observer callbacks run before any
    # RAF, so the close provably precedes the next frame. On the unfixed hook
    # the pending RAF calls a dropped wasm-bindgen closure and throws (RED);
    # the fix cancels the pending ID before dropping (GREEN).
    # Assumes View open, Layers closed, settled.
    setup = evaluate(
        "(()=>{"
        "if(window.__raceCleanup){try{window.__raceCleanup();}catch(_){}}"
        "const trigger=document.querySelector('#m1-layers-trigger');"
        "const popup=document.querySelector('#m1-layers-list');"
        "if(!trigger||!popup) return {setup:false};"
        "const origRAF=window.requestAnimationFrame.bind(window);"
        "const origCancel=window.cancelAnimationFrame.bind(window);"
        "window.__raceScheduled=[];window.__raceFired=[];window.__raceCancelled=[];"
        "window.__raceErrors=[];window.__raceClosed=false;window.__raceAtClose=null;"
        "const onError=(e)=>{try{window.__raceErrors.push(String((e&&(e.error||e.message))||e));}catch(_){}};"
        "const onRejection=(e)=>{try{window.__raceErrors.push(String((e&&e.reason)||e));}catch(_){}};"
        "window.addEventListener('error',onError);"
        "window.addEventListener('unhandledrejection',onRejection);"
        "window.requestAnimationFrame=(cb)=>{let id;const wrapped=(...a)=>{window.__raceFired.push(id);return cb(...a);};"
        "id=origRAF(wrapped);window.__raceScheduled.push(id);return id;};"
        "window.cancelAnimationFrame=(id)=>{window.__raceCancelled.push(id);return origCancel(id);};"
        "window.__raceCleanup=()=>{window.requestAnimationFrame=origRAF;window.cancelAnimationFrame=origCancel;"
        "window.removeEventListener('error',onError);window.removeEventListener('unhandledrejection',onRejection);};"
        "const observer=new MutationObserver((muts)=>{"
        "for(const m of muts){if(m.attributeName==='style'&&popup.style.left){"
        "observer.disconnect();"
        "window.__raceAtClose={scheduled:[...window.__raceScheduled],fired:[...window.__raceFired]};"
        "trigger.click();window.__raceClosed=true;break;}}});"
        "observer.observe(popup,{attributes:true,attributeFilter:['style']});"
        "trigger.click();return {setup:true};"
        "})()"
    )
    print(f"RACE {tag} setup: {json.dumps(setup)}", flush=True)
    if not setup.get('setup'):
        failures.append((tag, f"race setup failed (missing trigger/list): {setup}"))
        return
    try:
        browser("wait", "--fn", "window.__raceClosed===true")
    except AssertionError:
        failures.append((tag, "race close never fired (hook effect did not write style before next frame)"))
        evaluate("(()=>{if(window.__raceCleanup)window.__raceCleanup();})()")
        return
    browser("wait", "--fn", "new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))")
    browser("wait", 300)
    result = evaluate(
        "(()=>{const t=document.querySelector('#m1-layers-trigger');"
        "const l=document.querySelector('#m1-layers-list');const b=l.getBoundingClientRect();"
        "const r={closed:window.__raceClosed,atClose:window.__raceAtClose,"
        "scheduled:[...window.__raceScheduled],fired:[...window.__raceFired],"
        "cancelled:[...window.__raceCancelled],errors:[...window.__raceErrors],"
        "expanded:t.getAttribute('aria-expanded'),"
        "listVisible:b.width>0.5||b.height>0.5,"
        "inlineLeft:l.style.left,inlineTop:l.style.top};"
        "if(window.__raceCleanup)window.__raceCleanup();return r;})()"
    )
    print(f"RACE {tag}: {json.dumps(result)}", flush=True)
    at_close = result.get('atClose') or {}
    if not at_close.get('scheduled'):
        failures.append((tag, f"no RAF scheduled before the race close (effect did not run): {result}"))
    if at_close.get('fired'):
        failures.append((tag, f"race close ran after a frame fired (not before next RAF): {result}"))
    if not result.get('cancelled'):
        failures.append((tag, f"pending placement RAF never cancelled before dropping its closure: {result}"))
    if result.get('errors'):
        failures.append((tag, f"page error from the pending placement RAF after close: {result}"))
    if result.get('expanded') != 'false' or result.get('listVisible'):
        failures.append((tag, f"Layers list not closed after the race: {result}"))
    if result.get('inlineLeft') or result.get('inlineTop'):
        failures.append((tag, f"stale inline positioning after the race close: {result}"))
    errors = browser("errors").get("errors", [])
    print(f"RACE {tag} browser errors: {json.dumps(errors)}", flush=True)
    if errors:
        failures.append((tag, f"browser errors after the race: {errors}"))
    # Reopen works with placed, hit-testable geometry.
    check_layers_open(f"{tag}-reopen", failures)


def select_label_report():
    # Select label oracle: expected label presence comes from ticket 002's density table (row content-box W
    # and pressed/checked state), never from a density attribute the product writes.
    # Expected labels: exact text, rendered bounds contained in the button, no icon overlap.
    # Unexpected labels must not paint; every button keeps a contained icon and a 32x32 target.
    return evaluate(
        "(()=>{"
        "const g=document.querySelector('[data-workbench-section=\"select\"]');"
        "if(!g) return {missing:true};"
        f"const FULL={FULL_LABEL_W};"
        "const row=document.querySelector('.workbench-tool-row'); const rcs=getComputedStyle(row);"
        "const W=row.clientWidth-parseFloat(rcs.paddingLeft)-parseFloat(rcs.paddingRight);"
        "const btns=[...g.querySelectorAll('button[aria-label^=\"Select \"]')];"
        "const expected=['Matrix','Row','Column','Key','Part'];"
        "const items=btns.map((b,idx)=>{"
        "const br=b.getBoundingClientRect();"
        "const svg=b.querySelector('svg');"
        "const lab=b.querySelector('.workbench-action-label');"
        "const sr=b.getBoundingClientRect?null:null;"
        "let icon=null, label=null;"
        "if(svg){ const r=svg.getBoundingClientRect(); const within= r.x+1e-6>=br.x&&r.y+1e-6>=br.y&&r.x+r.width-1e-6<=br.x+br.width&&r.y+r.height-1e-6<=br.y+br.height; icon={x:r.x,y:r.y,w:r.width,h:r.height,within}; }"
        "if(lab){ const r=lab.getBoundingClientRect(); const disp=getComputedStyle(lab).display; const txt=(lab.textContent||'').trim(); const within= disp==='none'?null:(r.x+1e-6>=br.x&&r.y+1e-6>=br.y&&r.x+r.width-1e-6<=br.x+br.width&&r.y+r.height-1e-6<=br.y+br.height); label={x:r.x,y:r.y,w:r.width,h:r.height,display:disp,text:txt.slice(0,40),within}; }"
        "let overlap=null;"
        "if(icon&&label&&label.display!=='none'){ overlap=!(icon.x+icon.w<=label.x+1e-6||label.x+label.w<=icon.x+1e-6||icon.y+icon.h<=label.y+1e-6||label.y+label.h<=icon.y+1e-6); }"
        "return {aria:(b.getAttribute('aria-label')||'').slice(0,40),x:br.x,y:br.y,w:br.width,h:br.height,expectLabel:W>=FULL||b.getAttribute('aria-pressed')==='true'||b.getAttribute('aria-checked')==='true',icon,label,overlap};"
        "});"
        "let labelsFit=true, reasons=[];"
        "if(btns.length!==5){ labelsFit=false; reasons.push('expected 5 Select buttons, got '+btns.length); }"
        "for(const exp of expected){"
        "const found=items.find(it=>it.aria==='Select '+exp);"
        "if(!found){ labelsFit=false; reasons.push('missing button Select '+exp); }"
        "}"
        "for(const it of items){"
        "if(it.w+1e-6<32||it.h+1e-6<32){ labelsFit=false; reasons.push(it.aria+' size '+it.w+'x'+it.h); continue; }"
        "if(!it.icon){ labelsFit=false; reasons.push(it.aria+' missing icon'); continue; }"
        "if(!it.icon.within){ labelsFit=false; reasons.push(it.aria+' icon overflows button'); }"
        "const expName=it.aria.startsWith('Select ')?it.aria.slice(7):null;"
        "if(expName&&!expected.includes(expName)){ labelsFit=false; reasons.push(it.aria+' unexpected, want one of '+expected.join(',')); }"
        "if(it.expectLabel){"
        "if(!it.label){ labelsFit=false; reasons.push(it.aria+' missing expected label element'); }"
        "else{"
        "if(it.label.display==='none'){ labelsFit=false; reasons.push(it.aria+' expected label hidden'); }"
        "if(!(it.label.w>0&&it.label.h>0)){ labelsFit=false; reasons.push(it.aria+' label not visible '+it.label.w+'x'+it.label.h); }"
        "if(it.label.text!==expName){ labelsFit=false; reasons.push(it.aria+' label text '+it.label.text+' != '+expName); }"
        "if(!it.label.within){ labelsFit=false; reasons.push(it.aria+' label overflows '+it.label.w+'px in '+it.w+'px button'); }"
        "if(it.overlap){ labelsFit=false; reasons.push(it.aria+' icon/label overlap'); }"
        "}"
        "}else{"
        "if(it.label&&it.label.display!=='none'&&it.label.w>0){ labelsFit=false; reasons.push(it.aria+' label '+it.label.text+' shown below the full-label band'); }"
        "}"
        "}"
        "const texts=items.map(i=>i.label?i.label.text:null);"
        "return {W,full:FULL,items,labelsFit,reasons,expected,texts};"
        "})()"
    )


def text_only_action_report(key):
    # A marked action with no icon of its own shows only its text, so hiding the label leaves a blank button.
    return evaluate(
        "(()=>{"
        f"const key={json.dumps(key)};"
        "const g=document.querySelector('[data-workbench-section=\"'+key+'\"]');"
        "if(!g) return {key,missing:true};"
        "const items=[...g.querySelectorAll('.workbench-tool-track [data-workbench-action]')].filter(b=>!b.querySelector(':scope > svg')).map(b=>{"
        "const l=b.querySelector(':scope > .workbench-action-label');"
        "const cs=l?getComputedStyle(l):null; const r=l?l.getBoundingClientRect():null;"
        "const text=l?(l.textContent||'').trim():'';"
        "const shown=!!l&&text.length>0&&cs.display!=='none'&&cs.visibility==='visible'&&Number(cs.opacity)>0&&r.width>0&&r.height>0;"
        "return {aria:b.getAttribute('aria-label'),pressed:b.getAttribute('aria-pressed'),text,display:cs?cs.display:null,w:r?r.width:0,h:r?r.height:0,shown};"
        "});"
        "return {key,items,blank:items.filter(i=>!i.shown).map(i=>i.aria)};"
        "})()"
    )


def transform_tooltip_report():
    # Transform guidance tooltip oracle: when inline truncated, full text must be visible
    # via existing Tooltip on keyboard focus (proven separately via real Tab/arrow keys, NOT via imperative focus here)
    # AND pointer hover, within viewport, hit-testable.
    # Caller must first focus guidance via real keyboard traversal; this checks current focus without focusing.
    # Returns focus/hover proof with content rect, text match, viewport containment, hit.
    return evaluate(
        "(async ()=>{"
        "const g=document.querySelector('[data-workbench-section=\"transform\"]');"
        "if(!g) return {missing:true};"
        "const p=g.querySelector('.layout-selection-guidance');"
        "if(!p) return {missing:true};"
        "const full=(p.getAttribute('aria-label')||p.textContent||'').trim();"
        "const checkTip=()=>{"
        "const tip=g.querySelector('[data-slot=tooltip][data-state=open] [data-slot=tooltip-content]');"
        "if(!tip) return {open:false};"
        "const b=tip.getBoundingClientRect();"
        "const txt=(tip.textContent||'').trim();"
        "const withinViewport=b.x+1e-6>=0&&b.y+1e-6>=0&&b.x+b.width-1e-6<=innerWidth&&b.y+b.height-1e-6<=innerHeight;"
        "const cx=b.x+b.width/2, cy=b.y+b.height/2;"
        "const el=document.elementFromPoint(cx,cy);"
        "const hit=el===tip||(el&&tip.contains(el));"
        "return {open:true,rect:{x:b.x,y:b.y,w:b.width,h:b.height},text:txt.slice(0,120),matches:txt===full,withinViewport,hit,el:el?el.tagName:null};"
        "};"
        "const focused=document.activeElement===p;"
        "const focusTip=checkTip();"
        "if(p.blur) p.blur();"
        "await new Promise(r=>setTimeout(r,200));"
        "try{ p.dispatchEvent(new MouseEvent('mouseenter',{bubbles:true})); p.dispatchEvent(new MouseEvent('mouseover',{bubbles:true})); }catch(_){}"
        "await new Promise(r=>setTimeout(r,300));"
        "const hoverTip=checkTip();"
        "try{ p.dispatchEvent(new MouseEvent('mouseleave',{bubbles:true})); }catch(_){}"
        "return {full:full.slice(0,120),focused,tag:p.tagName,slot:p.getAttribute('data-slot'),tabindex:p.getAttribute('tabindex'),focusTip,hoverTip};"
        "})()"
    )


def roving_tabstop_report():
    # Single roving tab stop oracle: exactly one rendered tabindex 0 in toolbar, no hidden/inert in roving.
    # Rendered = rect>0 and no closest [hidden],[inert],[aria-hidden=true]. Returns count, list, guidance tabindex/slot.
    return evaluate(
        "(()=>{"
        "const row=document.querySelector('.workbench-tool-row');"
        "if(!row) return {missing:true};"
        "const all=[...row.querySelectorAll('[tabindex]')];"
        "const isRendered=(e)=>{ const b=e.getBoundingClientRect(); if(!(b.width>0&&b.height>0)) return false; if(e.closest('[hidden],[inert],[aria-hidden=true]')) return false; return true; };"
        "const rendered=all.filter(isRendered);"
        "const zeros=rendered.filter(e=>e.getAttribute('tabindex')==='0').map(e=>({tag:e.tagName,aria:(e.getAttribute('aria-label')||e.textContent||'').slice(0,60),slot:e.getAttribute('data-slot'),cls:(e.className||'').toString().slice(0,60)}));"
        "const guids=[...row.querySelectorAll('.layout-selection-guidance')].map(e=>({tag:e.tagName,tabindex:e.getAttribute('tabindex'),slot:e.getAttribute('data-slot'),rendered:isRendered(e)}));"
        "const hiddenZeros=all.filter(e=>!isRendered(e)&&e.getAttribute('tabindex')==='0').length;"
        "return {total:all.length,rendered:rendered.length,zeroCount:zeros.length,zeros,guidance:guids,hiddenZeros};"
        "})()"
    )


def handles_visibility_report():
    # Handles-specific usable oracle: button[aria-label='Canvas handles'] must be whole >=32x32, enabled,
    # not inert, fully within every clipping ancestor+viewport, center topmost, via lane scroll.
    # Programmatic scroll only; keyboard/touch scrolling unproved.
    return evaluate(
        "(()=>{"
        "const g=document.querySelector('[data-workbench-section=\"view\"]');"
        "if(!g) return {missing:true};"
        "const row=document.querySelector('.workbench-tool-row');"
        "const track=g.querySelector('.workbench-tool-track');"
        "const btn=track?[...track.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Canvas handles'):null;"
        "if(!row||!track||!btn) return {missing:true};"
        "const rr=row.getBoundingClientRect();"
        "const getTrack=()=>track.getBoundingClientRect();"
        "const isEffDisabled=(e)=>{ if(e.disabled) return true; if(e.hasAttribute('disabled')) return true; if(e.closest('[disabled]')) return true; return false; };"
        "const isEffInert=(e)=>{ if(e.hasAttribute('inert')) return true; if(e.closest('[inert]')) return true; if(e.closest('[hidden]')) return true; return false; };"
        "const getClippers=(e)=>{ const out=[]; let p=e.parentElement; while(p){ try{ const cs=getComputedStyle(p); const ox=cs.overflowX, oy=cs.overflowY; if(ox==='hidden'||ox==='auto'||ox==='scroll'||ox==='clip'||oy==='hidden'||oy==='auto'||oy==='scroll'||oy==='clip'){ const r=p.getBoundingClientRect(); out.push({tag:p.tagName,cls:(p.className||'').toString().slice(0,60),x:r.x,y:r.y,w:r.width,h:r.height}); } }catch(_){} p=p.parentElement; } return out; };"
        "const withinAll=(b,clippers)=>{ for(const c of clippers){ if(!(b.x+1e-6>=c.x&&b.y+1e-6>=c.y&&b.x+b.width-1e-6<=c.x+c.w&&b.y+b.height-1e-6<=c.y+c.h)) return false; } return true; };"
        "const detail=()=>{"
        "const b=btn.getBoundingClientRect();"
        "const cx=b.x+b.width/2, cy=b.y+b.height/2;"
        "const t=getTrack();"
        "const withinTrack=b.x+1e-6>=t.x&&b.y+1e-6>=t.y&&b.x+b.width-1e-6<=t.x+t.width&&b.y+b.height-1e-6<=t.y+t.height;"
        "const withinRow=b.x+1e-6>=rr.x&&b.y+1e-6>=rr.y&&b.x+b.width-1e-6<=rr.x+rr.width&&b.y+b.height-1e-6<=rr.y+rr.height;"
        "const withinViewport=b.x+1e-6>=0&&b.y+1e-6>=0&&b.x+b.width-1e-6<=innerWidth&&b.y+b.height-1e-6<=innerHeight;"
        "const inside=cx>=0&&cx<=innerWidth&&cy>=0&&cy<=innerHeight;"
        "const clippers=getClippers(btn);"
        "const withinAllClip=withinAll(b,clippers);"
        "const el=document.elementFromPoint(cx,cy);"
        "const hit=el===btn||(el&&btn.contains(el));"
        "return {label:(btn.getAttribute('aria-label')||'').slice(0,80),x:b.x,y:b.y,w:b.width,h:b.height,effDis:isEffDisabled(btn),effInert:isEffInert(btn),cx,cy,withinTrack,withinRow,withinViewport,withinAllClip,clipCount:clippers.length,inside,hit,el:el?el.tagName+':'+(el.getAttribute('aria-label')||'').slice(0,60):null,track:{x:t.x,y:t.y,w:t.width,h:t.height}};"
        "};"
        "const before=detail();"
        "const isUsable=(r)=>!r.effDis&&!r.effInert&&r.w+1e-6>=32&&r.h+1e-6>=32&&r.withinTrack&&r.withinRow&&r.withinViewport&&r.withinAllClip&&r.inside&&r.hit;"
        "let after=null, scrolled=false, strategy=null;"
        "const origScroll=track.scrollLeft;"
        "if(!isUsable(before)){"
        "try{btn.scrollIntoView({block:'nearest',inline:'nearest',behavior:'auto'});}catch(_){try{btn.scrollIntoView();}catch(_){}}"
        "after=detail();"
        "scrolled=true; strategy='nearest';"
        "if(!isUsable(after)){ try{ btn.scrollIntoView({block:'nearest',inline:'start',behavior:'auto'}); after=detail(); strategy='start'; }catch(_){} }"
        "if(after&&!isUsable(after)){ try{ btn.scrollIntoView({block:'nearest',inline:'center',behavior:'auto'}); after=detail(); strategy='center'; }catch(_){} }"
        "if(after&&!isUsable(after)){ try{ btn.scrollIntoView({block:'nearest',inline:'end',behavior:'auto'}); after=detail(); strategy='end'; }catch(_){} }"
        "}"
        "const usable=isUsable(before)?before:(after&&isUsable(after)?after:null);"
        "try{track.scrollLeft=origScroll;}catch(_){}"
        "return {before,after,scrolled,strategy,usable:!!usable,usableDetail:usable};"
        "})()"
    )


def keyboard_guidance_traversal():
    # Real-keyboard proof: from Transform trigger (initial focus after trusted click), ArrowRight reaches guidance,
    # tooltip opens from reached focus, Home/End work. No imperative focus. Caller must have Transform open with guidance.
    # Uses browser press (trusted keys), evaluate only for inspection.
    initial = evaluate("(()=>{const a=document.activeElement; return {tag:a?a.tagName:null,aria:a?a.getAttribute('aria-label'):null};})()")
    tabstops_before = roving_tabstop_report()
    browser("press", "ArrowRight")
    browser("wait", 350)
    after_arrow = evaluate(
        "(()=>{"
        "const g=document.querySelector('[data-workbench-section=\"transform\"]');"
        "const p=g?g.querySelector('.layout-selection-guidance'):null;"
        "const a=document.activeElement;"
        "const isGuid=(a===p);"
        "let tip=null;"
        "const tipEl=g?g.querySelector('[data-slot=tooltip][data-state=open] [data-slot=tooltip-content]'):null;"
        "if(tipEl){ const b=tipEl.getBoundingClientRect(); const txt=(tipEl.textContent||'').trim(); const full=(p?(p.getAttribute('aria-label')||p.textContent||'').trim():''); const withinViewport=b.x+1e-6>=0&&b.y+1e-6>=0&&b.x+b.width-1e-6<=innerWidth&&b.y+b.height-1e-6<=innerHeight; const cx=b.x+b.width/2, cy=b.y+b.height/2; const el=document.elementFromPoint(cx,cy); const hit=el===tipEl||(el&&tipEl.contains(el)); tip={open:true,rect:{x:b.x,y:b.y,w:b.width,h:b.height},text:txt.slice(0,120),matches:txt===full,withinViewport,hit}; }"
        "else{ tip={open:false}; }"
        "let vis=null;"
        "if(p){ const b=p.getBoundingClientRect(); vis={x:b.x,y:b.y,w:b.width,h:b.height,visible:b.width>0&&b.height>0}; }"
        "return {active:{tag:a?a.tagName:null,aria:((a&&(a.getAttribute('aria-label')||a.textContent))||'').slice(0,80),isGuid},vis,tip};"
        "})()"
    )
    browser("press", "ArrowLeft")
    browser("wait", 250)
    back = evaluate("(()=>{const a=document.activeElement; return {tag:a?a.tagName:null,aria:a?a.getAttribute('aria-label'):null};})()")
    browser("press", "End")
    browser("wait", 250)
    end_focus = evaluate("(()=>{const a=document.activeElement; return {tag:a?a.tagName:null,aria:a?a.getAttribute('aria-label'):null};})()")
    browser("press", "Home")
    browser("wait", 250)
    home_focus = evaluate("(()=>{const a=document.activeElement; return {tag:a?a.tagName:null,aria:a?a.getAttribute('aria-label'):null};})()")
    tabstops_after = roving_tabstop_report()
    return {"initial":initial,"tabstops_before":tabstops_before,"after_arrow":after_arrow,"back":back,"end_focus":end_focus,"home_focus":home_focus,"tabstops_after":tabstops_after}


def motion_probe():
    # Install the observer before trusted clicks so short/reduced transitions cannot escape.
    evaluate("""(() => {
      const row=document.querySelector('.workbench-tool-row');
      window.__motionFrames=[]; window.__motionStart=performance.now();
      window.__motionStop=false;
      function sample(){
        const groups=[...row.querySelectorAll('[data-workbench-section]')].map(g=>{
          const clip=g.querySelector('.workbench-tool-reveal-inner').getBoundingClientRect();
          const track=g.querySelector('.workbench-tool-track');
          const units=[...track.querySelectorAll('button')].map(b=>{
            const i=b.querySelector('svg'), l=b.querySelector('.workbench-action-label');
            if(!i||!l||!l.getBoundingClientRect().width) return null;
            const ir=i.getBoundingClientRect(),lr=l.getBoundingClientRect();
            let opacity=1; for(let p=l;p&&p!==row;p=p.parentElement) opacity*=Number(getComputedStyle(p).opacity);
            return {label:b.getAttribute('aria-label'),dx:lr.x-ir.x,dy:lr.y-ir.y,opacity};
          }).filter(Boolean);
          return {key:g.dataset.workbenchSection,open:g.dataset.open,clip:clip.toJSON(),
            transform:getComputedStyle(track).transform,units};
        });
        window.__motionFrames.push({t:performance.now()-window.__motionStart,groups,
          running:document.getAnimations().filter(a=>a.playState==='running'&&row.contains(a.effect?.target)).length});
        if(!window.__motionStop) requestAnimationFrame(sample);
      } requestAnimationFrame(sample);
    })()""")
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    browser("wait", 70)
    browser("find", "role", "button", "click", "--name", "Snap", "--exact")
    browser("wait", 50)
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    browser("wait", 400)
    frames = evaluate("(() => {window.__motionStop=true; return window.__motionFrames;})()")
    print(f"MOTION-FRAMES: {json.dumps(frames)}", flush=True)
    assert len(frames) >= 6, "motion observer did not sample frames"
    units = [u for f in frames for g in f['groups'] if g['key'] in ('select', 'snap') for u in g['units']]
    assert units, "no rendered icon+label units sampled"
    for label in {u['label'] for u in units}:
        same = [u for u in units if u['label'] == label]
        assert max(u['dx'] for u in same)-min(u['dx'] for u in same) < 0.5, label
        assert max(u['dy'] for u in same)-min(u['dy'] for u in same) < 0.5, label
    assert all(abs(u['opacity']-1) < 0.001 for u in units), "opacity motion"
    # Clip must remain stationary throughout each observed open segment.
    for key in SECTION_KEYS.values():
        segment=[]
        for f in frames:
            g=next(g for g in f['groups'] if g['key']==key)
            if g['open']=='true':
                segment.append(g['clip'])
                for axis in ('x','y','width','height'):
                    assert max(r[axis] for r in segment)-min(r[axis] for r in segment)<0.5, f"moving clip {key} {axis}"
            else:
                segment=[]
    moving = [f for f in frames if any(g['key']=='select' and g['open']=='true' and g['transform'] not in ('none','matrix(1, 0, 0, 1, 0, 0)') for g in f['groups'])]
    assert moving, 'no intermediate solid translation sampled'
    snap_frames = [f for f in frames if any(g['key']=='snap' and g['open']=='true' for g in f['groups'])]
    assert snap_frames and any(f['running'] for f in snap_frames), 'rapid switch did not observe motion'
    switched_back = next(f for f in frames if f['t'] > snap_frames[-1]['t'] and any(g['key']=='select' and g['open']=='true' for g in f['groups']))
    assert switched_back['t'] - snap_frames[0]['t'] < 280, 'switch was not rapid (<280ms)'
    assert frames[-1]['running']==0, "stale workbench animation"
    opened=[g for g in frames[-1]['groups'] if g['open']=='true']
    assert [g['key'] for g in opened]==['select']
    # Settled transform is exactly `none`: an identity matrix would still create
    # a fixed-descendant containing block.
    assert all(g['transform'] == 'none' for g in opened), opened
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    wait_for_settled()
    # Opening a section's tooltip may unclip that section only; closed sections' inert actions stay clipped.
    browser("find", "role", "button", "click", "--name", "Transform", "--exact")
    browser("wait", 400)
    unclipped = evaluate("[...document.querySelectorAll('.workbench-tool-group[data-open=false] .workbench-tool-reveal-inner')].filter(e=>getComputedStyle(e).overflow!=='hidden').map(e=>e.closest('[data-workbench-section]').dataset.workbenchSection)")
    assert unclipped == [], f"closed sections paint through: {unclipped}"
    browser("find", "role", "button", "click", "--name", "Transform", "--exact")
    wait_for_settled()
    browser("set", "media", "light", "reduced-motion")
    assert evaluate("matchMedia('(prefers-reduced-motion: reduce)').matches")
    evaluate("""(() => {
      window.__reduced=[]; window.__reducedPhase='open';
      const row=document.querySelector('.workbench-tool-row');
      window.__reducedObserver=new MutationObserver(()=>{
        const g=row.querySelector('[data-workbench-section="select"]');
        const t=g?g.querySelector('.workbench-tool-track'):null;
        window.__reduced.push({phase:window.__reducedPhase,open:g?g.dataset.open:null,
          running:document.getAnimations().filter(a=>a.playState==='running'&&(row.contains(a.effect?.target)||a.effect?.target?.closest('.workbench-popout'))).length,
          transform:t?getComputedStyle(t).transform:null});
      }); window.__reducedObserver.observe(row,{subtree:true,attributes:true});
    })()""")
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='select'][data-open='true']\")!==null")
    wait_for_settled()
    evaluate("window.__reducedPhase='close'")
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='select'][data-open='false']\")!==null")
    wait_for_settled()
    reduced=evaluate("(() => {window.__reducedObserver.disconnect();return window.__reduced;})()")
    print(f"REDUCED-MOTION: {json.dumps(reduced)}", flush=True)
    reduced_open=[r for r in reduced if r.get('phase')=='open']
    reduced_close=[r for r in reduced if r.get('phase')=='close']
    assert reduced_open, f"reduced observer missed the opening transition: {reduced}"
    assert reduced_close, f"reduced observer missed the closing transition: {reduced}"
    assert all(r['running']==0 for r in reduced), reduced
    assert all(r.get('transform')=='none' for r in reduced), reduced
    assert any(r.get('open')=='true' for r in reduced_open), reduced
    assert any(r.get('open')=='false' for r in reduced_close), reduced
    browser("set", "media", "light")
    wait_for_settled()
    # Slice 4 row M: entry travels exactly one allocated track box (not the
    # measured scroll width): the first open frame captures ~full displacement
    # against the untransformed track border box, with genuinely intermediate
    # moving samples before settling.
    evaluate("""(() => {
      const row=document.querySelector('.workbench-tool-row');
      const g=row.querySelector('[data-workbench-section="snap"]');
      const track=g.querySelector('.workbench-tool-track');
      window.__entryFrames=[]; window.__entryStop=false;
      function sample(){
        const t=getComputedStyle(track).transform;
        let tx=0; const m=t.match(/matrix\\(([^)]+)\\)/);
        if(m) tx=Number(m[1].split(',')[4].trim());
        window.__entryFrames.push({open:g.dataset.open,tx,
          trackW:track.getBoundingClientRect().width,
          running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===track).length,
          clip:g.querySelector('.workbench-tool-reveal-inner').getBoundingClientRect().toJSON()});
        if(!window.__entryStop) requestAnimationFrame(sample);
      } requestAnimationFrame(sample);
    })()""")
    browser("find", "role", "button", "click", "--name", "Snap", "--exact")
    browser("wait", 400)
    entry = evaluate("(() => {window.__entryStop=true; return window.__entryFrames;})()")
    print(f"MOTION-ENTRY-BOX: frames {len(entry)} maxTx {max(f['tx'] for f in entry):.1f}", flush=True)
    assert len(entry) >= 6, "entry sampler did not run"
    open_entry = [f for f in entry if f['open'] == 'true']
    assert open_entry, "no open frames sampled during entry"
    # Reference is the settled (untransformed) track border box itself, not the clip.
    track_w = open_entry[-1]['trackW']
    assert track_w > 0, f"entry reference width not allocated: {open_entry[-1]}"
    max_tx = max(f['tx'] for f in open_entry)
    assert abs(max_tx - track_w) <= 8, f"entry starts {max_tx:.1f}px off one {track_w:.1f}px track box"
    assert open_entry[0]['tx'] >= track_w - 24, f"first open frame already past the one-box start: {open_entry[0]} vs {track_w:.1f}px"
    assert any(f['running'] >= 1 for f in open_entry), "no running entry sample"
    assert any(0 < f['tx'] < max_tx - 1 for f in open_entry), "no genuinely intermediate entry sample"
    for axis in ('x', 'y', 'width', 'height'):
        spread = max(f['clip'][axis] for f in open_entry) - min(f['clip'][axis] for f in open_entry)
        assert spread < 0.5, f"entry clip moves on {axis}: {spread}"
    # Slice 4: closing is immediate - the first frames after the close render
    # show a collapsed clip and no running track animation. Only track targets
    # count: the closing chevron keeps its own 170ms affordance transition.
    evaluate("""(() => {
      const row=document.querySelector('.workbench-tool-row');
      const g=row.querySelector('[data-workbench-section="snap"]');
      window.__closeFrames=[];
      function sample(){
        const track=g.querySelector('.workbench-tool-track');
        window.__closeFrames.push({open:g.dataset.open,
          w:g.querySelector('.workbench-tool-reveal-inner').getBoundingClientRect().width,
          transform:getComputedStyle(track).transform,
          running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target?.classList?.contains('workbench-tool-track')).length});
        if(window.__closeFrames.length<8) requestAnimationFrame(sample);
      } requestAnimationFrame(sample);
    })()""")
    browser("find", "role", "button", "click", "--name", "Snap", "--exact")
    browser("wait", 300)
    closed = [f for f in evaluate("window.__closeFrames") if f['open'] == 'false']
    print(f"MOTION-CLOSE: {json.dumps(closed[:6])}", flush=True)
    assert closed, "close never rendered"
    assert all(f['w'] < 0.5 for f in closed), f"closed clip not immediate: {closed}"
    assert all(f['running'] == 0 for f in closed), f"exit animation still runs: {closed}"
    assert all(f['transform'] == 'none' for f in closed), f"closed track transformed: {closed}"
    wait_for_settled()
    # Slice 4 row M: with a nonzero lane offset, earlier scrolled content must
    # not leak into the stationary outer clip while the scrolled track actually
    # moves; the offset survives close/reopen and stays put through entry.
    browser("find", "role", "button", "click", "--name", "Snap", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='snap'][data-open='true']\")!==null")
    browser("wait", 400)
    scrolled = evaluate("(()=>{const t=document.querySelector('[data-workbench-section=snap] .workbench-tool-track');t.scrollLeft=200;return {scroll:t.scrollLeft};})()")
    assert scrolled['scroll'] == 200, scrolled
    browser("find", "role", "button", "click", "--name", "Snap", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='snap'][data-open='false']\")!==null")
    wait_for_settled()
    evaluate("""(() => {
      const row=document.querySelector('.workbench-tool-row');
      const g=row.querySelector('[data-workbench-section="snap"]');
      const track=g.querySelector('.workbench-tool-track');
      const off=[...track.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Snap increment Off');
      window.__scrollFrames=[]; window.__scrollStop=false;
      function sample(){
        const b=off.getBoundingClientRect();
        const el=document.elementFromPoint(b.x+b.width/2,b.y+b.height/2);
        const t=getComputedStyle(track).transform;
        let tx=0; const m=t.match(/matrix\\(([^)]+)\\)/);
        if(m) tx=Number(m[1].split(',')[4].trim());
        window.__scrollFrames.push({open:g.dataset.open,tx,scroll:track.scrollLeft,
          running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===track).length,
          clip:g.querySelector('.workbench-tool-reveal-inner').getBoundingClientRect().toJSON(),
          hitSelf:el===off||off.contains(el)});
        if(!window.__scrollStop) requestAnimationFrame(sample);
      } requestAnimationFrame(sample);
    })()""")
    browser("find", "role", "button", "click", "--name", "Snap", "--exact")
    browser("wait", 400)
    scroll_frames = evaluate("(() => {window.__scrollStop=true; return window.__scrollFrames;})()")
    retained = evaluate("document.querySelector('[data-workbench-section=snap] .workbench-tool-track').scrollLeft")
    open_scroll = [f for f in scroll_frames if f['open'] == 'true']
    print(f"MOTION-SCROLL-ENTRY: frames {len(scroll_frames)} open {len(open_scroll)} retained {retained}", flush=True)
    assert open_scroll, "no open frames sampled"
    assert retained == 200, f"lane offset not retained across close/reopen: {retained}"
    assert all(f['scroll'] == 200 for f in open_scroll), "lane offset moved during the scrolled entry"
    scroll_max = max(f['tx'] for f in open_scroll)
    assert any(f['running'] >= 1 and 0 < f['tx'] < scroll_max - 1 for f in open_scroll), "no intermediate moving sample in the scrolled entry"
    for axis in ('x', 'y', 'width', 'height'):
        spread = max(f['clip'][axis] for f in open_scroll) - min(f['clip'][axis] for f in open_scroll)
        assert spread < 0.5, f"scroll-entry clip moves on {axis}: {spread}"
    assert all(not f['hitSelf'] for f in open_scroll), "scrolled-out content leaked into the clip during entry"
    browser("find", "role", "button", "click", "--name", "Snap", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='snap'][data-open='false']\")!==null")
    wait_for_settled()
    # Slice 4 row M: a labelled disabled unit keeps its pre-entry disabled
    # opacity while its own lane moves. The Align lane carries disabled
    # tool buttons (Left, Center X, ...) without a selection.
    disabled_baseline = evaluate("(()=>{const row=document.querySelector('.workbench-tool-row');"
        "const track=row.querySelector('[data-workbench-section=\"align\"] .workbench-tool-track');"
        "return [...track.querySelectorAll('button[disabled]')].map(b=>{"
        "let o=1; for(let p=b;p&&p!==row;p=p.parentElement) o*=Number(getComputedStyle(p).opacity);"
        "return {label:b.getAttribute('aria-label'),opacity:o};});})()")
    assert disabled_baseline, "no labelled disabled unit to pin"
    print(f"MOTION-DISABLED-BASELINE: {json.dumps(disabled_baseline)}", flush=True)
    evaluate("""(() => {
      const row=document.querySelector('.workbench-tool-row');
      const g=row.querySelector('[data-workbench-section="align"]');
      const track=g.querySelector('.workbench-tool-track');
      const labels=[...track.querySelectorAll('button[disabled]')].map(b=>b.getAttribute('aria-label'));
      window.__disabledFrames=[]; window.__disabledStop=false;
      function sample(){
        const t=getComputedStyle(track).transform;
        let tx=0; const m=t.match(/matrix\\(([^)]+)\\)/);
        if(m) tx=Number(m[1].split(',')[4].trim());
        const ops=labels.map(name=>{
          const b=[...track.querySelectorAll('button')].find(x=>x.getAttribute('aria-label')===name);
          let o=1; for(let p=b;p&&p!==row;p=p.parentElement) o*=Number(getComputedStyle(p).opacity);
          return o;});
        window.__disabledFrames.push({open:g.dataset.open,tx,
          running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===track).length,
          ops});
        if(!window.__disabledStop) requestAnimationFrame(sample);
      } requestAnimationFrame(sample);
    })()""")
    browser("find", "role", "button", "click", "--name", "Align", "--exact")
    browser("wait", 400)
    disabled_frames = evaluate("(() => {window.__disabledStop=true; return window.__disabledFrames;})()")
    disabled_open = [f for f in disabled_frames if f['open'] == 'true']
    print(f"MOTION-DISABLED: frames {len(disabled_frames)} open {len(disabled_open)}", flush=True)
    assert disabled_open, "no open frames sampled during the disabled-unit entry"
    disabled_max = max(f['tx'] for f in disabled_open)
    assert any(f['running'] >= 1 and 0 < f['tx'] < disabled_max - 1 for f in disabled_open), "disabled-unit entry never sampled mid-motion"
    for f in disabled_open:
        assert len(f['ops']) == len(disabled_baseline), f"disabled set changed mid-entry: {f}"
        for got, want in zip(f['ops'], disabled_baseline):
            assert abs(got - want['opacity']) < 0.001, f"disabled {want['label']} opacity moved {want['opacity']} -> {got}"
    browser("find", "role", "button", "click", "--name", "Align", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='align'][data-open='false']\")!==null")
    wait_for_settled()
    # Slice 4 row M: a resize mid-entry reallocates the final clip immediately
    # and settles cleanly - no leaked content, no stale animation. The sampler
    # proves the entry was still running when the parent stepped, and that the
    # first post-resize frame already shows the final clip.
    evaluate("""(() => {
      const row=document.querySelector('.workbench-tool-row');
      const g=row.querySelector('[data-workbench-section="select"]');
      const track=g.querySelector('.workbench-tool-track');
      window.__resizeFrames=[]; window.__resizeStop=false;
      function sample(){
        const t=getComputedStyle(track).transform;
        let tx=0; const m=t.match(/matrix\\(([^)]+)\\)/);
        if(m) tx=Number(m[1].split(',')[4].trim());
        window.__resizeFrames.push({open:g.dataset.open,tx,
          running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===track).length,
          parentW:row.parentElement.getBoundingClientRect().width,
          clip:g.querySelector('.workbench-tool-reveal-inner').getBoundingClientRect().toJSON()});
        if(!window.__resizeStop) requestAnimationFrame(sample);
      } requestAnimationFrame(sample);
    })()""")
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='select'][data-open='true']\")!==null")
    for _ in range(12):
        if evaluate("document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===document.querySelector(\"[data-workbench-section='select'] .workbench-tool-track\")).length") >= 1:
            break
    parent_before = evaluate("document.querySelector('.workbench-tool-row').parentElement.getBoundingClientRect().width")
    # Same-task bracketing: capture entry state immediately before and after
    # the parent-width step so the resize-mid-entry claim cannot pass when the
    # entry finished between the last RAF sample and the width change.
    mid = evaluate(
        "(()=>{const row=document.querySelector('.workbench-tool-row');"
        "const g=row.querySelector('[data-workbench-section=\"select\"]');"
        "const track=g.querySelector('.workbench-tool-track');"
        "const cap=()=>{const t=getComputedStyle(track).transform;let tx=0;const m=t.match(/matrix\\(([^)]+)\\)/);"
        "if(m)tx=Number(m[1].split(',')[4].trim());"
        "return {running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===track).length,tx,transform:t};};"
        f"const pre=cap();const p=row.parentElement;p.style.width='{parent_before - 40}px';p.style.maxWidth='{parent_before - 40}px';const post=cap();"
        "const r=row.getBoundingClientRect();const rcs=getComputedStyle(row);"
        "const W=r.width-parseFloat(rcs.paddingLeft)-parseFloat(rcs.paddingRight)-(parseFloat(rcs.borderLeftWidth)||0)-(parseFloat(rcs.borderRightWidth)||0);"
        "return {parentW:p.getBoundingClientRect().width,rowW:r.width,rowH:r.height,W,vw:innerWidth,pre,post};})()"
    )
    print(f"MOTION-RESIZE-STEP: pre {json.dumps(mid['pre'])} post {json.dumps(mid['post'])}", flush=True)
    assert mid['pre']['running'] >= 1 and mid['pre']['transform'] != 'none', f"entry not running before the resize step: {mid['pre']}"
    assert mid['post']['running'] >= 1 and mid['post']['transform'] != 'none', f"entry not running after the resize step: {mid['post']}"
    browser("wait", 400)
    wait_for_settled()
    resize_frames = evaluate("(() => {window.__resizeStop=true; return window.__resizeFrames;})()")
    resize_open = [f for f in resize_frames if f['open'] == 'true']
    first_parent = resize_open[0]['parentW']
    stepped_at = next((i for i, f in enumerate(resize_open) if abs(f['parentW'] - first_parent) > 1.0), None)
    print(f"MOTION-RESIZE-FRAMES: open {len(resize_open)} stepped_at {stepped_at} parent {first_parent}", flush=True)
    assert resize_open and stepped_at is not None and stepped_at > 0 and stepped_at < len(resize_open) - 1, "resize never landed mid-entry"
    assert resize_open[stepped_at - 1]['running'] >= 1 and resize_open[stepped_at]['running'] >= 1, f"entry not running bracketing the resize sample: {resize_open[max(0, stepped_at - 1):stepped_at + 1]}"
    final_clip_w = resize_open[-1]['clip']['width']
    assert abs(resize_open[stepped_at]['clip']['width'] - final_clip_w) < 0.5, f"clip did not reallocate immediately: {resize_open[stepped_at]['clip']} vs final {final_clip_w}"
    for seg in (resize_open[:stepped_at], resize_open[stepped_at:]):
        for axis in ('x', 'y', 'width', 'height'):
            spread = max(f['clip'][axis] for f in seg) - min(f['clip'][axis] for f in seg)
            assert spread < 0.5, f"resize-segment clip moves on {axis}: {spread}"
    resize_max = max(f['tx'] for f in resize_open)
    bracket = resize_open[max(0, stepped_at - 1):stepped_at + 2]
    assert any(f['running'] >= 1 and 0 < f['tx'] < resize_max - 1 for f in bracket), f"no intermediate moving sample bracketing the resize: {bracket}"
    resized = evaluate("(()=>{const row=document.querySelector('.workbench-tool-row');"
        "const g=row.querySelector('[data-workbench-section=\"select\"]');"
        "const track=g.querySelector('.workbench-tool-track');"
        "return {open:g.dataset.open,transform:getComputedStyle(track).transform,"
        " clip:g.querySelector('.workbench-tool-reveal-inner').getBoundingClientRect().width,"
        " rowH:row.getBoundingClientRect().height,"
        " running:document.getAnimations().filter(a=>a.playState==='running'&&row.contains(a.effect?.target)).length};})()")
    print(f"MOTION-RESIZE: parent {parent_before} -> {json.dumps(mid)} state {json.dumps(resized)}", flush=True)
    assert resized['open'] == 'true' and resized['running'] == 0, resized
    assert resized['transform'] == 'none', resized
    assert abs(resized['rowH'] - 42.0) < 0.5, resized
    for name in ENTRIES:
        rep = entry_report(name)
        assert rep.get('found') and rep['hit'], f"entry {name} lost across mid-entry resize: {rep}"
    evaluate("(()=>{const row=document.querySelector('.workbench-tool-row');const p=row.parentElement; p.style.width=''; p.style.maxWidth='';})()")
    wait_for_settled()
    parent_after = evaluate("document.querySelector('.workbench-tool-row').parentElement.getBoundingClientRect().width")
    assert abs(parent_after - parent_before) < 0.5, f"parent not restored: {parent_before} -> {parent_after}"
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='select'][data-open='false']\")!==null")
    wait_for_settled()
    # Slice 4 row R: panel toggles also run zero animations under reduced motion.
    browser("set", "media", "light", "reduced-motion")
    assert evaluate("matchMedia('(prefers-reduced-motion: reduce)').matches")
    evaluate("""(() => {
      window.__reducedPanel=[]; window.__reducedPanelPhase='open';
      const row=document.querySelector('.workbench-tool-row');
      window.__reducedPanelObserver=new MutationObserver(()=>{
        const panel=document.querySelector('#workbench-objects');
        window.__reducedPanel.push({phase:window.__reducedPanelPhase,
          hidden:panel?panel.hasAttribute('hidden'):null,
          running:document.getAnimations().filter(a=>a.playState==='running'&&(row.contains(a.effect?.target)||a.effect?.target?.closest('.workbench-popout'))).length});
      }); window.__reducedPanelObserver.observe(document.body,{subtree:true,attributes:true});
    })()""")
    browser("find", "role", "button", "click", "--name", "Objects", "--exact")
    browser("wait", "--fn", "!!document.querySelector('#workbench-objects:not([hidden])')")
    wait_for_settled()
    evaluate("window.__reducedPanelPhase='close'")
    browser("find", "role", "button", "click", "--name", "Objects", "--exact")
    browser("wait", "--fn", "!!document.querySelector('#workbench-objects[hidden]')")
    wait_for_settled()
    panel_reduced = evaluate("(() => {window.__reducedPanelObserver.disconnect();return window.__reducedPanel;})()")
    print(f"REDUCED-PANEL: {json.dumps(panel_reduced)}", flush=True)
    panel_open=[r for r in panel_reduced if r.get('phase')=='open']
    panel_close=[r for r in panel_reduced if r.get('phase')=='close']
    assert panel_open, f"reduced panel observer missed the opening transition: {panel_reduced}"
    assert panel_close, f"reduced panel observer missed the closing transition: {panel_reduced}"
    assert all(r['running'] == 0 for r in panel_reduced), panel_reduced
    assert any(r.get('hidden') is False for r in panel_open), panel_reduced
    assert any(r.get('hidden') is True for r in panel_close), panel_reduced
    browser("set", "media", "light")
    wait_for_settled()


def matrix_setup_report():
    return evaluate(
        "(() => {"
        "const p=document.querySelector('[aria-label=\"New matrix\"]');"
        "if(!p) return {open:false};"
        "const r=p.getBoundingClientRect();"
        "const cs=getComputedStyle(p);"
        "const btns={};"
        "for(const label of ['Create matrix','Cancel']){"
        " const b=[...p.querySelectorAll('button')].find(e=>e.textContent.trim()===label);"
        " if(!b){btns[label]=null;continue;}"
        " const b2=b.getBoundingClientRect();"
        " const cx=b2.x+b2.width/2, cy=b2.y+b2.height/2;"
        " const el=document.elementFromPoint(cx,cy);"
        " btns[label]={x:b2.x,y:b2.y,w:b2.width,h:b2.height,bottom:b2.bottom,right:b2.right,hit:el===b||(el&&b.contains(el))};"
        "}"
        "const vis=(e)=>{if(!e) return false; const b=e.getBoundingClientRect(); return b.width>0&&b.height>0&&!e.closest('[hidden]');};"
        "const add=document.querySelector('[aria-label=\"Add\"]');"
        "const toggle=document.querySelector('[aria-label=\"Add matrix mode\"]');"
        "const t=toggle?toggle.getBoundingClientRect():null;"
        "const row=document.querySelector('.workbench-tool-row');"
        "const rr=row?row.getBoundingClientRect():null;"
        "const canvas=[...document.querySelectorAll('#m1-workspace-panel svg')].find(s=>s.getAttribute('tabindex')==='0');"
        "const cr=canvas?canvas.getBoundingClientRect():null;"
        "const objs=document.querySelector('#workbench-objects');"
        "return {open:true,"
        " rect:{x:r.x,y:r.y,w:r.width,h:r.height,bottom:r.bottom,right:r.right},"
        " overflowY:cs.overflowY, overscroll:cs.overscrollBehavior,"
        " scrollH:p.scrollHeight, clientH:p.clientHeight, scrollTop:p.scrollTop,"
        " btns, addVisible:vis(add), objectsHidden: objs?objs.hidden:null,"
        " toggle: t?{x:t.x,y:t.y,w:t.width,h:t.height,bottom:t.bottom}:null,"
        " row: rr?{x:rr.x,y:rr.y,w:rr.width,h:rr.height,bottom:rr.bottom}:null,"
        " canvas: cr?{x:cr.x,y:cr.y,w:cr.width,h:cr.height,bottom:cr.bottom,right:cr.right}:null,"
        " backdrop: !!document.querySelector('[data-backdrop]')};"
        "})()"
    )


def matrix_setup_tests(width, height, failures):
    # 005/M1 defect section (320/390 only): Project -> Add object... -> Matrix...
    # opens the bounded New matrix task surface. The panel must stay inside the
    # visual viewport, scroll internally (overflow-y auto, overscroll contain),
    # and Create/Cancel must be reachable by scrolling and by Tab; the stale
    # Add chooser must be gone and the canvas must not resize or reframe.
    canvas_before = evaluate(
        "(() => {const c=[...document.querySelectorAll('#m1-workspace-panel svg')].find(s=>s.getAttribute('tabindex')==='0');"
        "if(!c) return null; const r=c.getBoundingClientRect();"
        "return {x:r.x,y:r.y,w:r.width,h:r.height};})()"
    )
    browser("find", "role", "button", "click", "--name", "Project", "--exact")
    browser("find", "role", "menuitem", "click", "--name", "Add object\u2026", "--exact")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Add\"]')")
    browser("find", "role", "button", "click", "--name", "Matrix\u2026")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"New matrix\"]')")
    wait_for_settled()
    rep = matrix_setup_report()
    print(f"MATRIX-SETUP {width}x{height}: {json.dumps(rep)}", flush=True)
    if not rep.get("open"):
        failures.append(("matrix-setup", "New matrix surface did not open"))
        return
    r = rep["rect"]
    if not (r["x"] >= -0.5 and r["y"] >= -0.5 and r["right"] <= width + 0.5 and r["bottom"] <= height + 0.5):
        failures.append(("matrix-setup", f"panel rect {r} outside {width}x{height} viewport"))
    if not (r["w"] <= width - 16 + 1):
        failures.append(("matrix-setup", f"panel width {r['w']} exceeds viewport minus 16px safe inset"))
    if rep.get("overflowY") not in ("auto", "scroll"):
        failures.append(("matrix-setup", f"overflow-y {rep.get('overflowY')}, require auto/scroll"))
    if "contain" not in str(rep.get("overscroll")):
        failures.append(("matrix-setup", f"overscroll-behavior {rep.get('overscroll')}, require contain"))
    if rep.get("addVisible"):
        failures.append(("matrix-setup", "stale Add popover still visible after setup opened"))
    if rep.get("objectsHidden") is False:
        failures.append(("matrix-setup", "Objects popout still open after setup opened"))
    if rep.get("backdrop"):
        failures.append(("matrix-setup", "backdrop present with setup open"))
    toggle, row = rep.get("toggle"), rep.get("row")
    if toggle and row and toggle["y"] < row["bottom"] - 0.5 and toggle["bottom"] > row["y"] + 0.5:
        failures.append(("matrix-setup", f"Add matrix mode toggle {toggle} overlaps tool row {row}"))
    if canvas_before and rep.get("canvas"):
        c = rep["canvas"]
        if any(abs(c[k] - canvas_before[k]) > 0.5 for k in ("x", "y", "w", "h")):
            failures.append(("matrix-setup", f"canvas rect changed {canvas_before} -> {c}"))
        if not (c["bottom"] > r["bottom"] or c["right"] > r["right"]):
            failures.append(("matrix-setup", f"canvas/preview area fully covered by panel {r}, canvas {c}"))
    # Scrolling to the end must make Create and Cancel fully visible + hit-testable.
    evaluate("(()=>{const p=document.querySelector('[aria-label=\"New matrix\"]'); p.scrollTop=p.scrollHeight;})()")
    browser("wait", 200)
    end = matrix_setup_report()
    print(f"MATRIX-SETUP-SCROLLED {width}x{height}: {json.dumps(end)}", flush=True)
    er = end["rect"]
    for label in ("Create matrix", "Cancel"):
        b = end.get("btns", {}).get(label)
        if not b:
            failures.append(("matrix-setup", f"{label} button missing after scroll"))
            continue
        if not (b["x"] >= er["x"] - 0.5 and b["y"] >= er["y"] - 0.5 and b["right"] <= er["right"] + 0.5 and b["bottom"] <= er["bottom"] + 0.5):
            failures.append(("matrix-setup", f"{label} not fully inside panel after scroll: {b} vs panel {er}"))
        if not (b["x"] >= 0 and b["y"] >= 0 and b["right"] <= width + 0.5 and b["bottom"] <= height + 0.5):
            failures.append(("matrix-setup", f"{label} outside viewport after scroll: {b}"))
        if not b["hit"]:
            failures.append(("matrix-setup", f"{label} not hit-testable after scroll: {b}"))
    # Tab from the surface's initial focus must reach Create and scroll it into view.
    evaluate("(()=>{const p=document.querySelector('[aria-label=\"New matrix\"]'); p.scrollTop=0;})()")
    initial = evaluate(
        "(()=>{const a=document.activeElement; const p=document.querySelector('[aria-label=\"New matrix\"]');"
        "return {inside: !!(a&&p&&p.contains(a)), label: a?a.getAttribute('aria-label'):null};})()"
    )
    print(f"MATRIX-SETUP-FOCUS {width}x{height}: {json.dumps(initial)}", flush=True)
    if not initial.get("inside"):
        failures.append(("matrix-setup", f"initial focus not inside New matrix surface: {initial}"))
    reached = None
    for _ in range(30):
        browser("press", "Tab")
        state = evaluate(
            "(()=>{const a=document.activeElement; if(!a) return null;"
            "const p=document.querySelector('[aria-label=\"New matrix\"]');"
            "const b=a.getBoundingClientRect(); const pr=p?p.getBoundingClientRect():null;"
            "return {text:(a.textContent||'').trim(), aria:a.getAttribute('aria-label'),"
            " y:b.y, bottom:b.bottom, panelTop:pr?pr.y:null, panelBottom:pr?pr.bottom:null, scrollTop:p?p.scrollTop:null};})()"
        )
        if state and state.get("text") == "Create matrix":
            reached = state
            break
    print(f"MATRIX-SETUP-TAB {width}x{height}: {json.dumps(reached)}", flush=True)
    if not reached:
        failures.append(("matrix-setup", "Tab never reached Create matrix"))
    elif not (reached["y"] >= reached["panelTop"] - 0.5 and reached["bottom"] <= reached["panelBottom"] + 0.5):
        failures.append(("matrix-setup", f"focused Create not scrolled into panel view: {reached}"))
    over = overflow_state()
    print(f"MATRIX-SETUP-OVERFLOW {width}x{height}: {json.dumps(over)}", flush=True)
    if not (over["sw"] <= over["cw"] and over["sh"] <= over["ch"]):
        failures.append(("matrix-setup", f"page overflow with setup open {over}"))
    # Close through the surface's own Cancel so later oracles start clean.
    browser("find", "role", "button", "click", "--name", "Cancel", "--exact")
    browser("wait", "--fn", "!document.querySelector('[aria-label=\"New matrix\"]')")


VIEW_LANE_PATH = ["View", "2D", "3D assembly", "Footprints", "Canvas visibility", "Canvas handles", "Objects", "Inspect"]
BANK_JS = "[...document.querySelectorAll('[data-workbench-section] > button, .workbench-utility')].map(e=>e.getBoundingClientRect().x)"


def toolbar_focus_probe():
    # Focused control: its label, whether it is inside the row, hit at its centre
    # (oversized controls are reached, not whole), plus bank/page positions.
    return evaluate(
        "(()=>{const a=document.activeElement,row=document.querySelector('.workbench-tool-row');"
        "const b=a.getBoundingClientRect(),cx=b.x+b.width/2,cy=b.y+b.height/2;"
        "const el=(cx>=0&&cx<=innerWidth&&cy>=0&&cy<=innerHeight)?document.elementFromPoint(cx,cy):null;"
        "const t=a.closest('.workbench-tool-track');"
        "return {aria:a.getAttribute('aria-label'),inRow:!!(row&&row.contains(a)),hit:el===a||!!(el&&a.contains(el)),"
        "el:el&&!(el===a||a.contains(el))?el.tagName+':'+(el.getAttribute('aria-label')||el.getAttribute('data-slot')||el.className||'').toString().slice(0,60):null,"
        "scroll:t?t.scrollLeft:null,x:b.x,w:b.width,"
        f"bank:{BANK_JS},pageX:scrollX,pageY:scrollY}};}})()"
    )


def toolbar_tab_stops():
    # Every rendered control in the row that real Tab can land on, native
    # controls without an explicit tabindex included. Inert/hidden lanes are out.
    return evaluate(
        "(()=>{const row=document.querySelector('.workbench-tool-row');"
        "const shown=e=>{const b=e.getBoundingClientRect();return b.width>0&&b.height>0&&!e.closest('[hidden],[inert]');};"
        "return [...row.querySelectorAll('button,a[href],input,select,textarea,[tabindex]')]"
        ".filter(e=>shown(e)&&!e.disabled&&e.tabIndex>=0)"
        ".map(e=>e.getAttribute('aria-label')||(e.textContent||'').trim().slice(0,30));})()"
    )


def press_and_probe(key):
    browser("press", key)
    browser("wait", 120)
    return toolbar_focus_probe()


def rove_to(label, key="ArrowRight", limit=14):
    probe = toolbar_focus_probe()
    for _ in range(limit):
        if probe.get("aria") == label:
            break
        probe = press_and_probe(key)
    return probe


def view_lane_arrow_paths():
    # From View: trusted ArrowRight across the lane to Inspect, then ArrowLeft back.
    forward = [toolbar_focus_probe()]
    for _ in VIEW_LANE_PATH[1:]:
        forward.append(press_and_probe("ArrowRight"))
    reverse = [forward[-1]]
    for _ in VIEW_LANE_PATH[1:]:
        reverse.append(press_and_probe("ArrowLeft"))
    return forward, reverse


def view_lane_keyboard_checks(tag, failures):
    # Ticket 002 item 6: View-open toolbar navigation with trusted keys. One
    # toolbar Tab stop; arrows cross every lane control in DOM order; focus
    # reveals clipped lane controls while banks and page stay put.
    evaluate("(()=>{ document.querySelectorAll('[data-slot=tooltip-trigger], [data-slot=tooltip] > button, .layout-selection-guidance').forEach(t=>{ try{ t.dispatchEvent(new MouseEvent('mouseleave',{bubbles:true})); }catch(_){} }); })()")
    if toolbar_focus_probe().get("aria") != "View":
        browser("press", "Shift+Tab")
        browser("press", "Tab")
    entry = toolbar_focus_probe()
    if entry.get("aria") != "View":
        failures.append((f"view-keys-{tag}", f"real Tab re-entry not on the View trigger: {entry}"))
        return
    stops = toolbar_tab_stops()
    print(f"VIEW-KEYS-STOPS {tag}: {json.dumps(stops)}", flush=True)
    if stops != ["View"]:
        failures.append((f"view-keys-{tag}", f"View open must leave exactly one toolbar Tab stop (View), got {stops}"))
    forward, reverse = view_lane_arrow_paths()
    print(f"VIEW-KEYS-FORWARD {tag}: {json.dumps(forward)}", flush=True)
    print(f"VIEW-KEYS-REVERSE {tag}: {json.dumps(reverse)}", flush=True)
    if [p.get("aria") for p in forward] != VIEW_LANE_PATH:
        failures.append((f"view-keys-{tag}", f"ArrowRight path {[p.get('aria') for p in forward]}, want {VIEW_LANE_PATH}"))
    if [p.get("aria") for p in reverse] != VIEW_LANE_PATH[::-1]:
        failures.append((f"view-keys-{tag}", f"ArrowLeft path {[p.get('aria') for p in reverse]}, want {VIEW_LANE_PATH[::-1]}"))
    for probe in forward + reverse:
        if not (probe.get("inRow") and probe.get("hit")):
            failures.append((f"view-keys-{tag}", f"reached control not revealed and hit in the row: {probe}"))
        if probe.get("bank") != entry.get("bank") or (probe.get("pageX"), probe.get("pageY")) != (0, 0):
            failures.append((f"view-keys-{tag}", f"banks/page moved during arrow traversal: {entry} -> {probe}"))
    end = press_and_probe("End")
    home = press_and_probe("Home")
    print(f"VIEW-KEYS-HOME-END {tag}: end {json.dumps(end)} home {json.dumps(home)}", flush=True)
    if end.get("aria") != "Inspect" or home.get("aria") != "Select":
        failures.append((f"view-keys-{tag}", f"End/Home must reach Inspect/Select, got {end.get('aria')}/{home.get('aria')}"))
    # Tab exits from a retained lane stop and Shift+Tab re-enters on it.
    rove_to("Footprints")
    exited = press_and_probe("Tab")
    back = press_and_probe("Shift+Tab")
    retained = toolbar_tab_stops()
    print(f"VIEW-KEYS-TAB {tag}: exit {json.dumps(exited)} back {json.dumps(back)} stops {json.dumps(retained)}", flush=True)
    if exited.get("inRow") or back.get("aria") != "Footprints" or retained != ["Footprints"]:
        failures.append((f"view-keys-{tag}", f"Tab must exit and Shift+Tab return to the retained Footprints stop: exit {exited}, back {back}, stops {retained}"))
    # Enter/Space keep the existing view-only toggles and the disclosure working.
    # Enter toggles and Space restores on every control, so both keys activate each.
    for label, attr in (("Footprints", "aria-pressed"),
                        ("Canvas visibility", "aria-expanded"),
                        ("Canvas handles", "aria-pressed")):
        reached = rove_to(label)
        read = f"[...document.querySelectorAll('.workbench-tool-row button')].find(e=>e.getAttribute('aria-label')==={json.dumps(label)}).getAttribute('{attr}')"
        before = evaluate(read)
        toggled = press_and_probe("Enter")
        during = evaluate(read)
        restored = press_and_probe("Space")
        after = evaluate(read)
        print(f"VIEW-KEYS-ACTIVATE {tag} {label}: {before} -Enter-> {during} -Space-> {after} focus {toggled.get('aria')}/{restored.get('aria')}", flush=True)
        if reached.get("aria") != label or during == before or after != before or toggled.get("aria") != label or restored.get("aria") != label:
            failures.append((f"view-keys-{tag}", f"Enter then Space on {label} must toggle and restore {attr} and keep focus: {before}->{during}->{after}, focus {toggled}/{restored}"))
    # Closing View excludes its retained lane controls from both Tab and arrows.
    rove_to("View", key="ArrowLeft")
    browser("press", "Enter")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'][data-open='false']\")!==null")
    wait_for_settled()
    closed_stops = toolbar_tab_stops()
    closed_next = press_and_probe("ArrowRight")
    print(f"VIEW-KEYS-CLOSED {tag}: stops {json.dumps(closed_stops)} next {json.dumps(closed_next)}", flush=True)
    if closed_stops != ["View"] or closed_next.get("aria") != "Objects":
        failures.append((f"view-keys-{tag}", f"closed View must exclude lane controls: stops {closed_stops}, ArrowRight reached {closed_next.get('aria')}"))
    rove_to("View", key="ArrowLeft")
    browser("press", "Enter")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'][data-open='true']\")!==null")
    wait_for_settled()


def production_viewport_tests(url, width=320, height=480, setup_matrix=False):
    browser("set", "viewport", width, height)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
    # Current packaged flow (verified 2026-11-11): Project -> New project -> Hide guide.
    # Do not assume legacy "4 Case"/"Open Layout workspace" steps; Layout is default.
    browser("find", "role", "button", "click", "--name", "Project", "--exact")
    browser("find", "role", "menuitem", "click", "--name", "New project", "--exact")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Keyboard setup\"]')")
    if setup_matrix:
        # Slice 3 B2 enabled-Transform receipt: create a matrix through the setup
        # guide so Transform/Align see a real selection. The existing per-section
        # oracles already branch on guidance-only vs enabled lanes; none change.
        create_matrix_via_setup_guide()
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
    if width == 728:
        motion_probe()
    for name in ENTRIES:
        rep = entry_report(name)
        print(f"ENTRY {name}: {json.dumps(rep)}", flush=True)
        if not rep.get("found"):
            failures.append((name, "missing"))
            continue
        r = rep["rect"]
        if not (r["w"] > 0 and r["h"] > 0):
            failures.append((name, f"non-positive rect {r}"))
        if not (r["w"] + 1e-6 >= MIN_HIT and r["h"] + 1e-6 >= MIN_HIT):
            failures.append((name, f"hit region {r['w']:.3f}x{r['h']:.3f}, require >={MIN_HIT}px"))
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

    # Each-section-open proof: trusted clicks, one-open, seven entries remain
    # displayed/32px/topmost, 4px stationary gaps, no page overflow, lane room.
    for sec in SECTIONS:
        key = SECTION_KEYS[sec]
        browser("find", "role", "button", "click", "--name", sec, "--exact")
        browser("wait", "--fn", f"document.querySelector(\"[data-workbench-section='{key}'][data-open='true']\")!==null")
        wait_for_settled()
        opened = open_state_report()
        print(f"OPEN {sec}: {json.dumps(opened)}", flush=True)
        open_keys = [o["key"] for o in opened if o.get("open") == "true"]
        if open_keys != [key]:
            failures.append((f"open-{sec}", f"expected one open [{key}], got {open_keys} full {opened}"))
        row_o = evaluate(
            "(() => {const rows=[...document.querySelectorAll('.workbench-tool-row')];"
            "const r=rows[0].getBoundingClientRect();"
            "return {count:rows.length,x:r.x,y:r.y,w:r.width,h:r.height}})()"
        )
        print(f"ROW open-{sec}: {json.dumps(row_o)}", flush=True)
        if row_o["count"] != 1:
            failures.append((f"open-{sec}", f"expected one .workbench-tool-row, got {row_o['count']}"))
        if abs(row_o["h"] - 42.0) >= 0.5:
            failures.append((f"open-{sec}", f"row must be 42px high, got {row_o['h']}"))
        for name in ENTRIES:
            rep = entry_report(name)
            print(f"ENTRY open-{sec} {name}: {json.dumps(rep)}", flush=True)
            if not rep.get("found"):
                failures.append((f"open-{sec} {name}", "missing"))
                continue
            r = rep["rect"]
            if not (r["w"] > 0 and r["h"] > 0):
                failures.append((f"open-{sec} {name}", f"non-positive rect {r}"))
            if not (r["w"] + 1e-6 >= MIN_HIT and r["h"] + 1e-6 >= MIN_HIT):
                failures.append((f"open-{sec} {name}", f"hit region {r['w']:.3f}x{r['h']:.3f}, require >={MIN_HIT}px"))
            if not rep["inside"]:
                failures.append((f"open-{sec} {name}", f"center {rep['center']} outside {width}x{height}"))
            if not rep["hit"]:
                failures.append((f"open-{sec} {name}", f"elementFromPoint hit {rep['el']}, occluder {rep['occluder']}"))
        spacing7 = seven_spacing_report()
        print(f"SPACING open-{sec}: {json.dumps(spacing7)}", flush=True)
        if len(spacing7["rects"]) != 7 or any(not r.get("found") for r in spacing7["rects"]):
            failures.append((f"spacing-{sec}", f"entry triggers missing {spacing7['rects']}"))
        else:
            for g in spacing7["gaps"]:
                if g["gap"] + 1e-6 < MIN_GAP:
                    failures.append((f"spacing-{sec}", f"adjacent {g['pair']} separated by {g['gap']:.3f}px, require >={MIN_GAP}px"))
        over_o = overflow_state()
        print(f"OVERFLOW open-{sec}: {json.dumps(over_o)}", flush=True)
        if not (over_o["sw"] <= over_o["cw"] and over_o["sh"] <= over_o["ch"]):
            failures.append((f"open-{sec}", f"page overflow with section open {over_o}"))
        # Close hover-opened tooltips (mouse may rest over lane after trusted click)
        # so lane overflow (auto/scroll) is measured steady-closed. Tooltip access
        # itself is proven separately via explicit focus/hover.
        evaluate("(()=>{ document.querySelectorAll('[data-slot=tooltip-trigger], [data-slot=tooltip] > button, .layout-selection-guidance').forEach(t=>{ try{ if(document.activeElement===t) t.blur(); t.dispatchEvent(new MouseEvent('mouseleave',{bubbles:true})); }catch(_){} }); })()")
        browser("wait", "--fn", "document.querySelectorAll('[data-slot=tooltip][data-state=open]').length===0")
        if sec == "Snap" and width == 320:
            # Slice 4 row K (coordinator disposition 2026-11-14): the fitting-era
            # full-visibility demand is superseded by the approved oversized-control
            # rule. Geometry snap is pressed-by-default, so the <850 policy shows its
            # label (113px button in a 42px lane): partially exposed, never whole.
            # Prove real keyboard traversal reaches, scrolls and activates it.
            browser("press", "Shift+Tab")
            browser("press", "Tab")
            initial = evaluate("({label:document.activeElement.getAttribute('aria-label'),tag:document.activeElement.tagName})")
            print(f"KEYBOARD-ENTRY: {json.dumps(initial)}", flush=True)
            assert initial['label'] == 'Snap', initial
            # Reach an initially clipped action with real toolbar keys.
            before = evaluate("(() => { const g=document.querySelector('[data-workbench-section=\"snap\"]'); const t=g.querySelector('.workbench-tool-track'); const r=t.getBoundingClientRect(); const a=[...t.querySelectorAll('button:not(:disabled)')].find(e=>e.getAttribute('aria-label')==='Geometry snap' && e.getBoundingClientRect().right>r.right+1); return {label:a?.getAttribute('aria-label'), scroll:t.scrollLeft, bank:[...document.querySelectorAll('[data-workbench-section] > button, .workbench-utility')].map(e=>e.getBoundingClientRect().x), pageX:scrollX, pageY:scrollY}; })()")
            assert before.get('label'), f"no initially offscreen enabled Snap action: {before}"
            reached = None
            for _ in range(20):
                browser("press", "ArrowRight")
                reached = evaluate("(() => {const a=document.activeElement, t=document.querySelector('[data-workbench-section=\"snap\"] .workbench-tool-track'), r=t.getBoundingClientRect(), b=a.getBoundingClientRect(), row=document.querySelector('.workbench-tool-row').getBoundingClientRect(), lab=a.querySelector(':scope > .workbench-action-label'), lb=lab?lab.getBoundingClientRect():null, hit=document.elementFromPoint(b.x+b.width/2,b.y+b.height/2); return {label:a.getAttribute('aria-label'), scroll:t.scrollLeft, enabled:!a.hasAttribute('disabled'), h:b.height, withinRowV:b.y+1e-6>=row.y&&b.y+b.height-1e-6<=row.y+row.height, labelW:lb?lb.width:null, intersects:b.x<r.x+r.width-1e-6&&b.x+b.width>r.x+1e-6, centerHit:hit===a || a.contains(hit), bank:[...document.querySelectorAll('[data-workbench-section] > button, .workbench-utility')].map(e=>e.getBoundingClientRect().x), pageX:scrollX, pageY:scrollY};})()")
                print(f"KEYBOARD-TRACE {_}: {json.dumps(reached)}", flush=True)
                if reached['label'] == before['label']:
                    break
            print(f"KEYBOARD-SCROLL: {json.dumps({'before': before, 'after': reached})}", flush=True)
            assert reached['label'] == before['label'], f"keyboard traversal never reached {before['label']}: {reached}"
            assert reached['scroll'] > before['scroll'], f"focused action did not scroll into the lane: {before}, {reached}"
            assert reached['enabled'] and reached['h'] + 1e-6 >= 32.0, f"reached action not enabled 32px-high: {reached}"
            assert reached['labelW'] is not None and reached['labelW'] > 0, f"pressed label not shown: {reached}"
            assert reached['withinRowV'] and reached['intersects'] and reached['centerHit'], f"oversized control not in-row/reachable/hit-at-centre: {reached}"
            assert reached['bank'] == before['bank'], f"trigger bank moved during keyboard scroll: {before['bank']} vs {reached['bank']}"
            assert (reached['pageX'], reached['pageY']) == (before['pageX'], before['pageY']) == (0, 0), f"page scrolled during keyboard traversal: {before}, {reached}"
            # Activation: trusted Enter toggles the pressed snap; focus stays on the
            # action; toggle back so downstream oracles see the settled lane.
            pressed_before = evaluate("(()=>[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Geometry snap').getAttribute('aria-pressed'))()")
            browser("press", "Enter")
            browser("wait", 250)
            toggled = evaluate("(()=>{const g=[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Geometry snap');return {pressed:g.getAttribute('aria-pressed'),focus:document.activeElement.getAttribute('aria-label')};})()")
            print(f"KEYBOARD-ACTIVATE: {json.dumps({'before': pressed_before, 'after': toggled})}", flush=True)
            assert toggled['pressed'] != pressed_before and toggled['focus'] == 'Geometry snap', f"Enter did not activate the reached action: {pressed_before} -> {toggled}"
            browser("press", "Enter")
            browser("wait", 250)
            restored = evaluate("(()=>[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Geometry snap').getAttribute('aria-pressed'))()")
            assert restored == pressed_before, f"restore toggle failed: {pressed_before} -> {restored}"
            port = action_lane_report(key)['rev']
            wheel_before = evaluate("(() => {const t=document.querySelector('[data-workbench-section=snap] .workbench-tool-track'); return {scroll:t.scrollLeft,bank:[...document.querySelectorAll('[data-workbench-section] > button, .workbench-utility')].map(e=>e.getBoundingClientRect().x)};})()")
            browser('mouse', 'move', round(port['x'] + port['w']/2), round(port['y'] + port['h']/2))
            browser('mouse', 'wheel', 0, -100)
            browser('wait', 120)
            wheel = evaluate("(() => {const t=document.querySelector('[data-workbench-section=snap] .workbench-tool-track'); return {scroll:t.scrollLeft,bank:[...document.querySelectorAll('[data-workbench-section] > button, .workbench-utility')].map(e=>e.getBoundingClientRect().x)};})()")
            print(f"NATIVE-WHEEL: {json.dumps({'before': wheel_before, 'after': wheel})}", flush=True)
            if wheel['scroll'] < wheel_before['scroll'] and wheel['bank'] == wheel_before['bank']:
                print(f"NATIVE-WHEEL-SCROLLED: lane moved natively {wheel_before['scroll']} -> {wheel['scroll']}, triggers stable", flush=True)
            else:
                print('NATIVE-WHEEL-UNPROVED: agent-browser wheel did not move the narrow lane; touch gestures unproved', flush=True)
        lane = action_lane_report(key)
        print(f"LANE open-{sec}: {json.dumps(lane)}", flush=True)
        if lane.get("missing"):
            failures.append((f"lane-{sec}", "section missing"))
        else:
            cw = lane.get("clientW")
            sw = lane.get("scrollW")
            if cw is None or cw + 1e-6 < MIN_HIT:
                failures.append((f"lane-{sec}", f"track clientWidth {cw}, require >={MIN_HIT}px for one 32px action, full {lane}"))
            ox = lane.get("overflowX")
            if ox not in ("auto", "scroll"):
                failures.append((f"lane-{sec}", f"track overflow-x {ox}, require auto/scroll, full {lane}"))
            if sw is not None and cw is not None and sw > cw + 1:
                print(f"LANE-OVERFLOW open-{sec}: clientW {cw} scrollW {sw} overflowX {ox}", flush=True)
            else:
                print(f"LANE-NO-OVERFLOW open-{sec}: clientW {cw} scrollW {sw} (no valid overflow fixture, unproved)", flush=True)
            btns = [a for a in lane.get("actions", []) if a.get("tag") == "BUTTON" and not a.get("isGuidance") and a.get("w", 0) + 0.5 >= MIN_HIT and a.get("h", 0) + 0.5 >= MIN_HIT]
            has_guidance = any(a.get("isGuidance") for a in lane.get("actions", []))
            if not btns and not has_guidance:
                failures.append((f"lane-{sec}", f"no 32px action and no guidance, full {lane}"))
            elif not btns and has_guidance and sec != "Transform":
                failures.append((f"lane-{sec}", f"guidance-only unexpected for {sec}, full {lane}"))
            # Sound usable-action oracle (does not weaken above): enabled, >=32, fully within track+row, center topmost, with scroll proof.
            usable = usable_action_report(key)
            print(f"USABLE open-{sec}: {json.dumps(usable)}", flush=True)
            if sec == "Transform":
                guid = guidance_report(key)
                print(f"GUIDANCE open-{sec}: {json.dumps(guid)}", flush=True)
                enabled_btns = [a for a in lane.get("actions", []) if a.get("tag") == "BUTTON" and not a.get("isGuidance") and not a.get("disabled") and a.get("w", 0) + 1e-6 >= MIN_HIT and a.get("h", 0) + 1e-6 >= MIN_HIT]
                if has_guidance and not enabled_btns:
                    if not guid.get("hasGuidance"):
                        failures.append((f"guidance-{sec}", f"expected truthful guidance, got {guid}"))
                    elif not guid.get("withinRow"):
                        failures.append((f"guidance-{sec}", f"guidance box not within 42px row, full {guid}"))
                    elif not guid.get("withinTrack"):
                        failures.append((f"guidance-{sec}", f"guidance box not within track scrollport, full {guid}"))
                    else:
                        kb = keyboard_guidance_traversal()
                        print(f"GUIDANCE-KEYBOARD open-{sec}: {json.dumps(kb)}", flush=True)
                        tb = kb.get("tabstops_before", {})
                        if tb.get("zeroCount") != 1:
                            failures.append((f"guidance-{sec}", f"roving toolbar must have exactly one tabindex 0 when Transform open, got {tb}"))
                        if kb.get("initial", {}).get("aria") != "Transform":
                            failures.append((f"guidance-{sec}", f"initial focus must be Transform trigger after trusted click, got {kb.get('initial')}"))
                        aa = kb.get("after_arrow", {})
                        if not aa.get("active", {}).get("isGuid"):
                            failures.append((f"guidance-{sec}", f"ArrowRight from Transform trigger must reach guidance via roving, got {aa}, tabstops {tb}"))
                        if not (aa.get("vis", {}).get("visible") and aa.get("vis", {}).get("w", 0) + 1e-6 >= 32 and aa.get("vis", {}).get("h", 0) + 1e-6 >= 32):
                            failures.append((f"guidance-{sec}", f"guidance reached via arrows must be visible >=32x32 actionable, got {aa}"))
                        if kb.get("back", {}).get("aria") != "Transform":
                            failures.append((f"guidance-{sec}", f"ArrowLeft from guidance must return to Transform trigger, got {kb.get('back')}"))
                        if kb.get("end_focus", {}).get("aria") not in ("Inspect", "Objects"):
                            failures.append((f"guidance-{sec}", f"End must reach last roving stop (Inspect/Objects), got {kb.get('end_focus')}"))
                        if kb.get("home_focus", {}).get("aria") != "Select":
                            failures.append((f"guidance-{sec}", f"Home must reach first roving stop (Select), got {kb.get('home_focus')}"))
                        if not guid.get("truncated"):
                            print(f"GUIDANCE-READABLE open-{sec}: inline fully visible (scrollW {guid.get('scrollW')} <= clientW {guid.get('clientW')}), roving still required", flush=True)
                        else:
                            atip = aa.get("tip", {})
                            if not (atip.get("open") and atip.get("matches") and atip.get("withinViewport") and atip.get("hit")):
                                failures.append((f"guidance-{sec}", f"tooltip must open from reached focused guidance (full text, in viewport, hit): {aa}, inline {guid}"))
                            tip = transform_tooltip_report()
                            print(f"GUIDANCE-TOOLTIP open-{sec}: {json.dumps(tip)}", flush=True)
                            hover_ok = tip.get("hoverTip", {}).get("open") and tip.get("hoverTip", {}).get("matches") and tip.get("hoverTip", {}).get("withinViewport") and tip.get("hoverTip", {}).get("hit")
                            if not hover_ok:
                                failures.append((f"guidance-{sec}", f"truncated guidance not pointer-accessible via tooltip hover: {tip}, inline {guid}"))
                else:
                    if not usable.get("usable"):
                        failures.append((f"usable-{sec}", f"no complete usable enabled action (fully within track+row, 32px, topmost, with scroll), full {usable}"))
            elif sec == "Align":
                align_cmds = [a for a in lane.get("actions", []) if a.get("tag") == "BUTTON" and a.get("label") in ("Left", "Center X", "Right", "Top", "Center Y", "Bottom")]
                all_disabled = len(align_cmds) == 6 and all(a.get("disabled") for a in align_cmds)
                if all_disabled:
                    print(f"ALIGN-NO-SELECTION open-{sec}: 6 disabled guards preserved, usable unproved without valid selection, full {usable}", flush=True)
                    if not any(a.get("label") == "Alignment reference" for a in lane.get("actions", []) if a.get("tag") == "BUTTON"):
                        failures.append((f"usable-{sec}", f"Alignment reference missing with no selection, full {lane}"))
                else:
                    if not usable.get("usable"):
                        failures.append((f"usable-{sec}", f"no complete usable enabled action with selection, full {usable}"))
            else:
                if not usable.get("usable"):
                    failures.append((f"usable-{sec}", f"no complete usable enabled action (fully within track+row, 32px, topmost, with scroll), full {usable}"))
            text_only = text_only_action_report(key)
            print(f"TEXT-ONLY open-{sec}: {json.dumps(text_only)}", flush=True)
            if text_only.get("blank"):
                failures.append((f"blank-{sec}", f"text-only actions render blank: {text_only['blank']}"))
            if sec == "Select":
                sel = select_label_report()
                print(f"SELECT-LABELS open-{sec}: {json.dumps(sel)}", flush=True)
                if not sel.get("labelsFit"):
                    failures.append((f"select-labels-{sec}", f"label presence/containment failed (W {sel.get('W')}): {sel.get('reasons')}, full {sel}"))
            if sec == "View":
                icons = view_icon_report()
                print(f"VIEW-ICONS open-{sec}: {json.dumps(icons)}", flush=True)
                if icons.get("small"):
                    failures.append((f"view-icon-{sec}", f"icon hit regions <32x32 {icons['small']}, full {icons}"))
                canvas = canvas_visibility_report()
                print(f"CANVAS open-{sec}: {json.dumps(canvas)}", flush=True)
                # The trigger keeps its natural width (ticket 002 item 6, user-approved CSS
                # exception 2026-11-14); wider than its lane, it follows the Handles reach rule.
                seen_canvas = canvas.get("reach") or {}
                canvas_partial = bool(seen_canvas) and seen_canvas["w"] > seen_canvas["track"]["w"] + 0.5
                if canvas_partial:
                    print(f"CANVAS-PARTIAL open-{sec}: {seen_canvas['w']}px control in {seen_canvas['track']['w']}px lane, reachable and hit", flush=True)
                if not (canvas.get("usable") or canvas_partial):
                    failures.append((f"canvas-{sec}", f"Canvas visibility not reachable via lane scroll (whole >=32x32 when it fits the lane, otherwise enabled, in row and hit), full {canvas}"))
                handles = handles_visibility_report()
                print(f"HANDLES open-{sec}: {json.dumps(handles)}", flush=True)
                # A control wider than its lane cannot be whole: it stays full width, scrolls into reach and
                # must be enabled, 32px high, in the row and hit at its centre (user decision 2026-11-13).
                seen = handles.get("after") or handles.get("before") or {}
                oversized = bool(seen) and seen["w"] > seen["track"]["w"] + 0.5
                partial = oversized and not seen["effDis"] and not seen["effInert"] and seen["h"] + 1e-6 >= MIN_HIT and seen["withinRow"] and seen["inside"] and seen["hit"]
                if partial:
                    print(f"HANDLES-PARTIAL open-{sec}: {seen['w']}px control in {seen['track']['w']}px lane, reachable and hit", flush=True)
                if not (handles.get("usable") or partial):
                    failures.append((f"handles-{sec}", f"Handles not reachable via lane scroll (whole >=32x32 when it fits the lane, otherwise enabled, in row and hit), full {handles}"))
                # Layers list placement (002 390 right-edge defect); shared proof,
                # also runnable standalone via --scenario layers (320 full flow now
                # passes with the rewritten K oracle; the scenario stays a scoped run).
                check_layers_open(f"open-{sec}", failures)
                view_lane_keyboard_checks(f"open-{sec}", failures)

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

    if width <= 390:
        matrix_setup_tests(width, height, failures)

    errors = browser("errors").get("errors", [])
    print(f"BROWSER ERRORS: {json.dumps(errors)}", flush=True)
    if errors:
        failures.append(("browser", f"errors {errors}"))

    print(f"FAILURES ({len(failures)}): {json.dumps(failures, indent=2)}", flush=True)
    assert not failures, failures
    print(f"PASS: real {width}x{height} viewport, 42px row, seven entry true hit tests closed+each-open, 4px/32px/no-overflow, lanes, Objects/Inspect panels", flush=True)


def closed_density_report():
    # Slice 3 B2 packaged ledger: real five-section closed density at content-box W.
    # W comes from the fractional rect (clientWidth rounds to int); Wclient is diagnostic.
    return evaluate(
        "(()=>{"
        "const row=document.querySelector('.workbench-tool-row');"
        "const r=row.getBoundingClientRect();"
        "const rcs=getComputedStyle(row);"
        "const padL=parseFloat(rcs.paddingLeft), padR=parseFloat(rcs.paddingRight);"
        "const bL=parseFloat(rcs.borderLeftWidth)||0, bR=parseFloat(rcs.borderRightWidth)||0;"
        "const W=r.width-padL-padR-bL-bR;"
        "const Wclient=row.clientWidth-padL-padR;"
        "const open=[...document.querySelectorAll('[data-workbench-section]')].filter(g=>g.getAttribute('data-open')==='true').map(g=>g.getAttribute('data-workbench-section'));"
        "const sections={};"
        "for(const g of document.querySelectorAll('[data-workbench-section]')){"
        "const k=g.getAttribute('data-workbench-section');"
        "const h=g.querySelector('.workbench-tool-heading');"
        "const c=g.querySelector('.workbench-tool-chevron');"
        "const cap=g.querySelector('.workbench-tool-caption');"
        "sections[k]={heading:h?getComputedStyle(h).display:null,chevron:c?getComputedStyle(c).display:null,"
        "caption:cap?{display:getComputedStyle(cap).display,text:(cap.textContent||'').trim().slice(0,40)}:null};"
        "}"
        "const utils=[...row.querySelectorAll('.workbench-utility .workbench-utility-label')].map(l=>({display:getComputedStyle(l).display,text:(l.textContent||'').trim().slice(0,20)}));"
        "return {W,Wclient,rowW:r.width,rowH:r.height,vw:innerWidth,open,sections,utils};"
        "})()"
    )


def closed_density_expectations(W):
    # Ticket 002 closed bands: per-section headings (partial Select+Transform 500-700),
    # captions at the calibrated 800, utility labels at 360. Chevron follows heading.
    if W < 500:
        headings = {s: False for s in SECTIONS}
    elif W < 700:
        headings = {s: s in ("Select", "Transform") for s in SECTIONS}
    else:
        headings = {s: True for s in SECTIONS}
    return headings, W >= 800, W >= 360


def check_closed_density(tag, failures):
    rep = closed_density_report()
    print(f"DENSITY-CLOSED {tag}: {json.dumps(rep)}", flush=True)
    if rep["open"]:
        failures.append((f"density-{tag}", f"expected all closed, got open {rep['open']}"))
        return rep
    headings, captions, utils = closed_density_expectations(rep["W"])
    for sec, shown in headings.items():
        got = rep["sections"][SECTION_KEYS[sec]]["heading"]
        if shown == (got == "none"):
            failures.append((f"density-{tag}", f"{sec} heading display {got} at W {rep['W']}"))
        chev = rep["sections"][SECTION_KEYS[sec]]["chevron"]
        if shown == (chev == "none"):
            failures.append((f"density-{tag}", f"{sec} chevron display {chev} at W {rep['W']}"))
        cap = rep["sections"][SECTION_KEYS[sec]]["caption"]
        if cap is not None:
            if captions == (cap["display"] == "none"):
                failures.append((f"density-{tag}", f"{sec} caption {cap} at W {rep['W']}"))
            elif captions and not cap["text"]:
                failures.append((f"density-{tag}", f"{sec} shown caption is blank at W {rep['W']}"))
    for u in rep["utils"]:
        if utils == (u["display"] == "none"):
            failures.append((f"density-{tag}", f"utility {u} at W {rep['W']}"))
    return rep


def check_entry_reports(tag, names, failures, width, height):
    # Shared entry oracle for the new B2 scenarios (the page scenario keeps its own
    # inline blocks untouched): every named entry rendered >=32px, in viewport, topmost.
    for name in names:
        rep = entry_report(name)
        print(f"ENTRY {tag} {name}: {json.dumps(rep)}", flush=True)
        if not rep.get("found"):
            failures.append((f"{tag} {name}", "missing"))
            continue
        r = rep["rect"]
        if not (r["w"] > 0 and r["h"] > 0):
            failures.append((f"{tag} {name}", f"non-positive rect {r}"))
        if not (r["w"] + 1e-6 >= MIN_HIT and r["h"] + 1e-6 >= MIN_HIT):
            failures.append((f"{tag} {name}", f"hit region {r['w']:.3f}x{r['h']:.3f}, require >={MIN_HIT}px"))
        if not rep["inside"]:
            failures.append((f"{tag} {name}", f"center {rep['center']} outside {width}x{height}"))
        if not rep["hit"]:
            failures.append((f"{tag} {name}", f"elementFromPoint hit {rep['el']}, occluder {rep['occluder']}"))


def check_page_overflow(tag, failures):
    over = overflow_state()
    print(f"OVERFLOW {tag}: {json.dumps(over)}", flush=True)
    if not (over["sw"] <= over["cw"] and over["sh"] <= over["ch"]):
        failures.append((tag, f"page overflow {over}"))


def check_browser_errors(failures):
    errors = browser("errors").get("errors", [])
    print(f"BROWSER ERRORS: {json.dumps(errors)}", flush=True)
    if errors:
        failures.append(("browser", f"errors {errors}"))


def create_matrix_via_setup_guide():
    # Shared setup: create a matrix through the setup guide so Transform/Align
    # see a real selection. The guide must be visible (call before Hide guide).
    browser("find", "role", "button", "click", "--name", "Layout & assemblies", "--exact")
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"New matrix\"]')")
    browser("wait", "--fn", "!![...document.querySelectorAll('[aria-label=\"New matrix\"] button')].find(b=>b.textContent.trim()==='Create matrix')")
    # The fresh button reaches the accessibility tree a beat after the DOM wait
    # passes; retry the trusted click until it is exposed (setup sync, not an oracle).
    for attempt in range(6):
        try:
            browser("find", "role", "button", "click", "--name", "Create matrix", "--exact")
            break
        except AssertionError:
            if attempt == 5:
                raise
            browser("wait", 500)
    browser("wait", "--fn", "!document.querySelector('[aria-label=\"New matrix\"]')")


def check_reference_geometry(tag, failures):
    # Reference dialog placement asserts shared by every Reference observation:
    # dialog inside the viewport, horizontally anchored to its trigger
    # (clamped to 8px side margins via the shared hook), Close hit-testable
    # at centre, nested control (Select button or Combobox input, whichever
    # the setup admits) hit-testable.
    ref = reference_detail_report()
    print(f"REFERENCE {tag}: {json.dumps(ref)}", flush=True)
    if not ref.get('present') or ref.get('state') != 'open':
        failures.append((f"reference-{tag}", f"Reference detail not open, full {ref}"))
        return
    menu = ref['menu']
    trig = ref.get('trigger')
    if not trig:
        failures.append((f"reference-{tag}", f"Reference trigger missing for anchoring, full {ref}"))
    else:
        max_left = max(ref['vw'] - menu['w'] - 8.0, 8.0)
        expected = min(max(trig['x'], 8.0), max_left)
        if abs(menu['x'] - expected) > 2.0:
            failures.append((f"reference-{tag}", f"dialog x {menu['x']:.1f}px not anchored to trigger x {trig['x']:.1f}px (clamped {expected:.1f}px), full {ref}"))
    if not ref['close'].get('hit') or not (ref.get('nested') or {}).get('hit'):
        occ = evaluate("(()=>{const m=document.querySelector('#m1-layout-align-menu');"
            "const chain=(el)=>{const out=[];let p=el;while(p&&p!==document.body){const cs=getComputedStyle(p);"
            "out.push(p.tagName+'.'+(p.className.baseVal!==undefined?p.className.baseVal:(p.className||'')).toString().split(' ').slice(0,2).join('.')"
            "+' pos='+cs.position+' z='+cs.zIndex+' tf='+cs.transform.slice(0,18)+' op='+cs.opacity"
            "+' flt='+cs.filter.slice(0,10)+' iso='+cs.isolation+' wc='+cs.willChange.slice(0,10)+' ct='+cs.contain.slice(0,14)+' ctn='+cs.containerType.slice(0,12)+' mbm='+cs.mixBlendMode);p=p.parentElement;}return out;};"
            "const c=[...m.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Close Alignment reference');"
            "const cb=c.getBoundingClientRect();const el=document.elementFromPoint(cb.x+cb.width/2,cb.y+cb.height/2);"
            "return {dlgPos:getComputedStyle(m).position,el:el?el.tagName:null,dlg:chain(m)};})()")
        print(f"REFERENCE-OCCLUDER {tag}: {json.dumps(occ)}", flush=True)
    if not ref['close'].get('hit'):
        failures.append((f"reference-{tag}", f"Close not hit-testable: {ref}"))
    menu = ref['menu']
    if menu['x'] + 1e-6 < 0 or menu['right'] - 1e-6 > ref['vw'] or menu['bottom'] - 1e-6 > ref['vh']:
        failures.append((f"reference-{tag}", f"dialog outside the viewport: {menu} vw {ref['vw']}"))
    nested = ref.get('nested') or {}
    if not nested.get('hit'):
        failures.append((f"reference-{tag}", f"nested reference control not hit-testable: {nested} full {ref}"))


def open_fresh_layout(url, width, height, setup_matrix=False):
    # Shared B2 setup: exact viewport, packaged page, new project, guide hidden.
    browser("set", "viewport", width, height)
    browser("open", url)
    browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
    browser("find", "role", "button", "click", "--name", "Project", "--exact")
    browser("find", "role", "menuitem", "click", "--name", "New project", "--exact")
    # A fresh page in a reused session confirms discarding the previous
    # untouched project; accept it whenever it appears (absent on first open).
    # Any agent-browser wait fails while a dialog is open, so poll in Python.
    for _ in range(60):
        try:
            if browser("dialog", "status").get("hasDialog"):
                try:
                    browser("dialog", "accept")
                except AssertionError:
                    pass
                break
            if evaluate("!!document.querySelector('[aria-label=\"Keyboard setup\"]')"):
                break
        except AssertionError:
            pass
        time.sleep(0.5)
    browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Keyboard setup\"]')")
    if setup_matrix:
        create_matrix_via_setup_guide()
    browser("find", "role", "button", "click", "--name", "Hide guide", "--exact")
    browser("wait", "--fn", "!!document.querySelector('.workbench-tool-row')")
    state = viewport_state()
    print(f"VIEWPORT {width}x{height}: {json.dumps(state)}", flush=True)
    assert state["w"] == width and state["h"] == height, (
        f"browser viewport must be exactly {width}x{height}, got {state['w']}x{state['h']}"
    )
    wait_for_settled()


def constrain_row_parent(px):
    # Constrain the row's parent at constant viewport: the container query must follow
    # the parent, not the viewport. px may be fractional. Returns achieved geometry.
    return evaluate(
        "(()=>{"
        "const row=document.querySelector('.workbench-tool-row');"
        f"const p=row.parentElement; p.style.width='{px}px'; p.style.maxWidth='{px}px';"
        "const r=row.getBoundingClientRect();"
        "const rcs=getComputedStyle(row);"
        "const W=r.width-parseFloat(rcs.paddingLeft)-parseFloat(rcs.paddingRight)-(parseFloat(rcs.borderLeftWidth)||0)-(parseFloat(rcs.borderRightWidth)||0);"
        "return {parentW:p.getBoundingClientRect().width,rowW:r.width,rowH:r.height,W,vw:innerWidth};"
        "})()"
    )


def constrained_parent_ledger(url):
    # Slice 3 B2: desktop viewport, parent-constrained row. Fractional and 849/850
    # points the integer viewport grid cannot hit (integer T-1/T come from page runs).
    width, height = 1440, 900
    open_fresh_layout(url, width, height)
    failures = []
    for target in (499.5, 699.5, 849.0, 850.0, 1199.5):
        parent = target + 4.0
        geo = constrain_row_parent(parent)
        wait_for_settled()
        rep = closed_density_report()
        print(f"CONSTRAIN W{target}: parent {parent}px -> {json.dumps(geo)} report {json.dumps(rep)}", flush=True)
        if geo["vw"] != width:
            failures.append((f"constrain-{target}", f"viewport moved to {geo['vw']}"))
        if abs(rep["W"] - target) >= 0.05:
            failures.append((f"constrain-{target}", f"achieved W {rep['W']}, want {target}"))
        if abs(rep["rowH"] - 42.0) >= 0.5:
            failures.append((f"constrain-{target}", f"row height {rep['rowH']}"))
        check_closed_density(f"W{target}", failures)
        check_entry_reports(f"closed-W{target}", ENTRIES, failures, width, height)
        if target in (849.0, 850.0):
            browser("find", "role", "button", "click", "--name", "Select", "--exact")
            browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='select'][data-open='true']\")!==null")
            wait_for_settled()
            sel = select_label_report()
            print(f"SELECT-LABELS constrained-W{target}: {json.dumps(sel)}", flush=True)
            if not sel.get("labelsFit"):
                failures.append((f"select-labels-W{target}", f"{sel.get('reasons')}, full {sel}"))
            check_entry_reports(f"open-select-W{target}", ENTRIES, failures, width, height)
            browser("find", "role", "button", "click", "--name", "Select", "--exact")
            browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='select'][data-open='false']\")!==null")
            wait_for_settled()
    evaluate(
        "(()=>{const row=document.querySelector('.workbench-tool-row');"
        "const p=row.parentElement; p.style.width=''; p.style.maxWidth='';})()"
    )
    wait_for_settled()
    check_page_overflow("constrained", failures)
    check_browser_errors(failures)
    print(f"FAILURES ({len(failures)}): {json.dumps(failures, indent=2)}", flush=True)
    assert not failures, failures
    print("PASS: constrained-parent ledger at 1440x900 viewport (W499.5/699.5/849/850/1199.5)", flush=True)


def assembly_3d_checks(url, width=1440, height=900):
    # Slice 3 B2: the 3D ToolRow branch has no accordion sections; it follows the closed
    # utility policy while 2D/3D controls and Objects/Inspect stay reachable.
    open_fresh_layout(url, width, height)
    browser("find", "role", "button", "click", "--name", "View", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'][data-open='true']\")!==null")
    wait_for_settled()
    browser("find", "role", "button", "click", "--name", "3D assembly", "--exact")
    browser("wait", "--fn", "document.querySelectorAll('[data-workbench-section]').length===0")
    wait_for_settled()
    failures = []
    row = evaluate(
        "(() => {const rows=[...document.querySelectorAll('.workbench-tool-row')];"
        "const r=rows[0].getBoundingClientRect();"
        "const rcs=getComputedStyle(rows[0]);"
        "const W=r.width-parseFloat(rcs.paddingLeft)-parseFloat(rcs.paddingRight);"
        "return {count:rows.length,x:r.x,y:r.y,w:r.width,h:r.height,W}})()"
    )
    print(f"ROW-3D {width}x{height}: {json.dumps(row)}", flush=True)
    assert row["count"] == 1, f"expected one .workbench-tool-row, got {row['count']}"
    assert abs(row["h"] - 42.0) < 0.5, f"row must be 42px high, got {row['h']}"
    check_entry_reports("3d", ["2D", "3D assembly", "Objects", "Inspect"], failures, width, height)
    utils = evaluate(
        "([...document.querySelectorAll('.workbench-utility .workbench-utility-label')]"
        ".map(l=>({display:getComputedStyle(l).display,text:(l.textContent||'').trim().slice(0,20)})))"
    )
    print(f"UTILS-3D {width}x{height} W{row['W']}: {json.dumps(utils)}", flush=True)
    want_shown = row["W"] >= 360
    if len(utils) != 2:
        failures.append(("3d-utils", f"expected 2 utility labels, got {utils}"))
    for u in utils:
        if want_shown == (u["display"] == "none"):
            failures.append(("3d-utils", f"utility {u} at W {row['W']}"))
    check_page_overflow("3d", failures)
    # Ticket 002 item 6: the 3D row keeps one toolbar Tab stop, End/Home cross
    # it with trusted keys and Enter on 2D returns to the 2D layout. Imperative
    # focus only seeds the remounted row.
    evaluate("[...document.querySelectorAll('.workbench-tool-row button')].find(e=>e.getAttribute('aria-label')==='3D assembly').focus()")
    stops_3d = toolbar_tab_stops()
    end_3d = press_and_probe("End")
    home_3d = press_and_probe("Home")
    path_3d = ["2D", "3D assembly", "Objects", "Inspect"]
    forward_3d = [home_3d.get("aria")] + [press_and_probe("ArrowRight").get("aria") for _ in path_3d[1:]]
    reverse_3d = [forward_3d[-1]] + [press_and_probe("ArrowLeft").get("aria") for _ in path_3d[1:]]
    print(f"KEYS-3D: stops {json.dumps(stops_3d)} end {json.dumps(end_3d)} home {json.dumps(home_3d)} forward {json.dumps(forward_3d)} reverse {json.dumps(reverse_3d)}", flush=True)
    if stops_3d != ["3D assembly"] or end_3d.get("aria") != "Inspect" or home_3d.get("aria") != "2D":
        failures.append(("3d-keys", f"3D row must have one Tab stop and End/Home Inspect/2D: stops {stops_3d}, end {end_3d.get('aria')}, home {home_3d.get('aria')}"))
    if forward_3d != path_3d or reverse_3d != path_3d[::-1]:
        failures.append(("3d-keys", f"3D row arrows {forward_3d} / {reverse_3d}, want {path_3d} and back"))
    if reverse_3d[-1] == "2D":
        browser("press", "Enter")
        returned_js = ("!!document.querySelector(\"[data-workbench-section='view']\")&&"
            "[...document.querySelectorAll('.workbench-tool-row button')].some(e=>e.getAttribute('aria-label')==='2D'&&e.getAttribute('aria-pressed')==='true')")
        returned = False
        for _ in range(40):
            returned = evaluate(returned_js)
            if returned:
                break
            time.sleep(0.25)
        print(f"KEYS-3D-RETURN: {returned}", flush=True)
        if not returned:
            failures.append(("3d-keys", "Enter on 2D did not restore the 2D layout (2D pressed, View lane present)"))
        else:
            wait_for_settled()
    else:
        failures.append(("3d-keys", f"keyboard could not reach 2D to return: {reverse_3d}"))
    check_browser_errors(failures)
    print(f"FAILURES ({len(failures)}): {json.dumps(failures, indent=2)}", flush=True)
    assert not failures, failures
    print(f"PASS: 3D branch at real {width}x{height} viewport, 42px row, closed-utility policy, reachable controls", flush=True)


def layers_placement_checks(url, width=390, height=844):
    # Scoped Layers placement proof at one viewport: fresh page, View open, the
    # shared list-placement oracle, page overflow and browser errors. Exists as a
    # scoped run (the 320 full flow now passes with the rewritten K oracle).
    open_fresh_layout(url, width, height)
    browser("find", "role", "button", "click", "--name", "View", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'][data-open='true']\")!==null")
    wait_for_settled()
    failures = []
    check_layers_open("layers", failures)
    check_page_overflow("layers", failures)
    check_browser_errors(failures)
    print(f"FAILURES ({len(failures)}): {json.dumps(failures, indent=2)}", flush=True)
    assert not failures, failures
    print(f"PASS: Layers list placement at real {width}x{height} viewport, 8px margins, below trigger, first choice hit", flush=True)


def overlay_surface_checks(url, width=390, height=844):
    # Slice 4 row O: overlay matrix on existing surfaces - Layers, tooltips,
    # Alignment Reference - across initial/during-entry/after-scroll opening,
    # scroll/resize while open, close/switch/reopen and Escape. Reference runs
    # on a setup page (real matrix + key selection) so its nested picker
    # admits choice; Layers/tooltips/guidance run on the fresh page above.
    open_fresh_layout(url, width, height)
    failures = []
    followups = []
    view_track_sel = "[data-workbench-section='view'] .workbench-tool-track"

    def track_motion_state(sel):
        return evaluate(
            "(()=>{const t=document.querySelector(\"" + sel + "\");" +
            "return {transform:getComputedStyle(t).transform," +
            " running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===t).length," +
            " scroll:t.scrollLeft,max:t.scrollWidth-t.clientWidth};})()"
        )

    def ensure_section_open(name, key):
        is_open = evaluate(f"document.querySelector(\"[data-workbench-section='{key}']\").getAttribute('data-open')==='true'")
        if not is_open:
            browser("find", "role", "button", "click", "--name", name, "--exact")
            browser("wait", "--fn", f"document.querySelector(\"[data-workbench-section='{key}'][data-open='true']\")!==null")
        wait_for_settled()

    # Overlay visibility expressions for the first-frame recorder: each evaluates
    # to {visible, rect, hit, triggerX, triggerBottom, trackTx, vw, vh} for
    # its site (anchor fields null for tooltips). Installed before the entry
    # starts. The trackTx lets the oracle compare against the untransformed
    # (settled) trigger, matching the production hook's anchoring contract.
    layers_overlay_js = (("(()=>{const l=document.querySelector('#m1-layers-list');"
        "const trig=document.querySelector('#m1-layers-trigger');"
        "const exp=trig&&trig.getAttribute('aria-expanded')==='true';"
        "if(!exp||!l||!trig) return {visible:false,rect:null,hit:null,triggerX:null,trackTx:null,vw:null,vh:null,triggerBottom:null};"
        "const b=l.getBoundingClientRect();const f=l.querySelector('button');const fb=f.getBoundingClientRect();"
        "const el=document.elementFromPoint(fb.x+fb.width/2,fb.y+fb.height/2);"
        "const tb=trig.getBoundingClientRect();"
        "const track=trig.closest('.workbench-tool-track');"
        "let tx=0;try{const tt=getComputedStyle(track).transform;const mm=tt.match(/matrix\\(([^)]+)\\)/);if(mm)tx=Number(mm[1].split(',')[4].trim());}catch(_){}"
        "return {visible:b.width>0.5&&b.height>0.5,"
        "rect:{x:b.x,y:b.y,w:b.width,h:b.height},hit:el===f||f.contains(el),"
        "triggerX:tb.x,triggerBottom:tb.y+tb.height,trackTx:tx,vw:innerWidth,vh:innerHeight};})()"))

    def tip_overlay_js_for(trigger_js):
        # Scope the recorder to the INTENDED tip via the trigger's
        # aria-describedby association: a hover tip on a neighboring trigger
        # (entry translation can slide one under a parked pointer) must not
        # stand in for the focus tip under test.
        return (("(()=>{const t=(" + trigger_js + ");"
            "if(!t) return {visible:false,rect:null,hit:null,triggerX:null,trackTx:null,vw:null,vh:null,triggerBottom:null};"
            "const id=t.getAttribute('aria-describedby');"
            "const tip=id?document.getElementById(id):null;"
            "if(!tip||tip.getAttribute('data-state')!=='open') return {visible:false,rect:null,hit:null,triggerX:null,trackTx:null,vw:null,vh:null,triggerBottom:null};"
            "const b=tip.getBoundingClientRect();"
            "const el=document.elementFromPoint(b.x+b.width/2,b.y+b.height/2);"
            "return {visible:b.width>0.5&&b.height>0.5,rect:{x:b.x,y:b.y,w:b.width,h:b.height},"
            "hit:el===tip||tip.contains(el),triggerX:null,trackTx:null,vw:innerWidth,vh:innerHeight,triggerBottom:null};})()"))

    def install_cancel_recorder(track_sel, overlay_js):
        # First-frame cancellation recorder, installed before the entry starts:
        # every RAF frame captures entry state plus overlay visibility/geometry
        # and the anchor inputs (trigger x, track translation, viewport).
        evaluate(
            "(()=>{const t=document.querySelector(\"" + track_sel + "\");" +
            "const g=t.closest('[data-workbench-section]');" +
            "window.__cancelFrames=[]; window.__cancelStop=false; window.__cancelError=null;" +
            "function sample(){try{let ov=null; try{ov=(" + overlay_js + ");}catch(_){ov=null;}" +
            "window.__cancelFrames.push({t:performance.now(),open:g.dataset.open," +
            " transform:getComputedStyle(t).transform," +
            " running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===t).length," +
            " scroll:t.scrollLeft,overlay:!!(ov&&ov.visible),rect:ov?ov.rect:null,hit:ov?ov.hit:null," +
            " triggerX:ov?ov.triggerX:null,trackTx:ov?ov.trackTx:null,vw:ov?ov.vw:null,vh:ov?ov.vh:null,triggerBottom:ov?ov.triggerBottom:null});" +
            "}catch(e){window.__cancelError=String((e&&e.stack)||e);}finally{"
            "if(!window.__cancelStop) requestAnimationFrame(sample);}} requestAnimationFrame(sample);})()"
        )

    def assert_first_frame_cancel(tag, action_t, pre, anchor_kind=None):
        # `pre` is the entry state sampled in the same JS task immediately
        # before the opening action (no look-back window): the entry must be
        # active at that instant. The first post-action visible frame must then
        # carry exact `none`/zero AND placed, hit-testable geometry (plus
        # clamped trigger anchoring for the shared-hook dialogs).
        browser("wait", "--fn", "new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))")
        frames = evaluate("(() => {window.__cancelStop=true; return window.__cancelFrames;})()")
        post = [f for f in frames if f['t'] >= action_t]
        print(f"{tag} diag total {len(frames)} post {len(post)} overlay_true {sum(1 for f in frames if f['overlay'])} last {json.dumps(frames[-1]) if frames else None} action_t {action_t} pre {json.dumps(pre)} err {evaluate('window.__cancelError')}", flush=True)
        if not (pre.get('running', 0) >= 1 and pre.get('transform') != 'none'):
            failures.append((tag, f"entry not active at the overlay action (timing): {pre}"))
            return
        visible = [f for f in frames if f['t'] >= action_t and f['overlay']]
        if not visible:
            failures.append((tag, "overlay never became visible in the recorder window"))
            return
        first = visible[0]
        print(f"{tag} first-visible {json.dumps(first)}", flush=True)
        # Exact `none`: an identity matrix still creates a fixed-descendant
        # containing block, the precise regression this contract excludes.
        if first['transform'] != 'none' or first['running'] != 0:
            failures.append((tag, f"first overlay-visible frame still carries the entry: {first}"))
        rect = first.get('rect') or {}
        if not rect:
            failures.append((tag, f"first overlay-visible frame has no geometry: {first}"))
            return
        if not first.get('hit'):
            failures.append((tag, f"first overlay-visible frame not hit-testable: {first}"))
        if anchor_kind in ('layers', 'reference'):
            trig_x, tx, vw = first.get('triggerX'), first.get('trackTx'), first.get('vw')
            if trig_x is None or tx is None or vw is None:
                failures.append((tag, f"first frame missing anchor inputs: {first}"))
            else:
                max_left = max(vw - rect['w'] - 8.0, 8.0)
                expected = min(max(trig_x - tx, 8.0), max_left)
                if abs(rect['x'] - expected) > 2.0:
                    failures.append((tag, f"first frame x {rect['x']:.1f}px not anchored to untransformed trigger {trig_x - tx:.1f}px (clamped {expected:.1f}px): {first}"))
                bottom = first.get('triggerBottom')
                if bottom is not None and rect['y'] + 1e-6 < bottom:
                    failures.append((tag, f"first frame top {rect['y']:.1f}px above trigger bottom {bottom:.1f}px: {first}"))
                if rect['x'] + 1e-6 < 8.0 or rect['x'] + rect['w'] - 1e-6 > vw - 8.0:
                    failures.append((tag, f"first frame outside 8px side margins: {first}"))
                vh = first.get('vh')
                if vh is not None and rect['y'] + rect['h'] - 1e-6 > vh:
                    failures.append((tag, f"first frame bottom {rect['y'] + rect['h']:.1f}px below viewport {vh:.1f}px: {first}"))
        else:
            vw, vh = first.get('vw'), first.get('vh')
            if vw is not None and vh is not None:
                if rect['x'] + 1e-6 < 0 or rect['y'] + 1e-6 < 0 or rect['x'] + rect['w'] - 1e-6 > vw or rect['y'] + rect['h'] - 1e-6 > vh:
                    failures.append((tag, f"first frame outside the viewport: {first}"))

    # ---- Layers: settled initial opening ----
    ensure_section_open("View", "view")
    check_layers_open("overlay-initial", failures)

    # ---- F1: placement RAF race (close before the pending frame) ----
    check_placement_raf_race("layers-race", failures)

    # ---- Layers: opening during entry cancels the entry ----
    install_cancel_recorder(view_track_sel, layers_overlay_js)
    browser("find", "role", "button", "click", "--name", "View", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'][data-open='false']\")!==null")
    browser("find", "role", "button", "click", "--name", "View", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'][data-open='true']\")!==null")
    layers_action = evaluate("(()=>{const t=document.querySelector(\"[data-workbench-section='view'] .workbench-tool-track\");const pre={running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===t).length,transform:getComputedStyle(t).transform};document.querySelector('#m1-layers-trigger').click();return {t:performance.now(),pre};})()")
    browser("wait", "--fn", "document.querySelector(\"#m1-layers-trigger\").getAttribute(\"aria-expanded\")===\"true\"")
    assert_first_frame_cancel("LAYERS-DURING-ENTRY", layers_action['t'], layers_action['pre'], 'layers')
    wait_for_settled()
    check_layers_geometry("overlay-during-entry", failures)
    evaluate("document.querySelector('#m1-layers-trigger').click()")
    browser("wait", "--fn", "document.querySelector(\"#m1-layers-trigger\").getAttribute(\"aria-expanded\")===\"false\"")
    wait_for_settled()

    # ---- Layers: after nonzero scroll, scroll while open ----
    lane = track_motion_state(view_track_sel)
    if lane['max'] < 50:
        print(f"LAYERS-SCROLL-SKIP: lane max scroll {lane['max']} at {width}x{height}, no overflow to vary", flush=True)
    else:
        evaluate(f"(()=>{{document.querySelector(\"{view_track_sel}\").scrollLeft=100;}})()")
        browser("wait", 150)
        evaluate("document.querySelector('#m1-layers-trigger').click()")
        browser("wait", "--fn", "document.querySelector(\"#m1-layers-trigger\").getAttribute(\"aria-expanded\")===\"true\"")
        wait_for_settled()
        check_layers_geometry("overlay-after-scroll", failures)
        for pos in (0, lane['max']):
            evaluate(f"(()=>{{document.querySelector(\"{view_track_sel}\").scrollLeft={pos};}})()")
            browser("wait", 150)
            check_layers_geometry(f"overlay-scroll-{pos}", failures)
        check_page_overflow("overlay-scroll", failures)
        evaluate("document.querySelector('#m1-layers-trigger').click()")
        browser("wait", "--fn", "document.querySelector(\"#m1-layers-trigger\").getAttribute(\"aria-expanded\")===\"false\"")
        wait_for_settled()

    # ---- Layers: resize across a density boundary while open ----
    cross_w = 350 if width <= 390 else 1180
    evaluate("document.querySelector('#m1-layers-trigger').click()")
    browser("wait", "--fn", "document.querySelector(\"#m1-layers-trigger\").getAttribute(\"aria-expanded\")===\"true\"")
    wait_for_settled()
    geo = constrain_row_parent(cross_w + 4.0)
    wait_for_settled()
    print(f"LAYERS-RESIZE: {json.dumps(geo)}", flush=True)
    check_layers_geometry("overlay-resize", failures)
    evaluate("(()=>{const row=document.querySelector('.workbench-tool-row');const p=row.parentElement; p.style.width=''; p.style.maxWidth='';})()")
    wait_for_settled()
    check_layers_geometry("overlay-resize-restored", failures)
    evaluate("document.querySelector('#m1-layers-trigger').click()")
    browser("wait", "--fn", "document.querySelector(\"#m1-layers-trigger\").getAttribute(\"aria-expanded\")===\"false\"")
    wait_for_settled()

    # ---- Layers: switch away and back, then Escape ----
    evaluate("document.querySelector('#m1-layers-trigger').click()")
    browser("wait", "--fn", "document.querySelector(\"#m1-layers-trigger\").getAttribute(\"aria-expanded\")===\"true\"")
    wait_for_settled()
    browser("find", "role", "button", "click", "--name", "Snap", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='snap'][data-open='true']\")!==null")
    wait_for_settled()
    switched = evaluate("(()=>{const t=document.querySelector('#m1-layers-trigger');const l=document.querySelector('#m1-layers-list');" +
        "const lb=l?l.getBoundingClientRect():null;" +
        "return {expanded:t.getAttribute('aria-expanded'),listGone:!l||lb.width<0.5&&lb.height<0.5};})()")
    focus = active_focus_state()
    print(f"LAYERS-SWITCH: {json.dumps(switched)} focus {json.dumps(focus)}", flush=True)
    if switched['expanded'] != 'false' or not switched['listGone']:
        failures.append(("layers-switch", f"stale open list after section switch: {switched}"))
    if not focus['visible']:
        failures.append(("layers-switch", f"invisible focus after section switch: {focus}"))
    # The switch defect above leaves the list open; close it explicitly so the
    # reopen proof starts from a known closed state.
    if switched['expanded'] != 'false':
        evaluate("document.querySelector('#m1-layers-trigger').click()")
        browser("wait", "--fn", "document.querySelector(\"#m1-layers-trigger\").getAttribute(\"aria-expanded\")===\"false\"")
        wait_for_settled()
    ensure_section_open("View", "view")
    check_layers_open("overlay-reopen", failures)
    # ---- Layers: Escape from inside the open list closes it (real keys) ----
    # LayerPanel binds Escape on the disclosure itself, so focus must be inside
    # the list: Tab in from the View trigger with real keystrokes, then Escape
    # must collapse the list and return focus visibly to the trigger. (Escape
    # from the section trigger is not a Layers action; the overlay model names
    # dismiss/hide on parent close, proven at layers-switch.)
    evaluate("document.querySelector('#m1-layers-trigger').click()")
    browser("wait", "--fn", "document.querySelector(\"#m1-layers-trigger\").getAttribute(\"aria-expanded\")===\"true\"")
    wait_for_settled()
    if active_focus_state().get('aria') != 'View':
        browser("press", "Shift+Tab")
        browser("press", "Tab")
    if active_focus_state().get('aria') != 'View':
        failures.append(("layers-escape", f"could not establish View-trigger focus entry: {active_focus_state()}"))
    else:
        for _ in range(8):
            if (active_focus_state().get('aria') or '').startswith(('Hide ', 'Show ')):
                break
            browser("press", "Tab")
        in_list = active_focus_state()
        print(f"LAYERS-ESCAPE-FOCUS: {json.dumps(in_list)}", flush=True)
        if not (in_list.get('aria') or '').startswith(('Hide ', 'Show ')):
            failures.append(("layers-escape", f"real Tab never entered the open list: {in_list}"))
        else:
            browser("press", "Escape")
            browser("wait", 300)
            wait_for_settled()
            esc = evaluate("document.querySelector('#m1-layers-trigger').getAttribute('aria-expanded')")
            focus = active_focus_state()
            print(f"LAYERS-ESCAPE: expanded {esc} focus {json.dumps(focus)}", flush=True)
            if esc != 'false':
                failures.append(("layers-escape", f"Escape from inside the list left it expanded: {esc}"))
            stale = evaluate("(()=>{const l=document.querySelector('#m1-layers-list');if(!l) return false;const b=l.getBoundingClientRect();return b.width>0.5||b.height>0.5;})()")
            if stale:
                failures.append(("layers-escape", "stale visible list after Escape"))
            if not focus['visible'] or focus.get('aria') != 'Canvas visibility':
                occ = evaluate("(()=>{const t=[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Canvas visibility');"
                    "const b=t.getBoundingClientRect();"
                    "const lane=document.querySelector(\"[data-workbench-section='view'] .workbench-tool-track\").getBoundingClientRect();"
                    "const el=document.elementFromPoint(b.x+b.width/2,b.y+b.height/2);const eb=el?el.getBoundingClientRect():null;"
                    "return {trigger:{x:b.x,y:b.y,w:b.width,h:b.height},lane:{x:lane.x,y:lane.y,w:lane.width,h:lane.height},"
                    "occluder:el?{tag:el.tagName,aria:el.getAttribute('aria-label'),role:el.getAttribute('role'),id:el.id,cls:(el.className||'').toString().slice(0,160),x:eb.x,y:eb.y,w:eb.width,h:eb.height}:null};})()")
                print(f"LAYERS-ESCAPE-OCCLUDER: {json.dumps(occ)}", flush=True)
                failures.append(("layers-escape", f"focus not visibly back on the Layers trigger after Escape: {focus}"))
    wait_for_settled()

    # ---- Tooltips: Handles hover, settled ----
    ensure_section_open("View", "view")
    handles = evaluate("(()=>{const t=document.querySelector(\"" + view_track_sel + "\");" +
        "const h=[...t.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Canvas handles');" +
        "h.scrollIntoView({block:'nearest',inline:'nearest',behavior:'auto'});" +
        "const b=h.getBoundingClientRect();" +
        "return {x:b.x+b.width/2,y:b.y+b.height/2,scroll:t.scrollLeft};})()")
    browser('mouse', 'move', round(handles['x']), round(handles['y']))
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\")!==null")
    tip = tooltip_report('view')
    print(f"TIP-HOVER: {json.dumps(tip)} scroll {handles['scroll']}", flush=True)
    if not (tip.get('open') and tip.get('text') and tip.get('withinViewport') and tip.get('hit')):
        failures.append(("tip-hover", f"Handles hover tooltip not exposed and hit: {tip}"))
    browser('mouse', 'move', width // 2, height // 2)
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\")===null")
    wait_for_settled()
    handles_hit = evaluate("(()=>{const t=document.querySelector(\"" + view_track_sel + "\");" +
        "const h=[...t.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Canvas handles');" +
        "const b=h.getBoundingClientRect();const el=document.elementFromPoint(b.x+b.width/2,b.y+b.height/2);" +
        "return el===h||h.contains(el);})()")
    if not handles_hit:
        failures.append(("tip-hover", "Handles trigger not hit after tooltip dismissal"))

    # ---- Tooltips: an overlay cycle must not replay the entry (no restart) ----
    evaluate("(()=>{const t=document.querySelector(\"" + view_track_sel + "\");" +
        "window.__restartFrames=[]; window.__restartStop=false;" +
        "function sample(){window.__restartFrames.push({transform:getComputedStyle(t).transform," +
        " running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===t).length});" +
        "if(!window.__restartStop) requestAnimationFrame(sample);} requestAnimationFrame(sample);})()")
    handles = evaluate("(()=>{const t=document.querySelector(\"" + view_track_sel + "\");" +
        "const h=[...t.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Canvas handles');" +
        "const b=h.getBoundingClientRect();return {x:b.x+b.width/2,y:b.y+b.height/2};})()")
    browser('mouse', 'move', round(handles['x']), round(handles['y']))
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\")!==null")
    browser('mouse', 'move', width // 2, height // 2)
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\")===null")
    restart_frames = evaluate("(() => {window.__restartStop=true; return window.__restartFrames;})()")
    print(f"TIP-NO-RESTART frames {len(restart_frames)}: {json.dumps(restart_frames[:8])}", flush=True)
    if not restart_frames:
        failures.append(("tip-no-restart", "restart sampler did not run"))
    elif not all(f['transform'] == 'none' and f['running'] == 0 for f in restart_frames):
        failures.append(("tip-no-restart", f"overlay cycle replayed the entry: {restart_frames}"))
    wait_for_settled()

    # ---- Tooltips: Handles via real keyboard focus (toolbar arrows) ----
    # The View lane is inside the toolbar roving scope (ticket 002 item 6):
    # real Tab re-enters on the retained stop (Canvas visibility after the
    # Escape proof above), trusted arrows reach View and then Handles, and the
    # tooltip opens from that focus.
    retained_stop = toolbar_tab_stops()
    browser("press", "Shift+Tab")
    kb_reentry = press_and_probe("Tab")
    print(f"TIP-KEYBOARD reentry {json.dumps(kb_reentry)} retained {json.dumps(retained_stop)}", flush=True)
    if [kb_reentry.get('aria')] != retained_stop:
        failures.append(("tip-keyboard", f"real Tab re-entry not on the retained toolbar stop {retained_stop}: {kb_reentry}"))
    kb_entry = rove_to('View', key='ArrowLeft', limit=8)
    if kb_entry.get('aria') != 'View':
        failures.append(("tip-keyboard", f"roving entry not on View trigger: {kb_entry}"))
    else:
        kb_focus = rove_to('Canvas handles', limit=6)
        print(f"TIP-KEYBOARD focus {json.dumps(kb_focus)}", flush=True)
        if kb_focus.get('aria') != 'Canvas handles':
            failures.append(("tip-keyboard", f"toolbar arrows never reached Handles: {kb_focus}"))
        else:
            browser("wait", 300)
            tip = tooltip_report('view')
            print(f"TIP-KEYBOARD tip {json.dumps(tip)}", flush=True)
            if not (tip.get('open') and tip.get('hit') and tip.get('withinViewport')):
                failures.append(("tip-keyboard", f"tooltip did not open from reached focus: {tip}"))
            rove_to('View', key='ArrowLeft', limit=6)
            browser("wait", 250)

    # ---- View-lane roving (ticket 002 item 6, required) ----
    # One toolbar Tab stop with View open; arrows cross every lane control.
    browser("press", "Shift+Tab")
    reentry = press_and_probe("Tab")
    if reentry.get('aria') != 'View':
        failures.append(("roving-view-lane", f"real Tab re-entry must return to the retained View stop: {reentry}"))
    rove_to('View', key='ArrowLeft', limit=8)
    row_stops = toolbar_tab_stops()
    forward, reverse = view_lane_arrow_paths()
    arrow_path = [p.get('aria') for p in forward]
    back_path = [p.get('aria') for p in reverse]
    print(f"ROVING path {json.dumps(arrow_path)} back {json.dumps(back_path)} tabStops {json.dumps(row_stops)}", flush=True)
    if row_stops != ['View'] or arrow_path != VIEW_LANE_PATH or back_path != VIEW_LANE_PATH[::-1]:
        failures.append(("roving-view-lane", f"View open: stops {row_stops}, arrows {arrow_path}, back {back_path}; want one View stop and {VIEW_LANE_PATH}"))

    # ---- Tooltips: Handles focus during entry cancels the entry ----
    # Imperative focus here is timing determinism only; focus-open itself is
    # proven with real keys above and in the page flow (Transform guidance).
    install_cancel_recorder(view_track_sel, tip_overlay_js_for("[...document.querySelector(\"" + view_track_sel + "\").querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Canvas handles')"))
    browser("find", "role", "button", "click", "--name", "View", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'][data-open='false']\")!==null")
    browser("find", "role", "button", "click", "--name", "View", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'][data-open='true']\")!==null")
    tip_action = evaluate("(()=>{const t=document.querySelector(\"" + view_track_sel + "\");const h=[...t.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Canvas handles');const pre={running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===t).length,transform:getComputedStyle(t).transform};h.focus();return {t:performance.now(),pre};})()")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\")!==null")
    assert_first_frame_cancel("TIP-DURING-ENTRY", tip_action['t'], tip_action['pre'])
    tip = tooltip_report('view')
    if not (tip.get('open') and tip.get('hit') and tip.get('withinViewport')):
        failures.append(("tip-during-entry", f"tooltip not placed and hit during entry: {tip}"))
    # Park the pointer (real move) before Tab-away: trusted clicks park it over
    # the lane and the entry slides neighboring triggers under it, so a
    # hover-held neighbor tip would otherwise pollute the focus-release proof.
    browser('mouse', 'move', width // 2, height // 2)
    browser("wait", 200)
    # Release the focus-held tip with a real Tab: imperative blur can strand it open.
    browser("press", "Tab")
    for _ in range(15):
        if evaluate("document.querySelectorAll(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\").length") == 0:
            break
        browser("wait", 200)
    ghosts = evaluate("document.querySelectorAll(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\").length")
    if ghosts:
        ghost_focus = active_focus_state()
        ghost_hover = evaluate("(()=>{const h=[...document.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Canvas handles');return h?h.matches(':hover'):null;})()")
        ghost_tips = evaluate("[...document.querySelectorAll(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\")].map(t=>{const c=t.querySelector('[data-slot=tooltip-content]');return (c?c.textContent:'<no-content>').trim().slice(0,40);})")
        print(f"TIP-DURING-ENTRY-GHOST focus {json.dumps(ghost_focus)} hover {ghost_hover} tips {json.dumps(ghost_tips)}", flush=True)
        failures.append(("tip-during-entry", "focus tip stuck open after real Tab away (ghost)"))
    wait_for_settled()

    # ---- Tooltips: scroll and resize while open, close with tip open ----
    lane = track_motion_state(view_track_sel)
    if lane['max'] < 50:
        print(f"TIP-SCROLL-SKIP: lane max scroll {lane['max']} at {width}x{height}", flush=True)
    else:
        evaluate(f"(()=>{{document.querySelector(\"{view_track_sel}\").scrollLeft={min(120, lane['max'])};}})()")
        browser("wait", 150)
        handles = evaluate("(()=>{const t=document.querySelector(\"" + view_track_sel + "\");" +
            "const h=[...t.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Canvas handles');" +
            "const b=h.getBoundingClientRect();return {x:b.x+b.width/2,y:b.y+b.height/2};})()")
        browser('mouse', 'move', round(handles['x']), round(handles['y']))
        browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\")!==null")
        tip = tooltip_report('view')
        if not (tip.get('open') and tip.get('hit')):
            failures.append(("tip-scroll", f"tooltip not hit after lane scroll: {tip}"))
        evaluate(f"(()=>{{document.querySelector(\"{view_track_sel}\").scrollLeft=0;}})()")
        browser("wait", 200)
        tip = tooltip_report('view')
        print(f"TIP-SCROLL-WHILE-OPEN: {json.dumps(tip)}", flush=True)
        if tip.get('open'):
            if not (tip.get('hit') and tip.get('withinViewport')):
                failures.append(("tip-scroll", f"open tooltip misplaced after lane scroll: {tip}"))
        else:
            ghosts = evaluate("document.querySelectorAll(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\").length")
            if ghosts:
                failures.append(("tip-scroll", "ghost tooltip after lane scroll"))
        browser('mouse', 'move', width // 2, height // 2)
        browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\")===null")
    handles = evaluate("(()=>{const t=document.querySelector(\"" + view_track_sel + "\");" +
        "const h=[...t.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Canvas handles');" +
        "h.scrollIntoView({block:'nearest',inline:'nearest',behavior:'auto'});" +
        "const b=h.getBoundingClientRect();return {x:b.x+b.width/2,y:b.y+b.height/2};})()")
    browser('mouse', 'move', round(handles['x']), round(handles['y']))
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\")!==null")
    geo = constrain_row_parent(cross_w + 4.0)
    wait_for_settled()
    tip = tooltip_report('view')
    print(f"TIP-RESIZE {json.dumps(geo)}: {json.dumps(tip)}", flush=True)
    if tip.get('open') and not (tip.get('hit') and tip.get('withinViewport')):
        failures.append(("tip-resize", f"open tooltip misplaced across resize: {tip}"))
    evaluate("(()=>{const row=document.querySelector('.workbench-tool-row');const p=row.parentElement; p.style.width=''; p.style.maxWidth='';})()")
    wait_for_settled()
    # Close with the tip actually open: re-hover Handles, then close via real
    # keyboard activation. Trusted clicks would park the pointer away from the
    # trigger (pre-dismissing via mouseleave as part of the click); Enter keeps
    # the pointer over Handles so only the close itself can clear the tip.
    handles = evaluate("(()=>{const t=document.querySelector(\"" + view_track_sel + "\");" +
        "const h=[...t.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Canvas handles');" +
        "const b=h.getBoundingClientRect();return {x:b.x+b.width/2,y:b.y+b.height/2};})()")
    browser('mouse', 'move', round(handles['x']), round(handles['y']))
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\")!==null")
    tip = tooltip_report('view')
    if not tip.get('open'):
        failures.append(("tip-close", f"tooltip not open before the close-with-tip-open story: {tip}"))
    # Real Shift+Tab re-enters the toolbar on its retained stop (Handles, after
    # the Tab-away above); toolbar arrows then reach View (ticket 002 item 6).
    if not toolbar_focus_probe().get('inRow'):
        browser("press", "Shift+Tab")
    rove_to('View', key='ArrowLeft', limit=8)
    if active_focus_state().get('aria') != 'View':
        failures.append(("tip-close", f"could not focus View trigger for keyboard close: {active_focus_state()}"))
    else:
        browser("press", "Enter")
        browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='view'][data-open='false']\")!==null")
        wait_for_settled()
        ghosts = evaluate("document.querySelectorAll(\"[data-workbench-section='view'] [data-slot=tooltip][data-state=open]\").length")
        if ghosts:
            failures.append(("tip-close", f"{ghosts} ghost tooltips after keyboard close with tip open"))
    # Hygiene after measurement: park the pointer so later stories start clean.
    browser('mouse', 'move', width // 2, height // 2)
    browser("wait", 200)

    # ---- Tooltips: guidance during entry (truncated widths only) ----
    if width > 390:
        print("TIP-GUIDANCE-SKIP: guidance readable inline at wide widths", flush=True)
    else:
        transform_track_sel = "[data-workbench-section='transform'] .workbench-tool-track"
        ensure_section_open("Transform", "transform")
        install_cancel_recorder(transform_track_sel, tip_overlay_js_for("document.querySelector(\"[data-workbench-section='transform'] .layout-selection-guidance\")"))
        browser("find", "role", "button", "click", "--name", "Transform", "--exact")
        browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='transform'][data-open='false']\")!==null")
        browser("find", "role", "button", "click", "--name", "Transform", "--exact")
        browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='transform'][data-open='true']\")!==null")
        guidance_action = evaluate("(()=>{const t=document.querySelector(\"[data-workbench-section='transform'] .workbench-tool-track\");const pre={running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===t).length,transform:getComputedStyle(t).transform};document.querySelector(\"[data-workbench-section='transform'] .layout-selection-guidance\").focus();return {t:performance.now(),pre};})()")
        browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='transform'] [data-slot=tooltip][data-state=open]\")!==null")
        assert_first_frame_cancel("TIP-GUIDANCE-ENTRY", guidance_action['t'], guidance_action['pre'])
        tip = tooltip_report('transform')
        if not (tip.get('open') and tip.get('hit') and tip.get('withinViewport')):
            failures.append(("tip-guidance-entry", f"guidance tooltip not placed and hit during entry: {tip}"))
        # Close with the focus-held tip still open: no blur pre-dismiss. The
        # close-focus rescue moves focus out; the tip must follow, not ghost.
        browser("find", "role", "button", "click", "--name", "Transform", "--exact")
        browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='transform'][data-open='false']\")!==null")
        wait_for_settled()
        ghosts = evaluate("document.querySelectorAll(\"[data-workbench-section='transform'] [data-slot=tooltip][data-state=open]\").length")
        if ghosts:
            failures.append(("tip-guidance-entry", f"{ghosts} ghost tooltips after section close"))

    # ---- Alignment Reference (setup page: admitted nested picker) ----
    # Fresh page with a real matrix + key selection: the nested reference
    # picker admits choice (30 keys -> Combobox form; the <=12 Select form is
    # proven mounted). Every Reference story below runs on this page.
    open_fresh_layout(url, width, height, setup_matrix=True)
    # Admit the nested picker: open Objects, expand Column 1, select its first
    # key (trusted clicks; no guard touched), then close the panel again.
    browser("find", "role", "button", "click", "--name", "Objects", "--exact")
    browser("wait", "--fn", "!!document.querySelector('#workbench-objects:not([hidden])')")
    browser("wait", 500)
    browser("find", "role", "button", "click", "--name", "Expand Column 1", "--exact")
    browser("wait", 500)
    browser("find", "role", "treeitem", "click", "--name", "Key 1.1 SW1", "--exact")
    browser("wait", 800)
    browser("press", "Escape")
    browser("wait", "--fn", "!!document.querySelector('#workbench-objects[hidden]')")
    wait_for_settled()
    align_track_sel = "[data-workbench-section='align'] .workbench-tool-track"
    reference_overlay_js = ("(()=>{const m=document.querySelector('#m1-layout-align-menu');"
        "const trig=document.querySelector('#m1-layout-align-trigger');"
        "if(!m||!trig||m.getAttribute('data-state')!=='open') return {visible:false,rect:null,hit:null,triggerX:null,trackTx:null,vw:null,vh:null,triggerBottom:null};"
        "const b=m.getBoundingClientRect();"
        "const c=[...m.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Close Alignment reference');"
        "const cb=c.getBoundingClientRect();"
        "const cx=cb.x+cb.width/2,cy=cb.y+cb.height/2;"
        "const el=(cx>=0&&cx<=innerWidth&&cy>=0&&cy<=innerHeight)?document.elementFromPoint(cx,cy):null;"
        "const tb=trig.getBoundingClientRect();"
        "const track=trig.closest('.workbench-tool-track');"
        "let tx=0;try{const tt=getComputedStyle(track).transform;const mm=tt.match(/matrix\\(([^)]+)\\)/);if(mm)tx=Number(mm[1].split(',')[4].trim());}catch(_){}"
        "return {visible:b.width>0.5&&b.height>0.5,"
        "rect:{x:b.x,y:b.y,w:b.width,h:b.height},hit:el===c||c.contains(el),"
        "triggerX:tb.x,triggerBottom:tb.y+tb.height,trackTx:tx,vw:innerWidth,vh:innerHeight};})()")

    def close_detail_programmatic(tag):
        # Close click for state transitions (not the Close-button proof):
        # records a failure instead of crashing when the detail is already
        # gone (e.g. an Escape closed it early under test).
        found = evaluate("(()=>{const m=document.querySelector('#m1-layout-align-menu');if(!m) return 'gone';const c=[...m.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Close Alignment reference');if(!c) return 'no-button';c.click();return 'clicked';})()")
        if found != 'clicked':
            failures.append((tag, f"expected open detail for programmatic Close, menu {found}"))

    def nested_picker_placement():
        return evaluate(
            "(()=>{const lb=document.querySelector(\"#m1-layout-align-menu [role='listbox']\");" +
            "if(!lb) return {open:false};" +
            "const b=lb.getBoundingClientRect();" +
            "const o=lb.querySelector('[role=option]');" +
            "const ob=o.getBoundingClientRect();" +
            "const el=document.elementFromPoint(ob.x+ob.width/2,ob.y+ob.height/2);" +
            "return {open:true,x:b.x,y:b.y,w:b.width,h:b.height,scrollTop:lb.scrollTop,optY:ob.y," +
            " withinViewport:b.x+1e-6>=0&&b.y+1e-6>=0&&b.x+b.width-1e-6<=innerWidth&&b.y+b.height-1e-6<=innerHeight," +
            " firstHit:el===o||o.contains(el),"
            " el:el?el.tagName+':'+((el.getAttribute&&(el.getAttribute('aria-label')||el.id||el.className))||'').toString().slice(0,60):null," +
            " options:lb.querySelectorAll('[role=option]').length};})()"
        )

    # ---- Reference: opening during entry cancels the entry (first frame) ----
    install_cancel_recorder(align_track_sel, reference_overlay_js)
    browser("find", "role", "button", "click", "--name", "Align", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='align'][data-open='true']\")!==null")
    reference_action = evaluate("(()=>{const t=document.querySelector('[data-workbench-section=align] .workbench-tool-track');" +
        "const r=[...t.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Alignment reference');" +
        "r.scrollIntoView({block:'nearest',inline:'nearest',behavior:'auto'});const pre={running:document.getAnimations().filter(a=>a.playState==='running'&&a.effect?.target===t).length,transform:getComputedStyle(t).transform};r.click();return {t:performance.now(),pre};})()")
    browser("wait", "--fn", "document.querySelector(\"#m1-layout-align-menu[data-state='open']\")!==null")
    assert_first_frame_cancel("REFERENCE-DURING-ENTRY", reference_action['t'], reference_action['pre'], 'reference')
    wait_for_settled()
    check_reference_geometry("during-entry", failures)
    # Nested timing inside the during-entry story: the admitted picker opens
    # promptly with placed, hit-testable choices once its dialog is up. (The
    # dialog open above already cancelled the track entry, so no second
    # first-frame assertion applies to the nested surface itself.)
    # Keyboard-only: the trusted click point inside the dialog can read as
    # covered at narrow widths; Tab reaches the input deterministically.
    for _ in range(4):
        if active_focus_state().get('tag') == 'INPUT':
            break
        browser("press", "Tab")
    if active_focus_state().get('tag') != 'INPUT':
        failures.append(("reference-nested-entry", f"Tab never reached the picker input: {active_focus_state()}"))
    else:
        browser("press", "ArrowDown")
        browser("wait", "--fn", "!!document.querySelector(\"#m1-layout-align-menu [role='listbox']\")")
        browser("wait", 300)
        nested_entry = nested_picker_placement()
        print(f"REFERENCE-NESTED-ENTRY: {json.dumps(nested_entry)}", flush=True)
        if not (nested_entry.get('open') and nested_entry.get('withinViewport') and nested_entry.get('firstHit')):
            failures.append(("reference-nested-entry", f"nested picker not placed and hit in the during-entry story: {nested_entry}"))
        browser("press", "Escape")
        browser("wait", "--fn", "!document.querySelector(\"#m1-layout-align-menu [role='listbox']\")")
    close_detail_programmatic("reference-nested-entry")
    browser("wait", "--fn", "(()=>{const m=document.querySelector('#m1-layout-align-menu');return m===null||m.getAttribute('data-state')!=='open';})()")
    wait_for_settled()

    # ---- Reference: settled opening + placement ----
    ensure_section_open("Align", "align")
    evaluate("(()=>{const t=document.querySelector('[data-workbench-section=align] .workbench-tool-track');" +
        "const r=[...t.querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Alignment reference');" +
        "r.scrollIntoView({block:'nearest',inline:'nearest',behavior:'auto'});})()")
    browser("wait", 200)
    browser("find", "role", "button", "click", "--name", "Alignment reference", "--exact")
    browser("wait", "--fn", "document.querySelector(\"#m1-layout-align-menu[data-state='open']\")!==null")
    browser("wait", 300)
    ref = reference_detail_report()
    print(f"REFERENCE-OPEN: {json.dumps(ref)}", flush=True)
    if not ref.get('present') or ref.get('state') != 'open':
        failures.append(("reference-open", f"Reference detail did not open: {ref}"))
    else:
        check_reference_geometry("open", failures)
        nested = ref.get('nested') or {}
        if nested.get('disabled') or nested.get('form') != 'INPUT':
            failures.append(("reference-nested", f"nested picker not admitted as an enabled Combobox: {nested}"))

    # ---- Reference: opening after scroll, scroll + resize while open ----
    # Programmatic Close (trusted clicks misfire inside the fixed dialog;
    # Close-button activation is proven via keyboard in the mechanics below).
    close_detail_programmatic("reference-scroll")
    browser("wait", "--fn", "(()=>{const m=document.querySelector('#m1-layout-align-menu');return m===null||m.getAttribute('data-state')!=='open';})()")
    wait_for_settled()
    lane = track_motion_state(align_track_sel)
    if lane['max'] < 50:
        print(f"REFERENCE-SCROLL-SKIP: lane max scroll {lane['max']} at {width}x{height}, no overflow to vary", flush=True)
        browser("find", "role", "button", "click", "--name", "Alignment reference", "--exact")
        browser("wait", "--fn", "document.querySelector(\"#m1-layout-align-menu[data-state='open']\")!==null")
        wait_for_settled()
    else:
        evaluate(f"(()=>{{document.querySelector(\"{align_track_sel}\").scrollLeft=100;}})()")
        browser("wait", 150)
        browser("find", "role", "button", "click", "--name", "Alignment reference", "--exact")
        browser("wait", "--fn", "document.querySelector(\"#m1-layout-align-menu[data-state='open']\")!==null")
        wait_for_settled()
        check_reference_geometry("after-scroll", failures)
        for pos in (0, lane['max']):
            evaluate(f"(()=>{{document.querySelector(\"{align_track_sel}\").scrollLeft={pos};}})()")
            browser("wait", 150)
            check_reference_geometry(f"scroll-{pos}", failures)
        check_page_overflow("reference-scroll", failures)
    geo = constrain_row_parent(cross_w + 4.0)
    wait_for_settled()
    print(f"REFERENCE-RESIZE: {json.dumps(geo)}", flush=True)
    check_reference_geometry("resize", failures)
    evaluate("(()=>{const row=document.querySelector('.workbench-tool-row');const p=row.parentElement; p.style.width=''; p.style.maxWidth='';})()")
    wait_for_settled()
    check_reference_geometry("resize-restored", failures)

    # ---- Reference: nested Combobox choice + Escape ladder (real keys) ----
    for _ in range(4):
        if active_focus_state().get('tag') == 'INPUT' and active_focus_state().get('aria') == 'Alignment reference':
            break
        browser("press", "Tab")
    combo_focus = active_focus_state()
    if combo_focus.get('tag') != 'INPUT' or combo_focus.get('aria') != 'Alignment reference':
        failures.append(("reference-nested", f"Tab never reached the picker input: {combo_focus}"))
    else:
        browser("press", "ArrowDown")
        browser("wait", "--fn", "!!document.querySelector(\"#m1-layout-align-menu [role='listbox']\")")
        browser("wait", 300)
        placed = nested_picker_placement()
        print(f"REFERENCE-NESTED-PLACEMENT: {json.dumps(placed)}", flush=True)
        if not (placed.get('withinViewport') and placed.get('firstHit')):
            failures.append(("reference-nested", f"open picker misplaced: {placed}"))
        options = evaluate("[...document.querySelectorAll(\"#m1-layout-align-menu [role='option']\")].map(o=>o.textContent.trim())")
        print(f"REFERENCE-NESTED-OPTIONS: {len(options)} {json.dumps(options[:8])}", flush=True)
        if not options:
            failures.append(("reference-nested", "picker opened with no literal options"))
        else:
            needle = "SW13" if any("SW13" in o for o in options) else options[0]
            browser("keyboard", "type", needle)
            browser("wait", 300)
            filtered = evaluate("[...document.querySelectorAll(\"#m1-layout-align-menu [role='option']\")].map(o=>o.textContent.trim())")
            print(f"REFERENCE-NESTED-FILTERED: {json.dumps(filtered)}", flush=True)
            if not filtered:
                failures.append(("reference-nested", f"filter {needle!r} matched nothing of {options}"))
            else:
                browser("press", "ArrowDown")
                browser("wait", 200)
                chosen = evaluate("(()=>{const m=document.querySelector('#m1-layout-align-menu');const inp=m.querySelector('input');const id=inp.getAttribute('aria-activedescendant');const o=id?document.getElementById(id):null;return o?o.textContent.trim():null;})()")
                print(f"REFERENCE-NESTED-CHOSEN: {json.dumps(chosen)}", flush=True)
                if not chosen:
                    failures.append(("reference-nested", "no active descendant after ArrowDown"))
                else:
                    browser("press", "Enter")
                    browser("wait", "--fn", "!document.querySelector(\"#m1-layout-align-menu [role='listbox']\")")
                    value = evaluate("document.querySelector('#m1-layout-align-menu input').value")
                    print(f"REFERENCE-NESTED-ACCEPTED: {json.dumps(value)}", flush=True)
                    if value != chosen:
                        failures.append(("reference-nested", f"accepted {value!r} != chosen literal {chosen!r}"))
                    ref_after = reference_detail_report()
                    if not (ref_after.get('present') and ref_after.get('state') == 'open'):
                        failures.append(("reference-nested", f"detail did not stay open after choice: {ref_after}"))
                    else:
                        check_reference_geometry("after-choice", failures)
                    # Escape ladder: first Escape closes the picker only.
                    browser("press", "ArrowDown")
                    browser("wait", "--fn", "!!document.querySelector(\"#m1-layout-align-menu [role='listbox']\")")
                    browser("press", "Escape")
                    for _ in range(15):
                        if evaluate("document.querySelector(\"#m1-layout-align-menu [role='listbox']\")") is None:
                            break
                        browser("wait", 200)
                    ref_ladder = reference_detail_report()
                    focus_ladder = active_focus_state()
                    print(f"REFERENCE-LADDER-1: {json.dumps(ref_ladder)} focus {json.dumps(focus_ladder)}", flush=True)
                    if not (ref_ladder.get('present') and ref_ladder.get('state') == 'open'):
                        failures.append(("reference-nested", f"first Escape closed the detail, expected picker-only: {ref_ladder}"))
                    if focus_ladder.get('tag') != 'INPUT' or focus_ladder.get('aria') != 'Alignment reference':
                        failures.append(("reference-nested", f"focus not on the picker input after first Escape: {focus_ladder}"))
                    # Second Escape closes the detail and restores Reference.
                    browser("press", "Escape")
                    for _ in range(15):
                        ref_ladder2 = reference_detail_report()
                        if ref_ladder2.get('focus') == 'Alignment reference' and not (ref_ladder2.get('present') and ref_ladder2.get('state') == 'open'):
                            break
                        browser("wait", 200)
                    ref_ladder2 = reference_detail_report()
                    focus_ladder2 = active_focus_state()
                    print(f"REFERENCE-LADDER-2: {json.dumps(ref_ladder2)} focus {json.dumps(focus_ladder2)}", flush=True)
                    if ref_ladder2.get('present') and ref_ladder2.get('state') == 'open':
                        failures.append(("reference-nested", "second Escape left the detail open"))
                    if focus_ladder2.get('tag') != 'BUTTON' or focus_ladder2.get('aria') != 'Alignment reference':
                        failures.append(("reference-nested", f"second Escape did not restore to Reference: {focus_ladder2}"))
    # The ladder ends with the detail closed; reopen for the Close mechanics below.
    browser("find", "role", "button", "click", "--name", "Alignment reference", "--exact")
    browser("wait", "--fn", "document.querySelector(\"#m1-layout-align-menu[data-state='open']\")!==null")
    browser("wait", 300)
    # Close via keyboard activation from wherever the reopen left focus: Tab
    # until Close (auto-focus may land on the combobox once a value is
    # accepted). Enter avoids the harness click-coverage check that misfires
    # inside the fixed dialog (Close hit-testability is proven by oracle).
    for _ in range(6):
        close_focus = active_focus_state()
        if close_focus.get('tag') == 'BUTTON' and close_focus.get('aria') == 'Close Alignment reference':
            break
        browser("press", "Shift+Tab")
    close_focus = active_focus_state()
    print(f"REFERENCE-CLOSE-FOCUS: {json.dumps(close_focus)}", flush=True)
    if close_focus.get('tag') != 'BUTTON' or close_focus.get('aria') != 'Close Alignment reference':
        failures.append(("reference-close", f"focus not on Close before keyboard activation: {close_focus}"))
        evaluate("([...document.querySelector('#m1-layout-align-menu').querySelectorAll('button')].find(e=>e.getAttribute('aria-label')==='Close Alignment reference')).click()")
    else:
        browser("press", "Enter")
    for _ in range(15):
        ref_probe = reference_detail_report()
        if not (ref_probe.get('present') and ref_probe.get('state') == 'open'):
            break
        browser("wait", 200)
    ref = reference_detail_report()
    print(f"REFERENCE-CLOSE: {json.dumps(ref)}", flush=True)
    if ref.get('present') and ref.get('state') == 'open':
        failures.append(("reference-close", "detail still open after Close"))
    if ref.get('focus') != 'Alignment reference':
        failures.append(("reference-close", f"focus not on Reference after Close: {ref.get('focus')}"))
    browser("find", "role", "button", "click", "--name", "Alignment reference", "--exact")
    browser("wait", "--fn", "document.querySelector(\"#m1-layout-align-menu[data-state='open']\")!==null")
    browser("wait", 300)
    browser("press", "Escape")
    for _ in range(15):
        ref = reference_detail_report()
        if ref.get('focus') == 'Alignment reference' and not (ref.get('present') and ref.get('state') == 'open'):
            break
        browser("wait", 200)
    ref = reference_detail_report()
    print(f"REFERENCE-ESCAPE: {json.dumps(ref)}", flush=True)
    if ref.get('present') and ref.get('state') == 'open':
        failures.append(("reference-escape", "detail still open after Escape"))
    if ref.get('focus') != 'Alignment reference':
        failures.append(("reference-escape", f"Escape did not restore to Reference: {ref.get('focus')}"))
    browser("find", "role", "button", "click", "--name", "Alignment reference", "--exact")
    browser("wait", "--fn", "document.querySelector(\"#m1-layout-align-menu[data-state='open']\")!==null")
    browser("wait", 300)
    browser("find", "role", "button", "click", "--name", "Snap", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='snap'][data-open='true']\")!==null")
    wait_for_settled()
    ref = reference_detail_report()
    focus = active_focus_state()
    print(f"REFERENCE-PARENT-CLOSE: {json.dumps(ref)} focus {json.dumps(focus)}", flush=True)
    if ref.get('present') and ref.get('state') == 'open':
        failures.append(("reference-parent-close", "detail survived its section close"))
    if not focus['visible'] or focus.get('aria') != 'Snap':
        failures.append(("reference-parent-close", f"focus not visibly on Snap after parent close: {focus}"))
    browser("find", "role", "button", "click", "--name", "Align", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='align'][data-open='true']\")!==null")
    wait_for_settled()
    ref = reference_detail_report()
    if ref.get('present') and ref.get('state') == 'open':
        failures.append(("reference-parent-close", "detail reopened on section return"))

    check_page_overflow("overlays", failures)
    check_browser_errors(failures)
    print(f"FOLLOWUPS ({len(followups)}): {json.dumps(followups, indent=2)}", flush=True)
    print(f"FAILURES ({len(failures)}): {json.dumps(failures, indent=2)}", flush=True)
    assert not failures, failures
    print(f"PASS: overlay matrix at real {width}x{height} viewport (Layers, tooltips, Reference mechanics)", flush=True)


def content_response_checks(url, width=820, height=900):
    # Slice 4 row B: at fixed W the row responds to font swap and to
    # selection/caption change without viewport resize, stale callbacks or
    # density breakage. Container queries own density; no fit key exists.
    open_fresh_layout(url, width, height)
    failures = []
    check_closed_density("content-initial", failures)
    before = closed_density_report()
    print(f"CONTENT-INITIAL W{before['W']}: {json.dumps(before)}", flush=True)
    if abs(before["W"] - 800.0) >= 1.0:
        failures.append(("content-initial", f"expected fixed W800 at 820px viewport, got {before['W']}"))

    # ---- Font swap at fixed W: fallback stack, then back to the Theme stack ----
    for tag, family in (("fallback", "monospace"), ("restored", "")):
        swapped = evaluate(
            "(()=>{const row=document.querySelector('.workbench-tool-row');" +
            f"row.style.fontFamily={json.dumps(family)};" +
            "return row.getBoundingClientRect().width;})()"
        )
        wait_for_settled()
        rep = closed_density_report()
        print(f"CONTENT-FONT-{tag}: rowW {swapped} {json.dumps(rep)}", flush=True)
        if abs(rep["W"] - before["W"]) >= 0.5:
            failures.append((f"content-font-{tag}", f"W moved {before['W']} -> {rep['W']}"))
        check_closed_density(f"content-font-{tag}", failures)
        check_entry_reports(f"content-font-{tag}", ENTRIES, failures, width, height)
    if evaluate("document.querySelector('.workbench-tool-row').style.fontFamily") != "":
        failures.append(("content-font", "row font override not restored"))

    # ---- Selection/caption change at fixed W: Key -> Row scope ----
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='select'][data-open='true']\")!==null")
    wait_for_settled()
    browser("find", "role", "button", "click", "--name", "Select Row", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='select'] [aria-label='Select Row'][aria-pressed='true']\")!==null")
    wait_for_settled()
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    browser("wait", "--fn", "document.querySelector(\"[data-workbench-section='select'][data-open='false']\")!==null")
    wait_for_settled()
    rep = closed_density_report()
    print(f"CONTENT-CAPTION: {json.dumps(rep)}", flush=True)
    if abs(rep["W"] - before["W"]) >= 0.5:
        failures.append(("content-caption", f"W moved {before['W']} -> {rep['W']}"))
    caption = evaluate("(()=>{const c=document.querySelector(\"[data-workbench-section='select'] .workbench-tool-caption\");" +
        "return c ? {text:(c.textContent||'').trim(),display:getComputedStyle(c).display} : null;})()")
    print(f"CONTENT-CAPTION-TEXT: {json.dumps(caption)}", flush=True)
    if not caption or caption.get('text') != 'Row' or caption.get('display') == 'none':
        failures.append(("content-caption", f"Select caption did not become shown 'Row': {caption}"))
    check_closed_density("content-caption", failures)
    check_entry_reports("content-caption", ENTRIES, failures, width, height)
    check_page_overflow("content", failures)
    check_browser_errors(failures)
    print(f"FAILURES ({len(failures)}): {json.dumps(failures, indent=2)}", flush=True)
    assert not failures, failures
    print(f"PASS: content response at fixed W (font swap + caption change) at real {width}x{height} viewport", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page-url", required=True, help="packaged production page URL (no fixture mode)")
    parser.add_argument("--width", type=int, default=320)
    parser.add_argument("--height", type=int, default=480)
    parser.add_argument("--setup-matrix", action="store_true",
                        help="Slice 3 B2: create a matrix via the setup guide before Hide guide (enabled Transform/Align)")
    parser.add_argument("--scenario", choices=["page", "constrained", "assembly3d", "layers", "overlays", "content"], default="page",
                        help="page: full viewport proof (default, unchanged); constrained: parent-constrained ledger at 1440x900; assembly3d: 3D branch checks at --width/--height; layers: scoped Layers placement at --width/--height; overlays: Slice 4 overlay matrix at --width/--height; content: Slice 4 font/selection response at fixed W")
    args = parser.parse_args()
    try:
        if args.scenario == "constrained":
            constrained_parent_ledger(args.page_url)
        elif args.scenario == "assembly3d":
            assembly_3d_checks(args.page_url, args.width, args.height)
        elif args.scenario == "layers":
            layers_placement_checks(args.page_url, args.width, args.height)
        elif args.scenario == "overlays":
            overlay_surface_checks(args.page_url, args.width, args.height)
        elif args.scenario == "content":
            content_response_checks(args.page_url, args.width, args.height)
        else:
            production_viewport_tests(args.page_url, args.width, args.height, setup_matrix=args.setup_matrix)
    finally:
        browser("close")


if __name__ == "__main__":
    main()

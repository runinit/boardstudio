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
SECTION_KEYS = {"Select": "select", "Transform": "transform", "Align": "align", "Snap": "snap", "View": "view"}
MIN_HIT = 32.0
MIN_GAP = 4.0


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
        "try{track.scrollLeft=origScroll;}catch(_){}"
        "return {before,after,scrolled,strategy,usable:!!usable,usableDetail:usable};"
        "})()"
    )


def select_label_report():
    # Select label containment oracle: when fitter says full, icon+label must fit inside button.
    # Checks rendered bounds fully contained, non-overlap, correct icon/text. No false GREEN on data flag alone.
    # When adaptive icon-only (label display none), verifies icon within 32x32 button.
    return evaluate(
        "(()=>{"
        "const g=document.querySelector('[data-workbench-section=\"select\"]');"
        "if(!g) return {missing:true};"
        "const mode=g.getAttribute('data-action-labels');"
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
        "return {aria:(b.getAttribute('aria-label')||'').slice(0,40),x:br.x,y:br.y,w:br.width,h:br.height,showLabel:b.hasAttribute('data-show-label'),icon,label,overlap};"
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
        "if(mode==='full'){"
        "if(!it.label){ labelsFit=false; reasons.push(it.aria+' missing label element in full mode'); }"
        "else{"
        "if(it.label.display==='none'){ labelsFit=false; reasons.push(it.aria+' label hidden in full mode'); }"
        "if(!(it.label.w>0&&it.label.h>0)){ labelsFit=false; reasons.push(it.aria+' label not visible '+it.label.w+'x'+it.label.h); }"
        "if(it.label.text!==expName){ labelsFit=false; reasons.push(it.aria+' label text '+it.label.text+' != '+expName); }"
        "if(!it.label.within){ labelsFit=false; reasons.push(it.aria+' label overflows '+it.label.w+'px in '+it.w+'px button'); }"
        "if(it.overlap){ labelsFit=false; reasons.push(it.aria+' icon/label overlap'); }"
        "}"
        "}else{"
        "if(it.label&&it.label.display!=='none'){"
        "if(!it.label.within){ labelsFit=false; reasons.push(it.aria+' label overflows '+it.label.w+'px in '+it.w+'px button'); }"
        "if(it.overlap){ labelsFit=false; reasons.push(it.aria+' icon/label overlap'); }"
        "}"
        "}"
        "}"
        "const texts=items.map(i=>i.label?i.label.text:null);"
        "return {mode,items,labelsFit,reasons,expected,texts};"
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
    assert all(g['transform'] in ('none','matrix(1, 0, 0, 1, 0, 0)') for g in opened)
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    wait_for_settled()
    browser("set", "media", "light", "reduced-motion")
    assert evaluate("matchMedia('(prefers-reduced-motion: reduce)').matches")
    evaluate("""(() => {
      window.__reduced=[];
      const row=document.querySelector('.workbench-tool-row');
      window.__reducedObserver=new MutationObserver(()=>{
        window.__reduced.push({open:row.querySelector('[data-open=true]')?.dataset.workbenchSection,
          running:document.getAnimations().filter(a=>a.playState==='running'&&(row.contains(a.effect?.target)||a.effect?.target?.closest('.workbench-popout'))).length,
          transform:getComputedStyle(row.querySelector('[data-open=true] .workbench-tool-track')).transform});
      }); window.__reducedObserver.observe(row,{subtree:true,attributes:true});
    })()""")
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    reduced=evaluate("(() => {window.__reducedObserver.disconnect();return window.__reduced;})()")
    print(f"REDUCED-MOTION: {json.dumps(reduced)}", flush=True)
    assert reduced and all(r['open']=='select' and r['running']==0 and r['transform']=='none' for r in reduced), reduced
    browser("find", "role", "button", "click", "--name", "Select", "--exact")
    browser("set", "media", "light")
    wait_for_settled()


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
        evaluate("(()=>{ document.querySelectorAll('[data-slot=tooltip-trigger], .layout-selection-guidance').forEach(t=>{ try{ if(document.activeElement===t) t.blur(); t.dispatchEvent(new MouseEvent('mouseleave',{bubbles:true})); }catch(_){} }); })()")
        browser("wait", "--fn", "document.querySelectorAll('[data-slot=tooltip][data-state=open]').length===0")
        if sec == "Snap" and width == 320:
            # Trace demonstrated Geometry snap is a roving action; Snap increment
            # is a secondary Select trigger and is not reached by toolbar arrows.
            browser("press", "Shift+Tab")
            browser("press", "Tab")
            initial = evaluate("({label:document.activeElement.getAttribute('aria-label'),tag:document.activeElement.tagName})")
            print(f"KEYBOARD-ENTRY: {json.dumps(initial)}", flush=True)
            assert initial['label'] == 'Snap', initial
            # Reach an initially clipped action with real toolbar keys.
            before = evaluate("(() => { const g=document.querySelector('[data-workbench-section=\"snap\"]'); const t=g.querySelector('.workbench-tool-track'); const r=t.getBoundingClientRect(); const a=[...t.querySelectorAll('button:not(:disabled)')].find(e=>e.getAttribute('aria-label')==='Geometry snap' && e.getBoundingClientRect().right>r.right+1); return {label:a?.getAttribute('aria-label'), scroll:t.scrollLeft, bank:[...document.querySelectorAll('[data-workbench-section] > button, .workbench-utility')].map(e=>e.getBoundingClientRect().x)}; })()")
            assert before.get('label'), f"no initially offscreen enabled Snap action: {before}"
            reached = None
            for _ in range(20):
                browser("press", "ArrowRight")
                reached = evaluate("(() => {const a=document.activeElement, t=document.querySelector('[data-workbench-section=\"snap\"] .workbench-tool-track'), r=t.getBoundingClientRect(), b=a.getBoundingClientRect(), hit=document.elementFromPoint(b.x+b.width/2,b.y+b.height/2); return {label:a.getAttribute('aria-label'), scroll:t.scrollLeft, visible:b.left>=r.left-0.01 && b.right<=r.right+0.01 && b.top>=r.top-0.01 && b.bottom<=r.bottom+0.01, hit:hit===a || a.contains(hit), bank:[...document.querySelectorAll('[data-workbench-section] > button, .workbench-utility')].map(e=>e.getBoundingClientRect().x)};})()")
                print(f"KEYBOARD-TRACE {_}: {json.dumps(reached)}", flush=True)
                if reached['label'] == before['label']:
                    break
            print(f"KEYBOARD-SCROLL: {json.dumps({'before': before, 'after': reached})}", flush=True)
            assert reached['label'] == before['label'] and reached['scroll'] > before['scroll'] and reached['visible'] and reached['hit'] and reached['bank'] == before['bank'], f"keyboard scroll failed: {before}, {reached}"
            port = action_lane_report(key)['rev']
            browser('mouse', 'move', round(port['x'] + port['w']/2), round(port['y'] + port['h']/2))
            browser('mouse', 'wheel', 0, -100)
            browser('wait', 120)
            wheel = evaluate("(() => {const t=document.querySelector('[data-workbench-section=snap] .workbench-tool-track'); return {scroll:t.scrollLeft,bank:[...document.querySelectorAll('[data-workbench-section] > button, .workbench-utility')].map(e=>e.getBoundingClientRect().x)};})()")
            print(f"NATIVE-WHEEL: {json.dumps({'before': reached, 'after': wheel})}", flush=True)
            if wheel['scroll'] >= reached['scroll'] or wheel['bank'] != before['bank']:
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
            if sec == "Select":
                sel = select_label_report()
                print(f"SELECT-LABELS open-{sec}: {json.dumps(sel)}", flush=True)
                if not sel.get("labelsFit"):
                    failures.append((f"select-labels-{sec}", f"label/icon containment failed (mode {sel.get('mode')}): {sel.get('reasons')}, full {sel}"))
            if sec == "View":
                icons = view_icon_report()
                print(f"VIEW-ICONS open-{sec}: {json.dumps(icons)}", flush=True)
                if icons.get("small"):
                    failures.append((f"view-icon-{sec}", f"icon hit regions <32x32 {icons['small']}, full {icons}"))
                canvas = canvas_visibility_report()
                print(f"CANVAS open-{sec}: {json.dumps(canvas)}", flush=True)
                if not canvas.get("usable"):
                    failures.append((f"canvas-{sec}", f"Canvas visibility not reachable as whole >=32x32 via lane scroll (fully within all clippers+viewport, hit, enabled), full {canvas}"))
                handles = handles_visibility_report()
                print(f"HANDLES open-{sec}: {json.dumps(handles)}", flush=True)
                if not handles.get("usable"):
                    failures.append((f"handles-{sec}", f"Handles not reachable as whole >=32x32 via lane scroll (fully within all clippers+viewport, hit, enabled), full {handles}"))

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
    print(f"PASS: real {width}x{height} viewport, 42px row, seven entry true hit tests closed+each-open, 4px/32px/no-overflow, lanes, Objects/Inspect panels", flush=True)


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

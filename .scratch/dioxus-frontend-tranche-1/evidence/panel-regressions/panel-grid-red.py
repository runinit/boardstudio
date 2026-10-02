#!/usr/bin/env python3
import subprocess,json,sys,time
from pathlib import Path
session=sys.argv[2] if len(sys.argv)>2 else 'frontend-panel-grid-fixer'
url=sys.argv[1] if len(sys.argv)>1 else 'http://127.0.0.1:34647/'
out=Path(sys.argv[3] if len(sys.argv)>3 else '/tmp/frontend-run/panel-grid-red.json')
def ab(*args):
 p=subprocess.run(['agent-browser','--session',session,*args],capture_output=True,text=True)
 if p.returncode: raise RuntimeError(p.stdout+p.stderr)
 return p.stdout.strip()
def ev(s):return json.loads(ab('eval',s))
ab('open',url);ab('wait','500')
if not ev('!!document.querySelector(".m1-editor-body")'):
 ab('find','role','button','click','--name','REVIUNG41 copy','--exact');ab('wait','1000')
ev('localStorage.setItem("boardstudio:v2:panel:left",JSON.stringify({mode:"pinned",width:320}));localStorage.setItem("boardstudio:v2:panel:right",JSON.stringify({mode:"pinned",width:360}));true')
ab('reload');ab('wait','800')
states=[]
for width,height in [(1280,650),(1000,650),(1280,500)]:
 ab('set','viewport',str(width),str(height));ab('wait','100')
 states.append(ev('(()=>{const b=document.querySelector(".m1-editor-body");const g=getComputedStyle(b);return {viewport:[innerWidth,innerHeight],grid:g.gridTemplateColumns,inline:b.getAttribute("style"),leftMode:document.querySelector("#m1-objects-panel").dataset.mode,rightMode:document.querySelector("#m1-inspector-panel").dataset.mode}})()'))
failed=[s for s in states if abs(float(s['grid'].split()[0][:-2])-320)>1 or abs(float(s['grid'].split()[2][:-2])-360)>1]
result={'url':url,'states':states,'expected':'Pinned stored widths 320/360 within reference caps survive narrow-desktop and short-height media rules','pass':not failed}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
if failed:print('FAIL: responsive media rules override valid panel widths',file=sys.stderr);sys.exit(1)

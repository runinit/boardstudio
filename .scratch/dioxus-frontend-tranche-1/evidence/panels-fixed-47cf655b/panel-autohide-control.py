#!/usr/bin/env python3
"""Public compact->desktop auto-hide oracle; no DOM/handler instrumentation."""
import subprocess,json,sys,time
from pathlib import Path
url=sys.argv[1]
session=sys.argv[2]
out=Path(sys.argv[3])
def ab(*args):
 p=subprocess.run(['agent-browser','--session',session,*args],capture_output=True,text=True)
 if p.returncode:raise RuntimeError(p.stdout+p.stderr)
 return p.stdout.strip()
def ev(s):return json.loads(ab('eval',s))
def state():return ev('(()=>{const s=document.querySelector("#m1-objects-panel"),c=document.querySelector("#m1-objects-panel-content");return {viewport:[innerWidth,innerHeight],mode:s.dataset.mode,revealed:s.dataset.revealed,shellInert:s.inert,contentInert:c.inert,activeId:document.activeElement.id,activeTag:document.activeElement.tagName,footer:document.querySelector(".m1-editor-footer").textContent}})()')
ab('open',url);ab('wait','500')
if not ev('!!document.querySelector(".m1-editor-body")'):
 ab('find','role','button','click','--name','REVIUNG41 copy','--exact');ab('wait','1000')
ab('set','viewport','1280','650')
compact=state()
ab('set','viewport','1280','650');ab('wait','150')
desktop=state()
if desktop['shellInert'] or desktop['contentInert']:raise AssertionError('Prerequisite panel visibility/inert fix missing')
ab('find','role','button','click','--name','Objects options','--exact')
ab('find','role','button','click','--name','Auto-hide objects','--exact')
revealed=state()
ab('find','role','tab','click','--name','Layout','--exact')
ab('wait','450')
idle=state()
result={'url':url,'compact':compact,'desktop':desktop,'after_autohide':revealed,'after_idle':idle,'expected':'Desktop autohide closes after 280ms with pointer and focus outside regardless of prior compact-open state','pass':idle['mode']=='autohide' and idle['revealed']=='false' and idle['contentInert'] and not idle['shellInert']}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
if not result['pass']:print('FAIL: desktop auto-hide remains revealed after idle following compact open',file=sys.stderr);sys.exit(1)
print('PASS: desktop auto-hide closes after compact-to-desktop transition')

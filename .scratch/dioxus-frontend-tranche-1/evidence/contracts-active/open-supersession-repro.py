#!/usr/bin/env python3
"""Isolated public UI + real Runtime/Session/worker/provider regression oracle.
No host-source changes. Worker transport gate holds one open AFTER Session submits.
"""
import json, subprocess, time, sys
from pathlib import Path
SESSION = "frontend-contract-race"
BASE = "http://127.0.0.1:34645/"
CONTROL = "--no-B-control" in sys.argv
HOLD_REPLY = "--hold-reply" in sys.argv
OUT = Path("/tmp/frontend-run/open-supersession-control.json" if CONTROL else ("/tmp/frontend-run/open-supersession-reply-result.json" if HOLD_REPLY else "/tmp/frontend-run/open-supersession-result.json"))
def ab(*args):
    p = subprocess.run(["agent-browser", "--session", SESSION, *args], capture_output=True, text=True)
    if p.returncode: raise RuntimeError(p.stderr + p.stdout)
    return p.stdout.strip()
def ev(code):
    return json.loads(ab("eval", code))
def wait(code):
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        result = ev(code)
        if result: return result
        time.sleep(.08)
    raise AssertionError("Timed out: " + code)
def state():
    return ev('({name:document.querySelector(".m1-project-menu summary")?.textContent,footer:document.querySelector(".m1-editor-footer")?.textContent,active:localStorage.getItem("boardstudio-m1-active-project:boardstudio-m1-root"),status:document.body.innerText.slice(-220),trace:window.__openRace?.trace})')
ab("open", BASE)
wait('!!document.querySelector(".m1-library")')
if not ev('!!document.querySelector(".m1-project-menu summary")'):
    ab("find", "role", "button", "click", "--name", "REVIUNG41 copy", "--exact")
    wait('document.querySelector(".m1-editor-footer")?.textContent.includes("Saved")')
seed = ev("""(async()=>{
 const db=await new Promise((resolve,reject)=>{const r=indexedDB.open('boardstudio-m1-root');r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error)});
 const docs=await new Promise((resolve,reject)=>{const r=db.transaction('projects').objectStore('projects').getAll();r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error)});
 const p=docs.find(d=>d.name==='REVIUNG41');if(!p)throw Error('baseline fixture missing');
 await new Promise((resolve,reject)=>{const tx=db.transaction('projects','readwrite');for(const id of ['race-a','race-b'])tx.objectStore('projects').put({...structuredClone(p),id,name:id==='race-a'?'Race A':'Race B'});tx.oncomplete=resolve;tx.onabort=()=>reject(tx.error)});
 localStorage.setItem('boardstudio-m1-active-project:boardstudio-m1-root',p.id);db.close();return {id:p.id,name:p.name,revision:p.revision};
})()""")
ab("reload")
wait('document.querySelector(".m1-project-menu summary")?.textContent === "REVIUNG41" && document.querySelector(".m1-editor-footer")?.textContent.includes("Saved")')
wait('Array.from(document.querySelectorAll(".m1-library button")).some(b=>b.textContent==="Race B")')
before=state()
ev("window.__holdReply=" + ("true" if HOLD_REPLY else "false"))
ev("""(async()=>{
 const db=await new Promise((resolve,reject)=>{const r=indexedDB.open('boardstudio-m1-root');r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error)});
 await new Promise((resolve,reject)=>{const tx=db.transaction('projects','readwrite');tx.objectStore('projects').delete('race-b');tx.oncomplete=resolve;tx.onabort=()=>reject(tx.error)});db.close();
 const original=Worker.prototype.postMessage;
 window.__openRace={trace:[],held:null,release(){const h=this.held;this.held=null;this.trace.push({event:'release-A'});if(h.resume)h.resume();else original.apply(h.worker,h.args)}};
 const originalGet=IDBObjectStore.prototype.get;
 IDBObjectStore.prototype.get=function(key){if(this.name==='projects')window.__openRace.trace.push({event:'provider-get',id:key});return originalGet.call(this,key)};
 Worker.prototype.postMessage=function(...args){const m=args[0];let frame;try{frame=JSON.parse(m?.frame)}catch{};
 if(m?.kind==='core' && frame?.kind==='open' && frame.document?.id==='race-a' && !window.__openRace.held){if(window.__holdReply){const worker=this;const handler=worker.onmessage;worker.onmessage=function(e){if(e.data?.request_id!==m.request_id){handler.call(worker,e);return}worker.onmessage=handler;window.__openRace.held={resume:()=>handler.call(worker,e)};window.__openRace.trace.push({event:'hold-A-reply-after-core-executed',requestId:m.request_id,documentId:frame.document.id})};return original.apply(this,args)}
 window.__openRace.held={worker:this,args};window.__openRace.trace.push({event:'hold-A-after-session-submit',requestId:m.request_id,documentId:frame.document.id});return;}
 return original.apply(this,args)};return true;
})()""")
ab("click", ".m1-project-menu summary")
ab("find", "role", "button", "click", "--name", "Race A", "--exact")
wait('!!window.__openRace.held')
a_pending=state()
if not CONTROL:
    ab("click", ".m1-project-menu summary")
    ab("find", "role", "button", "click", "--name", "Race B", "--exact")
    wait('document.body.innerText.includes("The saved keyboard is unavailable")')
b_failed=state()
ev('window.__openRace.release(); true')
wait('!window.__openRace.held && document.querySelector(".m1-editor-footer")?.textContent.includes("Saved")')
# Wait for durable active identity/visible adoption (bounded polling, no arbitrary fixed race sleep).
deadline=time.monotonic()+3
while time.monotonic()<deadline:
    after=state()
    if after['name']!='REVIUNG41': break
    time.sleep(.05)
result={'source':'598b2c026941130f9b95ff4fee57353f8eefcb0d (coordinator supplied artifact)','url':BASE,'session':SESSION,'seed':seed,'before':before,'A_pending':a_pending,'B_unavailable':b_failed,'after_A_release':after,'expected':('A opens successfully when no B intent supersedes it' if CONTROL else 'REVIUNG41 remains current after B supersedes A then fails unavailable'),'control':CONTROL,'hold_reply':HOLD_REPLY,'pass':(after['name']=='Race A' and after['active']=='race-a') if CONTROL else (after['name']=='REVIUNG41' and after['active']==seed['id'])}
OUT.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
if not result['pass']:
    print('FAIL: superseded A adopted after newer B was unavailable',file=sys.stderr)
    sys.exit(1)
print('PASS: no-B control opens A normally' if CONTROL else 'PASS: latest intent preserved previous current project')

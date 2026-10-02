#!/usr/bin/env python3
import hashlib,json,subprocess,sys
from pathlib import Path
url,session,output=sys.argv[1:]
fixture=Path('/tmp/frontend-run/tree-hierarchy-fix/unowned-component.boardstudio')
def ab(*args):
 p=subprocess.run(['agent-browser','--session',session,*args],capture_output=True,text=True)
 if p.returncode:raise RuntimeError(p.stdout+p.stderr)
 return p.stdout.strip()
def ev(js):
 x=json.loads(ab('eval',js));return json.loads(x) if isinstance(x,str) else x
ab('open',url);ab('wait','250');ab('upload','input[type=file]',str(fixture));ab('wait','1000')
def state():return ev("[...document.querySelectorAll('[role=treeitem]')].map(e=>({text:e.innerText.replace(/\\n/g,' '),level:Number(e.getAttribute('aria-level')),expanded:e.getAttribute('aria-expanded')}))")
items=state()
root=next(i for i in items if i['level']==1)
if root['expanded']!='true':ab('find','role','button','click','--name','Expand Keyboard PCB','--exact')
items=state()
layout=next(i for i in items if i['level']==2 and i['text']=='Layout')
if layout['expanded']!='true':ab('find','role','button','click','--name','Expand Layout','--exact')
ab('wait','100');items=state()
record=ev('''(async()=>{const key=Object.keys(localStorage).find(k=>k==='boardstudio-v2-active-project'||k.startsWith('boardstudio-m1-active-project:'));const id=localStorage.getItem(key),names=await indexedDB.databases(),q=indexedDB.open(names[0].name);const db=await new Promise((r,j)=>{q.onsuccess=()=>r(q.result);q.onerror=()=>j(q.error)});const tx=db.transaction('projects','readonly'),get=tx.objectStore('projects').get(id);const data=await new Promise((r,j)=>{get.onsuccess=()=>r(get.result);get.onerror=()=>j(get.error)});db.close();return JSON.stringify(data)})()''')
matches=[i for i in items if i['text'].startswith('main-RST ') and i['level']==3]
result={'url':url,'fixture_sha256':hashlib.sha256(fixture.read_bytes()).hexdigest(),'record':record,'items':items,'pass':len(matches)==1}
Path(output).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'pass':result['pass'],'items':items},indent=2))
if not result['pass']:raise SystemExit('RED: unowned standalone RST missing from expanded Layout alongside owned matrices')

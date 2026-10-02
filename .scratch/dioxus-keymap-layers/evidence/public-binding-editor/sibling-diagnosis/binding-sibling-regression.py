import subprocess,json,sys
session=sys.argv[1]
mode=sys.argv[2] if len(sys.argv)>2 else 'select'
def ab(*a):
 r=subprocess.run(['agent-browser','--session',session,*a],text=True,capture_output=True)
 if r.returncode: raise RuntimeError(r.stderr+r.stdout)
 return r.stdout.strip()
def ev(s):
 x=json.loads(ab('eval',s))
 return json.loads(x) if isinstance(x,str) else x
ab('find','role','button','click','--name','Edit key left-keys-SW7')
ab('select','select[aria-label="left-keys-SW7 behavior"]','Mod tap')
ab('wait','500')
ev('''(()=>{window.__bindingTrace=[]; if(window.__bindingProbe)window.__bindingProbe.abort();window.__bindingProbe=new AbortController();for(const type of ['focusin','focusout','input','change','blur'])document.addEventListener(type,e=>{if(e.target.closest?.('.m1-keymap-binding-editor'))window.__bindingTrace.push({type,label:e.target.getAttribute('aria-label'),value:e.target.value,active:document.activeElement?.getAttribute('aria-label'),at:performance.now()})},{capture:true,signal:window.__bindingProbe.signal});return JSON.stringify(true)})()''')
ab('fill','input[aria-label="left-keys-SW7 tap"]','Y')
if mode=='focus':
 ab('focus','select[aria-label="left-keys-SW7 hold modifier"]')
ab('select','select[aria-label="left-keys-SW7 hold modifier"]','LCTRL')
ab('wait','500')
result=ev('''(async()=>{const q=indexedDB.open('boardstudio-m1-root');const db=await new Promise((r,j)=>{q.onsuccess=()=>r(q.result);q.onerror=()=>j(q.error)});const g=db.transaction('projects','readonly').objectStore('projects').get(localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root'));const d=await new Promise((r,j)=>{g.onsuccess=()=>r(g.result);g.onerror=()=>j(g.error)});db.close();return JSON.stringify({keys:Object.keys(d),document:d,trace:window.__bindingTrace,draft:document.querySelector('input[aria-label="left-keys-SW7 tap"]')?.value,active:document.activeElement?.getAttribute('aria-label')})})()''')
d=result.pop('document')
if 'document' in d:d=d['document']
if isinstance(d,str):d=json.loads(d)
result['binding']=[l for l in d['keymap']['layers'] if l['name']=='Function'][0]['bindings']['matrix/left-keys/r1c0']
result['revision']=d['revision']
print(json.dumps(result,indent=2))
assert result['binding'].get('tap')=='Y', 'RED: persisted sibling tap was lost'
print('PASS: tap retained')

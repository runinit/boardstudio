import json,subprocess,sys,pathlib
session=sys.argv[1];url=sys.argv[2];out=pathlib.Path(sys.argv[3]);out.mkdir(parents=True,exist_ok=True)
steps=[]
def run(*args):
 p=subprocess.run(['agent-browser','--session',session,*args,'--json'],text=True,capture_output=True)
 try:r=json.loads(p.stdout)
 except ValueError:r={'raw':p.stdout,'error':p.stderr}
 steps.append({'args':list(args),'exit':p.returncode,'result':r});(out/'browser-steps.json').write_text(json.dumps(steps,indent=2))
 if p.returncode or not r.get('success',True):raise RuntimeError(str(steps[-1]))
 return r.get('data',r)
def probe(name,expr):
 r=run('eval',f'JSON.stringify({expr})');v=r.get('result',r)
 if isinstance(v,str):v=json.loads(v)
 (out/(name+'.json')).write_text(json.dumps(v,indent=2));return v
def wait(expr):run('wait','--fn',expr)
run('set','viewport','390','844')
run('click','button[aria-controls="m1-objects-panel"]');run('click','button[aria-controls="m1-inspector-panel"]');run('click','#m1-object-0');run('scrollintoview','#m1-position-x')
base=probe('before','({x:Number(document.querySelector("#m1-position-x").value)})')['x']
run('fill','#m1-position-x',str(base+1));run('press','Enter');wait('document.querySelector(".m1-save-state").textContent.includes("Saved")')
run('scrollintoview','button[aria-label="Undo"]')
expr='({footer:(()=>{let x=document.querySelector(".m1-editor-footer").getBoundingClientRect();return {top:x.top,bottom:x.bottom}})(),status:(()=>{let x=document.querySelector(".m1-status").getBoundingClientRect();return {top:x.top,bottom:x.bottom}})(),undoHit:(()=>{let x=document.querySelector("button[aria-label=Undo]"),r=x.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return hit===x||x.contains(hit)})()})'
r=probe('expanded-panels-footer',expr);assert r['footer']['bottom']<=r['status']['top']+1 and r['undoHit'],r
run('scroll','up','2000');run('click','button[aria-controls="m1-objects-panel"]');run('click','button[aria-controls="m1-inspector-panel"]');run('scrollintoview','button[aria-label="Undo"]')
r=probe('closed-panels-footer',expr);assert r['footer']['bottom']<=r['status']['top']+1 and r['undoHit'],r
run('click','button[aria-label="Undo"]');run('click','button[aria-controls="m1-inspector-panel"]');wait('Number(document.querySelector("#m1-position-x").value)==='+str(base))
probe('undo-result','({x:Number(document.querySelector("#m1-position-x").value),saved:document.querySelector(".m1-save-state").textContent})')
run('click','button[aria-controls="m1-inspector-panel"]')
try:
 run('set','offline','on');run('reload');run('wait','.m1-canvas');probe('root-offline','({online:navigator.onLine,theme:document.documentElement.dataset.theme,worker:navigator.serviceWorker.controller.scriptURL})')
finally:run('set','offline','off')
run('set','viewport','1440','900');(out/'browser-errors.json').write_text(json.dumps(run('errors'),indent=2))
print('PASS compact visible history controls in both panel states, actual Undo, and final root offline')
